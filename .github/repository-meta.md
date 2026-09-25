# Repository About + rename checklist

GitHub Apps used by cloud agents often **cannot** rename the repository or PATCH
description/topics (403). A maintainer should apply these in the GitHub UI once.

## 1) Rename the GitHub repository

**Settings → General → Repository name**

```text
agentos  →  oasis
```

Until that lands, clone URLs still work as `new-world-coder/agentos` (GitHub redirects
after rename). Docs/badges already point at `new-world-coder/oasis`.

## 2) About

**Description** (short):

```text
Oasis — durable Rust agent runtime: hash-chained effect journal, policy-before-effect, HITL, crash-resume.
```

**Website** (optional):

```text
https://github.com/new-world-coder/oasis
```

**Topics** (add as tags):

```text
rust
oasis
agents
ai-agents
llm
agent-runtime
durable-execution
crash-recovery
sqlite
axum
hitl
policy-engine
apache-2
open-source
```

## 3) Releases

Tags `v0.1.0` / `v0.2.0` remain valid history (AgentOS era). This rename ships as
part of the next docs/release notes entry (`docs/RELEASE_NOTES.md`).
