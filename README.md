# MCP Plan

<!-- ANCHOR: body -->

MCP server that provides planning tooling.

## Installation

`mcp-plan` is packaged as a Nix flake. Run it directly without installing:

```sh
nix run github:haras-unicorn/mcp-plan -- run
```

or build the `mcp-plan` binary with:

```sh
nix build github:haras-unicorn/mcp-plan
```

### Releases

Prebuilt binaries for `x86_64-linux` and `aarch64-linux`, for each supported
database backend (`sqlite`, `postgres`, `mysql` or all), are attached to each
[GitHub release] as tarballs containing the `mcp-plan` binary.

To run with a specific backend (in this example `sqlite`) from releases:

```sh
curl -L -o mcp-plan.tar.gz \
  https://github.com/haras-unicorn/mcp-plan/releases/latest/download/mcp-plan-x86_64-linux-sqlite.tar.gz
tar -xzf mcp-plan.tar.gz
./mcp-plan-x86_64-linux-sqlite run
```

To run with a binary supporting all backends from releases:

```sh
curl -L -o mcp-plan.tar.gz \
  https://github.com/haras-unicorn/mcp-plan/releases/latest/download/mcp-plan-x86_64-linux.tar.gz
tar -xzf mcp-plan.tar.gz
./mcp-plan-x86_64-linux run
```

Pick the archive matching your backend:

- `mcp-plan-x86_64-linux.tar.gz` (all backends)
- `mcp-plan-x86_64-linux-sqlite.tar.gz`
- `mcp-plan-x86_64-linux-postgres.tar.gz`
- `mcp-plan-x86_64-linux-mysql.tar.gz`
- `mcp-plan-aarch64-linux.tar.gz` (all backends)
- `mcp-plan-aarch64-linux-{sqlite,postgres,mysql}.tar.gz`

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
      "args": ["run", "github:haras-unicorn/mcp-plan", "--", "run"]
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
      "args": ["run"]
    }
  }
}
```

## Configuration

`mcp-plan` reads its configuration from `config.toml` in the working directory,
overlaid with `MCP_PLAN_*` environment variables. Every setting is optional. The
full schema and a worked example live in the [References](#references) section.

### Command line

The binary accepts a subcommand:

- `mcp-plan run` — start the MCP server over stdio (default).
- `mcp-plan migrate` — open the database and apply pending migrations, then
  exit.
- `mcp-plan schema` — write the configuration JSON schema to `--output`.

`--config <path>` is a global flag selecting a different configuration file
(defaults to `config.toml`), for example:

```sh
mcp-plan --config ./prod.toml migrate run
```

### Configuration file

```toml
[database]
url = "sqlite://data/mcp-plan.db"
```

The file is split into three sections:

- `database` — the database `url`. See below for the supported schemes.
- `runtime` — `tps_in`, `tps_out`, `max_task_duration_secs`, `queue_limit`,
  `max_retries`.
- `sources` — a list of statically configured sources.

See the JSON schema and the example for the exact keys and defaults (References
below).

### Environment

Environment variables override file values. Use the `MCP_PLAN` prefix with `__`
as the section separator:

```sh
MCP_PLAN__RUNTIME__TPS_IN=1000 mcp-plan run
```

### Logging

Logs are emitted as newline-delimited JSON on **stderr**, keeping stdout
exclusively for the MCP JSON-RPC protocol. Each line carries a `level`,
`timestamp`, `target`, a human-readable `message`, and structured fields (e.g.
`task_id`, `duration_ms`) that are safe to query with `jq`:

```sh
RUST_LOG=debug mcp-plan run 2> >(jq -r '"\(.level): \(.message)"')
```

Log verbosity is controlled via `RUST_LOG` (default `info`). Any standard
tracing filter is accepted (`error`, `warn`, `info`, `debug`, `trace`,
per-target filters, etc.).

### Database

`database.url` selects the backend by scheme:

- `sqlite://data/mcp-plan.db` — a SQLite file (relative to the working
  directory). The database file and its parent directory are created on first
  start. Use an absolute path (e.g. `sqlite:///var/lib/mcp-plan.db`) or
  `sqlite::memory:` for an in-memory database.
- `postgres://user:password@host:port/database` — PostgreSQL.
- `mysql://user:password@host:port/database` — MySQL.

Each binary is built against a single backend (`sqlite` by default, or the
`postgres`/`mysql` build variants from the releases).

Both `run` and `migrate` connect to the database and apply pending migrations at
startup; `migrate` exits immediately afterwards so migrations can be run as a
separate init step (e.g. in multi-tenant deployments). SQLite runs in WAL mode
with foreign keys enabled.

## Integration

`mcp-plan` is a regular MCP server, so any MCP client — including a custom agent
runtime — can drive it. Register it as a stdio server (see the [Usage](#usage)
example) and expose the `plan__*` tools to your agent.

To keep an autonomous agent working over time, add a scheduler of your choice (a
cron entry, a CI scheduled job, a loop inside an existing daemon or an OpenClaw
heartbeat/agent cron job) that periodically connects to `mcp-plan` and instructs
the agent to run a planning/delegation pass. An example heartbeat prompt is
provided in [References](#references) section—you can point a cron job at it
verbatim or use it as a template:

```sh
# example: run an agent-driven planning pass on an interval
*/30 * * * * mcp-plan-with-agent --instruct assets/heartbeat.md
```

Exact wiring depends on your runtime; the example shows the shape. The tasks and
sources live in the database, so each pass should continue where the previous
one stopped.

<!-- ANCHOR_END: body -->

## References

The JSON schema, an example configuration, and an example heartbeat are
generated and versioned under `assets`:

- [assets/config.schema.json](./assets/config.schema.json)
- [assets/config.example.toml](./assets/config.example.toml)
- [assets/heartbeat.md](./assets/heartbeat.md)

## Documentation

The documentation is available at <https://haras-unicorn.github.io/mcp-plan/>.
