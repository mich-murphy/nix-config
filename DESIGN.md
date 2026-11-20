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
| `laptop` | nix-darwin Mac, MacBook Pro M5 | The only Mac. |
| `dev@dev-01` | Ubuntu x86_64 VM, standalone Home Manager | Deployed from a Mac. |

Shared modules compose by platform and host: `modules/home` for every host,
`modules/darwin` and `modules/home/mac` for the Macs. Laptop-only apps and OS
preferences stay out of the remote profile. dev-01 stays on Ubuntu for now;
moving it to NixOS is planned, separate work.

## Ownership

| What | Owner | How it reaches a host |
| --- | --- | --- |
| Host profiles, installs, builds, deployment | this repo | Nix |
| Ghostty, Rex key bindings, Karabiner, skhd (Macs only) | this repo (`config/`) | live link into the checkout |
| Herdr, agent instructions | this repo (`config/`) | live link on a Mac; the deployed generation on dev-01 |
| Claude and Pi preferences, plugins, permissions | this repo | merged into the app's own settings file |
| Rex appearance and app settings (Macs only) | this repo | merged into its `com.superlogical.rex` defaults domain on each switch |
| Neovim configuration | `mich-murphy/neovim` | its own clone at `~/.config/nvim`; Nix installs only Neovim and its tools |
| Skills and custom agents | `mich-murphy/skills` | clone at `~/dev/skills`, linked live on every host |
| Pi extensions | `pi-extensions` repo | installed as packages; updated by hand |
| dev-01's bootstrap and work git identity | `machines` repo | outside this repo |
| Accounts, credentials, sessions, caches, histories, per-project trust, device identity | the application or OS | local and untracked |

Built-in agent definitions stay with the tool that ships them. Codex
`config.toml` mixes in per-project trust and `hooks.json` holds installer-owned
hooks, so both stay app-owned files; a switch or deploy merges one declared
key into `config.toml`, the one that turns off its update check. MCP
configuration is not shared. Model and effort choices are made on the fly and
stay local.

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

- Agent CLIs (Claude Code, Codex, Pi) come from numtide's llm-agents.nix on
  every host, through Home Manager's modules. It packages upstream's latest
  release within a day, where the nixpkgs-unstable channel lags by days, and
  its cache has them prebuilt. A switch or deploy is their only update, every
  host runs the same version and a rollback covers them. None of them
  replaces or announces itself: its wrappers set Claude Code's
  `DISABLE_AUTOUPDATER` and Pi's `PI_SKIP_VERSION_CHECK`, and Codex's startup
  check is turned off in its config. The input has no `follows`, so its
  packages match what the cache built; `nix flake update llm-agents` takes
  new releases.
- A program comes from Nix when it is open source and might be patched, runs
  as a service, must be the same version on every host, or is a command-line
  tool. It comes from Homebrew when it is a closed-source GUI app, or needs a
  system extension, a privileged helper, a fixed `/Applications` path or macOS
  permissions that reset when its signature changes. Patching needs the
  source, and rolling back a GUI app does not roll back its data.
- Homebrew upgrades on every switch. Cleanup uninstalls whatever is not
  declared, App Store apps included, and never zaps, because Nix rollback does
  not restore removed application data.
- Determinate Nix owns the Nix installation, so nix-darwin does not manage Nix.
  Its daemon is the only garbage collector, as Determinate recommends. A
  weekly job expires old system generations so that collection can free them,
  because the daemon collects by disk pressure and never expires generations.
- Each resource has one activation owner: one Home Manager composition per
  user, and no second tool managing the same link or service.
- Pi extensions are updated by hand with `pi update --extensions`, never by a
  deploy.

## Patches

A patch costs build time in proportion to how many packages depend on what it
changes, because every dependent misses cache.nixos.org and rebuilds. The
overlays in `overlays/` are applied to every package set, so modules use
`pkgs.<name>` for everything, and each kind of change has one flat file.
Patch files sit in `overlays/patches/<package>/`.

- `overlays/additions.nix` adds the packages nixpkgs lacks, from `pkgs/`.
  Nothing in nixpkgs depends on them, so adding them rebuilds nothing else.
- `overlays/modifications.nix` holds local fixes to packages nothing else
  depends on (leaf packages), with `overrideAttrs`: patches, and a newer
  release where nixpkgs is more than three releases behind upstream (a
  smaller lag is left to a lock update). Only that package
  rebuilds. Each asserts the upstream version it was written for, so a
  nixpkgs bump stops evaluation and the release is checked for an upstream
  fix before the patch carries on.
