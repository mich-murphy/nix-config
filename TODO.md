# TODO

Reviewed: 2026-04-06

This checklist captures improvements found while reviewing the
repository against the official manuals:

- [Nix Reference Manual](https://nixos.org/manual/nix/stable)
- [Nixpkgs Manual](https://nixos.org/manual/nixpkgs/stable)
- [NixOS Manual](https://nixos.org/manual/nixos/stable)
- [nix-darwin Manual](https://nix-darwin.github.io/nix-darwin/manual/index.html)
- [Home Manager Manual](https://nix-community.github.io/home-manager/)
- [Home Manager Options Reference](https://nix-community.github.io/home-manager/options.xhtml)

These are tracking items for follow-up work; not every item is an
immediate bug. They are ordered by expected project impact and by how
far the current setup diverges from the recommended declarative/
module-driven approach.

## Highest priority

- [x] **Make the repository scope explicit and the flake buildable for
  every retained host**
  - The active flake now exposes only `darwinConfigurations.macbook`.
  - The inactive media/NixOS tree has been moved under `archive/` and
    is no longer part of the active flake outputs.
  - This keeps the retained active scope buildable while preserving the
    old host for future reference.
  - Manual focus: flakes are intended to package Nix code with declared
    inputs and outputs in a reproducible way.

- [ ] **Remove the fake terminal package overlay hack**
  - `flake.nix` defines `fakepkg`, and Home Manager terminal modules
    currently use it:
    - `home/modules/wezterm.nix`
    - `home/modules/alacritty.nix`
    - `home/modules/kitty.nix`
  - This bypasses Home Manager module expectations instead of using a
    real package or managing config files separately from installation.
  - Prefer one clear model:
    1. install the terminal with Nix and let Home Manager manage it
       normally; or
    2. keep installation in Homebrew and manage only config files via
       `xdg.configFile` / `home.file`.
  - Manual focus: Nixpkgs overlays, nix-darwin Homebrew management, and
    Home Manager program modules.

- [ ] **Make the Home Manager Neovim setup declarative and
  self-contained**
  - `home/modules/neovim.nix` notes that `~/.config/nvim` must be
    cloned manually outside Nix.
  - This makes the active Home Manager configuration incomplete and
    undermines the reproducibility Home Manager is meant to provide.
  - Bring the Neovim config into this repository, reference it as a
    flake input, or manage it via `xdg.configFile` / `home.file`.
  - Manual focus: Home Manager-managed files and declarative home
    environments.

## Testing

Testing work is grouped here so module contracts, build coverage, and
system verification can be tracked together.

- [x] **Add `flake checks` for every retained system and activation
  package**
  - The repository now has a `checks` output covering the retained
    active system.
  - Current checks evaluate and build:
    - `darwinConfigurations.macbook`
    - the Home Manager activation package used by the laptop
      configuration
  - This is the baseline gate for `nix flake check` locally and in CI.
  - Manual focus: `nix flake check` and flake output validation.

- [ ] **Add assertion coverage for retained reusable modules**
  - Assertions are currently inconsistent:
    - most Home Manager modules have none
    - both Darwin modules have none
  - Add assertions for module invariants such as:
    - mutually exclusive terminal modules (`wezterm`, `kitty`,
      `alacritty`)
    - `darwin/modules/skhd.nix` requiring `common.yabai.enable`
  - Every retained reusable module should have explicit checks for the
    invalid combinations it knows how to reject.
  - Manual focus: module option contracts and assertion-based
    validation.

- [ ] **Add module evaluation tests with positive and negative cases**
  - Add lightweight Nix tests that instantiate reusable Darwin and Home
    Manager modules in isolation.
  - Positive tests should verify expected configuration output for valid
    option combinations.
  - Negative tests should verify that invalid combinations fail with the
    intended assertion message.
  - This is especially important for reusable modules under
    `darwin/modules/` and `home/modules/`.
  - Manual focus: module evaluation and assertion behaviour.

- [ ] **Add system-level tests that verify installed applications and
  generated settings**
  - Current validation proves the active Darwin system and Home Manager
    activation package can be built.
  - Add checks that inspect built system outputs and generated config
    files to confirm expected settings are present for key applications
    and shells, for example:
    - Home Manager generated configs for `git`, `ssh`, shells, `yazi`,
      terminals, and `karabiner`
    - nix-darwin generated Homebrew/Brewfile state for declared casks,
      brews, and MAS apps
    - system package presence in retained system closures
  - Manual focus: system tests and generated configuration
    verification.

- [ ] **Run the full test suite in CI without mutating tracked
  configuration**
  - Current GitHub Actions only builds the macOS configuration and
    rewrites tracked files with `sed` to adapt usernames for the
    runner.
  - Replace that with proper CI-specific config or checks so CI runs the
    same declarative test targets defined by the flake.
  - CI should run at least formatting, `nix flake check`, and retained
    system build/tests.
  - Manual focus: reproducible CI validation.

## High priority

- [ ] **Prefer nix-darwin's built-in Nix maintenance options over a
  custom launchd GC agent**
  - `hosts/laptop/default.nix` defines `launchd.agents.nix-gc`
    manually.
  - nix-darwin already provides typed, documented `nix.gc.*` and
    `nix.optimise.*` options.
  - If these are compatible with the Determinate installer setup,
    migrate to them; otherwise document why the custom launchd agent is
    still required.
  - Manual focus: nix-darwin `nix.gc` / `nix.optimise` options.

- [ ] **Replace duplicated shell PATH and hook setup with shared Home
  Manager session options**
  - `/opt/homebrew/bin` is added separately in
    `home/modules/cli/fish.nix` and `home/modules/cli/zsh.nix`.
  - `programs.direnv.enableZshIntegration = true` is already set in
    `home/modules/cli/apps.nix`, but `home/modules/cli/zsh.nix` also
    runs `eval "$(direnv hook zsh)"` manually.
  - Prefer `home.sessionPath` / `home.sessionVariables` for shared
    environment state and let Home Manager modules manage their shell
    hooks once.
  - Manual focus: Home Manager session variables/path and shell
    integrations.

- [ ] **Replace Home Manager SSH `extraOptions` with typed SSH options
  where available**
  - `home/modules/ssh.nix` uses `extraOptions` for `IdentityAgent` and
    `HashKnownHosts`, even though Home Manager exposes typed options
    such as `identityAgent` and `hashKnownHosts`.
  - Prefer structured options where available, leaving `extraOptions`
    only for settings that have no first-class module option.
  - Manual focus: Home Manager `programs.ssh.matchBlocks` options.

- [ ] **Set `system.configurationRevision` on the Darwin
  configuration**
  - The nix-darwin manual exposes `system.configurationRevision` for
    recording which flake revision produced the current system.
  - Set it from the repository revision so deployed systems are easier
    to audit and trace back to source.
  - Manual focus: nix-darwin system version metadata.

- [ ] **Review Karabiner file deployment and keep the activation copy
  only if mutability is required**
  - `home/modules/karabiner/default.nix` uses `home.activation` to copy
    `karabiner.json` into place.
  - Prefer `xdg.configFile."karabiner/karabiner.json".source = ...` if a
    normal declarative file is sufficient.
  - Keep the current copy-based approach only if Karabiner needs a
    writable/mutable config file in practice.
  - Manual focus: Home Manager managed files and activation DAG usage.

## Low priority / cleanup

- [ ] **Review duplicated `allowUnfree` configuration**
  - `hosts/laptop/default.nix` sets
    `nixpkgs.config.allowUnfree = true`.
  - `home/home.nix` also writes `~/.config/nixpkgs/config.nix` with
    `allowUnfree = true`.
  - Decide whether the per-user config file is still needed, document
    why it exists if it is, or consolidate to reduce drift between
    declarative and ad hoc package evaluation.
  - Manual focus: Nixpkgs global configuration.

- [ ] **Refresh README bootstrap guidance so it matches the actual
  flake**
  - README now reflects the archived media/NixOS tree, but it still has
    stale bootstrap and reference details.
  - Re-check the bootstrap/install commands against the current manuals
    and the repo's actual outputs.
  - While touching docs, replace the moved nix-darwin manual link and
    fix any stale workflow or CI references.
  - Manual focus: flake-based workflows and repository documentation
    alignment.

## Completed

- [x] Repository review against official Nix, Nixpkgs, NixOS,
  nix-darwin, and Home Manager manuals completed (2026-04-06)
