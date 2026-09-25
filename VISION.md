# Vision — the AgentOS oasis

## The desert

Most “agents” today are:

- a `while` loop around a chat API,
- tools invoked on trust from a prompt,
- state that evaporates when the process dies,
- audits that are log lines if you’re lucky.

That works for demos. It fails for products that must be **resumable**, **reviewable**, and **constrainable**.

## The oasis

AgentOS is a bet that agent infrastructure should look more like **durable execution** and less like improv theatre:

1. **Effects are first-class** — proposed, decided, applied, recorded.
2. **Policy runs before the effect** — deny / allow / human approval in-process.
3. **The journal is the source of truth** — append-only, hash-chained, verifiable on resume.
4. **Rust crates you can embed** — not a hosted black box you can’t reason about.

An oasis is not a mirage: we only claim phases that pass [LIFECYCLE.md](LIFECYCLE.md) exit criteria.

## The movement

“Oasis” means a community that:

- prefers **invariants** over vibes,
- documents architecture with **ADRs**,
- welcomes contributors who care about **crash-resume, policy, and audit**,
- stays **Apache-2.0** and fork-friendly,
- grows by shipping boring, correct runtime pieces—not hype launches.

If that resonates, you belong here. Start at [docs/GOOD_FIRST_ISSUES.md](docs/GOOD_FIRST_ISSUES.md).

## Non-goals (for now)

- Being the biggest prompt framework
- Hiding policy inside the model
- Claiming Phases 2–4 before they exist

## North star

An engineer can kill `-9` an AgentOS process mid-tool-loop, restart, **verify the journal**, resume, and show a reviewer *exactly* what was proposed, allowed, and applied.
