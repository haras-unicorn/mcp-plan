{
  description = "MCP server that provides planning tooling";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-26.05";

    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixos-unstable";

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
      nixpkgs-unstable,
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

          src = craneLib.cleanCargoSource self;

          nativeBuildInputs = [ pkgs.pkg-config ];
          tlsBuildInputs = [ pkgs.openssl ];
          sqliteBuildInputs = [ pkgs.sqlite ];

          baseArgs = {
            inherit src;
            strictDeps = true;
            pname = cargoToml.package.name;
            version = cargoToml.package.version;
          };

          depArgs =
            {
              buildInputs ? [ ],
              nativeBuildInputs ? [ ],
            }:
            baseArgs
            // {
              inherit buildInputs nativeBuildInputs;
              cargoExtraArgs = "-p mcp-plan --all-features";
            };

          vendor = craneLib.vendorCargoDeps (depArgs {
            buildInputs = tlsBuildInputs ++ sqliteBuildInputs;
            nativeBuildInputs = nativeBuildInputs;
          });

          cargoArtifacts =
            {
              buildInputs ? [ ],
              nativeBuildInputs ? [ ],
            }:
            craneLib.buildDepsOnly (depArgs {
              inherit buildInputs nativeBuildInputs;
            });

          buildVariant =
            {
              name,
              cargoExtraArgs,
              buildInputs ? [ ],
              nativeBuildInputs ? [ ],
            }:
            craneLib.buildPackage (
              baseArgs
              // {
                pname = name;
                inherit
                  cargoExtraArgs
                  buildInputs
                  nativeBuildInputs
                  ;
                cargoArtifacts = cargoArtifacts { inherit buildInputs nativeBuildInputs; };
                meta.mainProgram = "mcp-plan";
              }
            );

          unwrapped-sqlite = buildVariant {
            name = "mcp-plan";
            cargoExtraArgs = "-p mcp-plan";
            buildInputs = sqliteBuildInputs;
            nativeBuildInputs = nativeBuildInputs;
          };

          unwrapped-postgres = buildVariant {
            name = "mcp-plan-postgres";
            cargoExtraArgs = "-p mcp-plan --no-default-features --features postgres";
            buildInputs = tlsBuildInputs;
            nativeBuildInputs = nativeBuildInputs;
          };

          unwrapped-mysql = buildVariant {
            name = "mcp-plan-mysql";
            cargoExtraArgs = "-p mcp-plan --no-default-features --features mysql";
            buildInputs = tlsBuildInputs;
            nativeBuildInputs = nativeBuildInputs;
          };

          symlink =
            name: pkg:
            pkgs.callPackage
              (
                {
                  symlinkJoin,
                  mcp-plan-unwrapped,
                }:
                symlinkJoin {
                  name = name;
                  paths = [ mcp-plan-unwrapped ];
                  meta.mainProgram = "mcp-plan";
                }
              )
              {
                mcp-plan-unwrapped = pkg;
              };
        in
        {
          inherit
            rust
            unwrapped-sqlite
            unwrapped-postgres
            unwrapped-mysql
            vendor
            ;
          unwrapped = unwrapped-sqlite;
          package = symlink "mcp-plan" unwrapped-sqlite;
          package-sqlite = symlink "mcp-plan-sqlite" unwrapped-sqlite;
          package-postgres = symlink "mcp-plan-postgres" unwrapped-postgres;
          package-mysql = symlink "mcp-plan-mysql" unwrapped-mysql;
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

          sea-orm-cli =
            (import nixpkgs-unstable {
              system = pkgs.stdenv.hostPlatform.system;
            }).sea-orm-cli;

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
            sea-orm-cli
            pkg-config
            openssl
            sqlite
          ];

          devScriptText = pkgs.writeText "mcp-plan-dev.nu" ''
            def "main" [] {
              dev -h
            }

            def "main run" [] {
              cd (flake-root)
              cargo run --bin mcp-plan
            }

            def "main schema" [] {
              cd (flake-root)
              cargo run --bin mcp-plan schema --output ./assets/config.schema.json
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
                cargo clippy --all-features -- -D warnings
                cargo test --all-features
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
                cargo clippy --all-features -- -D warnings
                cargo test --all-features
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

              app =
                backend: pkg:
                let
                  name = if backend == null then "mcp-plan" else "mcp-plan-${backend}";
                in
                {
                  type = "app";
                  program = lib.getExe pkg;
                  meta.description = "MCP server that provides planning tooling (${name})";
                };

              unwrappedApp = backend: pkg: {
                type = "app";
                program = lib.getExe pkg;
                meta.description = "MCP server that provides planning tooling (${backend})";
              };
            in
            {
              default = app null packages.package;

              mcp-plan = app null packages.package;
              mcp-plan-sqlite = app "sqlite" packages.package-sqlite;
              mcp-plan-postgres = app "postgres" packages.package-postgres;
              mcp-plan-mysql = app "mysql" packages.package-mysql;

              mcp-plan-unwrapped = unwrappedApp "unwrapped" packages.unwrapped;
              mcp-plan-unwrapped-sqlite = unwrappedApp "unwrapped-sqlite" packages.unwrapped-sqlite;
              mcp-plan-unwrapped-postgres = unwrappedApp "unwrapped-postgres" packages.unwrapped-postgres;
              mcp-plan-unwrapped-mysql = unwrappedApp "unwrapped-mysql" packages.unwrapped-mysql;
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
              mcp-plan-sqlite = packages.package-sqlite;
              mcp-plan-postgres = packages.package-postgres;
              mcp-plan-mysql = packages.package-mysql;

              mcp-plan-unwrapped = packages.unwrapped;
              mcp-plan-unwrapped-sqlite = packages.unwrapped-sqlite;
              mcp-plan-unwrapped-postgres = packages.unwrapped-postgres;
              mcp-plan-unwrapped-mysql = packages.unwrapped-mysql;
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
