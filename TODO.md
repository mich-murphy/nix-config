# TODO

Reviewed: 2026-11-10

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
- [ ] After the next switch, unplug and reconnect the external display, then
      press Ctrl+3. The signals are proven with a virtual display: the Dock
      restarted two seconds after it appeared and again after it went, and
      Ctrl+3 worked afterwards. A virtual display did not break Ctrl+3 by
      itself, though, so only a real unplug shows whether the restart prevents
      the break. If Ctrl+3 still fails, check that `yabai -m signal --list`
      shows both signals. Remove the signals and the binding when a macOS
      update fixes the shortcuts.
- [ ] After the first switch on the laptop, check in System Settings that
      Siri is off. It is written as a plain preference, which is proven only
      on a Mac where Siri had already been turned off by hand.

## Agents

- [ ] Claude's skill links are a hand-written list of five in
      `modules/home/agents.nix`, so a new skill needs a Nix edit. Expose the
      whole skills repository instead, if that can be done without letting
      Claude's skill sync write into it.
- [ ] Herdr plugins are installed by hand, not by Nix.
- [ ] The Herdr `annotate` plugin is reported to run its commands with `bun`,
      which this configuration does not install. Confirm whether it works.

## Decisions

- [ ] Decide whether to show the Finder desktop again.
      `finder.CreateDesktop = false` hides the files in `~/Desktop`, and
      yabai's README says it "breaks focusing of empty spaces". Here,
      `yabai -m display --focus 2` reports success but does not move focus
      while the built-in display is empty, and works once a window is on it.
      Deleting the line brings the desktop icons back.
- [ ] Decide whether deploying only committed revisions should be enforced, and
      how. Today a deploy builds the working tree and records `-dirty`.
- [ ] Decide the future of CI: add a Linux job, or delete the workflow.
