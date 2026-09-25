# Competitive canvas — AgentOS

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Problem                                                                     │
│ Agent loops lose state on crash; policy is prompt-deep; audits are ad-hoc.  │
├─────────────────────────────────────────────────────────────────────────────┤
│ Solution                                                                    │
│ Rust runtime: propose → policy → (HITL) → apply, with hash-chained journal. │
├───────────────┬───────────────┬───────────────┬─────────────────────────────┤
│ Unique value  │ Unfair adv.   │ Channels      │ Cost structure              │
│ Journal+hash  │ Invariant     │ GitHub, CLI   │ Mostly eng time;            │
│ Policy gate   │ in-process    │ embedders     │ no GPU fleet required       │
│ Crash-resume  │ for Phase 1   │ API           │ for mock/dev                │
├───────────────┴───────────────┴───────────────┴─────────────────────────────┤
│ Customer segments                                                           │
│ Teams embedding agents in products; infra engineers who need audit trails.  │
├─────────────────────────────────────────────────────────────────────────────┤
│ Key metrics                                                                 │
│ Resume success after kill -9; policy deny rate on dangerous tools;          │
│ journal verify_chain = Ok on every durable test.                            │
├─────────────────────────────────────────────────────────────────────────────┤
│ Competitors (see docs/COMPETITORS.md)                                       │
│ LangGraph · AutoGen · Temporal+LLM · Hosted Assistants · DIY Python loops   │
└─────────────────────────────────────────────────────────────────────────────┘
```

Phase honesty: Phase 0/1 only. Hosted multi-tenant and real provider production
hardening are later lifecycle phases.
