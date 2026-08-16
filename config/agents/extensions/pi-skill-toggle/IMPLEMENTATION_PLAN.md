# Pi Skill Toggle Implementation Plan

## Purpose

This document hands off the recommended P0–P2 improvements to a follow-up
implementation session. It compares the current extension with Dillon Mulroy's
`pi-skill-toggle` and `pi-skillful`, then translates the useful ideas into a
phased implementation plan.

The extension currently provides persistent control over instruction files and
skills advertised to Pi's model. Its existing safety properties must be
preserved:

- policy is stored outside `AGENTS.md`, `CLAUDE.md`, and `SKILL.md`;
- state updates use locking and atomic replacement;
- running sessions refresh policy before each model turn;
- manual skill invocation remains available;
- source-level `disable-model-invocation: true` remains authoritative;
- prompt-format drift is reported rather than silently ignored.

## Research summary

### Dillon Mulroy's implementation

Dillon's extension uses an explicit workflow:

1. discover skill files;
2. build an inventory with diagnostics;
3. stage desired invocation modes in an overlay;
4. plan exact file changes;
5. reread each file to detect concurrent modification;
6. write changes atomically;
7. report applied changes and errors;
8. reload Pi.

Its strongest ideas are staged planning, conflict detection, structured apply
results, and rich source diagnostics. Its central persistence mechanism is not
suitable here because it adds or removes `disable-model-invocation` directly in
`SKILL.md`.

Primary source:
<https://github.com/dmmulroy/.dotfiles/tree/main/home/.pi/agent/extensions/pi-skill-toggle>

### `pi-skillful`

`pi-skillful` layers skill visibility across global, trusted-project, and
session state. Project settings inherit global settings until a real override
is introduced, and redundant project overrides are removed. It also supports
session-only visibility toggles and uses Pi's canonical `sourceInfo` metadata.

Its strongest ideas are layered policy, sparse inheritance, normalized
configuration, serialized UI saves, and source-aware presentation. Its regular
expression prompt replacement, non-atomic settings writes, private Pi UI patch,
and editor wrapping should not be copied.

Primary source:
<https://github.com/jvm/pi-mono/tree/main/packages/pi-skillful>

## Target design

The target combines:

- the current extension's external, locked, atomic state;
- `pi-skillful`'s global, directory, and session policy layers;
- Dillon's draft, plan, apply, and report workflow.

It must not add source mutation, independent skill discovery, skill creation or
deletion, inline skill expansion, usage analytics, or private Pi UI patches.

### Policy precedence

For a skill, resolve visibility in this order:

1. source-level `disableModelInvocation` forces `manual-only` and is read-only;
2. session override;
3. directory override;
4. global policy;
5. default `visible`.

For an instruction file, resolve visibility in this order:

1. session override;
2. directory policy;
3. default `included`.

Directory and session values need an explicit `inherit` state so redundant
overrides can be removed.

### Proposed policy types

The exact names may change during implementation, but the domain model should
be explicit:

```ts
type SkillVisibility = "visible" | "manual-only";
type InstructionVisibility = "included" | "excluded";
type Override<T> = T | "inherit";
type PolicyScope = "global" | "directory" | "session";

interface EffectivePolicy {
  cwd: string;
  instructions: EffectiveInstructionPolicy[];
  skills: EffectiveSkillPolicy[];
}

interface EffectiveSkillPolicy {
  name: string;
  visibility: SkillVisibility;
  sourceLocked: boolean;
  resolvedFrom: "source" | PolicyScope | "default";
}
```

### Deep policy module

Introduce one policy module that hides storage shape, inheritance, and
precedence behind a small interface. The UI, status formatter, and prompt
transformer must consume the same resolved policy.

A suitable initial interface is:

```ts
interface SkillPolicy {
  resolve(input: PolicyInput): EffectivePolicy;
  plan(scope: PolicyScope, draft: PolicyDraft): PolicyPlan;
  apply(plan: PolicyPlan): ApplyResult;
  reset(scope: PolicyScope, cwd: string): ApplyResult;
}
```

Do not expose migration fields, lock details, or persisted JSON structure
through this interface. The file store remains an internal adapter exercised
through temporary-directory tests.

## P0 — Correctness and robustness

### P0.1 Prevent stale policy after refresh failure

#### Problem

`restore()` currently catches a store-loading error but leaves `current`
unchanged. `before_agent_start` then continues and can filter with a snapshot
from a previous directory or session.

#### Implementation

- Make refresh return an explicit success or failure result.
- Associate every loaded snapshot with its canonical directory identifier.
- Never apply a snapshot belonging to another directory after refresh fails.
- On failure, leave the current prompt unchanged and display `skills !`.
- Notify only when the failure changes, avoiding repeated notifications per
  model turn.
- A last-known-good snapshot may be reused only when it belongs to the same
  directory and state generation.
- Clear or replace stale status state during session shutdown and replacement.

Suggested result shape:

