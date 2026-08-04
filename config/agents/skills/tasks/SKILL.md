---
name: tasks
description: Convert an approved, implementation-ready software plan—especially a finalized Neo implementation brief—into a dependency-ordered graph of independently admissible, context-bounded task plans for $ship. Use when a plan is too large for one Ship run, when task or ticket granularity needs review, or when separate fresh implementation contexts need self-contained handoffs. Do not use to settle requirements or architecture, implement the plan, schedule people, mutate an issue tracker, or split an already bounded Ship-ready task.
---

# Turn One Plan Into Ship Tasks

Produce the smallest graph of coherent vertical tasks that preserves the
approved plan. Treat context size as a risk estimate, not a token guarantee.
Do not implement a task or invoke `$ship`.

## Admit the Source Plan

1. Read repository instructions, the complete supplied plan, and its approval
   or version evidence.
2. For a Neo input, require an approved exact
   `.neo/tasks/<slug>/implementation-brief.md`; do not decompose a stale draft
   or substitute earlier Neo artifacts.
3. Inspect relevant source, tests, schemas, configuration, and local verification
   commands. Use this inspection to locate planned work and detect conflicts,
   not to reopen settled design.
4. Apply the admission contract in
   [readiness-and-stops.md](../ship/references/readiness-and-stops.md). If the
   plan lacks a consequential decision, is contradicted by the repository, or
   has no trustworthy verifier, stop without creating tasks. Return the exact
   evidence, missing decision, impact, and smallest upstream handoff.
5. Compute and retain the source plan SHA-256. Never edit the source plan.

## Set the Context Policy

Read [sizing.md](references/sizing.md). Use the user or harness limit when
supplied. Otherwise record a 200,000-token class window, a 100,000-token warning,
and 50 percent reserve as a conservative trial default.

Budget for the whole implementation loop: orientation, source and test reading,
edits, focused feedback, broader verification, remediation, and handoff. The
fresh independent `$ship` review uses another context, but likely remediation
must still fit the task's implementation context.

## Build the Task Graph

1. Inventory every requirement, non-goal, invariant, compatibility obligation,
   delivery slice, verification need, documentation change, and replan trigger.
2. Begin from approved Neo delivery slices when present, but do not assume one
   slice already equals one context-safe task.
3. Define one task around one observable behavior or one named migration or
   operational risk. Each task must:
   - cross the minimum real boundaries needed to prove that outcome;
   - leave the repository coherent and usable for its dependents;
   - have explicit prerequisites, non-goals, and preserved decisions;
   - have focused automated verification and real-interface or operational
     evidence where relevant;
   - state compatibility, rollout, rollback, and documentation effects; and
   - name evidence that forces clarification or replanning.
4. Add only genuine blocking edges. Prefer the smallest unblocked frontier;
   never serialize tasks merely because their identifiers are sequential.
5. Put the highest-value uncertainty early when it can be retired without
   inventing design. Use expand-migrate-contract tasks for wide mechanical
   migrations that cannot remain independently green as ordinary vertical
   slices.
6. Split or merge using the sizing rules. Reject horizontal layer batches,
   setup-only tasks with no independent proof, and tiny fragments whose shared
   setup and verifier cost more context than they save.

## Create Ship-Ready Plans

Copy [task-graph.json](assets/task-graph.json) and one
[task-plan.md](assets/task-plan.md) per task into a user-selected local output
directory. If none is specified, use `.ship/plans/<source-plan-slug>/`.

Make each Markdown plan independently satisfy `$ship` admission in a fresh
context. Include the source plan path and hash, only the relevant approved
decisions, repository evidence, acceptance criteria, non-goals, dependencies,
verification, operational constraints, context assessment, and replan triggers.
Do not copy exploration transcripts or rewrite the source plan wholesale.

In `task-graph.json`:

- map every source requirement and non-goal to one or more task IDs;
- record the plan file for every task;
- record sizing drivers, confidence, warning threshold, and concrete split
  triggers;
- distinguish blocking dependencies from optional sequencing; and
- leave no unowned cross-task integration or final verification obligation.

Do not create or update external tickets without separate user authorization.

## Validate and Review Granularity

Resolve `<graph-validator>` as
[scripts/validate_task_graph.py](scripts/validate_task_graph.py) relative to
this skill's source location, independent of the current working directory.
Run:

```sh
python3 <graph-validator> <output-dir>/task-graph.json
```

Fix every structural or coverage failure. Then review semantically:

- every task is independently admissible by `$ship`;
- every task has one primary observable outcome;
- all required plan content is covered without contradictory ownership;
- dependency edges form a coherent, acyclic integration path;
- no task is assessed beyond the warning threshold or with low confidence;
- the graph is the smallest set that preserves safe boundaries; and
- the source plan hash still matches.

Present task boundaries, dependencies, sizing judgments, and any proposed
split or merge for approval. Finish with validated local artifacts, not
implemented code, Ship state, external tickets, or a claim that the token
estimate is guaranteed.

## Reference Route

- [sizing.md](references/sizing.md): context policy, sizing drivers, and
  split/merge rules.
- [sources.md](references/sources.md): evidence strength, provenance, and local
  evaluation measures.
