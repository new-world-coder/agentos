# Architecture

Oasis separates **proposal**, **authorization**, **application**, and **durability**.

## Control flow

```mermaid
sequenceDiagram
  participant U as User / API
  participant R as Runtime
  participant P as Provider
  participant Pol as Policy
  participant T as Tools
  participant S as Store

  U->>R: create_run(goal)
  R->>S: persist run + journal run_created
  U->>R: drive / resume
  R->>P: complete(messages, tools)
  P-->>R: ProposedEffect
  R->>S: journal effect_proposed
  R->>Pol: evaluate(effect)
  alt Deny
    R->>S: journal policy_denied / fail run
  else RequireApproval
    R->>S: status awaiting_approval
    U->>R: approve / reject
  else Allow
    R->>T: invoke (if tool)
    R->>S: journal effect_applied + update tip_hash
  end
```

## Hash chain

Each `JournalEntry` hashes a canonical record including `prev_hash`. Stores reject
mismatched `seq` / `prev_hash`. `Runtime::resume` calls `verify_chain` before continuing.

## Crate dependency direction

```text
cli / api
    ↓
runtime → policy, provider, tools, store
    ↓
core
```

Edges never depend on each other. Swapping SQLite for another `Store` should not
require changing policy or providers.

## Related ADRs

- [0001 Rust workspace](adr/0001-rust-workspace.md)
- [0002 Effect journal](adr/0002-effect-journal.md)
- [0003 Policy before effect](adr/0003-policy-before-effect.md)
- [0004 SQLite store](adr/0004-sqlite-store.md)
