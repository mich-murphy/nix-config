# Design

This repository configures the owner's machines. This file records why it is
shaped the way it is: the goals, the non-goals and the decisions in force. For
commands, setup and layout, see [README.md](README.md).

## Goals

- One repository is the authority for configuration, builds and deployment.
  Change something once and roll it out.
- Nix orchestrates tools; it does not replace their native configuration or
  workflows. Live edits stay live, and an application's own save keeps working.
- Changes show up in the Git repository that owns them.
- Latest versions by default. The lock records what was resolved, for
  provenance and rollback; it is not a pin-maintenance chore.
- Use established tools. Do not build custom tooling where one exists, and keep
  verification simple enough for one person to maintain.

## Non-goals

- Session capture. It is deprecated and removed from the repo.
- A fleet. The Macs never deploy to each other.
- Field-level settings sync, key-merging frameworks or a Nix-to-Lua layer.
- A general secret scanner. The repository is public and tracks no private
  configuration, so there is nothing for one to find.
- Project toolchains in the global environment. Project dev shells provide them.
- Deploys that upgrade Pi extensions. The owner updates those by hand.

## Hosts

| Host | What | Notes |
| --- | --- | --- |
| `macbook` | nix-darwin Mac, current | Removed once `laptop` is in service. |
| `laptop` | nix-darwin Mac, MacBook Pro M5 | Replaces `macbook`. |
| `dev@dev-01` | Ubuntu x86_64 VM, standalone Home Manager | Deployed from a Mac. |

Shared modules compose by platform and host: `modules/home` for every host,
`modules/darwin` and `modules/home/mac` for the Macs. Laptop-only apps and OS
preferences stay out of the remote profile. dev-01 stays on Ubuntu because it
gives easy access to fresh agent binaries; NixOS is possible later, separate
work.

## Ownership

| What | Owner | How it reaches a host |
| --- | --- | --- |
| Host profiles, installs, builds, deployment | this repo | Nix |
| Ghostty, Karabiner (Macs only) | this repo (`config/`) | live link into the checkout |
| skhd (Macs only) | this repo (`config/`) | `/etc/skhdrc` loads the file from the checkout |
| Herdr, agent instructions | this repo (`config/`) | live link on a Mac; the deployed generation on dev-01 |
| Claude and Pi preferences, plugins, permissions | this repo | merged into the app's own settings file |
| Neovim configuration | `mich-murphy/neovim` | its own clone at `~/.config/nvim`; Nix installs only Neovim and its tools |
| Skills and custom agents | `mich-murphy/skills` | clone at `~/dev/skills`, linked live on every host |
| Pi extensions | `pi-extensions` repo | installed as packages; updated by hand |
| dev-01's bootstrap, agent CLIs and work git identity | `machines` repo | outside this repo |
| Accounts, credentials, sessions, caches, histories, per-project trust, device identity | the application or OS | local and untracked |

Built-in agent definitions stay with the tool that ships them. Codex
`config.toml` mixes in per-project trust and `hooks.json` holds installer-owned
hooks, so both stay local. MCP configuration is not shared. Model and effort
choices are made on the fly and stay local.

## Writable files

Configuration is managed as whole files, never as fields. On the Macs an
out-of-store link points each application's path into the checkout, so the
application writes straight into Git. `nixConfig.checkout` holds the checkout
path (null means none) and `config.lib.nixConfig.file` produces the link, or a
store copy when there is no checkout. A flake check confirms every link points
into the checkout or the skills clone, and at a file that exists.

dev-01 has no checkout. Its copy of this repo's files is read-only and comes
from the deployed generation, so one deploy is the whole deploy.

Agent settings are the exception. Claude and Pi `settings.json` stay regular
files that the apps own, and Home Manager's `mutableSettings` merges only the
declared keys into them at activation. A link would not work: Herdr and Moshi
write hooks into these files, and Pi keeps its device id and changelog versions
there. Nix declares preferences, plugins and permissions; hooks, model choices
and per-model overrides stay local. A concurrent save makes activation abort
rather than overwrite it.

A whole directory is linked where the application watches it (Karabiner).
`~/.claude/skills` stays a real directory with a link per skill, so Claude
Code's skill sync cannot write into the skills repo.

## Software and updates

- Agent CLIs (Claude Code, Codex, Pi) come from Homebrew on the Macs and from
  official installers on dev-01, for freshness. Nix wires them up without
  owning their updates.
