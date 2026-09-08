# Acceptance sequencing and complete repairs

Read when a selected task needs external proof, changes a managed control, or
receives a missing-consumer or stale-reference finding. Preserve the original
acceptance criteria and operational authority.

## Establish the sequence before allocating a worker

Map each criterion to its proof, prerequisite and earliest verification point:
before merge, after merge, after a separately authorized operation, or human
acceptance. Use source and existing read-only access to determine the actual
sequence. A successful check cannot substitute for a different required proof.

If the repository requires merging a declaration before applying it, and the
task requires live readback before acceptance, this workflow's default merge
rule cannot complete that sequence. Record the conflict in the brief before
creating a PR. Present a concrete staged-delivery proposal with the code change,
remaining criterion, later operation, owner and required authority. Only a real
user decision can change delivery scope or sequencing. Until then, hold the task
and continue independent eligible work. Once authorized, use `begin-stage` and
`end-stage` under the controller contract; full criteria stay unchanged. Plan
follow-up and mandatory cleanup delivery, including its actual review allowance,
before dispatch. Do not implement an implicit merge
exception, change Jira requirements, or mark merged code Done.

## Prepare a useful hold

Distinguish missing evidence, missing access, missing authorization and a missing
business decision. Before declaring access unavailable, use existing supported
read-only discovery for the relevant account, subscription and resource. Scope
that investigation to the task. An inaccessible API does not prove the resource
is absent when another authorized read path exists.

Record the exact unmet criterion, observed state and source/time, discovery
already attempted, available tools, the next bounded action and what prevents
it. Finish authorized investigation and prepare exact proposed changes and
readbacks before asking the user. Name the instruction that actually requires
approval. An OAuth failure can require reconnection; it does not erase an
already-granted operational approval or require approving the same action again.

## Compare live controls with their managed source

For each changed identity, trust, permission or managed setting, identify:

- The declaration and its owner, including IaC, generated configuration or an
  explicitly unmanaged setting.
- The workflow's effective selector and fallback values.
- The native readback of the relevant identity, trust and permission scope.
- What a subsequent reconciliation would create, preserve or remove.
- The regression evidence and current operational documentation.

Put these references in the initial review packet. Resolve disagreement between
source and live state before claiming the control is durably repaired. Prepare
the source correction with the live proposal when both are needed. A configuration
readback proves configuration, not a successful token exchange or workload run.
Do not dispatch a deployment, apply or workload merely to strengthen evidence
unless that operation is within the user's authority.

## Repair all affected references once

When a review finds a missed consumer or stale statement, search the tracked
tree using stable identifiers and relevant alternative forms. For helper
retirement, include slash paths, basenames, dotted imports, module invocations,
tests and fixtures. For a retired capability, search its names, former trigger,
identity/resource names and linked decisions. Include package READMEs,
infrastructure comments, decision registers and operator runbooks.

Retain a compact search receipt with the searched revision and terms, affected
files and classification of active instructions, executable consumers and dated
history. Explain exclusions. Repair all task-caused contradictions found by the
search in the current round. Do not rewrite truthful historical evidence or
expand into unrelated debt. Review the complete affected set before spending
the next verification cycle. New discoveries remain valid findings; this is a
requirement to complete the investigation, not a quota or a reason to hide them.

Exercise documented diagnostics when their result supports acceptance. Include
a clean case and representative deliberate leftovers or invalid cases. Check
that the diagnostic does not match its own instructions. A prose-only file can
contain an operational procedure whose correctness needs executable proof.

Record reusable reference-search lessons under `reference-propagation` as well
as their originating task family. Add that tag to a later plan's
`lesson_families` when relevant. The controller still pins only accepted lessons;
proposals do not become binding instructions without reviewed delivery.

Before review, use the behavior/dependency coverage map and primary proof receipts
in [session continuity and proof](session-and-proof.md). Include relevant environment
and invocation variants, indirect enforcement paths, and contradictory identity
inputs. Validate the actual contract instead of duplicating its intended predicate.
