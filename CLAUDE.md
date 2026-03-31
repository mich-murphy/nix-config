# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Validate the build without activating (dry run — do this before switch)
darwin-rebuild build --flake .

# Rebuild and activate
darwin-rebuild switch --flake .

# Format all Nix files (alejandra)
nix fmt

# Update all flake inputs to latest
nix flake update

# Update a single input (e.g. nixpkgs only)
nix flake update nixpkgs

# First-time bootstrap (before darwin-rebuild is available)
nix run nix-darwin -- switch --flake ~/nix-config
```

## Architecture

This is a **Nix flake** for configuring a macOS M2 MacBook Air (aarch64-darwin). It composes three layers:

1. **nix-darwin** (`darwin/`, `hosts/laptop/`) — macOS system-level settings, Homebrew packages, fonts, and system environment
2. **home-manager** (`home/`) — user-level dotfiles and application configuration via the `common` module namespace
3. **Secrets** (`secrets/`) — `agenix`-managed `.age` encrypted secrets, currently scoped to a separate NixOS media host

### Directory Structure

- `flake.nix` — entry point; defines inputs (nixpkgs-unstable, nix-darwin, home-manager) and a single `darwinConfigurations.macbook` output
- `hosts/laptop/` — host-specific config split into `default.nix` (nix settings, GC), `system.nix` (macOS defaults), `apps.nix` (system packages + Homebrew), `user.nix` (user account)
- `darwin/modules/` — reusable darwin modules: `yabai.nix` (tiling WM), `skhd.nix` (hotkeys); exposed via `common.yabai` / `common.skhd` options
- `home/home.nix` — home-manager root; enables modules via `common.*` options
- `home/modules/` — home-manager modules, each following the `common.<name>.enable` pattern:
  - `cli/` — fish, zsh, fzf, zellij, apps (CLI tools)
  - `karabiner/`, `hammerspoon/` — macOS keyboard/automation
  - Individual files: git, neovim, wezterm, ssh, yazi, kitty, alacritty, firefox
- `nixos/modules/` — NixOS service modules for the media server host (not active in current flake outputs)
- `secrets/secrets.nix` — agenix secret declarations keyed to the media host's SSH public key

### Module Pattern

All reusable modules use a consistent `common.<name>.enable = true/false` option pattern. To add a new home-manager module:
1. Create `home/modules/<name>.nix` defining `options.common.<name>.enable` and `config = mkIf cfg.enable { ... }`
2. Import it in `home/modules/default.nix`
3. Enable it in `home/home.nix`

Darwin modules follow the same pattern under `darwin/modules/` and are imported via `hosts/laptop/apps.nix`.

### Key Decisions

- Nix management is disabled (`nix.enable = false`) in favor of the **Determinate Systems** Nix installer
- nixpkgs follows `nixpkgs-unstable`
- Homebrew is managed declaratively via nix-darwin; `cleanup = "zap"` removes any casks/formulae not listed in config
- Unfree packages are allowed both at the nixpkgs level and via home-manager's xdg config

## Nix Conventions

### Module structure

Every module follows this skeleton exactly:

```nix
{ lib, config, pkgs, ... }:
let
  cfg = config.common.<name>;
in {
  options.common.<name> = {
    enable = lib.mkEnableOption "<description>";
    # additional options use lib.mkOption with explicit type and description
  };

  config = lib.mkIf cfg.enable {
    # configuration body
  };
}
```

Always bind `cfg = config.common.<name>` at the top of the `let` block; never inline `config.common.<name>` in the body.

Use `lib.mkOption` with explicit `type` and `description` for any option beyond `enable`. Prefer specific types (`lib.types.str`, `lib.types.listOf lib.types.str`, `lib.types.attrsOf`) over `lib.types.anything`.

### Homebrew vs Nix packages

- **Prefer Nix** (`environment.systemPackages` or `home.packages`) for CLI tools and anything available in nixpkgs for Darwin.
- **Use Homebrew casks** only for GUI macOS apps unavailable or broken in nixpkgs.
- **Use Homebrew formulae** only as a last resort when a package is missing or non-functional in nixpkgs for `aarch64-darwin`.

### Formatting and commits

- Always run `nix fmt` before committing — alejandra is the formatter.
- Commit messages follow **Conventional Commits**: `fix(scope): message` or `feat(scope): message` (match the style in the git log).
- Run `darwin-rebuild build --flake .` before `darwin-rebuild switch --flake .` to catch errors without changing the running system.

### Change discipline

- **Read before editing.** Always read and understand a file before modifying it. Never propose changes to code you have not seen.
- **Minimal changes only.** Make only the changes the user requested. Do not refactor surrounding code, add comments, or "improve" unrelated sections.
- **Build-verify every change.** Run `darwin-rebuild build --flake .` after every edit. Do not run `switch` until the build succeeds and the user approves.
- **Beware Homebrew zap.** Because `cleanup = "zap"` is enabled, removing a cask or formula line will **uninstall** that application on the next `switch`. Always confirm with the user before removing any Homebrew entry.
- **Check git state first.** Before starting work, review `git status` and `git diff` to understand any uncommitted changes that could be affected.
- **One concern at a time.** When multiple files need changes, make and verify them incrementally rather than editing everything at once.

### Nix expressions

- **Prefer simple solutions.** Do not add complex patterns like `allowUnfreePredicate`, unnecessary abstractions, or over-engineered expressions unless explicitly asked. Keep Nix expressions minimal and readable.
- **`git add` before `nix build`.** After creating new files in a flake project, run `git add` on them before building — untracked files are invisible to Nix's flake evaluation.

### Common pitfalls

- Do not use bare `with pkgs;` in module `options` blocks — only in `config` bodies where the scope is clear.
- Do not duplicate settings across nix-darwin (`hosts/laptop/`) and home-manager (`home/`) layers; system-level config belongs in nix-darwin, user-level config belongs in home-manager.
- When adding a Homebrew cask, check if a Nix package exists first (`nix search nixpkgs <name>`).
- Do not set `nix.*` options that conflict with Determinate Systems management (e.g. avoid re-enabling `nix.enable = true`).
