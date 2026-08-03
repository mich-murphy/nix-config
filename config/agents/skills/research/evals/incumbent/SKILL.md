---
name: research
description: Investigate a question against high-trust primary sources and report evidence-backed findings. Use when the user wants a topic researched, documentation or API facts gathered, or reading legwork delegated. Return findings in chat by default; create a Markdown artifact only when the user requests persistent output or applicable instructions require it.
---

# Research a Question

## Route Model and Effort

Use a balanced model at medium effort for bounded primary-source lookup and
synthesis. Delegate independent reading or extraction to an efficient or
balanced model at low or medium effort when useful foreground work can
continue; keep the question, evidence boundaries, and final synthesis with the
parent. Escalate to a frontier model at high effort for consequential decisions,
conflicting evidence, or unfamiliar domains where a mistaken synthesis is
costly. Otherwise research directly.

## Investigate

1. Define the question and the facts needed to answer it.
2. Choose evidence suited to the claim, then prefer **primary sources** within
   that evidence type: official documentation, source code, specifications,
   first-party APIs, and original research. Use secondary sources for discovery
   or context, not as a substitute when the owning source is available.
3. Follow material claims back to their sources. Separate direct evidence from
   inference, anecdote, and source silence; note freshness limits for
   time-sensitive claims. Do not treat official product documentation as proof
   that a practice improves outcomes.
4. Stop when adequately direct evidence answers the question. Synthesize at the
   level of detail the user needs and cite sources beside the claims they
   support.

## Choose the Output

Default to returning the findings in chat. A request to research, investigate,
look up, or compare something does **not** by itself authorize creating or
modifying files, even when a repository is open. A parent agent asking a
background agent to report back also means return the findings to the parent,
not write a shared file.

Create a durable Markdown artifact only when:

- the user explicitly asks to save, write, capture, or document the research;
- the user supplies an output path; or
- applicable repository or workflow instructions require a research artifact.

When a file is required:

1. Use the supplied path, or an existing unambiguous convention for research
   notes in the repository.
2. If neither exists, ask where to save it instead of inventing a directory or
   filename. Do not overwrite an existing file unless updating it is clearly
   authorized.
3. Write one focused Markdown file, cite material claims, and report its path in
   the final response.

When a file is not required, create nothing and return the complete findings in
the final response.
