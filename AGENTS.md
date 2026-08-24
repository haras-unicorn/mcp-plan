# AGENTS.md

`mcp-plan` is a Rust MCP server that provides planning tooling. It is a Cargo
workspace with a single crate, `mcp-plan`, inside `src`.

## Structure

- `src/mcp-plan/src/lib.rs` - MCP server and tool definitions.
- `src/mcp-plan/src/main.rs` - stdio entry point.
- `src/mcp-plan/src/connect.rs` - database connection (backend-aware).
- `src/mcp-plan/src/db.rs` - sea-orm entities.
- `src/mcp-plan/src/models.rs` - tool-surface DTOs.
- `src/mcp-plan/src/service.rs` - DB access layer.
- `assets` - generated config schema and example configuration.
- `docs` - mdBook documentation.
- `flake.nix` - flake exposing the development shell, the `mcp-plan` package and
  runnable apps.

## Features

`mcp-plan` builds against one database backend at a time, selected by a Cargo
feature: `sqlite` (default), `postgres` or `mysql`. `--all-features` enables all
three at once for development and testing; the per-backend binaries reuse one
dependency graph in the flake.

## Development

The default development shell (assume you are already running inside it)
provides the following scripts:

- `dev run` - run the MCP server over stdio.
- `dev schema` - regenerate `assets/config.schema.json`.
- `dev format` - format the repository.
- `dev lint` - lint and test the repository.
- `dev test` - check clippy warnings and run rust tests.
