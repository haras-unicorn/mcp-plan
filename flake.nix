{
  description = "MCP server that provides planning tooling";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/release-26.05";

    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";

    crane.url = "github:ipetkov/crane";

    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-parts,
      crane,
      ...
    }@inputs:
    let
      makePackages =
        pkgs:
        let
          rust = (inputs.rust-overlay.lib.mkRustBin { } pkgs).stable.latest.default.override {
            extensions = [
              "rustfmt"
              "clippy"
              "rust-analyzer"
              "rust-src"
            ];
          };

          craneLib = crane.mkLib pkgs;

          cargoToml = builtins.fromTOML (builtins.readFile ./src/mcp-plan/Cargo.toml);

          # TODO: uncomment or remove when implemented

          # env = {
          #   PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          # };

          # nativeBuildInputs = with pkgs; [
          #   pkg-config
          #   openssl
          # ];

          commonArgs = {
            # inherit nativeBuildInputs env;
            pname = cargoToml.package.name;
            version = cargoToml.package.version;
            src = craneLib.cleanCargoSource self;
            strictDeps = true;
            cargoExtraArgs = "-p mcp-plan";
          };

          vendor = craneLib.vendorCargoDeps commonArgs;

          unwrapped = craneLib.buildPackage (
            commonArgs
            // {
              cargoArtifacts = craneLib.buildDepsOnly commonArgs;
              meta.mainProgram = "mcp-plan";
            }
          );
        in
        {
          inherit
            rust
            unwrapped
            vendor
            # nativeBuildInputs
            # env
            ;
          package =
            pkgs.callPackage
              (
                {
                  symlinkJoin,
                  mcp-plan-unwrapped,
                }:
                symlinkJoin {
                  name = "mcp-plan";
                  paths = [ mcp-plan-unwrapped ];
                  meta.mainProgram = "mcp-plan";
                }
              )
              {
                mcp-plan-unwrapped = unwrapped;
              };
        };
    in
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      flake.overlays =
        let
          overlay =
            final: prev:
            let
              packages = makePackages final;
            in
            {
              mcp-plan = packages.package;
              mcp-plan-unwrapped = packages.unwrapped;
            };
        in
        {
          default = overlay;
          mcp-plan = overlay;
        };

      perSystem =
        { pkgs, lib, ... }:
        let
          flake-root = pkgs.writeShellApplication {
            name = "flake-root";
            text = ''
              current="$PWD"
              while [[ "$current" != "/" ]]; do
                if [[ -f "$current/flake.nix" ]]; then
                  echo "$current"
                  exit 0
                fi
                current="$(dirname "$current")"
              done
              echo "no flake.nix found" >&2
              exit 1
            '';
          };

          external = with pkgs; [
            flake-root
            git
            nushell
            nil
            nixfmt
            markdownlint-cli
            marksman
            mdbook
            taplo
            fd
            delta
            cachix
            release-plz
            markdown-link-check
            cspell
            prettier
            vscode-langservers-extracted
            yaml-language-server
            cargo-edit
          ];

          devScriptText = pkgs.writeText "mcp-plan-dev.nu" ''
            def "main" [] {
              dev -h
            }

            def "main run" [] {
              cd (flake-root)
              cargo run --bin mcp-plan
            }

            def "main format" [] {
              cd (flake-root)
              prettier --write .
              nixfmt ...(fd '.*\.nix$' . | lines)
              cargo fmt --all
              cargo clippy --fix --allow-dirty
            }

            def "main test" [] {
              if ($env.NIX_BUILD_TOP? | is-empty) {
                cargo clippy -- -D warnings
                cargo test
              }
            }

            def "main lint" [] {
              cd (flake-root)
              prettier --check .
              cspell lint . --no-progress
              nixfmt --check ...(fd '.*\.nix$' . | lines)
              markdownlint --ignore-path .markdownignore .
              if ($env.NIX_BUILD_TOP? | is-empty) {
                (markdown-link-check
                  --config .markdown-link-check.json
                  --quiet
                  ...(fd '.*.md' . | lines))
                (taplo lint
                  --schema "https://raw.githubusercontent.com/release-plz/release-plz/refs/tags/release-plz-v0.3.148/.schema/latest.json"
                  .release-plz.toml)
                cargo clippy -- -D warnings
                cargo test
              }
            }
          '';

          packages = makePackages pkgs;

          devScript = pkgs.writeShellApplication {
            name = "dev";
            runtimeInputs = external ++ [ packages.rust ];
            text = ''nu ${devScriptText} "$@"'';
          };
        in
        {
          devShells = {
            default = pkgs.mkShell {
              # inherit (packages) nativeBuildInputs env;
              packages = external ++ [
                packages.rust
                devScript
              ];
              shellHook = ''
                mkdir -p .cargo
                ln -sf "${packages.vendor}/config.toml" .cargo/config.toml
              '';
            };
          };

          apps =
            let
              packages = makePackages pkgs;

              app = {
                type = "app";
                program = lib.getExe packages.package;
                meta.description = "MCP server that provides planning tooling";
              };
              unwrappedApp = {
                type = "app";
                program = lib.getExe packages.unwrapped;
                meta.description = "MCP server that provides planning tooling (unwrapped)";
              };
            in
            {
              default = app;
              mcp-plan = app;
              unwrapped = unwrappedApp;
              mcp-plan-unwrapped = unwrappedApp;
            };

          packages =
            let
              packages = makePackages pkgs;

              docs =
                pkgs.runCommand "mcp-plan-docs"
                  {
                    src = self;
                    nativeBuildInputs = [ pkgs.mdbook ];
                  }
                  ''
                    mdbook build -d "$out" "$src/docs"
                  '';
            in
            {
              inherit docs;
              default = packages.package;
              mcp-plan = packages.package;
              unwrapped = packages.unwrapped;
              mcp-plan-unwrapped = packages.unwrapped;
            };
        };
    };

  nixConfig = {
    extra-substituters = [
      "https://haras.cachix.org"
    ];
    extra-trusted-public-keys = [
      "haras.cachix.org-1:/HIo1JYqOIH1Nwk1EGXhuPPvDW0WekxIbY5CiXUZbYw="
    ];
  };
}
