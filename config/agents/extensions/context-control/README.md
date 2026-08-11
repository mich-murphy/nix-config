# Pi Context Control

Session-branch-local control over the instruction files and skills advertised to Pi's model.

## Commands

- `/context` — search and toggle loaded instruction files and model-visible skills.
- `/context-status` — show the current branch's exclusions.

Changes apply to the next prompt. They are stored as custom entries in Pi's session tree, so resuming a session restores them and different tree branches can carry different selections.

A skill set to `manual-only` remains available through `/skill:name`; only automatic model discovery is hidden. Skills already marked `disable-model-invocation: true` in their source are displayed as read-only because this extension does not override source policy.

The extension does not edit `AGENTS.md`, `CLAUDE.md`, or `SKILL.md` files and does not require `/reload` after each toggle.
