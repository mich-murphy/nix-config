# Rust quality gates

Use this contract when changing the bundled controller.

## Canonical command

```sh
"$EPIC_SKILL/controller/quality.sh"
```

`controller/env.sh` places the pinned Rust toolchain on `PATH` and moves Cargo
output under `${XDG_CACHE_HOME:-$HOME/.cache}/no-mistakes`. Always source it for
manual Cargo commands. `quality.sh` also exports `NO_MISTAKES_CONTROLLER`, the
directory the gate checks; the shared Cargo cache can serve an xtask binary
built from another checkout, and without that variable it would gate that
checkout instead. The full gate performs no model calls or live Jira or GitHub
mutations.

The gate runs:

- `cargo fmt --all --check`
- Clippy for every workspace target and feature with warnings denied
- direct cyclomatic complexity at most 10 per production function or closure
- Clippy function length at most 80 and nesting at most 3
- at most 400 lines in every Rust file
- architecture checks that forbid process execution outside `adapters` and
  `serde_json` in `domain`
- gate behavior tests and all workspace tests
- documentation tests
- `cargo build --release --locked`

`unwrap`/`expect` denial (`clippy::unwrap_used`, `clippy::expect_used`) is set
once, in the root `Cargo.toml`'s `[workspace.lints.clippy]`, and applies to
`domain`, `adapters`, `app`, and `cli` through each crate's own `[lints]
workspace = true` — library code and that crate's tests alike; `domain`'s
`lib.rs` restates it as an inner `#![deny(...)]`. The `xtask` maintenance
crate carries its own separate `[lints]` table with neither lint denied.

`quality-gates.json` pins Rust and the Mozilla analyzer with archive and binary
hashes. Complexity exceptions name an exact repository-relative path, function,
and reason. Bare function names are forbidden because they could exempt an
unrelated function added later. The current exceptions are the exhaustive
event fold (`apply`), the single command dispatcher (`execute`), the
exhaustive `ConflictReason` display, and the two next-action formatters that
turn a `NextAction` into its command tag and filled template. Do not add one
to hide mixed responsibilities.

For a quick diagnostic:

```sh
"$EPIC_SKILL/controller/quality.sh" --complexity-only
```

This does not replace the full gate.

## What the gates mean

The analyzer parses Rust and subtracts child function totals from each parent.
It checks closures independently. Missing files, parse errors, malformed output,
and empty scopes fail closed. Macros are not expanded, so compiler checks and
review still matter.

When a function exceeds a limit, separate policy validation, state transition,
external invocation, or observation. Keep transaction and operation order
visible. Do not raise thresholds, add lint suppression, omit files, or move logic
into opaque macros to pass.

## Behavioral test review

For each changed behavior, name the real failure and observable result the test
would catch.

- Drive setup through public domain decisions, app commands, the CLI, or adapter
  contracts.
- For rejection tests, first establish valid surrounding state and assert that
  the forbidden effect did not happen.
- Test restart, stale proof, ownership, concurrency, budgets, timeout, and
  recovery at their persistence or process boundary.
- Adapter tests use fake process records and canned output. They assert argv,
  input, parsed receipts, identities, and terminal results, never private helper
  order.
- Do not assert state layout, source text, schema versions, full snapshots, or a
  function name without behavior.
- A responsibility-preserving refactor should not require scenario expectation
  changes.

A green command cannot prove test quality. Challenge fixture assumptions and add
a focused mutation when it is unclear whether a test catches the defect. The
controller suite proves local enforcement only. It does not prove live connector
access, product correctness, or human acceptance.
