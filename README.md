# Nix configuration

The flake exposes the M2 MacBook Air as `darwinConfigurations.macbook` and the
M5 MacBook Pro as `darwinConfigurations.laptop`. nix-darwin owns Mac machine
settings and applications, and Home Manager — embedded in the system
configuration — owns the portable user environment for `mm` on macOS. The
Determinate installer installs Nix; Determinate's nix-darwin module
(`determinateNix.enable`) stops nix-darwin managing it and writes
`/etc/nix/nix.custom.conf`. Determinate Nixd collects garbage automatically; a
weekly launch daemon expires system generations older than 7 days.

## Structure

```text
flake.nix              inputs; mkDarwin (user, Home Manager wiring); checks
hosts/<host>/
├── default.nix        nix-darwin: host name, platform, stateVersion (Macs only)
└── home.nix           Home Manager: host-only modules, stateVersion, identity
modules/darwin/        nix-darwin modules shared by every Mac
modules/home/          Home Manager modules shared by every host
└── mac/               Home Manager modules only Macs import (incl. Git identity)
pkgs/<name>/package.nix  packages nixpkgs lacks; added to pkgs by an overlay
apps/                  nix run .#deploy and .#status (fleet.sh on the Mac,
                       fleet-host.sh on each host)
config/                raw application files, linked or read by the modules
```

To add a setting, put it in the shared module for its concern, or in
`hosts/<host>/` when only one machine needs it. Each `default.nix` lists the
modules for its directory. Concern files directly define existing nix-darwin
or Home Manager options. Modules receive the flake's `inputs` as a module
argument. A host's `home.nix` adds host-only concerns on top of the shared
home, so Darwin-only concerns never enter the portable home module graph. This
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
4. From the checkout, run
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
Nix formatting and lint on the changed files. Hook tools come from the flake's
locked nixpkgs. Building the hosts remains part of the validation commands below
instead of every commit.

## Validate and activate

Validate without changing the running system:

```sh
prek run --all-files
nix flake check --print-build-logs --no-write-lock-file --all-systems
darwin-rebuild build --flake . --no-write-lock-file
nvd diff /run/current-system ./result
```

`--all-systems` adds the `x86_64-linux` checks (dev-01's home and links, and
the Linux `moshi-hook`). They need a Linux builder; Determinate Nix on the Mac
provides one.

`nix flake check` is the single gate: its `lint` check runs `nixfmt --check` and
`nixf-diagnose` over every `.nix` file, and it builds every host. Each
`<host>-links` check confirms every writable link points into the checkout at a
file that exists in the repository. `nix develop` provides nixfmt,
nixf-diagnose and nvd. For direnv, create a local `.envrc` containing
`use flake`; it is globally ignored and never tracked.

Each generation records the commit it was built from (with `-dirty` for an
uncommitted tree): `darwin-version --configuration-revision` on a Mac, and
`~/.local/state/nix-config/revision` on every host.

Activation is intentionally separate:

```sh
darwin-rebuild switch --flake .
```

dev-01 runs Ubuntu with standalone Home Manager. The machines repo's bootstrap
owns everything outside the home directory; this flake owns the home. On
dev-01, from `~/dev/nix-config`:

```sh
home-manager switch --flake .#dev@dev-01
```

### Deploy and status

From a Mac, after publishing to `main`:

```sh
nix run .#status           # every host
nix run .#deploy           # every host
nix run .#deploy -- dev-01 # named hosts only
```

The host list comes from the flake (`darwinConfigurations` and
`homeConfigurations`). The Mac you run it on is handled directly. Every other
host gets one SSH session, under its flake name as the SSH alias.

`deploy` fast-forwards each host's Neovim, skills and nix-config checkouts with
`git pull --ff-only`. It then switches to the checkout's commit through
`git+file://…?rev=`, so uncommitted edits never deploy. A checkout is left as it
is and reported pending when Git refuses because local edits conflict, when it
has unpushed commits, or when it is on another branch. A pending nix-config
skips that host's switch, and the other hosts carry on. A host already running
the commit isn't switched again. A Mac switch runs `sudo darwin-rebuild`, which
asks for your password. The switch log is in
`~/.local/state/nix-config/deploy.log`. Agent CLIs and Pi extensions are not
updated: run `pi update --extensions` yourself.

`status` reports, for each host:

- the checkouts' commits compared with origin, and any local changes;
- the active revision, and whether a deploy is pending;
- Pi git packages that are behind their branch;
- managed files that are no longer links into the active generation;
- declared Claude and Pi settings changed in the app since the last switch;
- a table of agent, herdr and moshi-hook versions, marking those that differ.

Both commands exit 1 when any host is pending, failed or unreachable.

Local validation is required: GitHub Actions is currently unavailable because
of usage limits, so a badge or remote workflow result is not a readiness gate.
Builds validate generated
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
See the collector's `docs/install.md` for details.

## Live configuration

