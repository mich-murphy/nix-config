# Step 1 review, round 2: changes required

Reviewed commit: `65544fd`. Every round-1 item verified: no `/home/michael`
outside `design/`, Cargo config and symlink removed, `env.sh` six lines, xtask
derives the cache root from `XDG_CACHE_HOME`/`HOME` and the version from
`quality-gates.json`, one cache root, gate passes from my shell. Two changes
remain.

## Blocking: an unsourced invocation recreates `controller/target/`

From a shell that has not sourced `controller/env.sh`, the documented command
`cargo run --locked --manifest-path controller/Cargo.toml -p xtask -- quality`
passes, but the outer `cargo` (PATH's 1.97) compiles the xtask itself into
`controller/target/debug/` (101 MB observed) before the xtask sets
`CARGO_TARGET_DIR` for its inner commands. The xtask cannot control where it
is built, so "works without sourcing" was never achievable, and the step-1
goal is defeated by the first careless invocation. Required:

1. The xtask refuses to run when `std::env::current_exe()` lies inside the
   skill tree (an ancestor equals `controller()`). Do this first in `run()`,
   before touching the cache. The error names `controller/env.sh` and the
   wrapper below.
2. Add `controller/quality.sh`: sources `env.sh`, then runs the xtask quality
   command with the manifest path derived from the script's own location.
   Under 10 lines.
3. Docs (`SKILL.md`, `references/controller.md`,
   `references/rust-quality-gates.md`) name `controller/quality.sh` as the
   gate command. The raw `cargo run -p xtask` form appears nowhere.
4. Verify from a fresh unsourced shell: the raw `cargo run ... -p xtask --
   quality` fails with the new message; after `rm -rf controller/target`,
   `controller/quality.sh` passes and `ls controller/target` reports nothing.
   Then verify the sourced path as before.

## Revert `controller/maintenance/historical-slot-reuse-schema10.md`

It is a historical receipt. Rewriting a recorded artifact path is the kind of
history edit this controller exists to prevent. The no-`/home/michael` rule
covers configuration and instructions, not records. Restore the file from
`f8a5fc8`; it is the one permitted grep hit outside `design/`.

## Delivery

Amend into the single commit again. Same report format as round 1, plus the
failing unsourced output and the passing `quality.sh` output.
