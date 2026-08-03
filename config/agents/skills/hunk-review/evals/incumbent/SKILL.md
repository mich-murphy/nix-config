---
name: hunk-review
description: Inspect and control live Hunk diff-review sessions through the non-interactive `hunk session` CLI. Use when the user has Hunk running, wants an interactive diff walkthrough, or wants inline Hunk comments. The daemon uses localhost, so sandboxed agents must request network or sandbox access before discovering sessions.
---

# Review a Live Hunk Session

Use only `hunk session ...` commands. The TUI belongs to the user; never launch
interactive `hunk diff` or `hunk show` commands yourself.

## Route Model and Effort

Use a balanced model at low effort for discovery, navigation, and structured
session inspection; use medium effort when reviewing patches or drafting
comments. Escalate to a frontier model at high effort only for security,
data-loss, concurrency, release-critical, or nonlocal review risk. Treat the
session state and focused diff as the verifier.

## Connect to Hunk

Hunk's daemon uses localhost. From a sandboxed harness, run discovery outside
the sandbox on the first attempt and request reusable approval for the
`hunk session` prefix when supported:

```bash
hunk session list --json
```

- Do not trust a sandboxed empty list. If the direct or approved call is empty,
  ask the user to open Hunk.
- Match by `Repo`; ask when several sessions fit.
- Reuse the exact session ID and daemon access for later commands.

## Inspect, Then Act

```bash
hunk session get <id> --json
hunk session review <id> --json
```

1. Use `get` to confirm the repository, window path, and loaded source.
2. Use `review --json` before adding `--include-patch` for code that needs
   inspection. Use `context <id> --json` only when current focus matters.
3. Navigate, reload, or comment only as requested. Finish with a short summary.

## Common Actions

```bash
hunk session navigate <id> --file src/App.tsx --hunk 2
hunk session reload <id> -- diff
hunk session comment add <id> --file README.md --new-line 103 \
  --summary "Tighten this wording" --rationale "Explain why" --focus
hunk session comment apply <id> --stdin
```

- Navigation takes exactly one of `--hunk`, `--old-line`, or `--new-line`;
  hunk and line numbers are 1-based.
- Use `comment add` for one focused note and `comment apply` with JSON on stdin
  for a prepared batch.
- Put `--` before the nested `diff` or `show` command in `reload`.
- Do not remove or clear human comments unless the user explicitly asks.
- Use `hunk session --help` for uncommon operations instead of guessing flags.
