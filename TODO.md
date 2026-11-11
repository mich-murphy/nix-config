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
- [ ] Confirm what breaks the Ctrl+number desktop shortcuts. The next time
      Ctrl+3 fails, note whether the Mac had just woken or a display had just
      reconnected, then press Shift+Alt+D. If that always restores them, decide
      whether yabai should restart the Dock on `display_added` and
      `system_woke`. Remove the binding when a macOS update fixes it.
- [ ] After the first switch on the laptop, check in System Settings that
      Siri and AirPlay Receiver are off. Both are written as plain preferences,
      which is proven only on a Mac where they had already been turned off by
      hand.

## Agents

- [ ] Claude's skill links are a hand-written list of five in
      `modules/home/agents.nix`, so a new skill needs a Nix edit. Expose the
      whole skills repository instead, if that can be done without letting
      Claude's skill sync write into it.
- [ ] Herdr plugins are installed by hand, not by Nix.
- [ ] The Herdr `annotate` plugin is reported to run its commands with `bun`,
      which this configuration does not install. Confirm whether it works.

## Decisions

- [ ] Decide whether deploying only committed revisions should be enforced, and
      how. Today a deploy builds the working tree and records `-dirty`.
- [ ] Decide the future of CI: add a Linux job, or delete the workflow.
