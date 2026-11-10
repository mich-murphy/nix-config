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
flake.nix              inputs; mkDarwin (user, Home Manager wiring); dev-01's
                       deploy-rs node; checks
hosts/<host>/
├── default.nix        nix-darwin: host name, platform, stateVersion (Macs only)
└── home.nix           Home Manager: host-only modules, stateVersion, identity
modules/darwin/        nix-darwin modules shared by every Mac
modules/home/          Home Manager modules shared by every host
└── mac/               Home Manager modules only Macs import (incl. Git identity)
pkgs/<name>/package.nix  packages nixpkgs lacks; added to pkgs by an overlay
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

A package goes in `environment.systemPackages` when the system or root needs it,
in `home.packages` when it is a general command-line tool used in any directory,
in `programs.neovim.extraPackages` when only Neovim or Mason shells out to it,
in Homebrew when it is a Mac app, cask or formula that nixpkgs lacks or that
updates itself, and in a project's dev shell when it is that project's
toolchain.

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
file that exists in the repository, or into the skills clone (dev-01 has no
checkout). `nix develop` provides nixfmt,
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
owns everything outside the home directory; this flake owns the home.

### Deploy dev-01

From the Mac, inside `nix develop` (which provides `deploy`):

```sh
deploy --dry-activate .#dev-01   # build, copy and show what would change
deploy .#dev-01                  # build here, copy, activate, confirm
```

deploy-rs evaluates the flake on the Mac, copies the home closure to dev-01 over
SSH (alias `dev-01`, user `dev`, from `modules/home/mac/ssh.nix`) and activates
it. dev-01 needs no nix-config checkout: Herdr's config and `AGENTS.md` come
from the deployed generation there, so edit them on the Mac, then deploy; they
are read-only on dev-01. The Macs keep live links into their checkout.

`deploy .#dev-01` builds the Mac's working tree as it is, like
`darwin-rebuild switch --flake .` does, and the recorded revision ends in
`-dirty` when it was not committed. To deploy a specific commit:

```sh
deploy "git+file://$PWD?rev=$(git rev-parse HEAD)#dev-01"
```

The skills and Neovim clones on dev-01 are separate repositories, updated with a
plain `git pull`, not by a deploy; so is `pi update` for Pi packages. The skills
links stay live links on every host.

What each host is running:

```sh
darwin-version --configuration-revision          # Mac: commit it was built from
ssh dev-01 cat .local/state/nix-config/revision  # dev-01: commit, or -dirty
```

If activation fails, or dev-01 does not confirm within 30 seconds (magic
rollback), deploy-rs reverts dev-01 by itself. To undo a deploy that went
through, deploy the previous commit with the form above. On a Mac,
`sudo darwin-rebuild switch --rollback` returns to the previous generation.

First deploy (not yet tested on the host): the Nix daemon on dev-01 may refuse
the unsigned copied closure unless `dev` is in `trusted-users` there. If so, add
it, or use `deploy --remote-build .#dev-01`. The very first Home Manager switch
on dev-01, before it runs a deploy-rs generation, can still come from a clone
(`home-manager switch --flake .#dev@dev-01` in it) or from a `github:` flake
reference; either way it builds the files from the store, not from that clone.

Local validation is required: GitHub Actions is currently unavailable because
of usage limits, so a badge or remote workflow result is not a readiness gate.
Builds validate generated
configuration but do not activate services, run Homebrew upgrades, or prove a
complete fresh-machine bootstrap.

The GitHub workflow is manual-only to avoid automatic attempts while limits
apply. It never activates the runner.

## Live configuration

The `nixConfig.checkout` Home Manager option holds the checkout path, defaulting
to `~/dev/nix-config`. On the Macs, Home Manager uses
[out-of-store symlinks](https://nix-community.github.io/home-manager/usage/dotfiles.html)
only for configurations that are intentionally edited live:

- Ghostty;
- Herdr (on dev-01 it comes from the deployed generation instead);
- Karabiner;
- skhd, passed directly to its launch agent.

The WezTerm config is kept in `config/wezterm/` but is not linked or installed.

Coding-agent skills live in the public `mich-murphy/skills` repository, cloned
to `~/dev/skills` (the `nixConfig.skills` option). Home Manager links it to
`~/.agents/skills` for Codex and Pi, and links individual skills into
`~/.claude/skills` for Claude Code.

Claude Code and Pi settings (`~/.claude/settings.json`,
`~/.pi/agent/settings.json`) stay regular files owned by the applications.
Each switch merges the preferences, plugins and permissions declared in
`modules/home/agents.nix` into them and leaves every other key alone, so
installer hooks (herdr, Moshi), model choices and Pi's device ID
stay local. A declared value changed in the app is reset on the next switch;
change it in Nix instead. If a switch stops with "changed during activation",
an agent saved its settings at the same moment; run the switch again.

`nixConfig.checkout` is `null` on dev-01, which has no checkout: the same files
then come from the flake source in the store and are read-only. Modules take such
a file through `config.lib.nixConfig.file "<path in repo>"`. If a Mac's checkout
moves, set `nixConfig.checkout` (and the skhd path in
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
work accounts on dev-01. Optional fixed keys go in
`~/.config/underclass/config.toml`, which underclass reads itself and which is
never tracked (create it with mode 0600, then restart the service):

```toml
proxy_key = "sk-underclass-..."
ui_token = "..."
```

Without them, underclass generates both keys and stores them in
`~/.local/share/underclass/pool.db` with the accounts. Earlier revisions read
`~/.config/underclass/env`; underclass ignores that file, so a switch stops
while it exists. On any host that has one, move its keys into `config.toml`
(`UNDERCLASS_PROXY_KEY` becomes `proxy_key`, `UNDERCLASS_UI_TOKEN` becomes
`ui_token`) and delete it before deploying. Otherwise underclass would mint new
keys and every client would get 401.

Add accounts in the web UI at `http://127.0.0.1:8080/`, and
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