The `nixConfig.checkout` Home Manager option holds the checkout path, defaulting
to `~/dev/nix-config`. Home Manager uses
[out-of-store symlinks](https://nix-community.github.io/home-manager/usage/dotfiles.html)
only for configurations that are intentionally edited live:

- Ghostty and WezTerm;
- Herdr;
- Karabiner;
- skhd, passed directly to its launch agent.

Coding-agent skills live in the public `mich-murphy/skills` repository, cloned
to `~/dev/skills` (the `nixConfig.skills` option). Home Manager links it to
`~/.agents/skills` for Codex and Pi, and links individual skills into
`~/.claude/skills` for Claude Code.

Claude Code and Pi settings (`~/.claude/settings.json`,
`~/.pi/agent/settings.json`) stay regular files owned by the applications.
Each switch merges the preferences, plugins and permissions declared in
`modules/home/agents.nix` into them and leaves every other key alone, so
installer hooks (agent-sessions, herdr, Moshi), model choices and Pi's device ID
stay local. A declared value changed in the app is reset on the next switch;
change it in Nix instead. If a switch stops with "changed during activation",
an agent saved its settings at the same moment; run the switch again.

If the checkout moves, set `nixConfig.checkout` (and the skhd path in
`modules/darwin/window-management.nix`) and rebuild. Karabiner owns and watches
its whole configuration directory, so generated backups and complex
modifications are ignored while `config/karabiner/karabiner.json` remains
tracked.

Herdr comes from nixpkgs on every host, so `flake.lock` keeps a local client
and a remote server on the same version.

Moshi's hook daemon is `pkgs/moshi-hook`, pinned to one release for every host.
To update it, change `version` and copy both checksums from Moshi's
[Homebrew formula](https://github.com/rjyo/homebrew-moshi/blob/main/Formula/moshi-hook.rb).
`moshi-hook install` writes the path it runs from into each agent hook, and
Codex's approval covers that command, so hooks name a path that stays the same
across versions: `/Applications/Nix Apps/MoshiHook.app` on the Macs and
`~/.local/libexec/moshi-hook` on Linux. A switch reinstalls the hooks only when
`moshi-hook status` reports them out of date, and restarts the service when the
version changes.

[underclass](https://github.com/ghuntley/underclass) runs on every host as a
user service (launchd on the Macs, systemd on dev-01) listening on
`127.0.0.1:8080`. Each host has its own pool: personal accounts on the Macs,
work accounts on dev-01. Its keys go in `~/.config/underclass/env`, which is
never tracked:

```sh
UNDERCLASS_PROXY_KEY=sk-underclass-...
UNDERCLASS_UI_TOKEN=...
```

Create it with mode 0600 and restart the service. Without it, underclass
generates both keys and stores them in `~/.local/share/underclass/pool.db`
with the accounts. Add accounts in the web UI at `http://127.0.0.1:8080/`, and
watch the pool with `utop`. A new underclass revision compiles from source, and
the x86_64-linux build for the dev-01 check is slow on the Mac's Linux builder.

Clients are pointed at underclass by hand, per host, because the proxy key is
a secret. Nix never sets these values. To route Claude Code, add both variables
to the `env` block of `~/.claude/settings.json` and `chmod 600` the file.
Each switch merges the declared settings into the file and keeps these keys:

```json
"ANTHROPIC_BASE_URL": "http://127.0.0.1:8080",
"CLAUDE_CODE_OAUTH_TOKEN": "sk-underclass-..."
```

Set both or neither. With only the base URL, Claude sends its claude.ai login
to underclass and gets 401. Don't export `CLAUDE_CODE_OAUTH_TOKEN` from the
shell: Pi's Claude provider ignores `settings.json` and strips
`ANTHROPIC_BASE_URL`, so it would send the proxy key to Anthropic and fail with
"Not logged in". Pi's Claude therefore always uses the normal login.

To route Codex, add a provider to the local `~/.codex/config.toml` and
`chmod 600` the file:

```toml
model_provider = "underclass"

[model_providers.underclass]
name = "underclass"
base_url = "http://127.0.0.1:8080/v1"
experimental_bearer_token = "sk-underclass-..."
wire_api = "responses"
```

To stop routing a client, remove these lines. underclass's ADR 0023 notes that
pooling claude.ai OAuth credentials conflicts with Anthropic's terms, and those
accounts may be invalidated.

Herdr does not watch `config.toml`. After editing the live Herdr configuration,
press `Ctrl+A`, then `Shift+R` in each active session that should receive the
reload.

Nix installs Neovim and its foundational runtime dependencies. The editor
configuration is [mich-murphy/neovim](https://github.com/mich-murphy/neovim),
cloned to `~/.config/nvim`; Mason owns the declared editor-only tools. This
repository's `.nvim.lua` points nixd at the flake's nixpkgs and option sets;
run `:trust` once in Neovim to allow it.

## Homebrew warning

Homebrew activation updates Homebrew, upgrades packages and uses
`cleanup = "uninstall"`. Removing a cask or formula declaration uninstalls it on
the next switch, but leaves its associated files in place (no `zap`). Preserve
the complete declaration set unless that uninstall is intentional. See the
[nix-darwin cleanup option](https://nix-darwin.github.io/nix-darwin/manual/#opt-homebrew.onActivation.cleanup).
