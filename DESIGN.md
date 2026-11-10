# Workstation configuration design

## Status and authority

This document records the goals agreed through the user interview. It describes
the target, not the current state; progress is tracked in pull requests against
`main`.

Authority order:

1. Current user instructions and applicable repository/global instructions.
2. This design.

Earlier proposals are superseded. Do not resurrect an extra shared-environment
repo, reuse `~/dev/harness`, introduce field-level settings synchronization, add a
general secret scanner, or automatically pin/update components contrary to the
decisions below.

## 1. Purpose

One repository, **nix-config**, is the central authority for configuration, builds
and deployment of the user's machines. Change shared configuration once, publish
it, and roll it out with clear visibility into what each machine actually runs.

Nix orchestrates components; it must not replace their native configuration and
development workflows. Live edits should remain live, ordinary application saves
must work, and changes must be visible in the correct Git repository.

The immediate fleet is the user's Mac laptop and development box `dev-01`, plus
migration to a new M5 Pro MacBook Pro with 64 GB RAM and 2 TB SSD. The old/new Macs
may overlap during migration. Exact CPU topology and delivered OS remain unverified.

## 2. Repository and ownership boundaries

| Resource | Authority |
| --- | --- |
| User fleet configuration, host profiles, installs/integration, builds/deployment | nix-config |
| Infrastructure provisioning and necessary bootstrap | ~/businesscraft/machines |
| Personal Neovim Lua/plugin configuration | Separate personal Neovim repository, live at ~/.config/nvim |
| Skills, supporting resources, tests/evals and custom agent definitions | Separate skills repository extracted from config/agents/skills |
| Pi extension implementation | ~/dev/pi-extensions; authoritative and entirely active |
| Durable Claude/Codex/Pi settings and global instructions | nix-config |
| Harness/package-provided default agent definitions | Their owning harness/package, not copied into user repositories |
| Accounts, secrets and private runtime state | Application/OS-local, outside shared Git configuration |

`~/dev/harness` is outdated and irrelevant. Do not use it as the skills source or
restore its historical behavior. The skills extraction starts from
`~/dev/nix-config/config/agents/skills`, then reconciles other genuinely custom
resources by provenance and user intent.

Agent-definition directories (`**/agents/*`) are candidates for the skills repo,
not a blanket import rule. User-authored definitions such as a custom Claude Nix
agent or Pi web-researcher belong there. Built-in/default definitions stay with
their provider. Subagent-extension settings are configuration unless they are
custom agent-definition files. Global CLAUDE.md/AGENTS.md remain nix-config-owned.

Skills may be alpha, beta or stable. Experimental work is also shareable; its
maturity must be clear. Keep skill eval infrastructure out of nix-config.

## 3. Hosts, shared environment and infrastructure

Retain small shared modules with explicit platform/host composition inside
nix-config. Laptop-only GUI/apps/OS preferences do not belong in a remote profile.
Development boxes consume a smaller shared environment. Do not introduce another
repository solely to share the portable Nix module implementation.

On macOS, retain Determinate Nix, nix-darwin and Home Manager. On dev-01, NixOS is
an option, not a decided migration. Ubuntu was selected for easy access to fresh
agent binaries, not because it must remain the OS. Evaluate NixOS against native
installer/self-updater compatibility before choosing the target.

The machines repo manages `dev-01` and also a colleague's `dev-02`, which
**must not be affected**. Transition only the user's
ongoing system/user configuration into
nix-config; retain the colleague's existing bootstrap/configuration path.

Use one configuration/activation owner for each resource and one Home Manager
composition per user. Do not layer a competing independently activated personal HM
profile over the existing remote profile. Existing machines isolation rules require
an intentional, reviewed revision scoped to the user's host, not silent removal.

## 4. Writable whole-file configuration

Manage configuration as **individual whole files**, with a reliable relationship
to Git-tracked source. No field-level sync, semantic key-merging framework,
automatic bidirectional reconciler, or Nix-to-Lua translation layer.

The live-file arrangement must be tested before selection. A file symlink can be
replaced by an application's atomic save; a stable-looking link is not proof of
working write-back. Prefer native configuration paths/overrides and simple
whole-file arrangements. Do not choose a new manager such as chezmoi by default.
Copy-once defaults alone do not meet the requirement for ongoing tracked changes.

Durable customization tracked in nix-config includes:

- Compaction, UI and general harness configuration preferences.
- Global instructions and permission policies.
- Extension/plugin declarations and installation sources.
- MCP definitions and hooks, excluding credentials.
- Subagent-extension configuration apart from custom definition files.