```ts
type PolicyRefreshResult =
  | { ok: true; policy: EffectivePolicy; generation: string }
  | { ok: false; error: Error };
```

#### Acceptance criteria

- A malformed state file never causes another directory's exclusions to apply.
- Failed refresh leaves the prompt unchanged.
- The footer shows `skills !` while refresh is unhealthy.
- Recovery on a later turn clears the failure state.
- The same error does not notify on every turn.

### P0.2 Centralize effective policy resolution

#### Resolution problem

Policy meaning is currently spread across `state.ts`, `settings.ts`, and
`index.ts`. Adding scopes without a single resolver would duplicate precedence
and inheritance rules.

#### Resolver implementation

- Add a pure effective-policy resolver.
- Move all global, directory, session, and source precedence into it.
- Include provenance (`resolvedFrom`) in resolved entries.
- Have settings items, status output, and prompt filtering consume its result.
- Keep source-manual skills read-only at every scope.
- Normalize names and canonical paths at the module seam.

#### Resolver acceptance criteria

- One test table covers every precedence combination.
- UI values and prompt filtering cannot disagree about effective visibility.
- Returning a directory value to `inherit` removes its persisted override.
- Source-manual skills cannot become model-visible through any override.

### P0.3 Add lifecycle and concurrency integration tests

Add tests at the extension's behavioral seam for:

- startup, new session, resume, fork, tree navigation, reload, and shutdown;
- a state-load failure after changing directories;
- healthy, failed, and recovered footer transitions;
- notification deduplication;
- source-manual skills under every override scope;
- multiple store instances applying independent deltas;
- stale-lock recovery and lock timeout behavior;
- atomic temporary-file cleanup after a failed write;
- prompt filtering after an earlier extension modifies unrelated prompt text;
- context-only and skill-only prompt format changes;
- symlinked working directories and resource paths.

Prefer assertions on public behavior and persisted output over private helper
state.

### P0.4 Preserve prompt drift detection

Continue exact section replacement rather than adopting `pi-skillful`'s broad
regular expression. Isolate Pi-specific rendering in one internal prompt
adapter and add fixtures for the installed Pi version.

The adapter should return section-specific failures:

```ts
interface PromptPolicyResult {
  systemPrompt: string;
  failures: Array<"instructions" | "skills">;
}
```

If Pi later exposes structured system-prompt option replacement, migrate this
adapter to that interface and remove private prompt rendering.

## P1 — Policy capability and user experience

### P1.1 Add directory skill overrides

#### State migration

Migrate the current global `hiddenSkillNames` values into global skill policy.
Add sparse directory overrides keyed by canonical directory identifier.

An illustrative version 3 shape is:

```ts
interface StoredStateV3 {
  version: 3;
  globalSkillPolicy: Record<string, SkillVisibility>;
  skillPolicyByDirectory: Record<
    string,
    Record<string, SkillVisibility>
  >;
  instructionPolicyByDirectory: Record<
    string,
    Record<string, InstructionVisibility>
  >;
  migratedLegacySessionIds: string[];
}
```

The final representation may use sorted arrays instead of records if that makes
normalization and migration simpler. Preserve deterministic serialization.

#### Behavior

- Global policy remains the default across directories.
- A directory can explicitly make a globally hidden skill visible.
- A directory can explicitly make a globally visible skill manual-only.
- Resetting a directory skill to `inherit` deletes the override.
- Policies remain keyed by skill name so source paths can move.

### P1.2 Add session-only overrides

Keep session state in memory; do not persist it in the JSON store or session
history unless a later requirement explicitly asks for resume behavior.

Initial UX should expose session scope through `/skill-toggle`. Do not wrap the Pi
editor or implement numbered hotkeys in this phase.

Define lifecycle behavior explicitly:

- fresh startup: no session overrides;
- `/new`: decide and test whether overrides reset or carry within the process;
- resume, fork, clone, reload, and process restart: reset by default;
- source-manual skills remain locked.

### P1.3 Add scope-aware context UI

The `/skill-toggle` UI should make scope explicit rather than mixing settings with
hidden persistence rules.

Recommended scopes:

- **Global** — skills only;
- **Directory** — instruction files and skill overrides;
- **Session** — temporary instruction and skill overrides.

Each row should show:

- effective value;
- value in the selected scope;
- resolution source (`source`, `session`, `directory`, `global`, or `default`);
- canonical Pi provenance from `sourceInfo`;
- full path;
- whether the row is read-only.

Use Pi's `sourceInfo.scope`, `sourceInfo.origin`, `sourceInfo.source`, and path
instead of introducing custom discovery or source classification.

### P1.4 Add staged review and structured apply results

Keep editing isolated in a draft. Before persistence, produce a plan containing
exact transitions:

```text
research    global: visible -> manual-only
deploy      directory: inherit -> visible
AGENTS.md   directory: included -> excluded
```

Use structured results:

```ts
interface ApplyResult {
  applied: PolicyChange[];
  skipped: PolicyChange[];
  errors: PolicyError[];
}
```