- `overlays/security.nix` holds security fixes ahead of nixpkgs, and is empty
  between incidents. In order of preference: update nixpkgs to a revision
  with the fix (nothing rebuilds); override only the exposed programs
  (`curl.override { openssl = …; }`, a few rebuild); replace the library for
  everything (most of the closure rebuilds, so CI builds and caches it before
  any host switches, and build tools such as `gitMinimal` stay on the
  unpatched package). Each entry names its advisory and has a removal item in
  [TODO.md](TODO.md).
- Grafting with `replaceDependencies` is not used. It rewrites store paths
  inside finished binaries, which breaks their code signatures on macOS, and
  the kernel then kills them at launch.

## Binary cache

CI pushes what it builds to FlakeHub Cache, so the Macs substitute the paths
cache.nixos.org lacks (Moshi, patched packages, and a whole closure after a
deep security patch) instead of building them. It was chosen over
Cachix's free tier for its authentication: no long-lived push token exists,
and the cache is private, so unfree packages are not redistributed. It costs
$7 a month.

Today it holds little that is slow to build (Moshi, JankyBorders, Mole); it
is kept as headroom for deeper patches and a scheduled lock
update.

Only the Macs read it, so only a macOS job pushes. dev-01 is not logged in:
no cache credential sits on a machine agents run on, and the paths it needs
that cache.nixos.org lacks are a download (Moshi) or in Numtide's cache (the
agent CLIs).

