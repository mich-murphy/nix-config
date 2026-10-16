# MacBook Nix configuration

The flake exposes the M2 MacBook Air as `darwinConfigurations.macbook`.
nix-darwin owns Mac machine settings and applications, and Home Manager —
embedded in the system configuration — owns the portable user environment for
`mm` on macOS. Nix itself remains managed by the Determinate installer.

## Structure

```text
flake.nix
└── configuration.nix
    ├── darwin/default.nix
    │   └── Darwin concern files
    └── home-manager.users.mm
        ├── home.nix
        │   └── home/default.nix
        │       └── portable user concern files
        └── hosts/macbook.nix
```

Each `default.nix` is the static manifest for its directory. Concern files
directly define existing nix-darwin or Home Manager options. A dedicated host
module composes the host-only concerns on top of the shared home, so
Darwin-only concerns never enter the portable home module graph. This
follows the
[nix-darwin flake guide](https://github.com/nix-darwin/nix-darwin#flakes-recommended-for-beginners)
and
[Home Manager's nix-darwin integration](https://nix-community.github.io/home-manager/nix-flakes/nix-darwin.html).

The former media/NixOS configuration and encrypted age files were removed from
HEAD. They remain recoverable from ordinary Git history; that removal does not
purge historical objects.

## Mac bootstrap

1. Install Nix with the
   [Determinate installer](https://docs.determinate.systems/).
2. Install [Homebrew](https://brew.sh/).
3. Clone this repository to `/Users/mm/dev/nix-config`.
4. From the checkout, run `nix fmt -- --check .` and
   `nix flake check --print-build-logs --no-write-lock-file` before activation.
   For the first build, before `darwin-rebuild` is installed, use:

   ```sh
   nix run nix-darwin -- build --flake ~/dev/nix-config --no-write-lock-file
   ```

5. Bootstrap and activate only after validation succeeds:

   ```sh
   nix run nix-darwin -- switch --flake ~/dev/nix-config
   ```

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
nix fmt -- --check .
prek run --all-files
nix flake check --print-build-logs --no-write-lock-file
darwin-rebuild build --flake . --no-write-lock-file
```

Activation is intentionally separate:

```sh
darwin-rebuild switch --flake .
```

Local validation is required: GitHub Actions is currently unavailable because
of usage limits, so a badge or remote workflow result is not a readiness gate.
The flake checks include regression tests for safe Git branch cleanup and the
built GC launch daemon's weekly schedule. Builds validate generated
configuration but do not activate services, run Homebrew upgrades, or prove a
complete fresh-machine bootstrap.

The GitHub workflow is manual-only to avoid automatic attempts while limits
apply. It never activates the runner. Flake evaluation and builds do not need
access to the private session-collector repository or any deploy-key secret.

## Agent session collector

`agent-sessions` is installed separately from Nix using the release installer
in `~/dev/memory/install.sh`. Authenticate `gh` with an account that can read
`mich-murphy/memory`, then install an explicit release as your regular user:

```sh
VERSION=v0.3.0 sh ~/dev/memory/install.sh
~/.local/bin/agent-sessions install
```

The installer verifies the immutable release's attestation and checksums before
installing the binary in `~/.local/bin`, which Home Manager already puts on
`PATH`. Run it again with a new explicit version to upgrade. Nothing is added
to `flake.lock`, and neither Nix builds nor CI need private-repository access.

On a new machine without the checkout, download and verify the same installer:

```sh
(
  set -eu
  workdir=$(mktemp -d)
  trap 'rm -rf "$workdir"' EXIT
  gh release download v0.3.0 -R mich-murphy/memory -p install.sh -D "$workdir"
  gh release verify-asset v0.3.0 "$workdir/install.sh" -R mich-murphy/memory
  VERSION=v0.3.0 sh "$workdir/install.sh"
)
~/.local/bin/agent-sessions install
```

When migrating an existing Nix installation, install the release binary and
rerun its `install` command before removing the old active profile. This moves
managed hooks to the stable `~/.local/bin/agent-sessions` path. Preserve existing
credentials, state and machine identity; do not generate a new machine ID.
Review changed hooks in Codex `/hooks` and run `agent-sessions doctor` after
configuring the collector. See the collector's `docs/install.md` for details.

## Live configuration

Home Manager receives its checkout path as `repoRoot`:
`/Users/mm/dev/nix-config` on the Mac. Home Manager uses
[out-of-store symlinks](https://nix-community.github.io/home-manager/usage/dotfiles.html)
only for configurations that are intentionally edited live:

- coding-agent skills;
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

Nix installs Neovim and its foundational runtime dependencies.
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
