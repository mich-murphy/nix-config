# Step 1: housekeeping

Step 1 of `design/redesign.md` Section 13. Read that document's Sections 1, 3,
12 and 13, and `design/behaviours.md`, before starting. Do not begin any later
step.

## Goal

`controller/target/` is 3.8 GB and lives inside the skill tree. It contains
ordinary build output, a vendored rustup toolchain under
`target/maintenance/rustup-home`, and maintenance snapshots under
`target/maintenance/controller-209-*`. After this step the skill tree contains
no build output and no toolchain, and the build and quality gate still pass.

## Work

1. Establish how `xtask quality` obtains the Rust toolchain pinned by
   `controller/quality-gates.json` (`rust_version: 1.98.0`) and the
   `rust-code-analysis` binary. Read `controller/xtask/src/*.rs` and
   `references/rust-quality-gates.md`. Report what you find before deleting
   anything.
2. Add `controller/.cargo/config.toml` with a `target-dir` outside the skill
   tree, for example `/home/michael/.cache/holy-ai-agents-batman/target`.
   Prefer this over requiring every caller to set `CARGO_TARGET_DIR`. If the
   xtask or the toolchain resolution needs a home outside the tree too, give it
   one under the same cache root and make the change in the xtask.
3. Delete `controller/target/`. The maintenance snapshots are historical copies
   of files already in the tree; do not preserve them.
4. Update `SKILL.md` and `references/rust-quality-gates.md` wherever they state
   or imply the build location, so they describe the new layout.
5. Verify: `cargo build --release --locked --manifest-path controller/Cargo.toml`
   and `cargo run --locked --manifest-path controller/Cargo.toml -p xtask -- quality`
   both succeed from a clean cache. Then `cargo test --locked` for the
   controller crate. Record the commands and their outcomes in your final report.
6. Commit on `main` with one commit. The repository was initialised at
   `Baseline before controller redesign`; `controller/target/` is already
   ignored at the root.

## Constraints

- No changes to Rust source outside `xtask/`, and none there beyond toolchain
  or path resolution.
- No changes to `design/`.
- No `nix`, `rustup`, or package installation without reporting first and
  stopping. If the pinned 1.98.0 toolchain cannot be obtained without the
  vendored copy, stop and report; do not lower the pin.
- Names: short, saying what the thing is. See `redesign.md` Section 17 rule 5.

## Report

When finished, reply with: the commit hash, the finding from item 1, the
du -sh of the skill tree, each verification command with pass or fail, and
anything you were unable to do.
