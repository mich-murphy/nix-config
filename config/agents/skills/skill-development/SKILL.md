---
name: skill-development
description: Design, create, revise, evaluate, instrument, secure, and release reusable agent skills from observed successful workflows or recurring reviewed failures. Use when deciding whether behavior belongs in a skill, retrieval source, script, tool, hook, prompt, or repository instruction; creating or updating a skill package; defining triggers and progressive disclosure; setting up skill telemetry and tracing; selecting model and reasoning-effort lanes; comparing a candidate with no-skill or built-in skill creators; promoting reviewed traces into regression cases; or making an evidence-backed release decision. Do not use for one-off task instructions, speculative workflows with no inspected examples, or merely installing an existing skill.
---

# Develop an Evidence-Backed Skill

Build the smallest reusable control that demonstrates benefit on the tasks and
harnesses where it will run. Treat loadability as a structural check, not proof
that the skill helps.

Read [corpus-evidence.md](references/corpus-evidence.md) before making a
material design, telemetry, model-route, or release recommendation. Treat the
packaged evidence boundary as the local source of truth; follow its public
source links only when a time-sensitive product fact needs verification.

## Establish the Case

1. Name the demonstrated successful workflow or recurring reviewed failure.
2. Record its source, frequency or severity, workaround, owner, and observable
   accepted outcome using `assets/templates/proposal.json`.
3. Reject a speculative skill when no inspected evidence shows a reusable job.
   Gather examples or improve the interface first.
4. Read [evidence-and-containers.md](references/evidence-and-containers.md) to
   decide whether the control belongs in a skill, retrieval, executable
   mechanism, hook, repository instruction, or one task's prompt.

## Define One Operational Contract

Write one job, discriminative positive and negative triggers, explicit inputs
and outputs, a safe default, forbidden effects, and observable completion
checks. Keep consequential ambiguity with the strongest responsible agent or a
human owner.

Read [design-and-disclosure.md](references/design-and-disclosure.md) before
adding resources. Keep discovery metadata precise, `SKILL.md` operational, and
branch-specific references, scripts, and assets one direct link away. Apply the
deletion test to duplication, generic knowledge, sediment, sprawl, and
instructions that do not change observed behavior.

## Build the Package

Use the current harness's official initializer when one is available. Then run:

```sh
python3 scripts/scaffold_package.py <skill-directory> --proposal proposal.json
```

Resolve `scripts/scaffold_package.py` relative to this `SKILL.md`, not the task
working directory. Create the proposal from the adjacent
`assets/templates/proposal.json` schema and keep the scaffold's canonical
filenames; automation depends on them. Do not substitute prose reports such as
`RELEASE.md` or `telemetry-policy.md` for the machine-readable JSON contracts.

The scaffold adds the proposal, owned evaluation skeleton, privacy-first
telemetry policy, release record, and result-status file without overwriting
existing artifacts. Replace its example cases and route placeholders with the
real contract before evaluation.

Keep deterministic mechanics in scripts so agents execute them without loading
their implementation. Keep the portable behavioral core vendor-neutral; put
Codex, Claude, and Pi invocation or exact model configuration in `agents/` or
`evals/routes.json`, not portable frontmatter.

## Select Model, Effort, and Execution Topology

Read [routing-and-latency.md](references/routing-and-latency.md). Express the
skill's minimum lane as efficient, balanced, or frontier and place current
model IDs in the route manifest. Change one routing variable at a time while
holding the task, tools, permissions, environment, timeout, and verifier fixed.

Use a capable quality-ceiling run before optimizing ambiguous or consequential
work. Start bounded deterministic work on an efficient lane and escalate only
when the verifier exposes a capability gap. Delegate only independent bounded
work with an explicit return contract.

## Instrument Before Release

Read [telemetry-and-tracing.md](references/telemetry-and-tracing.md). Start with
metadata-only task traces. Record offered, selected, activated, expanded,
executed, and evaluated skill lifecycle events plus exact package, harness,
model, prompt, tool, case, and evaluator versions. Keep content capture off by
default and separate operational traces from immutable evaluation cases.

Do not infer quality from activation count. Join each valid run to an accepted,
failed, invalid, or delayed outcome and report tokens, cost, latency, retries,
tool and permission waits, and human rework per accepted task.

## Audit Before Executing or Releasing

Inventory every bundled file, dependency, permission, external fetch, and
third-party source. Run:

```sh
python3 scripts/audit_package.py <skill-directory>
python3 scripts/package_hash.py <skill-directory>
```

Inspect every finding; a green structural audit does not establish behavioral
quality or third-party trust. Read
[security-and-provenance.md](references/security-and-provenance.md) for the
release audit.

## Evaluate Against Real Controls

Read [evaluation-and-release.md](references/evaluation-and-release.md). Keep
cases, routing prompts, development and held-out splits, route manifests,
runner, comparator, raw results, release decision, and limitations inside the
skill's own `evals/` directory.

Evaluate separately:

1. automatic routing;
2. forced conditional efficacy; and
3. automatic end-to-end utility.

Compare no-skill, the current incumbent or built-in creator, and the candidate
within each harness using fresh sessions and clean workspaces. Use three
development repetitions and five held-out release repetitions. Preserve every
valid run; classify harness, environment, telemetry, evaluator, and task
failures separately.

Do not launch nested model-backed evaluation merely because a new package is
being scaffolded. When execution is outside the user's request, authorization,
or declared budget, create the runnable comparison, leave the release decision
at `defer`, and state exactly which runs remain. Structural and unit checks are
still required before handoff.

Start at alpha with explicit invocation. Promote only through the predeclared
quality-first gate. Freeze every release candidate; any behavioral change makes
a new candidate and requires held-out replay. State "unverified" outside the
tested task, model, effort, harness, and tool matrix.

## Finish

Finish only when the contract is clear, deterministic checks pass, source and
security review is complete, the candidate has credible paired evidence over
its controls, telemetry and evaluation artifacts are retained, and the owner
records a release decision with limitations. Do not claim general superiority
from one smoke run or from structural checks alone.
