# Good first issues

Welcome. These are sized for a first PR while teaching the real invariants.

## Starter checklist

- [ ] `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass locally
- [ ] Read [VISION.md](../VISION.md) and [ARCHITECTURE.md](ARCHITECTURE.md)
- [ ] Open an issue (or claim one) before a large design change

## Ideas (file an issue, then PR)

### Docs & DX

1. **API curl cookbook** — document create → drive → journal → approve with example JSON.
2. **Architecture SVG** — export the mermaid diagram to a checked-in figure for GitHub mobile.
3. **Windows / macOS doctor notes** — if anything breaks outside Linux CI.

### Runtime & store

4. **API integration test** — spin Axum with `SqliteStore::in_memory` / tempfile; create + drive.
5. **Journal export** — CLI subcommand `journal --verify` already verifies; add `--jsonl` dump.
6. **Idempotent `put_effect` tests** — concurrent updates / status transitions.

### Policy & tools

7. **JSON Schema for tools** — describe `echo` / `add` inputs; validate before invoke.
8. **Policy fixture table** — data-driven allow/deny/approval cases in `oasis-policy`.

### Phase 2 (slightly larger)

9. **OpenAI-compatible `Provider`** behind env flag (no default network in tests).
10. **Streaming stub** — trait method + mock that yields chunks (no real HTTP required).

When you open the issue, label it `good first issue` and `help wanted` in the GitHub UI.
