# Release notes

## v0.2.0 — Phase 1 (SQLite + crash-resume)

- `SqliteStore` with WAL, runs / effects / journal tables
- `Runtime::resume` verifies hash chain then continues
- Crash-resume + HITL reopen tests
- CLI `--db` for durable runs
- Docs: architecture, vision (oasis), community health files, CI

## v0.1.0 — Phase 0 (durable core)

- Apache-2.0 Rust workspace: core, store, provider, tools, policy, runtime, api, cli
- Effect journal + SHA-256 hash chain
- Policy-before-effect and HITL gates
- Mock provider, in-memory store, Axum API, CLI `doctor`
- LIFECYCLE, ADRs, COMPETITORS, CONTRIBUTING, GOVERNANCE
