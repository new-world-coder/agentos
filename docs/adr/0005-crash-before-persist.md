# ADR 0005: External success, crash before persist

## Status

**Proposed** — core design discussion (not implemented).
Tracking issue: https://github.com/new-world-coder/oasis/issues/7  
Target phases: **2+** (real tools / providers). Phase 0–1 only claim journaled *local* progress.

## One-line problem

An external side effect can succeed in the real world while Oasis dies *before* the
journal records that success—so on resume the runtime may **retry** and double-apply,
or **skip** and lose the result.

This is among the hardest problems in durable execution. Getting it wrong silently
corrupts money, mail, tickets, and trust.

---

## Context

### What Phase 0–1 actually guarantee

Today the apply path is roughly:

1. Journal `effect_proposed` / `effect_policy_allowed` (durable)
2. **Invoke the tool** (external world may change here)
3. Persist effect result + journal `effect_applied` (durable)

If the process is killed between (2) and (3), resume sees a policy-allowed (or
in-flight) effect **without** an `effect_applied` tip—and may call the tool again.

That is acceptable for `echo` / `add`. It is **not** acceptable for `charge_card`,
`send_email`, or `create_pr`.

### The gap on a timeline

```text
  journal: policy_allowed ──────────────────────────────────────────► crash
                              │
                              ▼
                         tool returns OK
                         (world changed)
                              │
                              ▼
                    ✗ never wrote effect_applied
                              │
                              ▼
                         resume / retry ???
```

Three bad outcomes after resume:

| Strategy | Risk |
|----------|------|
| Always re-invoke | **At-least-once** → duplicate side effects |
| Never re-invoke | **At-most-once** → lost success; agent continues as if tool never ran |
| Guess from HTTP status | Ambiguous timeouts look like failures but may have succeeded |

We cannot invent a free lunch: without cooperation from the *effect itself*
(idempotency keys, inquire APIs, compensations), the runtime must pick a
deliberate delivery semantic—and document it.

### Relation to existing ADRs

- [0002](0002-effect-journal.md) — journal is source of truth for *what Oasis believes happened*
- [0003](0003-policy-before-effect.md) — authorization before invoke; does not solve post-invoke durability
- [0004](0004-sqlite-store.md) — local durability of journal rows, not of remote systems

---

## Forces

1. **Crash windows are real** — `kill -9`, OOM, host reboot, disk full mid-fsync.
2. **Networks lie** — timeout ≠ failure; success response may be lost after the server committed.
3. **Tools vary** — some are naturally idempotent; some are not; some offer inquire; some offer compensate.
4. **Agents compose tools** — a wrong retry policy in one tool poisons the whole run’s audit story.
5. **Honesty** — Oasis must not claim “exactly-once” for the outside world. Exactly-once *effects* are an emergent property of runtime + tool contract.

---

## Decision drivers (what “correct” means for Oasis)

We optimize for, in order:

1. **No silent double-spend / double-send** when the tool participates in an idempotency contract.
2. **Auditable ambiguity** when the outcome is unknown—human or automated resolve, not quiet guesswork.
3. **Resumability** — a crashed run can always continue from a verified journal tip.
4. **Tool author ergonomics** — clear traits / attributes: idempotent key, inquire, compensate.

Non-goal for early phases: pretending the runtime alone provides distributed exactly-once.

---

## Options considered

### A. Blind at-least-once (status quo for real tools)

On resume, if `effect_applied` is missing, call the tool again.

- **Pros:** Simple; matches many workflow engines’ default.
- **Cons:** Unsafe for non-idempotent effects; Oasis would be dishonest as an “oasis” of reliability.

### B. Blind at-most-once

On resume, if invoke started (or policy allowed) but no result, mark failed / need HITL; never retry.

- **Pros:** No duplicates.
- **Cons:** Drops successful work; agents stall; still need human cleanup.

### C. Prepare → execute → confirm (two-phase journal)

Journal `effect_executing` *with intent + idempotency key* **before** the external call;
then invoke; then journal `effect_applied` with result (or `effect_uncertain` on timeout).

```text
  effect_policy_allowed
       → effect_executing { idempotency_key, intent_hash }   // fsynced
       → external call(key)
       → effect_applied { result }  |  effect_uncertain { reason }
```

On resume:

- If tip is `effect_executing` → **do not** invent a new key; re-enter with the **same** key or call `inquire(key)`.
- If tip is `effect_uncertain` → require inquire / HITL / compensate policy.

- **Pros:** Makes the crash window explicit in the hash chain; enables safe retries when tools are keyed.
- **Cons:** Requires tool contract; two extra journal events; careful fsync discipline.

### D. Outbox / transactional messaging

Write “intent to call X” in the same DB transaction as state, and a separate worker drains the outbox.

- **Pros:** Classic pattern; strong with local SQLite transaction boundaries.
- **Cons:** Still at-least-once to the worker unless combined with idempotent consumers; more moving parts (Phase 3 workers).