Model/reasoning choices are normally made on the fly. If the application saves
such a choice into a tracked file, incidental Git changes are acceptable: the user
reviews what to commit. Do not add machinery to filter them out. A committed
whole-file change can naturally carry that preference to other machines.

Accounts, credentials, sessions, caches, histories, usage, per-project trust,
generated approvals and device identity remain local/untracked. Do not place an
entire mixed agent-state directory under shared Git management. If a client cannot
separate sensitive/local state cleanly, raise that concrete constraint rather than
silently introduce field-level syncing or commit sensitive data.

## 5. Component lifecycle

### Neovim

Use a separate personal repository with the live working location `~/.config/nvim`.
Verify the current Git/symlink relationship before extraction; it is currently
linked into nix-config, not proven to be an independent checkout.

Nix installs Neovim and necessary runtime dependencies. Lua, lazy.nvim, Mason and
the editor's native mechanisms retain configuration/plugin ownership. Editing Lua
or plugin declarations must not require a Nix rebuild. Use normal native reload or
restart behavior; do not promise every plugin hot-reloads.

### Skills and custom agents

The separate skills repo is the content authority. Nix provisions dependencies and
coordinates installation/exposure for all three harnesses. Preserve client-specific
capabilities and intentional enable/disable choices without duplicating content.

Codex/Pi can consume the shared skills location; Claude needs its native exposure.
Keep app-managed/default content separate. In particular, do not replace or import
Claude's entire synced skills cache as authored source. Adding/removing skills
must have a clear whole-repository/exposure update operation; do not let Nix and
another mechanism both own the same links.

### Pi extensions

`~/dev/pi-extensions` is the active source repo. Preserve the installed-package
workflow: the user manually runs **`pi update --extensions`** to adopt updates.
Fleet deployment must not silently upgrade extensions for now. Manage installation
sources/integration and report actual installed state; do not force live development
checkout loading or a new fixed-version policy. Fleet-wide extension updates can
be a separately approved future feature.

## 6. Harness baseline and cleanup

Start from **Pi's intentional current behavior**, but clean Pi itself before
propagating that baseline. Leftover extension files, out-of-authority skills and
stale settings are not authoritative merely because they are present.

Align Claude and Codex to that common baseline through their native configuration.
Alignment means intentional common behavior, not identical files, identical model
names, enabling every skill, or copying built-in agents.

Classify custom resources as retain, move, replace, retire or unresolved. Verify
provenance, actual discovery/enabled state and effective behavior. Remove approved
obsolete resources and duplicate/conflicting wiring with recoverable changes.
Unknown provenance is not permission to delete.

Historical capture/hooks require explicit reconciliation. Current instructions
prohibit re-enabling session capture or reinstalling bc-heavy/Pueue. Existing
running services are not automatically authorized for deletion by this design;
do not restore or carry them forward simply because an old README says to.

## 7. Updates, deployment and state visibility

Normal workflow:

1. Edit directly or let the application save; changes take effect locally.
2. Git shows changes in the owning repository.
3. Review, commit and publish.
4. Deploy the working tree, or a chosen commit, to eligible hosts.

Latest is the default. Version holds are explicit exceptions for compatibility,
build or security problems. Recording resolved revisions/locks for provenance and
rollback is compatible with this policy; it must not turn normal updates into a
permanent manual pin-maintenance exercise. Neovim tracks main.

Claude/Codex/Pi binary freshness is especially important. Homebrew latest releases
on the Mac and official installers/self-updates on the development box are accepted
channels. Nix coordinates their installation/integration without necessarily owning
their update lifecycle. Prefer Nix for other tools, with explicit fallback for
freshness or packaging failures. Avoid duplicate install/service owners.

A deploy builds the operator's working tree, like `darwin-rebuild switch --flake .`,
and records the revision with `-dirty` when it was not committed. Components
that are separate clones (Neovim, skills) are never overwritten by a deploy.

Expose actual configuration/component revisions, local changes, pending/failure
reasons, intentional holds and independently updated application versions. Distinguish
config synchronization from binary-version equality. A build is not an activation.
The exact deploy command and state-record implementation remain engineering details.

## 8. System-settings correctness and efficiency

Treat live settings as evidence, not a template to reproduce uncritically. Select
stable desired preferences to declare, fix ineffective/obsolete declarations,
retain appropriate OS-managed defaults, and make manual onboarding checks explicit.

Concrete priorities:

- skhd/yabai upgrades currently require renewed macOS permission entries. Local
  binaries are ad-hoc signed with code-hash-specific requirements and store paths.
  Require a real permission-persistence upgrade test; stable symlinks or Brew alone
  are not proven fixes. Do not weaken SIP or edit TCC databases for convenience.
