# Oasis Lifecycle

Phased delivery plan for the durable agent runtime. Do not mark a later phase complete
until its exit criteria pass.

## Phase 0 — Durable core (tagged `v0.1.0`)

**Goal:** A compilable Rust workspace that can propose effects, enforce policy before
application, journal a hash chain, pause for HITL, and expose a mock-backed CLI + API.

### Exit criteria

- [x] Apache-2.0 workspace: `core`, `store`, `provider`, `tools`, `policy`, `runtime`, `api`, `cli`
- [x] Effect journal with SHA-256 hash chain + verification
- [x] Policy-before-effect (deny / allow / require approval)
- [x] HITL approval gate
- [x] Mock provider
- [x] In-memory store
- [x] Axum API (`/health`, `/v1/runs`, drive / resume / approve / journal)
- [x] CLI `doctor`
- [x] Docs: LIFECYCLE, ADRs, COMPETITORS, CONTRIBUTING, GOVERNANCE + competitive canvas
- [x] `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass

## Phase 1 — SQLite durability + crash-resume

**Goal:** Survive process death without losing run state or journal integrity.

### Exit criteria

- [x] SQLite `Store` backend (WAL, migrations)
- [x] Persist runs, effects, and journal across reopen
- [x] `Runtime::resume` verifies journal chain then continues
- [x] Automated crash-resume test (mid-flight `Running` + HITL reopen)
- [x] CLI `--db` path for durable runs
- [ ] Optional: API integration test against SQLite (nice-to-have)

## Phase 2 — Real providers + richer tools (not started)

- OpenAI / Anthropic / local provider adapters behind `Provider`
- Streaming tokens
- Sandboxed shell / filesystem tools with stronger policy profiles
- Structured tool schemas (JSON Schema)
- **Design gate:** [ADR 0005](docs/adr/0005-crash-before-persist.md) — mutating tools must not ship silent at-least-once without an idempotency / uncertainty story

## Phase 3 — Control plane (not started)

- Multi-run scheduler / worker pool
- Authn/z for API
- Observability (metrics, OpenTelemetry)
- Run search and retention policies

## Phase 4 — Distribution (not started)

- Remote journal replication / export
- Multi-node workers with lease-based ownership
- Formal threat model + supply-chain attestations

## Versioning

| Tag | Meaning |
|-----|---------|
| `v0.1.0` | Phase 0 complete |
| `v0.2.0` | Phase 1 complete (+ community presence) |
| `v0.3.0+` | Later phases |

Phase numbers in docs must stay honest: never claim Phase 2–4 done until exit criteria land.

Release narrative for humans: [docs/RELEASE_NOTES.md](docs/RELEASE_NOTES.md).
