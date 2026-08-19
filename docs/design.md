# DESIGN.md

## Overview

A minimal, stateful task orchestration layer for agent runtimes. The MCP server
holds state in SQLite; the agent drives planning and placement via explicit tool
calls. Token estimation + MCP-configured throughput determines whether a task is
`planning` or `execution`. No arbitrary routing, no hardcoded depth limits, no
context bloat.

## Architecture

```txt
[ZeroClaw Agent] ↔ [MCP Server] ↔ [SQLite DB]
       ↑                  ↓
  (Two-phase heartbeat)  (Schema: sources, tasks, archive)
```

- **MCP Server**: State machine. Enforces transactions, validates placements,
  calculates token-based duration, runs `queue()`, handles archiving.
- **Agent**: Reasoning layer. Traverses trees with `children()`, estimates
  tokens, decides where to place tasks, delegates execution, handles escalation.
- **SQLite**: Single file, WAL mode. Zero external deps. Holds everything in a
  flat but hierarchical graph.

## Data Model

### `sources`

```sql
sources (
  id TEXT PRIMARY KEY,
  title TEXT,
  description TEXT,        -- instructions on how to fetch/use this source
  type TEXT CHECK(type IN ('manual', 'poll'))
)
```

- Configured statically via Nix, synced into DB on rebuild.
- `poll`: Triggered by heartbeat. `manual`: Created by user or agent.

### `tasks`

```sql
tasks (
  id TEXT PRIMARY KEY,
  parent_id TEXT REFERENCES tasks(id),
  source_id TEXT REFERENCES sources(id),
  title TEXT,
  description TEXT,        -- execution instructions + research pointers for long tasks
  status TEXT DEFAULT 'ready' CHECK(status IN ('ready', 'success', 'failure', 'escalated')),
  priority TEXT DEFAULT 'medium' CHECK(priority IN ('critical', 'high', 'medium', 'low')),
  retries INTEGER DEFAULT 0,
  created_at TEXT,
  updated_at TEXT,
  estimated_tokens_in INTEGER,
  estimated_tokens_reasoning INTEGER,
  estimated_tokens_out INTEGER
)
```

- No `tags`, no `metadata`. Routing/context lives in `description` +
  `parent_id`.
- Status is append-only in practice (records transitions, never reverts).
- `description` length note: If a task requires extensive planning,
  `description` holds high-level instructions + explicit research references.
  The MCP still decides planning/exec based on token estimates, not description
  length.

### `archive`

```sql
archive (LIKE tasks);
```

- Successful trees are ported here to keep the working set lean.

## MCP Tool Surface

| Tool                                 | Args                               | Behavior                                                                              |
| ------------------------------------ | ---------------------------------- | ------------------------------------------------------------------------------------- |
| `task(id)`                           | `id: string`                       | Returns full task object                                                              |
| `children(parent_id?: string)`       | `parent_id?: string`               | Returns `{id, title, status, priority, estimated_tokens_*}` array. `null` = roots     |
| `insert(object, parent_id?: string)` | `object: task, parent_id?: string` | Inserts with `status='ready'`, returns new id                                         |
| `complete(task_id, status)`          | `task_id: string, status: string`  | Updates status to `success` or `failure`, logs transition                             |
| `fail(task_id, status)`              | `task_id: string, status: string`  | Increments `retries`, updates status to `failure`, logs reason                        |
| `escalate(task_id)`                  | `task_id: string`                  | Marks as `escalated`, prevents auto-requeue until human clears                        |
| `ready(object)`                      | `object: task`                     | Updates mutable fields (title, desc, priority, estimates), sets status=`ready`        |
| `queue()`                            | _(none)_                           | Returns sorted list of tasks needing work: escalation → failed → planning → execution |

## The `queue()` Engine

**Input**: None (MCP handles DB state) **Output**: Sorted array of tasks
requiring attention **Logic**:

1. Filter out `success` and `escalated`
2. Handle `failed` tasks:
   - If `retries >= max_retries` (MCP config) → mark `escalated`
   - Else → keep as `failed` for re-evaluation
3. Sort remaining by `priority` (critical > high > medium > low), then by type:
   `planning` tasks first, `ready` tasks second
4. Return truncated list (bounded by MCP `queue_limit`)

## Token-Based Chunking

- **MCP Config**: `tps_in`, `tps_out`, `max_task_duration_secs`
- **Formula**:

```txt
duration = estimated_tokens_in / tps_in
  + (estimated_tokens_reasoning + estimated_tokens_out)
    / (tps_in + tps_out)
```

- If `duration > max_task_duration_secs` → MCP flags task as `planning`
- Agent **never** decides planning vs execution. Only estimates tokens. MCP
  enforces the threshold.

## Operational Flow

### Phase 1: Fetch & Triage

1. Agent calls `queue()`
2. MCP returns triaged list
3. Agent handles escalation first (reports to human, marks `escalated`)
4. Agent processes failures (retries or escalates)
5. Agent separates `planning` vs `execution` based on MCP output

### Phase 2: Execute & Record

1. **Planning tasks**: Agent calls `children(task_id)` to find placement, calls
   `insert()` with one-level-deep children, marks parent as `success` (planned
   out)
2. **Execution tasks**: Agent delegates to worker agent with task `description`
   as prompt, monitors result, calls `complete()` or `fail()`
3. **Archiving**: When entire tree (parent + all children) reaches `success`,
   MCP moves nodes to `archive`

## Design Rationale

- **Agent-driven traversal** (`children()`) eliminates arbitrary routing.
  Placement is visible and auditable.
- **Token estimation over heuristic rules** makes chunking deterministic and
  model-agnostic.
- **One-level planning constraint** prevents context bloat and forces
  incremental progress.
- **Flat schema + explicit `parent_id`** keeps SQLite lightweight while
  preserving full HTN semantics.
- **`description` as the single source of truth** avoids tag sprawl. Execution
  instructions + research pointers live where they're needed.
- **`queue()` as the mechanical bridge** ensures the agent only reasons about
  the top of the priority list. Sorting, filtering, and retry logic stay in the
  DB.