The store currently commits one state file atomically, so partial application
should be exceptional. The result model still improves reporting and keeps the
command independent of persistence details.

### P1.5 Add bulk operations

Within the active search/filter and selected scope, support:

- make skills visible;
- make skills manual-only;
- include instructions;
- exclude instructions;
- reset selected resources to inherit;
- reset the entire active scope.

Every bulk operation must stage a draft and use the same review/apply path as an
individual change.

### P1.6 Improve status output

Extend `/skill-status` to distinguish persistent and temporary policy:

```text
Directory     /work/project
Instructions  2 included · 1 excluded
Skills        6 visible · 3 manual-only
Resolved      1 source · 1 session · 2 directory · 5 global
Overrides     directory 3 · session 1
```

Continue listing hidden skills that are not loaded in the current directory.
Do not expose full instruction contents or other sensitive prompt data.

## P2 — Maintainability and integration

### P2.1 Separate Pi adapters from policy implementation

Keep Pi-specific concerns at narrow seams:

- prompt adapter: exact system-prompt section replacement;
- resource adapter: converts `BuildSystemPromptOptions` into policy resources;
- UI adapter: converts effective policy into `SettingItem` rows;
- state adapter: locked and atomic JSON persistence.

The policy module should not import TUI classes or know Pi's prompt text format.
The prompt adapter should not know persisted state structure.

Avoid splitting every pure helper into its own shallow module. The goal is a
small policy interface with inheritance, normalization, planning, and
provenance hidden behind it.

### P2.2 Use canonical resource provenance

For skills, retain:

- name;
- description;
- `filePath`;
- `sourceInfo.path`;
- `sourceInfo.scope`;
- `sourceInfo.origin`;
- `sourceInfo.source`;
- source-level manual-only policy.

For instruction files, retain canonical path and classify user, project, or
inherited context in one resource adapter.

Policy identity remains skill name for skills and canonical path for instruction
files. Provenance is presentation and diagnostic data, not the persisted key.

### P2.3 Strengthen state recovery diagnostics

Keep strict validation: malformed state must not silently reactivate resources.
Improve the error message with:

- state path;
- unsupported or malformed version;
- recovery command or manual action;
- whether a same-directory last-known-good snapshot is in use.

Optionally retain one last-known-good backup after successful writes, but only
if recovery behavior is specified and tested. Do not silently replace corrupt
state with an empty policy.

### P2.4 Rename before publication

The npm package name `pi-skill-toggle` is already owned by another project. If
this extension is published, select a scoped or distinct package name. The
runtime command and state migration can remain backward-compatible.

The extension now controls more than skills, so candidate names should reflect
context policy rather than only skill toggling.

### P2.5 Document invariants beside the implementation

Add a short maintainer section to the extension README or an `AGENTS.md` in the
extension directory with these invariants:

- never edit instruction or skill source files;
- never override source-manual policy;
- never apply policy from another directory after refresh failure;
- preserve manual `/skill:name` invocation;
- persist sparse overrides and remove redundant inheritance entries;
- serialize state deterministically;
- report prompt drift visibly;
- use canonical Pi resources rather than independent discovery.

## Suggested implementation sequence

1. Add regression tests for stale policy after refresh failure.
2. Fix refresh failure handling without changing the state schema.
3. Introduce resource and effective-policy types.
4. Move current policy resolution behind the policy module.
5. Add table-driven precedence tests.
6. Add state version 3 and migrate existing global skill and directory
   instruction settings.
7. Add directory skill overrides.
8. Add scope-aware UI and staged plan reporting.
9. Add in-memory session overrides and lifecycle tests.
10. Add bulk operations and expanded status output.
11. Isolate and fixture-test the Pi prompt adapter.
12. Update README documentation and run all extension checks.

Make and verify each coherent step independently. Preserve state compatibility
throughout; do not combine schema migration, UI redesign, and lifecycle changes
in one unverified edit.

## Validation commands

From `config/agents/extensions/pi-skill-toggle`:

```bash
npm run check
npm test
```

From the repository root after each coherent implementation increment:

```bash
darwin-rebuild build --flake .
```

For Markdown changes:

```bash
npx --yes markdownlint-cli2 \
  --config config/markdownlint-cli2.yaml \
  "**/*.md" \
  "#node_modules" \
  "#.claude/skills"
```

## Completion criteria

The plan is complete when:

- stale policy cannot cross a directory or session refresh failure;
- one policy resolver owns source, global, directory, and session precedence;
- directory skill overrides inherit from global policy sparsely;
- session overrides work without persistence or source mutation;
- the UI displays scope and canonical provenance;
- draft, plan, apply, and report are separate observable stages;
- prompt drift remains visible and section-specific;
- lifecycle, migration, concurrency, and prompt integration tests pass;
- existing state migrates without changing effective behavior;
- no `AGENTS.md`, `CLAUDE.md`, or `SKILL.md` file is modified by the extension.
