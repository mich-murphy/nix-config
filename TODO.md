# TODO

Reviewed: 2026-11-16

Open work only. See [DESIGN.md](DESIGN.md) for the decisions behind it.

## dev-01

- [ ] Remove the nix-config checkout on dev-01. Nothing in this repository
      reads it.

## Mac

- [ ] Check Mason in Neovim (`:Mason`, `:checkhealth mason`), since go,
      python3 and nodejs now come only from Neovim's packages. Moshi is
      verified running from the installed bundle.
- [ ] Uninstall the deprecated `agent-sessions` binary and its launchd agent by
      hand; Nix never managed them.
- [ ] Retire `hosts/macbook` when the laptop is in service.
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
- [ ] After the first switch on the laptop, check in System Settings that
      Siri is off. It is written as a plain preference, which is proven only
      on a Mac where Siri had already been turned off by hand.

## Agents

- [ ] Claude's skill links are a hand-written list of five in
      `modules/home/agents.nix`, so a new skill needs a Nix edit. Expose the
      whole skills repository instead, if that can be done without letting
      Claude's skill sync write into it.
- [ ] Herdr plugins are installed by hand, not by Nix.
- [ ] On dev-01, confirm `herdr plugin list` shows Annotate at the ref in the
      README. Older versions run their commands with `bun`, which is not
      installed.

## Updates

Next check: 2026-12-16. On or after that date, check each pin below. Bump
any that are behind, then move the date on a month.

- [ ] GitHub Actions in `.github/workflows/check.yml`, pinned to commit SHAs:
      checkout v7.0.1, determinate-nix-action v3.24.2, flakehub-cache-action
      v3.24.2. For each, take the latest release's commit
      (`gh api repos/<owner>/<repo>/commits/<tag> -q .sha`) and update the
      SHA and the version comment together.
- [ ] Flake inputs (`flake.lock`, last updated 2026-11-16): run
      `nix flake update` on its own stack layer and let CI build every host.
- [ ] Moshi (`pkgs/moshi-hook`, 0.4.57): compare with Moshi's
      [Homebrew formula](https://github.com/rjyo/homebrew-moshi/blob/main/Formula/moshi-hook.rb).
      The README's Services section says how to update it.
- [ ] Herdr plugins (README, Set up a Mac step 8): herdr-splits.nvim
      `v0.5.4` against its
      [tags](https://github.com/lmilojevicc/herdr-splits.nvim/tags), and
      herdr-annotate `dd147a9` (plugin 0.10.0) against its
      [main branch](https://github.com/plannotator/herdr-annotate/commits/main).
      Reinstall on each host after a bump.
- [ ] JankyBorders patch (`overlays/modifications.nix`): the fix is not
      submitted upstream and stays in the overlay. If
      [JankyBorders](https://github.com/FelixKratz/JankyBorders/releases)
      has a release after `v1.9.0`, check whether it fixes the bottom corners
      near rounded display corners. If it does, delete the `jankyborders`
      entry and `overlays/patches/jankyborders`. Either way, evaluation
      stops at the version assertion once nixpkgs packages the new release.

## Decisions

- [ ] Decide whether to show the Finder desktop again.
      `finder.CreateDesktop = false` hides the files in `~/Desktop`, and
      yabai's README says it "breaks focusing of empty spaces". Here,
      `yabai -m display --focus 2` reports success but does not move focus
      while the built-in display is empty, and works once a window is on it.
      Deleting the line brings the desktop icons back.
- [ ] Decide whether deploying only committed revisions should be enforced, and
      how. Today a deploy builds the working tree and records `-dirty`.
