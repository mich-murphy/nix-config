# Pi Skill Toggle

Controls which user-managed instruction files and skills Pi advertises to the
model.

## Command

Run `/skill-toggle` to open one settings list. Changes apply immediately and
persist across projects and sessions.

Resources are ordered by hierarchy:

1. Global instructions
2. Global skills
3. Project and inherited instructions
4. Project skills

Each row is labelled `[global]` or `[project]`. Origin affects ordering and
explanation only. Every editable instruction uses `enabled` or `disabled`, and
every editable skill uses the same values.

Pi may load `AGENTS.override.md`, `AGENTS.md`, or `CLAUDE.md`. The extension
shows whichever files Pi loaded. It includes only user-managed global and
project resources. Package, extension-provided, internal, and temporary CLI
skills are outside its scope.

A disabled skill remains available through `/skill:name`; the extension only
removes it from automatic model discovery. A skill that declares
`disable-model-invocation: true` already requires explicit invocation, so it
appears as a read-only `manual only` row.

## State

Disabled resources are stored by absolute discovery path in
`~/.pi/agent/pi-skill-toggle.json`, or the agent directory selected by Pi's
configuration. Paths prevent collisions between projects or same-named skills.

Version 3 and older state is discarded when first loaded. Version 4 keeps only
disabled resources. Writes use a cross-process lock and atomic replacement.
Settings for existing files remain unchanged. Entries whose source path no
longer exists are removed during a successful state load.

## Failure behavior

If state cannot be loaded, the prompt remains unchanged. Prompt replacement is
exact and section-specific. If Pi changes the relevant prompt format while a
resource is disabled, the extension reports the affected section instead of
silently claiming success.

## Maintainer invariants

- Never edit an instruction file or `SKILL.md`.
- Never override source-level `disable-model-invocation`.
- Preserve manual `/skill:name` invocation.
- Use Pi's loaded resources rather than independently discovering skills.
- Keep global resources before project resources in the menu.
- Identify resources by path, never by display or project name.
- Preserve unrelated state during updates and cleanup.
