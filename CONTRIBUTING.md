# Contributing to Oasis

Thanks for helping grow the [oasis](VISION.md)—a durable, auditable agent runtime.

## Before you code

1. Skim [VISION.md](VISION.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
2. Prefer a [good first issue](docs/GOOD_FIRST_ISSUES.md) or file one with the template.
3. Keep phase claims honest per [LIFECYCLE.md](LIFECYCLE.md).

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run -p oasis-cli -- doctor
```

Durable resume smoke:

```bash
cargo run -p oasis-cli -- --db /tmp/oasis.db run "goal"
cargo run -p oasis-cli -- --db /tmp/oasis.db resume <run_id>
```

## Principles

1. **Journal first** — durable events before external side effects whenever possible.
2. **Policy before effect** — never invoke a tool until policy (or HITL) allows it.
3. **Hash chain integrity** — any store write that breaks `verify_chain` is a bug.
4. **Honest phases** — update LIFECYCLE.md exit criteria; do not skip ahead in claims.

## Pull requests

- Use the PR template; keep changes focused.
- Include tests for journal, policy, and resume paths you touch.
- Apache-2.0 only—no incompatible licenses.
- Link ADR updates under `docs/adr/`.

## Code of conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Maintainers may moderate and reject
contributions that violate community standards.
