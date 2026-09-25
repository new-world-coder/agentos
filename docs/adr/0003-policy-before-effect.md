# ADR 0003: Policy before effect

## Status

Accepted (Phase 0)

## Context

Prompt-level “don’t do dangerous things” instructions fail open. Tool calls must be
gated in-process before any side effect.

## Decision

The runtime evaluates `PolicyEngine::evaluate` on every `ProposedEffect` before
application. Outcomes: Allow, Deny (fail run), RequireApproval (HITL pause).

Denied tools (default: `shell`, `exec`) never execute. Approval-required tools and
explicit HITL effects park the run in `awaiting_approval`.

## Consequences

- Providers may propose anything; policy is the enforcement point.
- HITL is a first-class status, not an out-of-band hack.
- Policy config must be versioned carefully in later phases.
