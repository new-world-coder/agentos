# AgentOS

Durable Rust agent runtime with an append-only effect journal, policy-before-effect
gates, human-in-the-loop approvals, and crash-resume.

**License:** Apache-2.0 · **Status:** Phase 1 (SQLite store + crash-resume)

## Crates

| Crate | Role |
|-------|------|
| `agentos-core` | Runs, effects, journal entries, hash chain |
| `agentos-store` | `Store` trait, in-memory + SQLite backends |
| `agentos-provider` | Provider trait + mock LLM |
| `agentos-tools` | Tool registry (`echo`, `add`) |
| `agentos-policy` | Policy-before-effect engine |
| `agentos-runtime` | Durable propose → policy → HITL → apply loop |
| `agentos-api` | Axum HTTP API |
| `agentos-cli` | CLI (`doctor`, `run`, `resume`, …) |

## Quick start

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p agentos-cli -- doctor

# Ephemeral run (in-memory)
cargo run -p agentos-cli -- run "say hello"

# Durable run (SQLite)
cargo run -p agentos-cli -- --db ./agentos.db run "say hello"
cargo run -p agentos-cli -- --db ./agentos.db resume <run_id>

# HTTP API
cargo run -p agentos-api -- --db ./agentos.db --bind 127.0.0.1:8080
```

## Design highlights

- **Effect journal + hash chain** — every lifecycle event is appended and chained with SHA-256.
- **Policy-before-effect** — tools never run until the policy engine allows (or HITL approves).
- **HITL** — runs pause at `awaiting_approval` and resume after approve/reject.
- **Crash-resume** — SQLite persists runs/effects/journal; `resume` verifies the chain and continues.

See [LIFECYCLE.md](LIFECYCLE.md) for phases, [docs/](docs/) for ADRs, and
[canvases/](canvases/) for the competitive landscape.

## What this is not (yet)

Phases 2–4 are **not** done: no multi-tenant control plane, no real model providers in
production posture, no distributed workers. See LIFECYCLE.md.
