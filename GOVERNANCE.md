# Governance

Oasis is an Apache-2.0 project aimed at a durable, auditable agent runtime.

## Roles

| Role | Responsibility |
|------|----------------|
| Maintainers | Merge rights, release tags, ADR acceptance |
| Contributors | Patches via PR; no commit rights required |
| Users | Feedback via issues; security reports privately when possible |

## Decisions

- Architectural choices that bind multiple crates land as ADRs under `docs/adr/`.
- Lifecycle phase completion requires the exit criteria in `LIFECYCLE.md` to be checked
  and a release tag when noted.
- Security-sensitive policy defaults (deny lists, HITL) should not be silently weakened.

## Releases

1. Ensure `cargo test` and `cargo clippy -D warnings` pass.
2. Update `LIFECYCLE.md` checkboxes and crate `version` if needed.
3. Tag (`v0.1.0`, `v0.2.0`, …) from the commit that meets the phase exit criteria.
4. Announce notable changes in the GitHub release notes.

## Conflict resolution

Discuss on the PR or issue. If stuck, a maintainer records the decision in an ADR
or governance note. Forking remains available under Apache-2.0.
