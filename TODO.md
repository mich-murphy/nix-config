# TODO

Reviewed: 2026-11-11

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

## Decisions

- [ ] Decide whether to show the Finder desktop again.
      `finder.CreateDesktop = false` hides the files in `~/Desktop`, and
      yabai's README says it "breaks focusing of empty spaces". Here,
      `yabai -m display --focus 2` reports success but does not move focus
      while the built-in display is empty, and works once a window is on it.
      Deleting the line brings the desktop icons back.
- [ ] Decide whether deploying only committed revisions should be enforced, and
      how. Today a deploy builds the working tree and records `-dirty`.