- Determinate already manages Nix. `nix.enable = false` correctly prevents competing
  nix-darwin ownership. Make settings/update/maintenance responsibilities explicit,
  evaluate the official Determinate module, and reconcile custom GC with Nixd.
- Review Brew update/upgrade/zap activation behavior before migration; Nix rollback
  does not undo Brew upgrades or removed application data.
- Preserve selected input/locale, lock, LinearMouse and other desired preferences;
  distinguish them from hardware-specific defaults and private app state.
- Use project dev shells/nix-direnv for project toolchains. Track intended .envrc
  files, keep shell entry fast, and make runtime/tool precedence deliberate.
- Measure M5 workloads before tuning Nix concurrency, Docker resources or power.
  Lazy trees and parallel evaluation are already enabled on the current machine.

## 9. Quality and verification

Use what nixpkgs and the Nix community already provide and trust. Do not build
custom tooling where an established tool exists, and keep verification simple
enough for one person to maintain.

- Formatting: nixfmt through `nix fmt` (nixpkgs `nixfmt-tree`), replacing
  Alejandra.
- Linting: nixf-diagnose, the same analysis nixd shows in the editor and nixpkgs
  uses in CI. It replaces Statix and deadnix.
- The existing prek hygiene, config-syntax and private-key hooks, plus
  markdownlint-cli2.
- Scripts are built with `writeShellApplication`, which runs ShellCheck.
- `nix flake check` is the single gate: lint, a build of every host, module
  assertions and tests over built outputs.
- No Gitleaks or other general secret scanner. This repository is public, so it
  tracks no private configuration at all.

Test only what logic or the build derives, and invariants that must fail loudly:
module `assertions` and `runCommand` checks over built outputs. Do not re-assert
literal values the configuration just set.

Each pull request description records the commands run and their results, the
`nvd diff` of the built system, and any manual behaviour checks. A pure
restructure must produce identical system and home output paths. When a choice
depends on how something behaves, run a small experiment and record the result in
the pull request; do not commit evidence to the repository. The flake lock pins
tool versions.

## 10. Engineering decisions

1. **Writable files:** Home Manager `mkOutOfStoreSymlink` from each application's
   path to a checkout at `~/dev/nix-config` on the Macs; dev-01 has no checkout
   and takes this repository's files from the store. Codex `config.toml`
   (it mixes in per-project trust) stays local. MCP configuration is not shared.
2. **dev-01:** x86_64-linux; stays on Ubuntu with standalone Home Manager. NixOS is evaluated later as separate work.
3. **skhd/yabai:** unchanged until the configuration is correct; permission
   persistence is revisited afterwards.
4. **Fleet:** the Mac switches itself with `darwin-rebuild`; dev-01 is a
   deploy-rs node, deployed from the Mac (`deploy .#dev-01`), which evaluates
   the flake locally and pushes the closure. A `git+file://…?rev=` reference
   deploys a commit rather than the working tree. nix-darwin records the
   revision in `system.configurationRevision`; Home Manager hosts record it in a
   state file.
5. **Repositories:** Neovim is `mich-murphy/neovim`, cloned to `~/.config/nvim`.
   Skills are the public `mich-murphy/skills`, cloned to `~/dev/skills`.
   Packages nixpkgs lacks (Moshi's hook daemon) live in `pkgs/<name>/`.
   Herdr comes from nixpkgs on every host.
6. **Hosts:** `macbook` (current Mac), `laptop` (the M5, which replaces `macbook`)
   and `dev@dev-01`. Layout is `hosts/<name>/` plus `modules/{darwin,home}/`.
7. **Homebrew:** upgrades on switch; cleanup is `uninstall`, never `zap`.
8. **direnv:** `.envrc` stays globally ignored and is not tracked, including in
   this repository, because `.envrc` files can hold secrets. This replaces the
   §8 note about tracking intended `.envrc` files. `nix develop` provides the
   dev shell.
9. **Agent settings:** an exception to §4's whole-file rule. Claude
   `settings.json` and Pi `settings.json` stay regular, app-owned files, and
   Home Manager's `programs.{claude-code,pi-coding-agent}.mutableSettings`
   merges the declared keys into them on activation. Whole-file links do not
   work for these files: the agent-sessions installer refuses symlinked
   settings, herdr and Moshi write their hooks (with absolute home paths) into
   them, and Pi stores its per-install `deviceId` and changelog versions there.
   Nix declares preferences, plugins and permissions only; hooks, model and
   effort choices, `enabledModels` and per-model compaction overrides stay
   local. Codex `hooks.json` holds only installer-owned hooks and stays local.
   Running sessions need no restart: Pi re-reads and writes only changed keys
   under its lock, and Home Manager aborts activation ("changed during
   activation") rather than overwrite a concurrent save.
