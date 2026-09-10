# Quality and the judge

Use this reference when a judge launch needs recovery, a verdict is questioned,
or the Prefactor Quality tab is empty. Normal delivery never calls `judge`.

## What the judge is

The judge is an LLM-as-judge launched by the controller after a task
concludes. It runs through the run's own harness on the tier's reviewer model
at medium effort, with the reviewer's read-only tool restriction, and it grades
the task from three sources:

- the ledger: the task's projected state, budget limits, every launch and its
  output, and every event from the claim onward;
- the rejection log: every controller command refused during the run, with its
  class, kept in the run's SQLite beside the ledger;
- the coordinator transcript: a digest of the conversation that drove the run
  in the task's window, located through the skill's `PreToolUse` hook.

It returns one JSON object: four scored dimensions (`outcome`, `efficiency`,
`friction`, `process`, each 0 to 100 with a rationale), an `overall` score, a
`verdict` of `pass`, `degraded`, or `fail`, a list of `friction_events`, a
summary, and `improvements` to the skill or controller. Friction is any moment
where a person or the coordinator had to do administration to keep the run
moving: clearing slots, reconciling PRs, supplying evidence, granting
authority, answering questions, recovering processes, or reinterpreting a
rejection. The list is open; the judge names new kinds as it finds them.

## How it runs

`complete` records `Completed` and then starts `controller judge --run
<run> <task>` detached, in its own process group, logging to
`<run>/judge/<task>.log`. `usage-report` does the same for every claimed task
still without a verdict. The judge writes its prompt to
`<run>/judge/<task>.prompt.md`, records `LaunchStarted` with role `judge`, and
on a valid report records `LaunchEnded` and `Judged`. A malformed report
settles the launch as failed and records no judgment; the task can be judged
again.

A judge launch is not the coordinator's active launch: `next` never asks to
monitor it, `run-agent` does not wait for it, and it holds no process claim.
Writers wait up to ten seconds for the SQLite lock, so a judge committing
beside the coordinator does not fail either side.

Each task is judged once. `judge --check` returns the proposed launch event
without running anything. `judge` rejects a task that was never claimed.

## Prefactor

The agent's schema version declares one quality schema, `delivery-quality`,
whose payload is the run rollup plus one entry per judged task. After every
verdict the controller posts the whole payload to the run's instance, so the
Quality tab shows the latest rollup and Prefactor records each change as a
quality span. A judge that lands after `usage-report` closed the instance
posts to that closed instance; if judges are still running when
`usage-report` is called, the last one to finish closes the instance instead.

The rendered summary line reads, for example:
`pass 88/100 across 3 of 3 task(s): 3 pass, 0 degraded, 0 fail; friction
72/100 (100 = none), 4 event(s), 3 avoidable`.

## Recovery

A judge that was killed leaves an unsettled launch with role `judge`. Settle
it with `recover-operation --launch <id>` like any other launch, then run
`judge <task>` again. If the trace instance never closed because a judge died,
run `usage-report` again once no judge is pending.

If the brief says the coordinator transcript is unavailable, the hook did not
record a note for the working directory the controller ran from. The hook
needs `jq` and writes to `$XDG_CACHE_HOME/no-mistakes/coordinator/` (or
`~/.cache/...`), one file per working directory. Run the controller from the
directory the session works in, and keep the skill invoked in the session that
drives the run.
