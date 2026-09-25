# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| `v0.2.x` (Phase 1) | ✅ |
| `v0.1.x` | ✅ (security fixes cherry-picked when practical) |
| unreleased `main` | best-effort |

## What counts as a vulnerability

Please report:

- Ways to bypass **policy-before-effect** (tool runs without allow/HITL)
- Journal / hash-chain integrity bugs that allow silent tampering
- Crash-resume corruption or privilege issues in the SQLite store
- RCE or path traversal via tools/API once real tools land

Out of scope for now: “the mock provider is not a real LLM” and missing Phase 2+ features.

## How to report

Open a **private** GitHub Security Advisory on this repository if available, or email the
repository owner listed on the GitHub profile. Do **not** file a public issue for
exploitable bugs until a fix or mitigation is ready.

We aim to acknowledge within 7 days and coordinate disclosure.
