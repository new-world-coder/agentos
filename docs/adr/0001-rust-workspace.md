# ADR 0001: Rust workspace layout

## Status

Accepted (Phase 0)

## Context

AgentOS needs clear crate boundaries so policy, storage, providers, and the HTTP/CLI
surfaces can evolve independently without a monolith.

## Decision

Use a Cargo workspace with:

- `agentos-core` — types only
- `agentos-store` — persistence
- `agentos-provider` — model backends
- `agentos-tools` — tool implementations
- `agentos-policy` — authorization of effects
- `agentos-runtime` — orchestration loop
- `agentos-api` / `agentos-cli` — edges

## Consequences

Dependency direction flows inward toward `core`. Runtime depends on store/provider/
tools/policy but not on API/CLI. Enables swapping SQLite for other stores later.
