# Competitors & landscape

Oasis positions itself as a **durable, policy-gated agent runtime** with an
append-only effect journal—not a chat UI or a prompt framework.

| Project | Focus | Contrast with Oasis |
|---------|-------|------------------------|
| LangGraph / LangChain | Graph orchestration in Python | Rich ecosystem; less emphasis on hash-chained effect journals in Rust |
| AutoGen / Semantic Kernel | Multi-agent conversation patterns | Strong DX; durability often delegated to app layer |
| Temporal + LLM workers | Workflow durability | Excellent durability; not agent/policy-native out of the box |
| OpenAI Assistants / Responses | Hosted threads & tools | Convenient; vendor lock-in and opaque control plane |
| CrewAI / BabyAGI-style | Role-playing agent crews | Fast prototypes; weak crash-resume / policy story |
| In-house Python loops | Custom `while tool_call` | Flexible; reinvent journaling, HITL, and audit |

## Differentiation bets

1. **Effect journal + hash chain** as the source of truth for what the agent did.
2. **Policy-before-effect** as a hard runtime invariant, not a prompt suggestion.
3. **Crash-resume** as a Phase-1 requirement with automated tests.
4. **Rust workspace** for embeddability and a single static binary CLI/API.

See `canvases/competitive-landscape.md` for a one-page canvas.