- Homebrew upgrades on every switch. Cleanup uninstalls whatever is not
  declared, App Store apps included, and never zaps, because Nix rollback does
  not restore removed application data.
- Determinate Nix owns the Nix installation, so nix-darwin does not manage Nix.
  Its daemon runs garbage collection; a weekly job expires old system
  generations so that collection can free them.
- Each resource has one activation owner: one Home Manager composition per
  user, and no second tool managing the same link or service.
- Pi extensions are updated by hand with `pi update --extensions`, never by a
  deploy.

## Deployment

The Mac switches itself with the stock `darwin-rebuild switch`. dev-01 is a
deploy-rs node, deployed from the Mac. The owner chose deploy-rs over plain
commands and over a pull-based timer, and an established tool over about 390
lines of custom shell.

A switch or deploy builds the working tree as it stands. The recorded revision
ends in `-dirty` when the tree was not committed: `darwin-version
--configuration-revision` on the Macs, `~/.local/state/nix-config/revision` on
dev-01. Nothing enforces committed-only deploys; that is an open decision in
[TODO.md](TODO.md). Separate clones (Neovim, skills) are never touched by a
deploy.

## Verification

`nix flake check --all-systems` is the single gate. It covers lint (nixfmt,
nixf-diagnose, markdownlint), each Mac's system, home and link check, dev-01's
home, link check and deploy-rs checks, and the Moshi hook version test. The
Linux checks build on the Mac through Determinate Nix's native Linux builder.
prek runs the same lint tools before each commit.

CI runs the same checks on every pull request and push to `main`, with one
runner per system. Branch protection does not require it to pass.

Tests cover what logic or the build derives. They do not re-assert literal
values the configuration just set. The Moshi module holds the only assertions:
that nix-darwin installs the app bundle the hooks point at, and that every
activation step its hook check is ordered after exists on that platform.

A pure restructure should change no behaviour. The commit revision is embedded
in every build, and on dev-01 the repo source is part of the home, so output
paths cannot match across commits. The rule that can be followed: evaluate both
trees as non-git `path:` flakes, which holds the revision constant; the Macs'
system derivations are then identical. Otherwise `nvd diff` of the built system
or home shows no changes. Behaviour experiments are recorded in the pull
request, not committed.

## Decisions

