---
name: skill-development
description: Design, create, revise, evaluate, secure, and release reusable agent skills from observed successful workflows or recurring reviewed failures. Use when deciding whether behavior belongs in a skill, retrieval source, script, tool, hook, or repository instruction; defining skill triggers and progressive disclosure; creating per-skill evaluations; optimizing model, effort, latency, or context; promoting reviewed traces into regression cases; auditing a skill package; or moving a skill through alpha, beta, release-candidate, and stable stages.
---

# Develop an Evidence-Backed Skill

## Establish the case

1. Name the demonstrated successful workflow or recurring reviewed failure.
2. Record its source, frequency or severity, current workaround, owner, and an
   observable accepted outcome in `assets/templates/proposal.json`.
3. Reject a speculative skill when no inspected evidence shows a reusable job.
   Gather examples or improve the interface first.
4. Read [evidence-and-containers.md](references/evidence-and-containers.md) to
   decide whether the control belongs in a skill, retrieval, executable
   mechanism, hook, or repository instruction.

## Define one operational contract

Write one job, discriminative positive and negative triggers, explicit inputs
and outputs, a safe default, forbidden effects, and observable completion
checks. Make boundary examples realistic enough to distinguish the skill from
nearby skills. Keep ambiguous acceptance and consequential design with the
strongest responsible agent or human owner.

Read [design-and-disclosure.md](references/design-and-disclosure.md) before
adding resources. Keep discovery metadata precise, `SKILL.md` operational, and
branch-specific references, scripts, and assets one direct link away. Treat
500 lines as a warning. Apply the deletion test to duplication, sediment,
sprawl, generic knowledge, and behavioral no-ops.

## Build the package

Use the harness's official skill initializer when available. Then run:

```sh
python3 scripts/scaffold_package.py /path/to/skill --proposal proposal.json
```

Keep deterministic mechanics in scripts so agents execute them without loading
their implementation. Keep the portable behavioral core vendor-neutral and
put Codex, Claude, and Pi invocation or configuration in `agents/` or the eval
route manifest—not portable frontmatter.

Read [routing-and-latency.md](references/routing-and-latency.md) when selecting
a semantic model lane, reasoning effort, delegation boundary, or latency
experiment. Change one routing variable at a time and hold the task, tools,
permissions, environment, and verifier fixed.

## Audit before executing or releasing

Inventory every bundled file, dependency, permission, external fetch, and
third-party source. Run:

```sh
python3 scripts/audit_package.py /path/to/skill
python3 scripts/package_hash.py /path/to/skill
```

Inspect every finding; a green structural audit does not establish behavioral
quality or third-party trust. Read [security-and-provenance.md](references/security-and-provenance.md)
for the release audit.

## Evaluate and release

Read [evaluation-and-release.md](references/evaluation-and-release.md). Keep the
cases, positive and negative routing prompts, held-out split, cross-harness
routes, runner, comparator, no-skill baseline, incumbent and candidate results,
release decision, and limitations under this skill's own `evals/` directory.

Evaluate routing, forced conditional efficacy, and automatic end-to-end utility
separately. Use fresh sessions and clean snapshots. During development run
three repetitions of no-skill, incumbent, and candidate on Codex, Claude, and
Pi; use five repetitions per held-out release case. Preserve all valid runs and
separate task failure from harness, environment, telemetry, and evaluator
failure.

Start at alpha with explicit invocation. Promote only through the predeclared
quality-first gate in `assets/templates/release-decision.json`. Freeze every RC;
any behavioral change creates another RC and full held-out replay. Preserve
concrete misses as regression cases. Use observed resource navigation to move,
rewrite, or delete content.

## Finish

Finish only when the package contract is clear, deterministic checks pass,
cross-harness evidence and invalid runs are retained, security and provenance
are reviewed, the owner records a release decision and limitations, and every
claim is limited to the tested task/model/harness matrix.
