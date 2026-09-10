# Progress and feedback

Read this when preparing a task plan, recording checkpoints, or carrying lessons
between tasks.

## Plan from the original baseline

After `brief` and `bind-slot`, call `plan` before an implementer launch. Include
the smallest complete deliverable, expected files or directories, suitable local
examples, and one baseline digest for each criterion. Name extra lesson families
only when they apply. A verification delivery of work already on main also
needs a `plan`, for its baselines alone, before proof can be recorded.

Each criterion's baseline is fixed the first time a plan carries it. Later code
or verification deliveries use current main for their review snapshot but keep
those fixed baselines. Replanning may change components or examples and add a
baseline for a criterion that had none yet, such as one narrowing introduced,
but cannot rewrite one already fixed. A plan change invalidates proof and
review, not budgets.

## Checkpoints

After each terminal implementer turn, commit coherent work where possible and
call `checkpoint`; it snapshots the turn in the same command. Record a new
observation, whether it advanced the work, and one bounded next action. A new
SHA, repeated command, or rewritten summary is not progress by itself.

The controller records changed paths and diff size. Explain paths outside the
plan and assess whether they remain necessary to the same deliverable. Size is a
signal, not permission to omit acceptance. An integration commit made without
an agent launch is recorded with `snapshot` and then checkpointed; it does not
alter the stall count.

Three consecutive checkpoints without progress exhaust the default stall
budget. Replanning and resuming do not clear it. A supported new observation
clears the streak. Exhausted implementation still permits proof and review of
existing work.

## Measurements

Every automated proof entry includes a measurement against its baseline. State
what changed, the observed result, and whether it meets the target. Inventory
work compares stable identities and new violations, not only totals. Human-only
criteria use an actual human receipt instead of an automated measurement.

Changed snapshots, proof artifacts, or baseline artifacts invalidate dependent
acceptance. Preserve the old artifacts for audit. Reuse proof across deliveries
only after recording why it still applies to the current snapshot.

## Lessons

`lesson-record` stores proposed or accepted guidance with task families. Proposals
do not enter later prompts. Acceptance requires a verified source delivery and an
existing instruction or real human decision. A finding alone cannot create a new
business rule.

Use narrow family names. Add `reference-propagation` when a task changes active
references across code, configuration, or operator instructions. The plan loads
accepted lessons matching its families from the run and the optional committed
feedback file. The packet stays pinned for the task and cannot override authority,
criteria, budgets, or repository instructions.

Retire a wrong lesson with evidence from a later verified task, then add a new ID.
Do not mutate accepted guidance in place.

## Human review

Human-review mode requires `human-review` after independent PASS and before
merge. Bind the actual decision to the exact snapshot and receipt digest. A model
verdict is not a human receipt. Any change to source, criteria, proof, or snapshot
voids it. Autonomous mode has no added human review stop, but human-only product
criteria still require human evidence.
