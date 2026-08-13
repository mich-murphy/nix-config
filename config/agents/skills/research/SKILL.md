---
name: research
description: Investigate a question against high-trust primary sources and report the findings directly. Use when the user wants a topic researched, docs or API facts gathered, or reading legwork delegated to a background agent. Create a Markdown artifact only when the user explicitly asks for one.
---

Spin up a **background agent** to do the research, so you keep working while it reads.

Its job:

1. Investigate the question against **primary sources** — official docs, source code, specs, first-party APIs — not a secondary write-up of them. Follow every claim back to the source that owns it.
2. Report the findings directly, citing each claim's source.
3. Only when the user explicitly asks for an artifact, write the findings to a single Markdown file. Save it where the repo already keeps such notes; match the existing convention, and if there is none, put it somewhere sensible and say where. Otherwise, do not create or modify files.
