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
- [ ] Decide whether LinearMouse's config should be declared.
      `~/.config/linearmouse/linearmouse.json` names devices by serial number,
      which this public repository should not carry, and the app rewrites the
      file.

## Agents

- [ ] Claude's skill links are a hand-written list of five in
      `modules/home/agents.nix`, so a new skill needs a Nix edit. Expose the
      whole skills repository instead, if that can be done without letting
      Claude's skill sync write into it.
- [ ] Herdr plugins are installed by hand, not by Nix.
- [ ] The Herdr `annotate` plugin is reported to run its commands with `bun`,
      which this configuration does not install. Confirm whether it works.

## Decisions

- [ ] Decide whether a Mac on AC power should idle-sleep. `pmset -c sleep 1`
      sleeps it a minute after the display turns off unless something holds a
      `caffeinate` assertion, and a sleeping Mac is unreachable over the
      tailnet. `sleep 0` on AC would keep it awake.
- [ ] Decide whether `networking.dns` should keep `100.100.100.100` first.
      Tailscale's app already installs its own resolver for the tailnet, and
      the static entry makes every lookup wait on it when Tailscale is down.
- [ ] Decide whether to declare three settings that are set by hand today:
      the Dock's app list (`system.defaults.dock.persistent-apps`), Siri off
      and AirPlay Receiver off. The last two have no typed option, and AirPlay
      Receiver is a per-host preference that `CustomUserPreferences` cannot
      write.

- [ ] Decide whether deploying only committed revisions should be enforced, and
      how. Today a deploy builds the working tree and records `-dirty`.
- [ ] Decide the future of CI: add a Linux job, or delete the workflow.
