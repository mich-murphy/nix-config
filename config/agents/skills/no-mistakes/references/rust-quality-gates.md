# Rust quality gates

Use this contract when modifying the bundled controller. For a Rust task delivered
by the epic workflow, first use that crate's repository-owned gates and toolchain.
Apply the behavioral-test and maintainability review below to its acceptance.
Introducing repository-wide lint or CI policy belongs in an explicitly scoped
policy change; an ordinary product task does not authorize changing those gates.

## Controller command

From any working directory, run the complete gate after changing controller
source, dependencies, tests, the Rust xtask or policy:

```sh
"$EPIC_SKILL/controller/quality.sh"
```

The Cargo workspace contains the controller and a Rust `xtask` gate runner.
`controller/env.sh` derives the cache root as
`${XDG_CACHE_HOME:-$HOME/.cache}/no-mistakes`, exports its `target`
directory as `CARGO_TARGET_DIR`, and prepends its pinned toolchain to `PATH`.
The xtask independently derives the same paths and reads the Rust version from
`controller/quality-gates.json`. It invokes Cargo, rustc, rustfmt and Clippy from
that toolchain. The full controller gate runs on Linux x86_64 so process
ownership and crash-recovery tests actually execute. The xtask uses `curl` and
`tar` to download the pinned Mozilla analyzer once under the same cache root.
Rust verifies archive and executable SHA-256 digests before installing it and
verifies the cached executable on every run. Python is not required. No model
calls, live Jira/GitHub mutations or release operations occur.

The complete gate fails on any failed stage:

| Gate | Enforced behavior |
| --- | --- |
| `cargo fmt --all --check` | Rust formatting matches the toolchain across the workspace. |
| Clippy, all targets and features, `--locked`, `-D warnings` | No compiler or enabled Clippy warnings. Cargo lints also reject debug macros, unfinished placeholders, ignored must-use results and undocumented unsafe blocks. |
| Cyclomatic complexity | Every production function and closure has direct complexity at most 20. No baseline exceptions or average-score escape. |
| Nesting and function size | Clippy limits nesting to 5 and function length to 120 lines under its counting rules, including test targets. |
| Gate behavior tests | Rust integration tests drive the xtask CLI. The actual analyzer accepts simple code and the exact threshold, and rejects excessive branches, excessive closure complexity, malformed Rust and an empty source scope. |
| Controller tests, all targets and features | Behavioral scenarios exercise the real executable, Git, SQLite, subprocesses and recovery boundaries. |
| Documentation tests | Runnable documentation examples compile and pass when present. |
| Locked release build | The production executable builds without silently updating dependency resolution. |

`controller/quality-gates.json` pins tool versions, analyzer checksums and the
cyclomatic threshold. `controller/clippy.toml` owns nesting and length limits;
`controller/Cargo.toml` owns the enabled Rust/Clippy lints. A toolchain or threshold
change needs explicit quality-policy review and a complete gate run. Do not add
lint suppressions, increase limits, omit files or move logic into opaque macros
just to make a check pass. There are no current complexity exceptions.

For a diagnostic scan while editing:

```sh
"$EPIC_SKILL/controller/quality.sh" --complexity-only
```

This is not a complete quality pass. An existing exact pinned analyzer can be
supplied with `--analyzer /absolute/path/rust-code-analysis-cli`; its checksum is
still checked. Gate output names the failed function and its source location.

## What complexity measures

Mozilla rust-code-analysis parses Rust syntax instead of counting text matches.
Its per-space totals include child functions. The gate subtracts immediate child
space totals to measure each function's own control flow, then checks every child
function and closure independently. A complex closure cannot hide behind a simple
parent. The scan covers every `.rs` file below `controller/src`, including nested
modules, and every `.rs` file below `controller/xtask/src`. Workspace formatting,
Clippy and tests cover the gate runner too. Missing results, parse errors or an
empty scan fail closed. Rust macros
are not expanded by this analysis; compiler checks, Clippy and design review still
matter. [Mozilla analyzer](https://github.com/mozilla/rust-code-analysis)

The complexity limit is this controller's review policy, not a mathematical proof
of maintainability. Clippy explicitly cautions against treating its
`cognitive_complexity` lint as a cognitive measurement and suggests nesting and
function-size checks instead. We use those checks and the separate explicit
cyclomatic metric. [Clippy lint guidance](https://rust-lang.github.io/rust-clippy/stable/index.html#cognitive_complexity)

When a function exceeds a limit, separate actual responsibilities: command
routing, policy validation, state transition, external invocation or observation.
Keep transaction boundaries and operation ordering explicit. Prefer small named
handlers over adding a framework or generic indirection. The reviewer must check
that extraction improves readability across calls, not only the number reported
for one function.

## Behavioral test review

For each changed behavior, the implementer and independent reviewer identify the
real failure the test detects and its externally observable result. Record this
mapping in the task's evidence or controller-change summary. A green test command
alone does not establish that the tests cover the intended behavior.

- Drive normal setup and commands through the JSON CLI. Assert command results,
  permitted or rejected transitions, actual effects and documented status fields.
  Public domain-contract tests may supplement those process tests.
- For rejection cases, establish valid preceding state and verify the forbidden
  effect did not happen. Pair them with successful behavior when needed to show
  that a missing prerequisite is not causing every request to fail.
- Exercise restarts, replay, stale evidence, cross-run ownership, concurrent
  claims and exhausted budgets at their real persistence or process boundary.
  Fault injection into retained state is appropriate for crashes, old schema
  versions and external observations that normal commands cannot manufacture.
  Document why each such fixture needs it.
- Fake external GitHub and Codex executables to control their responses without
  model billing or live mutation. Their arguments, input, terminal receipts and
  side effects are integration contracts, so assertions about those are valid.
  Do not mock private Rust helpers or assert an internal helper's invocation order.
- Avoid tests for private layout, incidental struct construction, function names,
  exact full error wording, source-text patterns or snapshots of entire internal
  state. Counters and deadlines exposed by `status` are contractual behavior.
- A refactor preserving the public contract should leave scenario expectations
  intact. If expectations must change, explain the intended behavior change or
  remove the accidental coupling before accepting the refactor.

No numeric coverage threshold can prove this review was done. Do not add tests
that repeat assignments or compute expected answers using the implementation.
For consequential untested branches, add a real regression case. Use targeted
mutation experiments when a surviving incorrect behavior is uncertain, within
the task's validation budget, rather than treating a large mutation campaign as
an automatic gate for every change.

The controller suite proves its local enforcement boundaries. It does not prove
live connector availability, business correctness of a delivered epic, or actual
human acceptance. Those remain the run's evidence requirements.
