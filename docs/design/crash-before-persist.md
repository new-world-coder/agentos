# Design discussion: crash after external success

> Canonical decision record: **[ADR 0005](../adr/0005-crash-before-persist.md)** (Proposed).

This page is the short, shareable framing for contributors and reviewers.

## The scenario

1. Oasis decides to run a mutating tool (policy allowed, maybe HITL approved).
2. The external system **actually succeeds** (email sent, charge captured, PR opened).
3. Oasis crashes **before** `effect_applied` hits the journal / SQLite.
4. On resume, what should happen?

If we retry blindly → **duplicate side effect**.  
If we skip blindly → **lost success**, agent continues with a lie.  
If the call timed out → we may not even know which world we’re in.

## Why it belongs in Oasis’s core

The product thesis is durable, auditable agents—not prompt loops. Phase 0–1 proved
journal + hash chain + crash-resume for *local* state. The next integrity cliff is
exactly this window between **world change** and **durable belief**.

## Working stance (see ADR for full options)

- Checkpoint **intent** (`effect_executing` + idempotency key) **before** the call.
- Classify tools: Pure / Idempotent / AtMostOnce / Compensatable.
- Make **uncertainty** a first-class journal/run state—not a guessed failure.
- Never claim distributed exactly-once without a tool contract.

## How to participate

- Comment on PRs that touch `oasis-runtime` apply/resume or mutating tools.
- File issues linked to ADR 0005 open questions (keys, inquire, fsync strength, outbox).
- Add fault-injection tests when implementing—not only happy-path resume.
