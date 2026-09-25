# ADR 0004: SQLite as Phase-1 store

## Status

Accepted (Phase 1)

## Context

Phase 0 used an in-memory store. Crash-resume requires durable runs, effects, and
journal entries on a single node without standing up Postgres yet.

## Decision

Implement `SqliteStore` with WAL mode, bundled rusqlite, and tables for `runs`,
`effects`, and `journal`. CLI/API accept `--db <path>`. `Runtime::resume` reopens
the store, verifies the journal chain, and continues.

## Consequences

- Good enough for single-node durability and tests.
- Not a multi-writer control plane (Phase 3+).
- Migration SQL lives in `SqliteStore::migrate`; keep it additive.
