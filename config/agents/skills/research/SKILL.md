---
name: research
description: Investigate a question against high-trust primary sources and report the findings directly. Use when the user wants a topic researched, docs or API facts gathered. Create a Markdown artifact only when the user explicitly asks for one.
---

Spawn a **subagent** to do the research.

Its job:

1. Investigate the question against **primary sources**: official docs, source code, specs, first-party APIs. Avoid secondary write-ups. Follow every claim back to its original source.
2. Report the findings directly, citing each claim's source.
3. When a Markdown document is explicitly requested, create one in a location following current directory conventions.
