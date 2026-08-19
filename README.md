# MCP Plan

<!-- ANCHOR: body -->

MCP server that provides planning tooling.

## Installation

`mcp-plan` is packaged as a Nix flake. Run it directly without installing:

```sh
nix run github:haras-unicorn/mcp-plan
```

or build the `mcp-plan` binary with:

```sh
nix build github:haras-unicorn/mcp-plan
```

### Releases

Prebuilt binaries for `x86_64-linux` and `aarch64-linux` are attached to each
[GitHub release] as tarballs containing the `mcp-plan` binary.

```sh
curl -L -o mcp-plan.tar.gz \
  https://github.com/haras-unicorn/mcp-plan/releases/latest/download/mcp-plan-x86_64-linux.tar.gz
tar -xzf mcp-plan.tar.gz
./mcp-plan-x86_64-linux
```

[GitHub release]: https://github.com/haras-unicorn/mcp-plan/releases

### NixOS and home-manager

Add the flake as an input and apply its overlay so that `mcp-plan` is available
in your system configuration:

```nix
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    mcp-plan.url = "github:haras-unicorn/mcp-plan";
  };

  outputs =
    { nixpkgs, mcp-plan, ... }:
    {
      nixosConfigurations.my-machine = nixpkgs.lib.nixosSystem {
        modules = [
          { nixpkgs.overlays = [ mcp-plan.overlays.default ]; }
        ];
      };
    };
}
```

Then add `pkgs.mcp-plan` to your packages, either in NixOS:

```nix
{ pkgs, ... }:
{
  environment.systemPackages = [ pkgs.mcp-plan ];
}
```

or with home-manager:

```nix
{ pkgs, ... }:
{
  home.packages = [ pkgs.mcp-plan ];
}
```

### Binary cache

Builds are cached on the [haras cachix cache]. When the flake is used directly
(for example with `nix run github:haras-unicorn/mcp-plan`), the cache is
configured automatically through the flake's `nixConfig`. To use it when the
package comes from an overlay, add the following to your nix configuration:

```nix
{
  nix.settings = {
    substituters = [ "https://haras.cachix.org" ];
    trusted-public-keys = [
      "haras.cachix.org-1:/HIo1JYqOIH1Nwk1EGXhuPPvDW0WekxIbY5CiXUZbYw="
    ];
  };
}
```

[haras cachix cache]: https://app.cachix.org/cache/haras

## Usage

`mcp-plan` is an MCP server that speaks the Model Context Protocol over stdio.
Add it as a stdio MCP server to any MCP client, for example:

```json
{
  "mcpServers": {
    "mcp-plan": {
      "command": "nix",
      "args": ["run", "github:haras-unicorn/mcp-plan"]
    }
  }
}
```

If `mcp-plan` is already on your `PATH`, point the client at the binary directly
instead:

```json
{
  "mcpServers": {
    "mcp-plan": {
      "command": "mcp-plan",
      "args": []
    }
  }
}
```

<!-- ANCHOR_END: body -->

## Documentation

The documentation is available at <https://haras-unicorn.github.io/mcp-plan/>.
