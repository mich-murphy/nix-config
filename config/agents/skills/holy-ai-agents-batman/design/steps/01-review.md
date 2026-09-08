# Step 1 review: changes required

Reviewed commit: `4272b7d`. Verified independently: tree 1.5 MB, no in-tree
`target/` or toolchain, build, gate and 104 tests pass. Item 1 finding accepted.

## Blocking

**No `/home/michael` in any tracked file.** This skill is about to move to
`~/.agents/skills/` and be managed by nix-config; a per-user absolute path in
tracked configuration breaks that immediately. Currently present in
`.cargo/config.toml`, `controller/xtask/src/main.rs`, `SKILL.md`,
`references/controller.md`, `references/rust-quality-gates.md`.

The brief's preference for `.cargo/config.toml` over `CARGO_TARGET_DIR` is
withdrawn: Cargo config does not expand `~` or environment variables, so it
cannot express "outside the tree" portably. Required shape:

1. Derive one cache root at runtime from
   `${XDG_CACHE_HOME:-$HOME/.cache}/holy-ai-agents-batman`. Nothing tracked
   contains the expansion.
2. Delete `.cargo/config.toml` and the `controller/.cargo/config.toml` symlink.
3. Provide one small sourceable script, `controller/env.sh`, that exports
   `CARGO_TARGET_DIR` and prepends the pinned toolchain's `bin` to `PATH`,
   both computed from the cache root. Keep it under 20 lines; no other logic.
4. The xtask computes the same two paths itself from the environment and the
   policy, so `cargo run -p xtask -- quality` works without sourcing the
   script. Read the toolchain version from `quality-gates.json`
   (`policy.rust_version`); there must be one source of truth for `1.98.0`.
5. The xtask invokes `cargo`, `rustc`, `rustfmt` and `clippy` from the pinned
   toolchain, not PATH. Today `cargo()` still uses PATH's 1.97 cargo to drive
   1.98 tools.
6. Docs (`SKILL.md`, both references) describe: source `controller/env.sh`,
   then the existing build command; binary at
   `$CARGO_TARGET_DIR/release/epic-control`. State the cache root by its
   `XDG_CACHE_HOME`/`HOME` expression, never by a literal path.

## Non-blocking, do in the same round

- Move the analyzer cache from `~/.cache/batman-quality` under the same cache
  root. One root, not two.
- If the pinned toolchain directory is missing, the xtask's error should say
  where it expected it and that provisioning is a separate step. Do not
  download or install it.

## Out of scope, recorded

The toolchain is a relocated tarball in the cache. Its proper home is
nix-config with a Rust overlay pinning 1.98.0. Follow-up, not this step.

## Delivery

Amend into a single commit on `main` replacing `4272b7d`
(`git commit --amend` is fine; nothing has been pushed). Re-run the three
verification commands from a fresh shell that has not sourced anything, then
again after sourcing `controller/env.sh`. Report the same items as before plus
`git grep -n '/home/michael'` output, which must be empty.
