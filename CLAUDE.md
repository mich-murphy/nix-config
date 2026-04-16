# CLAUDE.md

Nix flake configuring a macOS M2 MacBook Air (aarch64-darwin) via nix-darwin + home-manager.

## Project map

- `flake.nix` — entry point; nixpkgs-unstable, nix-darwin, home-manager inputs; single `darwinConfigurations.macbook` output
- `hosts/laptop/` — `default.nix` (nix settings, GC), `system.nix` (macOS defaults), `apps.nix` (system packages + Homebrew), `user.nix` (user account)
- `darwin/modules/` — reusable darwin modules: `yabai.nix`, `skhd.nix`; exposed via `common.yabai` / `common.skhd`
- `home/home.nix` — home-manager root; enables modules via `common.*` options
- `home/modules/` — home-manager modules (`common.<name>.enable` pattern): `cli/`, `karabiner/`, `hammerspoon/`, git, neovim, wezterm, ssh, yazi, kitty, alacritty, firefox
- `archive/` — archived NixOS/media host config and agenix secrets; not part of active flake outputs

<important if="you need to run commands to build, test, lint, format, or update">

| Command | What it does |
| --- | --- |
| `darwin-rebuild build --flake .` | Validate build without activating (dry run) |
| `darwin-rebuild switch --flake .` | Rebuild and activate |
| `nix fmt` | Format all Nix files (alejandra) |
| `nix fmt -- file1.nix file2.nix` | Format specific files |
| `npx markdownlint-cli2 "**/*.md"` | Lint all Markdown files |
| `npx markdownlint-cli2 --fix "**/*.md"` | Lint and auto-fix Markdown files |
| `nix flake update` | Update all flake inputs |
| `nix flake update nixpkgs` | Update a single input |
| `nix run nix-darwin -- switch --flake ~/nix-config` | First-time bootstrap |

</important>

<important if="you are creating a new module or adding a new option">
All modules use `common.<name>.enable` pattern. See `home/modules/git.nix` for the canonical example.
To add a new home-manager module:
1. Create `home/modules/<name>.nix` with `options.common.<name>.enable` and `config = mkIf cfg.enable { ... }`
2. Import it in `home/modules/default.nix`
3. Enable it in `home/home.nix`
Darwin modules follow the same pattern under `darwin/modules/` and are imported via `hosts/laptop/apps.nix`.
</important>

<important if="you are adding, removing, or modifying packages">
- **Prefer Nix** (`environment.systemPackages` or `home.packages`) for CLI tools and anything in nixpkgs for Darwin
- **Use Homebrew casks** only for GUI macOS apps unavailable or broken in nixpkgs
- **Use Homebrew formulae** only as a last resort when a package is missing in nixpkgs for `aarch64-darwin`
- When adding a Homebrew cask, check if a Nix package exists first (`nix search nixpkgs <name>`)
- **Beware Homebrew zap:** `cleanup = "zap"` is enabled — removing a cask or formula line will **uninstall** that application on next `switch`. Always confirm with the user before removing any Homebrew entry.
</important>

<important if="you are modifying Nix expressions or module options">
- Do not use bare `with pkgs;` in module `options` blocks — only in `config` bodies
- Do not duplicate settings across nix-darwin (`hosts/laptop/`) and home-manager (`home/`) — system-level in nix-darwin, user-level in home-manager
- Do not set `nix.*` options that conflict with Determinate Systems management (e.g. avoid `nix.enable = true`)
- Keep Nix expressions minimal — no `allowUnfreePredicate`, unnecessary abstractions, or over-engineering unless asked
</important>

<important if="you are creating new files in this flake">
Run `git add` on new files before `nix build` — untracked files are invisible to flake evaluation.
</important>

<important if="you are editing or creating Markdown files">
Run `npx markdownlint-cli2 "**/*.md"` before committing. Config is in `.markdownlint-cli2.yaml`.
</important>

<important if="you are making changes across multiple files">
Make and verify changes incrementally — one concern at a time. Run `darwin-rebuild build --flake .` after each edit.
</important>
