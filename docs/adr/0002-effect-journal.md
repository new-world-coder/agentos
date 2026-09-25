# ADR 0002: Effect journal + hash chain

## Status

Accepted (Phase 0)

## Context

Agents produce side effects. Without an ordered, tamper-evident log, crash-resume and
audit are unreliable.

## Decision

Every run maintains an append-only journal. Each entry stores `prev_hash` and a
content hash over a canonical record (`seq`, `run_id`, `event_type`, `payload`,
`prev_hash`, `at`). Tip hash is mirrored on the `Run` record.

## Consequences

- Stores must reject entries that break seq / prev_hash.
- Resume verifies the chain before continuing.
- Payload tampering is detectable via `verify_hash` / `verify_chain`.
