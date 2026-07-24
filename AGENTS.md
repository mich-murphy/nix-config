# AGENTS.md

Nix flake configuring one macOS M2 MacBook Air (`aarch64-darwin`) with
nix-darwin and embedded Home Manager.

## Project map

- `flake.nix` — inputs, formatter, checks, and
  `darwinConfigurations.macbook`
- `configuration.nix` — machine entry point, identity, platform, state
  compatibility, Determinate integration, and Home Manager wiring
- `darwin/default.nix` — static manifest for Darwin concern files
- `darwin/system.nix` — networking, security, power, and macOS defaults
- `darwin/maintenance.nix` — Determinate-compatible generation and GC agents
- `darwin/applications.nix` — packages, fonts, Homebrew, and MAS inventory
- `darwin/window-management.nix` — yabai, skhd, and the live skhd path
- `home.nix` — user entry point and Home Manager baseline
- `home/default.nix` — static manifest for user concern files
- `home/*.nix` — direct Home Manager definitions grouped by concern
- `config/` — intentionally live application and coding-agent configuration

<important if="you need to run commands to build, test, lint, format, or update">

| Command | What it does |
| --- | --- |
| `darwin-rebuild build --flake .` | Validate build without activating (dry run) |
| `darwin-rebuild switch --flake .` | Rebuild and activate |
| `nix fmt` | Format all Nix files with Alejandra |
| `nix fmt -- file1.nix file2.nix` | Format specific files |
| `nix fmt -- --check .` | Check Nix formatting without writing |
| `npx --yes markdownlint-cli2 "**/*.md" "#node_modules" "#.claude/skills"` | Lint Markdown |
| `nix flake check --all-systems --print-build-logs` | Run all flake checks |
| `nix flake update` | Update all flake inputs |
| `nix flake update nixpkgs` | Update a single input |
| `nix run nix-darwin -- switch --flake ~/dev/nix-config` | First-time bootstrap |

</important>

<important if="you are creating a new concern or adding an option">
This is a single-host configuration. Concern files directly define existing
nix-darwin or Home Manager options and are enabled by a static import in the
adjacent `default.nix`. Do not add `common.*` switches or Darwin-to-Home
forwarding wrappers for always-enabled concerns.

To add a Home Manager concern:

1. Create `home/<name>.nix` with direct definitions.
2. Import it in `home/default.nix`.
3. Add packages beside the concern that owns them.

Add system packages, casks, formulae, fonts, or MAS applications directly to
`darwin/applications.nix`. Only introduce a typed option with `lib.mkIf` when a
real second host or profile needs conditional behavior; keep its import
unconditional.
</important>

<important if="you are adding, removing, or modifying packages">
- **Prefer Nix** (`environment.systemPackages` or `home.packages`) for CLI tools
  and anything in nixpkgs for Darwin
- **Use Homebrew casks** only for GUI macOS apps unavailable or broken in nixpkgs
- **Use Homebrew formulae** only as a last resort when a package is missing in nixpkgs for `aarch64-darwin`
- When adding a Homebrew cask, check if a Nix package exists first (`nix search nixpkgs <name>`)
- **Beware Homebrew zap:** `cleanup = "zap"` is enabled. Removing a cask or
  formula line will **uninstall** that application on the next `switch`.
  Always confirm with the user before removing any Homebrew entry.
</important>

<important if="you are modifying Nix expressions or module options">
- Keep imports static; never derive imports from `config`.
- Do not declare custom options for files that only define existing options.
- Do not pass the complete flake `inputs` set or override reserved module
  arguments such as `lib`.
- Keep Nixpkgs configuration on the Darwin side because Home Manager uses
  `useGlobalPkgs = true`.
- Do not duplicate settings across `darwin/` and `home/`: system-level settings
  belong in Darwin and user-level settings in Home Manager.
- Do not change either state version during routine cleanup.
- Do not set Nix options that conflict with Determinate management; preserve
  `nix.enable = false`.
- Avoid top-level `with pkgs;`, recursive attribute sets, dynamic import
  discovery, module factories, and unnecessary abstractions.
</important>

<important if="you are creating new files in this flake">
Run `git add` on new files before evaluating the flake; untracked files are
invisible to flake evaluation.
</important>

<important if="you are editing or creating Markdown files">
Run
`npx --yes markdownlint-cli2 "**/*.md" "#node_modules" "#.claude/skills"`
before committing. Configuration is in `.markdownlint-cli2.yaml`.
</important>

<important if="you are making changes across multiple files">
Make and verify changes incrementally, one concern at a time. Run
`darwin-rebuild build --flake .` after every coherent edit. Do not activate with
`switch` unless the user explicitly requests it.
</important>
