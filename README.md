[![build-macos](https://github.com/mich-murphy/nix-config/actions/workflows/build-macos.yml/badge.svg?branch=main)](https://github.com/mich-murphy/nix-config/actions/workflows/build-macos.yml)

# MacBook Nix configuration

Nix flake for one M2 MacBook Air, exposed as
`darwinConfigurations.macbook`. nix-darwin owns machine settings and
applications; embedded Home Manager owns the `mm` user environment. Nix itself
remains managed by the Determinate installer.

## Structure

```text
flake.nix
└── configuration.nix
    ├── darwin/default.nix
    │   ├── system.nix
    │   ├── maintenance.nix
    │   ├── applications.nix
    │   └── window-management.nix
    └── home.nix
        └── home/default.nix
            └── user concern files
```

Each `default.nix` is the static manifest for its directory. Concern files
directly define existing nix-darwin or Home Manager options; there is no
single-host `common.*` option layer. This follows the
[nix-darwin flake guide](https://github.com/nix-darwin/nix-darwin#flakes-recommended-for-beginners),
[Home Manager's nix-darwin integration](https://nix-community.github.io/home-manager/nix-flakes/nix-darwin.html),
and the [NixOS modularity model](https://nixos.org/manual/nixos/stable/#sec-modularity).
Imports stay static; a future real host-specific condition should use an
unconditional import with
[`lib.mkIf`](https://nixos.org/manual/nixos/stable/#sec-option-definitions-delaying-conditionals).

The former media/NixOS configuration and encrypted age files were removed from
HEAD. They remain recoverable from ordinary Git history; that removal does not
purge historical objects.

## Bootstrap

1. Install Nix with the
   [Determinate installer](https://docs.determinate.systems/).
2. Install [Homebrew](https://brew.sh/).
3. Clone this repository to `/Users/mm/dev/nix-config`.
4. Bootstrap and activate:

   ```sh
   nix run nix-darwin -- switch --flake ~/dev/nix-config
   ```

## Validate and activate

Validate without changing the running system:

```sh
nix fmt -- --check .
npx --yes markdownlint-cli2 "**/*.md" "#node_modules" "#.claude/skills"
nix flake check --all-systems --print-build-logs
darwin-rebuild build --flake .
```

Activation is intentionally separate:

```sh
darwin-rebuild switch --flake .
```

CI runs formatting, Markdown lint, and `nix flake check` on an ARM macOS runner.
It never activates the runner.

## Live configuration

`configuration.nix` defines `repoRoot`, currently
`/Users/mm/dev/nix-config`. Home Manager uses
[out-of-store symlinks](https://nix-community.github.io/home-manager/usage/dotfiles.html)
only for configurations that are intentionally edited live:

- coding-agent instructions and skills;
- Ghostty and WezTerm;
- Herdr;
- Karabiner;
- skhd, passed directly to its launch agent.

If the checkout moves, update `repoRoot` and rebuild. Karabiner owns and watches
its whole configuration directory, so generated backups and complex
modifications are ignored while `config/karabiner/karabiner.json` remains
tracked.

Neovim is installed by Nix, but its external configuration must still be cloned
to `~/.config/nvim` from `git@github.com:mich-murphy/neovim.git`. That dependency
is intentionally pending a separate review.

## Homebrew warning

Homebrew activation uses `cleanup = "zap"` with forced cleanup. Removing a cask
or formula declaration can uninstall the application and associated files on
the next switch. Preserve the complete declaration set unless that uninstall is
intentional. See the
[nix-darwin cleanup option](https://nix-darwin.github.io/nix-darwin/manual/#opt-homebrew.onActivation.cleanup)
and [Homebrew's zap warning](https://docs.brew.sh/Cask-Cookbook#stanza-zap).
