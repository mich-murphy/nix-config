---
name: research
description: Investigate a question against high-trust primary sources and report the findings directly. Use when the user wants a topic researched, docs or API facts gathered. Create a Markdown artifact only when the user explicitly asks for one.
---

Use the direct web tools when the question needs no more than two lookups.
Otherwise delegate a single bounded question to the `Research` subagent.

Before delegating:

1. Separate the research question from implementation and local inspection.
2. Keep work in the parent when the parent is already investigating it.
3. Define a concrete completion condition, a budget of at most five primary
   source pages, and an output limit of at most 800 words.
4. Do not bundle independent release, installation, compatibility, audit, and
   implementation questions into one prompt.

Spawn `Research` with `max_turns: 8`. Tell it exactly which question to answer,
what not to investigate, and whether local files are in scope. Require
first-party documentation, specifications, source code, or official APIs and
exact source URLs.

If the result is required before any other work can continue, run the subagent
in the foreground. Otherwise run it in the background, continue separate work,
and use its completion notification. Do not start a background agent and then
immediately call `get_subagent_result` with `wait: true`.

Report findings directly. Create a Markdown artifact only when the user
explicitly requests one.