| Decision | Reason |
| --- | --- |
| Moshi hooks name a fixed path: an app bundle in `/Applications/Nix Apps` on macOS, a copied binary on Linux | Codex's approval covers the hook command, so the path must survive version bumps. The Macs also run the service from it, never from the store, because macOS makes a store app bundle that has run undeletable for garbage collection |
| The Moshi launchd agent carries the package version in `NIX_RESTART_ON_VERSION` | Moshi never reads it. The program path no longer changes with the version, so this is what changes the plist and makes launchd restart the service on an update |
| The Moshi hook check runs after the settings merges and after the service starts | The merges keep Moshi's undeclared keys, and `moshi-hook install` needs the daemon's socket |
| dev-01's `TMPDIR` is `~/businesscraft/.scratch` | Node's temporary files and audit caches reach hundreds of MiB, so they sit on the data disk; the bootstrap creates the directory after mounting it |
| fish binds F12 to clear the screen in both vi modes | Ghostty sends Ctrl+Shift+L as F12, which keeps it distinct from Ctrl+L inside Herdr and shells |
| dev-01 builds its own profile (deploy-rs `remoteBuild`) | `dev` is not a trusted Nix user on dev-01, so a closure built on the Mac is refused as unsigned; making `dev` trusted would give that account root-equivalent control of the Nix store |
| underclass runs on every host and reads `~/.config/underclass/config.toml` itself | The same untracked file works on both platforms; each host keeps its own account pool |
| `go`, `python3` and (on Macs) `nodejs` are Neovim packages, not global | Mason builds and runs its tools with them; project toolchains come from project dev shells |
| WezTerm is no longer installed or linked; `config/wezterm/` is kept | The owner no longer uses it; the config is retained |
| `.envrc` is globally ignored and untracked | These files can hold secrets; `nix develop` provides this repo's shell |
| Home Manager stops a switch when a file it does not own is where a link goes (`backupFileExtension` is unset) | Moving the file aside would hide an application replacing a link this repo manages, and a second collision fails anyway once the backup exists |
| btop is overridden to build a man page | Stock btop ships none |
| The menu bar manager is Thaw from the `thaw@beta` cask, not Ice | Ice has had no code commit since June 2025 and hangs on "Loading menu bar items" on macOS 27. Thaw is its maintained fork, and only Thaw 3, which ships as a beta, supports macOS 27; the stable `thaw` cask is Thaw 2, for macOS 15 and 26. Move to `thaw` once Thaw 3 is stable |
| Containers run in OrbStack, not Docker Desktop | It starts faster and uses less memory and disk on Apple Silicon, and also runs Linux machines. Commercial use needs a paid licence |
| Microsoft AutoUpdate is its own cask | The Word, Excel and PowerPoint casks deselect it in their installers, so a new Mac would not get Office's updater from them |
| `killall Finder` and `SystemUIServer` run after defaults are written | nix-darwin restarts only the Dock |
| Both fzf integrations are enabled in fish | Each contributes live key bindings |
| Herdr comes from nixpkgs on every host | The lock keeps client and remote server on one version |
| Keyboard shortcuts are merged into `com.apple.symbolichotkeys` with `defaults write -dict-add` after activation | `system.defaults` writes whole top-level keys, so declaring `AppleSymbolicHotKeys` there would drop every shortcut not listed |
| yabai restarts the Dock two seconds after a display connects or disconnects, and skhd binds Shift+Alt+D to `killall Dock` | On macOS 27, Ctrl+3 and above stop switching desktops once a display has connected or disconnected, while their entries stay intact, and only a Dock restart brings them back. A switch only appeared to fix them because nix-darwin restarts the Dock. The delay lets a burst of display changes finish first. Remove both when Apple fixes it |
| yabai's rules end with `yabai -m rule --apply` | Since yabai 7 a rule only affects windows opened after it is added, and yabai loads the open windows before it runs its config. Without this line a yabai restart tiled windows the rules mean to leave alone |
| yabai and skhd log to `/tmp/<name>_<user>.out.log` and `.err.log` | Neither nix-darwin module sets a log file, so a failed rule or an `skhdrc` parse error was printed nowhere. The paths are the ones upstream's own launchd services use |
| skhd's `/etc/skhdrc` is one `.load` of the checkout's `skhdrc`, not a Home Manager link at `~/.config/skhd/skhdrc` | nix-darwin starts user launchd agents before Home Manager links files, and skhd that finds no config only warns and never starts its file watcher, so the first switch would leave the hotkeys dead. skhd watches the files it loads, so an edit in the checkout still applies live. With no checkout the file's text is inlined |
| Desktops are switched with the macOS Ctrl+number shortcuts, not `yabai -m space --focus` | With SIP enabled, yabai 7.1.25 reports success and does nothing on macOS 27 ([yabai issue 2822](https://github.com/asmvik/yabai/issues/2822)). `skip_window_focus_animation` relies on the same mechanism and is not set |
| The time zone is not declared | macOS sets it automatically, and `time.timeZone` would override that on every switch |
| A Mac on AC power never idle-sleeps (`pmset -c sleep 0`); on battery it sleeps after a minute | A sleeping Mac drops off the tailnet and nothing wakes it over Wi-Fi, so sshd and agent jobs were unreachable unless something held a `caffeinate` assertion. The display still sleeps, and closing the lid with no external display still sleeps the Mac |
| `networking.dns` lists only public resolvers, not `100.100.100.100` | The Tailscale app installs its own resolver for the tailnet's names. The static entry sent every other lookup through Tailscale and left a dead address first in the list whenever Tailscale was down |
| The Dock's icons are declared (`dock.persistent-apps`, `persistent-others`) | A new Mac gets the same Dock. A switch rewrites both lists, so an icon dragged in by hand lasts only until the next switch |
| Siri is turned off through `com.apple.assistant.support`, and AirPlay Receiver by a `defaults -currentHost write` after activation | Neither has a typed option. AirPlay Receiver is a per-host preference, which `system.defaults` cannot write ([nix-darwin issue 1721](https://github.com/nix-darwin/nix-darwin/issues/1721)) |
| LinearMouse's settings are not declared | The app owns `~/.config/linearmouse/linearmouse.json`, and its settings window names each device by serial number, which this public repository should not carry. Two schemes for the same mouse model differ only by serial, so the serials cannot simply be stripped |
