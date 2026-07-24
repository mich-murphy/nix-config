# TODO

Reviewed: 2026-07-25

This checklist contains only work that remains after the single-host module
refactor.

## Configuration

- [ ] **Make Neovim configuration reproducible**
  - [`home/neovim.nix`](home/neovim.nix) loads `config.lazy`, but the
    corresponding configuration must be cloned manually from
    `git@github.com:mich-murphy/neovim.git` into `~/.config/nvim`.
  - Choose and document one supported provisioning model:
    1. manage the configuration in this repository;
    2. add the external repository as a pinned flake input; or
    3. keep it external and provide a repeatable bootstrap step that validates
       the expected checkout.
  - The result should allow a new machine to reach a working Neovim
    configuration without undocumented manual state.

- [ ] **Record the source revision in Darwin generations**
  - Set `system.configurationRevision` from the flake revision without passing
    the complete `inputs` set into modules.
  - Preserve useful dirty-worktree behaviour while ensuring clean deployments
    identify the commit that produced them.

## Testing

- [ ] **Add targeted checks for important generated configuration**
  - The current flake checks build the Darwin system and Home Manager activation
    package, but they do not assert the contents of generated configuration.
  - Add focused regression checks where a successful build alone would not
    catch unintended changes, prioritising:
    - SSH settings;
    - live configuration links for terminal applications and Karabiner;
    - the declared Homebrew and system package inventory.
  - Keep checks tied to concrete invariants rather than recreating isolated
    module tests for the single-host concern files.

## Cleanup

- [ ] **Reconcile terminal assertions with the current terminal model**
  - [`home/terminals.nix`](home/terminals.nix) rejects Home Manager-managed
    Kitty and Alacritty with an "only one terminal emulator" message, while
    Ghostty and WezTerm are both intentionally installed and configured.
  - Remove the assertions if they no longer protect a real invariant, or rewrite
    them and their messages to describe the combination that must actually be
    rejected.