### E. Sagas / compensations

Forward actions always retry; every non-idempotent effect registers a compensate handler.

- **Pros:** Recovers from duplicates by undoing.
- **Cons:** Compensations are hard (partial refunds, irreversible emails); not always possible.

---

## Proposed direction (for implementation later)

**Adopt C as the default Oasis contract**, with A/B as *declared* per-tool semantics when C cannot apply.

### 1. Effect delivery classes (tool metadata)

Every tool declares one of:

| Class | Meaning | Resume behavior |
|-------|---------|-----------------|
| `Pure` / `ReadOnly` | No durable external change | Safe to retry |
| `Idempotent` | Same `idempotency_key` → same world outcome | Retry with **same** key |
| `AtMostOnce` | Must not duplicate | Never auto-retry; `uncertain` → HITL |
| `Compensatable` | May duplicate if compensate exists | Retry or compensate per policy |

Unknown / undeclared tools that mutate should default to **`AtMostOnce` + HITL on uncertainty** once real sandbox tools land—not silent at-least-once.

### 2. Checkpoint before side effect

Before any mutating external call:

1. Derive or accept `idempotency_key` (stable function of `run_id`, `effect_id`, intent hash).
2. Append and **fsync** journal event `effect_executing`.
3. Only then perform the call, passing the key when the protocol supports it (`Idempotency-Key`, provider request ids, etc.).
4. On clear success → `effect_applied`.
5. On clear failure → `effect_failed` (retry policy separate).
6. On timeout / I/O error / crash → leave `effect_executing` or write `effect_uncertain`.

SQLite already gives us a place to fsync; Phase 1 store must expose “durable append” (WAL + sync) as an explicit API so we do not lie about checkpoint strength.

### 3. Resume algorithm (sketch)

```text
verify_chain(journal)
match tip / pending effect:
  effect_executing | effect_uncertain →
      if tool.inquire(key) known →
          persist inquired result as effect_applied | effect_failed
      else if tool.class == Idempotent →
          re-invoke with same key
      else if tool.class == Compensatable AND policy allows →
          compensate or HITL
      else →
          park run: awaiting_resolution (HITL / operator)
  effect_applied → continue provider loop
  else → existing drive() logic
```

### 4. Ambiguity is a first-class state

Add run/effect statuses for uncertainty, e.g. `AwaitingResolution`, rather than overloading `Failed`.
The journal must show *that we do not know*, which is more honest than a guessed failure.

### 5. Provider / LLM boundary

Model calls are also external. Treat completions as `Idempotent` only if the provider
supports request IDs / replay; otherwise accept at-least-once *token spend* but never
treat a non-persisted tool call as done. Prefer checkpointing *tool* boundaries first;
LLM retry is cheaper and usually safer than tool retry.

---

## Consequences

### If we accept this ADR (when implementing)

- Tool trait grows: `delivery_class`, optional `inquire`, optional `compensate`, key plumbing.
- Runtime grows: `effect_executing` / `effect_uncertain` journal events; resume branch.
- Docs / CLI: `doctor` or `journal --verify` can flag unfinished `effect_executing` tips.
- Tests: mandatory fault-injection (“succeed externally, kill before persist”) for every mutating tool class.
- We will **not** market “exactly-once side effects” without the tool contract.

### If we reject / defer

- Phase 2 real tools must still document “at-least-once invoke” loudly.
- Shipping payment-like tools without this ADR would violate the oasis thesis.

---

## Open questions (discussion)

1. Should `idempotency_key` be runtime-owned always, or allow tools to supply their own?
2. Is `inquire` mandatory for `Idempotent` class, or is “same key, rely on server” enough?
3. Do we persist raw external responses in the journal (audit) or only hashes (privacy)?
4. How does HITL approval interact with `effect_executing` (approve intent vs approve uncertain resolve)?
5. Outbox (option D) in Phase 3 workers—compose with C or replace the in-process checkpoint?
6. What’s the minimum SQLite sync mode (`FULL` vs `NORMAL`) we require for “checkpoint”?

---

## References (conceptual)

- Temporal / Cadence: workflow history + activity heartbeats; at-least-once activities + idempotent handlers
- Classic outbox pattern; saga / compensation literature
- HTTP `Idempotency-Key` (IETF drafts / Stripe-style APIs)
- Oasis ADRs 0002–0004; `Runtime::apply_effect` crash window in `oasis-runtime`

## Implementation gate

Do not mark this ADR **Accepted** until:

- [ ] Tool delivery classes exist in `oasis-tools`
- [ ] `effect_executing` is journaled and fsynced before mutating invokes
- [ ] Resume handles executing/uncertain without blind duplicate invokes for `AtMostOnce`
- [ ] At least one fault-injection test demonstrates “external OK, kill, resume” for an `Idempotent` tool and an `AtMostOnce` tool

Until then, this document is the **standing design discussion** for contributors touching tools, runtime resume, or provider I/O.
