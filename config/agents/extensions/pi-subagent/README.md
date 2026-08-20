# pi-subagent (vendored)

Vendored verbatim from the pi 0.84.2 example extension:

```text
/opt/homebrew/Cellar/pi-coding-agent/0.84.2/libexec/lib/node_modules/@earendil-works/pi-coding-agent/examples/extensions/subagent/
```

Files copied byte-for-byte: `index.ts`, `agents.ts` (verified via `diff` at
vendor time — identical). No deviations.

Deliberately NOT vendored from the example:

- `prompts/` — workflow prompt presets (`/implement`, `/scout-and-plan`,
  `/implement-and-review`). Out of scope for this change.
- `agents/` — the example's sample agent definitions (`scout.md`,
  `planner.md`, `reviewer.md`, `worker.md`). This repo provides its own
  user-scope agent definition(s) under
  `config/agents/pi-agents/` instead, wired separately via
  `home/coding-agents.nix` into `~/.pi/agent/agents/`.

See the upstream README (same directory in the pi install) for the full
feature description, security model, and usage docs — not duplicated here to
avoid drift from upstream.

To re-vendor after a pi upgrade, diff this directory against the new
examples/extensions/subagent/{index.ts,agents.ts} and update deliberately.
