# Contributing to AgentOS

Thanks for helping build a durable agent runtime.

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo run -p agentos-cli -- doctor
```

Use a SQLite path when testing crash-resume:

```bash
cargo run -p agentos-cli -- --db /tmp/agentos.db run "goal"
# kill -9 the process mid-run in real tests, then:
cargo run -p agentos-cli -- --db /tmp/agentos.db resume <run_id>
```

## Principles

1. **Journal first** — durable events before external side effects whenever possible.
2. **Policy before effect** — never invoke a tool until policy (or HITL) allows it.
3. **Hash chain integrity** — any store write that breaks `verify_chain` is a bug.
4. **Honest phases** — update LIFECYCLE.md exit criteria; do not skip ahead in claims.

## Pull requests

- Keep PRs focused; prefer small crates-scoped changes.
- Include tests for journal, policy, and resume paths you touch.
- Follow Apache-2.0; do not add incompatible licenses.
- Link any ADR updates in `docs/adr/`.

## Code of conduct

Be respectful. No harassment, personal attacks, or gatekeeping. Maintainers may
moderate discussions and reject contributions that violate these norms.
