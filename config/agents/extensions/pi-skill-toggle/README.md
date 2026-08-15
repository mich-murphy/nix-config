# Pi Skill Toggle

Layered control over the instruction files and skills advertised to Pi's model.

## Commands

- `/context` — choose Global, Directory, or Session scope, stage changes, review
  exact transitions, and apply them.
- `/context-status` — show effective visibility, resolution sources, persistent
  directory overrides, and temporary session overrides.
- `/context-reset [global|directory|session|all]` — reset one policy layer.
  The legacy `context` and `skills` arguments remain aliases for `directory`
  and `global`.

Global policy applies to skills in every directory. Sparse directory policy can
make a globally manual-only skill visible, make a globally visible skill
manual-only, and include or exclude instruction files. Session policy has the
same override capabilities but stays in memory and resets on `/new`, resume,
fork, clone, reload, process restart, and shutdown. Returning a directory or
session value to `inherit` removes that override.

The UI shows each resource's effective value, selected-scope value, resolution
source, canonical Pi provenance, and path. It also provides bulk rows for
skills, instructions, and scope reset. Edits remain isolated in a draft until
an exact transition plan is confirmed. Apply reports distinguish applied,
skipped, and failed transitions; a concurrent change to the same resource is
skipped instead of overwritten.

Persistent changes are stored deterministically in
`~/.pi/agent/pi-skill-toggle.json` using a lock and atomic replacement. Version
2 state, `context-control.json`, path-based state, and earlier session-local
state migrate automatically.

A skill set to `manual-only` remains available through `/skill:name`; only
automatic model discovery is hidden. Skills marked
`disable-model-invocation: true` in source are read-only and always
manual-only.

## Failure behavior

Running sessions refresh policy before every model turn. If state cannot be
loaded, the prompt is left unchanged, no snapshot from another directory is
used, the footer shows `context !`, and repeated identical failures notify only
once. Recovery on a later turn clears the failure. Prompt replacement remains
exact and section-specific; instruction or skill prompt-format drift is
reported rather than hidden.

Malformed state is never treated as empty policy. The diagnostic names the
state path and tells the user to fix or move it before running
`/context-status` again.

## Maintainer invariants

- Never edit `AGENTS.md`, `CLAUDE.md`, `SKILL.md`, or another resource source.
- Never override source-level manual-only policy.
- Never apply policy from another directory after refresh failure.
- Preserve manual `/skill:name` invocation.
- Persist sparse overrides and remove entries returned to `inherit`.
- Serialize state deterministically and update it under a lock with atomic
  replacement.
- Report prompt drift visibly and by section.
- Use Pi's canonical resources and `sourceInfo`; do not independently discover
  skills.
