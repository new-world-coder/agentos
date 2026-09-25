# ADR 0001: Rust workspace layout

## Status

Accepted (Phase 0)

## Context

Oasis needs clear crate boundaries so policy, storage, providers, and the HTTP/CLI
surfaces can evolve independently without a monolith.

## Decision

Use a Cargo workspace with:

- `oasis-core` — types only
- `oasis-store` — persistence
- `oasis-provider` — model backends
- `oasis-tools` — tool implementations
- `oasis-policy` — authorization of effects
- `oasis-runtime` — orchestration loop
- `oasis-api` / `oasis-cli` — edges

## Consequences

Dependency direction flows inward toward `core`. Runtime depends on store/provider/
tools/policy but not on API/CLI. Enables swapping SQLite for other stores later.
