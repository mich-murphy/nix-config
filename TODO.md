# TODO

Reviewed: 2026-11-10

Open work only. See [DESIGN.md](DESIGN.md) for the decisions behind it.

## dev-01

- [ ] Before the first deploy, move any keys from `~/.config/underclass/env`
      into `config.toml`; activation refuses to switch while the env file
      exists. Once every host has switched, delete the guard in
      `modules/home/underclass.nix`.
- [ ] Run the first real deploy: `deploy --dry-activate .#dev-01`, then
      `deploy .#dev-01`. It is untested and may need `dev` in `trusted-users`
      on dev-01 or `--remote-build`.
- [ ] After a successful deploy, remove the nix-config checkout on dev-01.
      Nothing in this repository reads it.

## Mac

- [ ] After the next switch, check Mason in Neovim (`:Mason`,
      `:checkhealth mason`), since go, python3 and nodejs now come only from
      Neovim's packages, and check that the Moshi hooks are current.
- [ ] Uninstall the deprecated `agent-sessions` binary and its launchd agent by
      hand; Nix never managed them.
- [ ] Retire `hosts/macbook` when the laptop is in service.
- [ ] Test whether skhd and yabai keep their macOS permissions across an
      upgrade. This has never been done.
- [ ] Decide whether LinearMouse and locale preferences should be declared;
      neither is today.

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