The cache is filled by GitHub's free macOS runner, which has 3 cores. That is
enough for leaf patches. A patch deep enough to rebuild much of the Darwin
closure would take hours there; when one pushes the `cache` job past about
20 minutes, its `runs-on` moves to a paid runner (Namespace's M4 Pro Macs cost
about what GitHub's paid runners do, with twice the cores). The laptop builds
anything not yet cached itself, and it is faster than any rented Mac.

A path pushed to the cache runs as root on every host that substitutes it, and
Nix checks only the signature, not how the path was built. So the cache trusts
exactly what `main` is trusted with:

- Only the `cache` job pushes. It runs only for pushes to `main`, and it alone
  has `id-token: write`, the short-lived GitHub token FlakeHub accepts. Pull
  request jobs, including those from agents' branches, get no token.
- Merging into `main` requires the pull request job, `check`, to pass; the
  owner, as admin, can bypass it.
- Every action is pinned to a full commit SHA, and checkout keeps no
  credentials. [TODO.md](TODO.md) has the monthly bump.
- Hosts look in cache.nixos.org first (priority 30, ahead of FlakeHub's 38
  and 39), so nixpkgs paths never come from our cache. `require-sigs` stays
  on, `accept-flake-config` stays off (a flake's suggested substituters are
  ignored), and only root is a trusted user.
- Numtide's cache is the one substituter outside this repository's control,
  trusted by its key for the agent CLIs and asked last. A compromised key
  could serve any path the earlier caches lack, not only those CLIs. The Mac
  lists it in `modules/darwin/nix.nix`, CI in its workflow, and dev-01 in its
  system Nix configuration (the machines repo).
- If the cache or the repository is compromised, run `determinate-nixd
  logout` on each Mac so nothing more is fetched, and treat any Mac that
  switched since then as compromised: its substituted paths cannot be told
  apart from honest ones.

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

The gate is two commands. `prek run --all-files` lints (file hygiene,
nixfmt, nixf-diagnose, markdownlint), and runs before each commit.
`nix flake check --all-systems` builds: the Mac's system, home and link
check, dev-01's home, link check and deploy-rs checks, and the Moshi hook
version test. Lint is not also a flake check, so CI runs it once. The Linux
checks build on the Mac through Determinate Nix's native Linux builder.

CI checks pull requests on a Linux runner only. It runs the commit hooks,
evaluates every system with `nix flake check --all-systems --no-build`, so an
evaluation error in a Mac configuration fails there too, and builds the Linux
checks. Without `--no-build`, `--all-systems` would try to
build the Mac checks on Linux and fail. The Mac checks are built by the local
gate and, after a merge, by the macOS `cache` job. The evaluation forbids
import-from-derivation, which would need a Mac build during evaluation.
Branch protection requires `check` to pass before a merge into `main`.

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
| fish binds F12 to clear the screen in both vi modes | Ghostty and Rex send Ctrl+Shift+L as F12, which keeps it distinct from Ctrl+L inside Herdr and shells |
| dev-01 builds its own profile (deploy-rs `remoteBuild`) | `dev` is not a trusted Nix user on dev-01, so a closure built on the Mac is refused as unsigned; making `dev` trusted would give that account root-equivalent control of the Nix store |
| `go`, `python3` and (on Macs) `nodejs` are Neovim packages, not global | Mason builds and runs its tools with them; project toolchains come from project dev shells |
| Rex is installed by hand, not by Nix or Homebrew; `~/.local/bin/rex` is a store script that runs the CLI inside its bundle | It has no cask or nixpkgs package, and its preview builds update through Sparkle every few days, which a store copy could not do |
| Rex's key bindings follow Herdr's and skhd's: a Ctrl+A prefix, h/j/k/l for panes, an exclusive resize mode, and Ctrl+H/J/K/L moving between panes unless the pane's foreground program is Neovim, Vim, Herdr or tmux | Rex is on trial beside Ghostty and Herdr, so its keys match the ones already in use. Those programs have splits of their own and get the key instead, as herdr-splits does in Herdr |
| Rex's imported themes are kept as JSON in `config/rex/custom-themes.json` and imported as plist data | Rex stores them as JSON inside a `data` value, which `lib.generators.toPlist` cannot write. The dark theme is chosen by the saved theme's `id`, so the file is copied from Rex after an import, not written by hand |
| WezTerm is no longer installed or linked; `config/wezterm/` is kept | The owner no longer uses it; the config is retained |
| `.envrc` is globally ignored and untracked | These files can hold secrets; `nix develop` provides this repo's shell |
| Home Manager stops a switch when a file it does not own is where a link goes (`backupFileExtension` is unset) | Moving the file aside would hide an application replacing a link this repo manages, and a second collision fails anyway once the backup exists |
| The menu bar manager is Thaw from the `thaw@beta` cask, not Ice | Ice has had no code commit since June 2025 and hangs on "Loading menu bar items" on macOS 27. Thaw is its maintained fork, and only Thaw 3, which ships as a beta, supports macOS 27; the stable `thaw` cask is Thaw 2, for macOS 15 and 26. Move to `thaw` once Thaw 3 is stable |
| Containers run in OrbStack, not Docker Desktop | It starts faster and uses less memory and disk on Apple Silicon, and also runs Linux machines. Commercial use needs a paid licence |
| App Store apps stay in `homebrew.masApps`, not nix-darwin's `programs.mas`, and `mas` comes from nixpkgs, not Homebrew | When the App Store is signed out, `programs.mas` runs `exit 0` inside nix-darwin's single activation script, which skips Homebrew, Home Manager and the switch to the new generation while the switch still reports success. nix-darwin already gives `brew bundle` nixpkgs' `mas`, so the formula was a second copy |
| JankyBorders is nixpkgs' package plus patches in `overlays/modifications.nix`, not a package of our own | The version, source hash and build stay nixpkgs' to update; the overlay adds only three fixes, none submitted upstream: bottom corners near rounded display corners, borders over Mission Control on macOS 27, and borders over sheets. The patched build misses cache.nixos.org, but it is one small C program |
| plannotator-tui, the review pane of Herdr's Annotate plugin, is a package of our own in `pkgs/plannotator-tui`, built from source | nixpkgs lacks it, and it carries local patches (README.md, Patched packages); the plugin otherwise downloads upstream's prebuilt binary. libgit2 and SQLite are built from the copies the crates vendor, as upstream builds them: diffs-rs refuses a system libgit2. oniguruma, which highlighting brings in, links nixpkgs' library, as bat does |
| Herdr's Annotate plugin is a folder Nix builds (`pkgs/herdr-plugin-annotate`), registered at activation with `herdr plugin link`, not installed from GitHub | A GitHub install downloads upstream's prebuilt plannotator-tui, which would not have our patches and rejects the config keys they add. Herdr has no declarative plugin list, and `plugins.json` stays Herdr's file. `link` stores the resolved store path, so activation relinks only when the folder changes, passes the current enabled state back because `link` re-enables, and works with no server running. Linking replaces a GitHub install and keeps the plugin's config and saved annotations. Herdr writes nothing into the plugin folder, so a read-only store path works |
| Annotate's review-pane config is generated from the Tokyo Night Pi theme, not written here | Pi and the review pane then take each colour from one file. Roles Pi has no colour for (the comment and looks-good tints, the toolbar and the cursor) come from Tokyo Night's palette in the same input, the tints blended into the background as Tokyo Night makes its diff backgrounds, rather than plannotator-tui's xterm greys and tints. It is a store file, not a live link, because only the patched plannotator-tui accepts its keys and the two must ship together |
| plannotator-tui draws Mermaid with boxmaid, pinned exactly | Of the Rust crates that draw Mermaid as terminal text, its output was closest to Pi's renderer (lovely-mermaid) on every sample diagram, and it adds six crates. mermaid-text failed a standard class diagram and overran the width; merman's text backend failed a basic flowchart and pulled in 110 crates. boxmaid is young (one author, first released 2026-10), which the exact pin and the fallback to the diagram's source contain |
| plannotator-tui's patches are `cargoPatches` in its own package, not an `overrideAttrs` in `overlays/modifications.nix` | Two patches change `Cargo.lock`, and the vendored dependencies are fetched from the patched lock file. `overrideAttrs` cannot reach that fetch, and the package is ours anyway, so its version already stops the patches from silently applying to a new release |
| Microsoft AutoUpdate is its own cask | The Word, Excel and PowerPoint casks deselect it in their installers, so a new Mac would not get Office's updater from them |
| `killall Finder` and `SystemUIServer` run after defaults are written | nix-darwin restarts only the Dock |
| The Finder desktop is shown (`finder.CreateDesktop = true`) | With it hidden, switching to a desktop with no windows left macOS nothing to focus there, so it activated the previous app and switched back to that app's desktop. Ctrl+number, Alt+number and WhichSpace all bounced this way. It also made `yabai -m display --focus` fail on an empty display. `~/Desktop`'s files show on the desktop. It is set to `true` rather than left out, because nix-darwin never deletes a key it stopped writing |
| Both fzf integrations are enabled in fish | Each contributes live key bindings |
| Herdr comes from nixpkgs on every host | The lock keeps client and remote server on one version |
| Pi's theme is Tokyo Night Night, generated from folke/tokyonight.nvim's own Pi theme (the `tokyonight` input), not written here | Ghostty and Herdr already use it, and the theme's author keeps the file in step with the palette; a flake update takes changes. The name matches the file, because Pi finds a custom theme by `<name>.json`. The other changes replace Neovim background colours that upstream gives to roles Pi draws as text or lines, which hides them: `mdCodeBlock` is the terminal's foreground instead of the darkest background; `toolDiffAdded`, `toolDiffRemoved` and `toolDiffContext` take the theme's success, error and muted colours instead of Neovim's diff backgrounds; `borderMuted` (Mermaid box borders) takes the thinking-off border instead of near-black; `thinkingLow` takes `border_highlight` instead of `blue7`, which is darker than thinking off; and `userMessageBg` takes `selectedBg` instead of `bg_dark`, which is barely darker than the terminal, as Pi's built-in dark theme gives user messages and selection the same blue. Each replacement is read from the upstream file, so a flake update carries palette changes |
| Keyboard shortcuts are merged into `com.apple.symbolichotkeys` with `defaults write -dict-add` after activation | `system.defaults` writes whole top-level keys, so declaring `AppleSymbolicHotKeys` there would drop every shortcut not listed |
| yabai restarts the Dock two seconds after a display connects or disconnects, and skhd binds Shift+Alt+D to `killall Dock` | On macOS 27, Ctrl+3 and above stop switching desktops once a display has connected or disconnected, while their entries stay intact, and only a Dock restart brings them back. A switch only appeared to fix them because nix-darwin restarts the Dock. The delay lets a burst of display changes finish first. Remove both when Apple fixes it |
| yabai's rules end with `yabai -m rule --apply` | Since yabai 7 a rule only affects windows opened after it is added, and yabai loads the open windows before it runs its config. Without this line a yabai restart tiled windows the rules mean to leave alone |
| yabai logs to `/tmp/yabai_<user>.out.log` and `.err.log` | nix-darwin's module sets no log file, so a failed rule was printed nowhere. The paths are the ones upstream's own launchd service uses |
| skhd and JankyBorders run from Home Manager's `services.skhd` and `services.jankyborders`, not nix-darwin's | Home Manager starts its launchd agents after it links files, so `~/.config/skhd/skhdrc` and `bordersrc` exist when the agents first start. Its modules log to `~/Library/Logs`, and the JankyBorders agent restarts when `bordersrc` changes, since `borders` reads it only at launch. yabai has no Home Manager module and stays in nix-darwin |
| skhd's `~/.config/skhd/skhdrc` is a live link into the checkout, set with `xdg.configFile`, not `services.skhd.config` | That option takes a path or text, and a live link is neither. skhd resolves the link and watches the file it points to, so an edit in the checkout applies live |
| skhd's agent is given a shell's `PATH` | skhd runs each binding through the login shell, and Home Manager's agents get only launchd's minimal `PATH`, where `yabai` would not resolve |
| Desktops are switched with the macOS Ctrl+number shortcuts, not `yabai -m space --focus` | With SIP enabled, yabai 7.1.25 reports success and does nothing on macOS 27 ([yabai issue 2822](https://github.com/asmvik/yabai/issues/2822)). `skip_window_focus_animation` relies on the same mechanism and is not set |
| Alt+1 to Alt+5 switch desktops through a Karabiner rule that sends Ctrl+1 to Ctrl+5, not through skhd | skhd cannot switch desktops (above), and a macOS shortcut can be set to Ctrl+number but not to Alt+number. Karabiner rewrites the key before macOS sees it, so the switch has no delay. Alt+1 to Alt+5 no longer type ¡ ™ £ ¢ ∞ |
| Shift+Alt+1 to Shift+Alt+5 move a window with `yabai -m window --space N --focus` | yabai's wiki lists moving windows between spaces as needing SIP off, but on macOS 27 with SIP on it works, also onto a hidden desktop of the other display. `--focus` then focuses the window, which switches to its desktop |
| There are five desktops. Docked, they are on the external display and the laptop has the one desktop macOS requires, where Teams and Slack go. Undocked, both go to 3 | Desktops 1 to 5 stay on the main display whether docked or not. With SIP on, yabai cannot create or remove desktops, so the counts are set once in Mission Control. A yabai signal places Teams and Slack when a window of theirs opens and five seconds after a display connects or disconnects. The script picks the desktop by its position on each display, so it does not depend on the global numbers |
| The time zone is not declared | macOS sets it automatically, and `time.timeZone` would override that on every switch |
| A Mac on AC power never idle-sleeps (`pmset -c sleep 0`); on battery it sleeps after a minute | A sleeping Mac drops off the tailnet and nothing wakes it over Wi-Fi, so sshd and agent jobs were unreachable unless something held a `caffeinate` assertion. The display still sleeps, and closing the lid with no external display still sleeps the Mac |
| Battery Charge Limit is 80% and Optimized Battery Charging is off, set at activation through PowerUI's private `PowerUISmartChargeClient` | The Mac spends most of its life plugged in, and a battery held full and warm wears fastest. No `pmset` option or configuration profile key sets either setting, and since macOS 27 the firmware refuses third-party charge control, so AlDente and batt now ask this same macOS service for the limit. Calling it directly adds no app or root daemon. Optimized Charging is off so that one setting decides when to charge. If Apple renames the methods, activation prints a warning instead of failing |
| `networking.dns` lists only public resolvers, not `100.100.100.100` | The Tailscale app installs its own resolver for the tailnet's names. The static entry sent every other lookup through Tailscale and left a dead address first in the list whenever Tailscale was down |
| The Dock's icons are declared (`dock.persistent-apps`, `persistent-others`) | A new Mac gets the same Dock. A switch rewrites both lists, so an icon dragged in by hand lasts only until the next switch |
| Siri is turned off through `com.apple.assistant.support`, and AirPlay Receiver by a `defaults -currentHost write` after activation | Neither has a typed option. AirPlay Receiver is a per-host preference, which `system.defaults` cannot write ([nix-darwin issue 1721](https://github.com/nix-darwin/nix-darwin/issues/1721)) |
| The laptop's illuminated Apple logo is turned off with `LEBacklightMode = 0` in root's `com.apple.CoreBrightness` domain, set only on the laptop host | Apple's [documented method](https://support.apple.com/en-us/149248) has no System Settings control. Only MacBook Pros with a PROLED XDR display have the logo, so it stays in `hosts/laptop` |
| Local models run in llama.cpp's router from nixpkgs, not Homebrew or MLX | The router starts each model from the path of its own binary. Homebrew upgrades in place and deletes the old version, so a router left running across its 0.9.0 to 1.0.0 upgrade failed every model load. A store path survives until garbage collection, and a new one changes the agent's plist, so Home Manager restarts the router on an upgrade and only then. Both builds are 1.0.0 with Metal and decode Qwen3.6-35B-A3B at the same speed with the same MTP acceptance. Homebrew was used first because it had 0.9.0 when nixpkgs had 0.8.0. llama.cpp gave the lowest time to first token and a working prompt cache in agent benchmarks, and Pi's built-in `llama.cpp` provider and `/llama` command drive its router |
| llama-server runs always, not only while Pi is open | The idle router holds about 0.1 GB and a loaded model sleeps after 5 idle minutes, dropping from 29 GB to 0.4 GB. Starting and stopping it with Pi needed a Pi extension, a PID registry and a wrapper script to save that little |
| The router loads one model at a time (`--models-max 1`) with a 128K context, MTP speculative decoding and 32 context checkpoints | Two large models together exhaust the 64 GB of memory. MTP raised decode speed with no change to output (88 tok/s on Qwen3.6-35B-A3B). Qwen3.6 is a hybrid model, which reprocesses a partly matching prompt from the start without checkpoints |
| LinearMouse's settings are not declared | The app owns `~/.config/linearmouse/linearmouse.json`, and its settings window names each device by serial number, which this public repository should not carry. Two schemes for the same mouse model differ only by serial, so the serials cannot simply be stripped |
| SSH private keys live only in 1Password; the Mac's `~/.ssh/` holds just their public halves, written by `modules/home/mac/ssh.nix` | With `IdentitiesOnly`, ssh offers only agent keys that match an `IdentityFile`, and a `.pub` file is enough to match. A key on disk without a passphrase could be read by anything running as the user |
| `Host *` sets `IdentitiesOnly yes` and `IdentityFile none`, so a host is offered only the key its entry names and an unlisted host none | Otherwise ssh offered all of 1Password's keys plus the default files to every host: more than sshd's default `MaxAuthTries` of 6, and it shows every server each key held |
| github.com has two `Match` blocks, and git inside `~/businesscraft/` picks the work one with `ssh -P github-bc` | ssh offers agent keys in the agent's order, not `IdentityFile` order, and GitHub signs in as the first key it accepts. `ssh -i github_bc.pub` only added the work key beside the personal one, which 1Password lists first, so work repositories signed in as mich-murphy. `Match tagged` needs OpenSSH 9.4 or newer |
| Connection sharing (`ControlMaster`) is on for dev-01 only | It saves a 1Password approval per connection for ssh, scp and deploy-rs. On github.com a shared connection would carry one account's login into the other account's repositories |
| Pull requests are checked on Linux only; macOS runs only in the `cache` job on `main` | Linux evaluates the Mac configurations too, and the Mac builds run in the local gate on a far faster machine. A macOS pull request job waited up to 16 minutes in the queue and repeated the build the `cache` job does after the merge |
| Mole is nixpkgs' `mole-cleaner`, bumped in `overlays/modifications.nix`, not a package in `pkgs/` | nixpkgs packages tw93's Mole under that name because its `mole` is an unrelated SSH tunnel tool. Its derivation is careful (sandbox fixes, completions, `mo update` and `mo remove` refused); it was 15 releases behind (1.58.0 against 1.72.0) only because the package is Darwin-only and nixpkgs' update bot builds on x86_64-linux, so every automatic bump fails. The override keeps the derivation and changes only the source. It installs only `mo` |
| Codex's update check is turned off with `check_for_update_on_startup = false`, merged into `config.toml` | Codex has no build-time switch for it. Run from the store it offers no upgrade command, but it still checks GitHub and shows a new-version banner that only a switch can act on. The file also holds per-project trust and hook state that Codex writes, so it is merged, not linked |
| The agent CLIs come from numtide's llm-agents.nix, with its cache trusted, not from nixpkgs | nixpkgs-unstable had Claude Code 2.1.323 when upstream had 2.1.326. llm-agents.nix ships each release within a day and prebuilds it for both systems; without its cache, codex compiles from source for over half an hour. Writing and bumping prebuilt-binary packages here would avoid the third-party key, but would take a commit most days. The cost is that key (Binary cache) |
| `~/.local/bin` is appended to `PATH` in fish, not set with `home.sessionPath`, and comes last in Moshi's service `PATH` | `sessionPath` prepends, so installers' copies there (dev-01's `claude`, `codex`, `node`) shadowed Nix's |
