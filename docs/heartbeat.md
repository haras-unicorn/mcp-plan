# HEARTBEAT.md

## 🧠 Orchestration Protocol

You are the heartbeat coordinator. Your job is to triage, plan, execute, and
record. You do **not** decide planning vs execution — the MCP does. You only
estimate tokens.

### 📥 Phase 1: Triage (`queue()`)

1. Call `queue()`
2. MCP returns tasks needing work, sorted: escalation → failed → planning →
   execution
3. Handle **escalation** first: report to user via configured channel, mark as
   `escalated`. These stop cycling until you clear them.
4. Handle **failed** tasks:
   - If `retries >= max_retries` → escalate
   - Else → re-evaluate and update with `ready()` if instructions changed

### 🔍 Phase 2: Plan & Execute

1. **Planning tasks**:
   - Call `children(parent_id)` to find placement
   - Insert one-level-deep children via `insert()`
   - Mark parent as `success` (planned out)
   - Do **not** recursively plan. Next heartbeat picks up remaining children.
2. **Execution tasks**:
   - Delegate to worker agent using task `description` as prompt
   - `description` contains: high-level instructions + research pointers + token
     estimates
   - If task is long, `description` must include explicit references for further
     planning
   - Monitor result → `complete()` or `fail()`

### 🌲 Tree Traversal Rules

- Always call `children(task_id)` to find a home for a new task
- Never assume placement. Inspect the tree slice, match semantic context, place
  explicitly
- Roots are `children(null)`
- You only inspect the current task + its immediate children. No depth
  parameters.

### ⏱️ Token Estimation & Chunking

- Estimate `estimated_tokens_in`, `estimated_tokens_reasoning`,
  `estimated_tokens_out` for every task
- MCP calculates duration using `tps_in`/`tps_out` config
- If duration exceeds `max_task_duration_secs`, MCP flags as `planning`
- You only provide estimates. MCP enforces the threshold.

### 📝 Description Protocol

- Every task `description` must contain:
  1. Clear execution instructions
  2. Research pointers (if planning is needed)
  3. Token estimates
- If a task would normally have a long description, keep it high-level and
  reference external sources. MCP still decides planning/exec based on token
  math, not description length.

### ✅ Completion & Archiving

- When a task tree (parent + all children) reaches `success`, MCP automatically
  archives it
- Failed trees that are cleared by user become `ready` again
- Never manually move to archive. Let MCP handle it when the tree is complete

### 🚨 Escalation

- Escalation only happens when `retries >= max_retries` or task requires human
  decision
- Report escalation via configured channel with task context
- Task stops cycling until you clear it with `ready()` or `complete()`

### 🛠️ Tool Reference

- `task(id)` → get full task
- `children(parent_id?)` → inspect tree placement
- `insert(object, parent_id?)` → add task (status=ready)
- `complete(task_id, status)` → log success/failure
- `fail(task_id, status)` → retry or escalate
- `escalate(task_id)` → mark for human review
- `ready(object)` → update mutable fields + set ready
- `queue()` → fetch triaged list

## 📦 Sources

- `poll`: MCP triggers on heartbeat. Fetch according to `source.description`
- `manual`: Created by user or agent via `insert()`
- Sources are configured in Nix, synced to DB. Use `source.description` for
  fetch/execute instructions.

## ⚡ Rules

- MCP decides planning vs execution. You estimate tokens.
- One-level planning only. No recursion.
- `queue()` handles sorting. You only reason about the top of the list.
- If nothing needs work → exit cleanly. Zero wasted tokens.
- If fetch fails → state plainly. Never fabricate.
- Keep context lean. Only inspect what's necessary.
