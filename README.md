[![build](https://github.com/mich-murphy/nix-config/actions/workflows/build-macos.yml/badge.svg?branch=main)](https://github.com/mich-murphy/nix-config/actions/workflows/build-macos.yml)

# MacBook and ai-dev Nix configuration

The flake exposes the M2 MacBook Air as `darwinConfigurations.macbook` and the
x86_64 Linux ai-dev user as `homeConfigurations."michael@ai-dev"`. nix-darwin
owns Mac machine settings and applications. Home Manager owns the shared
portable user environment for `mm` on macOS and `michael` on ai-dev. Nix itself
remains managed by the Determinate installer on both hosts.

## Structure

```text
flake.nix
├── configuration.nix
│   └── darwin/default.nix
│       └── Darwin concern files
├── home.nix
│   └── home/default.nix
│       └── portable user concern files
└── hosts
    ├── macbook.nix
    └── ai-dev.nix
```

Each `default.nix` is the static manifest for its directory. Concern files
directly define existing nix-darwin or Home Manager options. The shared home
and a dedicated host module are composed in each Home Manager configuration,
so Darwin-only concerns never enter the ai-dev module graph. This follows the
[nix-darwin flake guide](https://github.com/nix-darwin/nix-darwin#flakes-recommended-for-beginners),
[Home Manager's nix-darwin integration](https://nix-community.github.io/home-manager/nix-flakes/nix-darwin.html),
and [Home Manager's standalone flake
guide](https://nix-community.github.io/home-manager/nix-flakes/standalone.html).

The former media/NixOS configuration and encrypted age files were removed from
HEAD. They remain recoverable from ordinary Git history; that removal does not
purge historical objects.

## Mac bootstrap

1. Install Nix with the
   [Determinate installer](https://docs.determinate.systems/).
2. Install [Homebrew](https://brew.sh/).
3. Clone this repository to `/Users/mm/dev/nix-config`.
4. Bootstrap and activate:

   ```sh
   nix run nix-darwin -- switch --flake ~/dev/nix-config
   ```

## ai-dev deployment

The `home-infra` Ansible role is the supported Linux deployment path. It clones
this public repository to `/home/michael/dev/nix-config`, fast-forwards the
checkout to `origin/main`, builds the activation package, and activates it as
`michael`. Check mode evaluates and builds without activation.

To validate the profile directly on ai-dev:

```sh
nix build --no-link '.#homeConfigurations."michael@ai-dev".activationPackage'
```

Home Manager owns portable CLI tools, Fish, Starship, FZF, Git behavior, Hunk,
Herdr configuration, Neovim, Yazi, OpenCode, and shared agent
instructions/skills. Ansible retains the operating-system bootstrap, stable
agent installers, and vaulted ai-dev Git identity fragments. It must not clone
or otherwise manage `~/.config/nvim`; Home Manager deploys that path as a live
link to this repository's `config/nvim`. macOS keeps its Home Manager-owned
personal and BusinessCraft identities under `~/businesscraft/`; ai-dev selects
its separate `0600` fragments under the same path.

## Commit hooks

Home Manager installs `prek`. Install this repository's pre-commit shim once
after activating the configuration:

```sh
prek install
```

Run every configured hook manually with:

```sh
prek run --all-files
```

The hooks perform fast file hygiene and syntax checks, lint Markdown, and check
Nix formatting. Flake evaluation remains part of the validation commands below
instead of every commit.

## Validate and activate

Validate without changing the running system:

```sh
prek run --all-files
nix flake check --all-systems --print-build-logs
darwin-rebuild build --flake .
nix build --no-link '.#homeConfigurations."michael@ai-dev".activationPackage'
```

Activation is intentionally separate:

```sh
darwin-rebuild switch --flake .
```

CI runs formatting, Markdown lint, and platform checks on ARM macOS and x86_64
Linux runners. It never activates either runner.

## Live configuration

Each host passes its checkout as `repoRoot`: `/Users/mm/dev/nix-config` on the
Mac and `/home/michael/dev/nix-config` on ai-dev. Home Manager uses
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

Herdr does not watch `config.toml`. After editing the live Herdr configuration,
press `Ctrl+A`, then `Shift+R` in each active session that should receive the
reload.

On both hosts, Nix installs Neovim and its foundational runtime dependencies.
Home Manager links `~/.config/nvim` directly to this checkout's `config/nvim`,
while Mason owns the declared editor-only tools. Before the first activation of
this model, move any existing `~/.config/nvim` directory out of the way so Home
Manager can create the directory link.

## Homebrew warning

Homebrew activation uses `cleanup = "zap"` with forced cleanup. Removing a cask
or formula declaration can uninstall the application and associated files on
the next switch. Preserve the complete declaration set unless that uninstall is
intentional. See the
[nix-darwin cleanup option](https://nix-darwin.github.io/nix-darwin/manual/#opt-homebrew.onActivation.cleanup)
and [Homebrew's zap warning](https://docs.brew.sh/Cask-Cookbook#stanza-zap).
