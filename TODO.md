# TODO

Reviewed: 2026-11-16

Open work only.

## dev-01

- [ ] Before the next deploy: add Numtide's cache to dev-01's system Nix
      configuration in the machines repo (`extra-substituters =
      https://cache.numtide.com`, `extra-trusted-public-keys =
      niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g=`).
      Without it the deploy compiles codex on dev-01.

- [ ] After the deploy that moves the agent CLIs to Nix: delete the
      installers' `claude`, `codex`, `node` and `npm` in `~/.local/bin` (now
      last on `PATH`, but they still update themselves), stop the machines
      repo installing them, and check `which claude codex pi`.

- [ ] Remove the nix-config checkout on dev-01. Nothing in this repository
      reads it.

## Mac

- [ ] Bring back commit signing through 1Password's `op-ssh-sign`. The removed
      layer is at `refs/backup/pre-signing-removal/feat/git-ssh-signing`.
      First add `~/.ssh/github.pub` (mich-murphy) and `~/.ssh/github_bc.pub`
      (michaelmbc) as "Signing Key" at <https://github.com/settings/ssh/new>,
      and check `gh api /users/<name>/ssh_signing_keys` lists each.
- [ ] Check Mason in Neovim (`:Mason`, `:checkhealth mason`), since go,
      python3 and nodejs now come only from Neovim's packages. Moshi is
      verified running from the installed bundle.
- [ ] Uninstall the deprecated `agent-sessions` binary and its launchd agent by
      hand; Nix never managed them.
- [ ] After the first switch that moves the agent CLIs to Nix: check that
      `which claude codex pi mo` resolve to `/etc/profiles/per-user/`, that
      Pi's Claude SDK provider and Moshi's hooks still work, that
      `pi update --extensions` installs with Nix's npm, and that Codex shows
      no update banner. Homebrew cleanup removes the casks and the `node`
      formula they pulled in.
- [ ] Test whether skhd and yabai keep their macOS permissions across an
      upgrade. This has never been done.
- [ ] When a macOS update fixes the Ctrl+number desktop shortcuts, remove the
      two Dock-restart signals in `modules/darwin/window-management.nix` and
      the Shift+Alt+D binding in `config/skhd/skhdrc`.
- [ ] Add the unifi-controller key and the docker-host `ansible` key to
      1Password, then give each host an entry in
      `modules/home/mac/ssh.nix`. The `ansible` key is also registered on
      GitHub as `github`.
- [ ] With Tailscale up, test `ssh docker-host true` and `ssh truenas true`;
      neither entry has been tried.
- [ ] Replace the `thaw@beta` cask with `thaw` once Thaw 3 is stable; Thaw 2
      does not support macOS 27.
- [ ] After the first switch on the laptop, check in System Settings that
      Siri is off. It is written as a plain preference, which is proven only
      on a Mac where Siri had already been turned off by hand.

## Agents

- [ ] Moshi's local gateway (`127.0.0.1:24543`) takes no token. It refuses
      browser origins, but a container reaches it through
      `host.orb.internal`, and it offers terminal input. mata
      (`127.0.0.1:59733`) answers containers the same way. Decide both with
      the agent hardening work.

- [ ] Claude's skill links are a hand-written list of five in
      `modules/home/agents.nix`, so a new skill needs a Nix edit. Expose the
      whole skills repository instead, if that can be done without letting
      Claude's skill sync write into it.
- [ ] herdr-splits.nvim is installed by hand, not by Nix. Annotate is
      linked from Nix (`modules/home/herdr.nix`); the same approach would
      work for it.
- [ ] After the first deploy with the Nix-built Annotate, confirm on dev-01
      that `herdr plugin list` shows annotate at `local:/nix/store/…`, and
      that an Annotate action from a Mac attached with `herdr --remote`
      runs it.

## Updates

Next check: 2027-01-16. On or after that date, check each pin below. Bump
any that are behind, then move the date on a month.

- [ ] GitHub Actions in `.github/workflows/check.yml`, pinned to commit SHAs:
      checkout v7.0.1, determinate-nix-action v3.24.2, flakehub-cache-action
      v3.24.2. For each, take the latest release's commit
      (`gh api repos/<owner>/<repo>/commits/<tag> -q .sha`) and update the
      SHA and the version comment together.
- [ ] Flake inputs (`flake.lock`, last updated 2026-11-23): run
      `nix flake update` on its own stack layer and let CI build every host.
- [ ] Moshi (`pkgs/moshi-hook`, 0.4.62): compare with Moshi's
      [Homebrew formula](https://github.com/rjyo/homebrew-moshi/blob/main/Formula/moshi-hook.rb).
      The README's Services section says how to update it.
- [ ] plannotator-tui (`pkgs/plannotator-tui`, 0.10.0): compare with its
      [releases](https://github.com/plannotator/plannotator-tui/releases).
      Refresh its patches on the new tag and bump `version`, `hash` and
      `cargoHash` together (README.md, Patched packages).
      `test-races.patch` is upstream's unreleased
      [a059de2](https://github.com/plannotator/plannotator-tui/commit/a059de2e74b5ca3d9cbb31a650902a3e6fa00ee6),
      trimmed to files that exist at v0.10.0: drop it once a release after
      v0.10.0 includes a059de2.
- [ ] herdr-splits.nvim (README, Set up a Mac step 8): `v0.5.4` against its
      [tags](https://github.com/lmilojevicc/herdr-splits.nvim/tags).
      Reinstall on each host after a bump.
- [ ] Annotate (`pkgs/herdr-plugin-annotate`): `dd147a9` (plugin 0.10.0)
      against herdr-annotate's
      [main branch](https://github.com/plannotator/herdr-annotate/commits/main).
      Bump `rev`, `hash`, the runtime's `version` and `cargoHash`; the build
      stops if the new plugin pins a plannotator-tui other than ours.
- [ ] JankyBorders patches (`overlays/modifications.nix`): the fixes are not
      submitted upstream and stay in the overlay. If
      [JankyBorders](https://github.com/FelixKratz/JankyBorders/releases)
      has a release after `v1.9.0`, check whether it fixes the bottom corners
      near rounded display corners, borders over Mission Control on macOS 27
      ([issue 209](https://github.com/FelixKratz/JankyBorders/issues/209)) or
      borders over sheets, and drop each patch it makes redundant. Either way,
      evaluation stops at the version assertion once nixpkgs packages the new
      release.

## Decisions

- [ ] Decide whether deploying only committed revisions should be enforced, and
      how. Today a deploy builds the working tree and records `-dirty`.
