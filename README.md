# Nix configuration

A personal flake. [nix-darwin](https://github.com/nix-darwin/nix-darwin) (the
Nix module system for macOS) owns each Mac's settings and applications, and
[Home Manager](https://nix-community.github.io/home-manager/) owns the user
environment: embedded in the Mac configuration, and standalone on the Linux VM.

| Host | What it is | Flake output | Switched with |
| --- | --- | --- | --- |
| `laptop` | MacBook Pro (M5), user `michael` | `darwinConfigurations.laptop` | `sudo darwin-rebuild switch --flake .` on the laptop |
| `dev-01` | Ubuntu VM on Azure | `homeConfigurations."dev@dev-01"` | CI's `deploy` job, after each push to `main` |

dev-01 is Home Manager only: the
machines repo's bootstrap owns everything outside its home directory.

## Structure

```text
flake.nix                inputs, mkDarwin, mkHome, deploy node, checks
hosts/<host>/
├── default.nix          nix-darwin: host name, platform, user name, stateVersion (Macs)
└── home.nix             Home Manager: host-only modules, identity, stateVersion
modules/darwin/          nix-darwin modules shared by the Macs
modules/home/            Home Manager modules shared by every host
└── mac/                 Home Manager modules only the Macs import
pkgs/<name>/             packages nixpkgs lacks (moshi-hook, plannotator-tui), added by overlays/additions.nix
overlays/                applied to every package set, so modules use pkgs.<name>
├── additions.nix        packages from pkgs/
├── modifications.nix    local fixes to nixpkgs packages (JankyBorders patches, Mole bump)
└── patches/<package>/   patch files the overlays apply
config/                  raw application files, linked or read by the modules
prek.toml                commit hooks
.github/workflows/       CI: pull requests on Linux; main built and cached on Namespace
```

Concern files define existing nix-darwin or Home Manager options. Add a setting
to the shared module for its concern, or to `hosts/<host>/` when only one
machine needs it.

| Module | What it does |
| --- | --- |
| `darwin/system.nix` | DNS, firewall, Touch ID for sudo, tailnet-only sshd, Dock, Finder, keyboard, power settings, battery Charge Limit |
| `darwin/applications.nix` | system packages, fonts, Homebrew casks, formulae and App Store apps |
| `darwin/nix.nix` | Determinate Nix owning the Nix installation and garbage collection, a weekly launchd job that expires system generations older than 7 days, no bundled `darwin-uninstaller` |
| `darwin/user.nix` | the primary user's account and shell (the name itself is set in `hosts/<host>/default.nix`), Home Manager's integration settings |
| `darwin/window-management.nix` | yabai |
| `home/live-files.nix` | `nixConfig.checkout`, `nixConfig.skills` and `lib.nixConfig.file`, see [Live configuration](#live-configuration) |
| `home/shell.nix`, `cli.nix`, `yazi.nix` | fish, command-line tools, yazi |
| `home/git.nix` | Git settings, `hunk` diff viewer, `gh`, `lazygit` |
| `home/neovim.nix` | Neovim and the runtime tools Mason needs |
| `home/herdr.nix` | Herdr (terminal multiplexer) and its live config, the Annotate plugin linked from Nix and its review pane's Tokyo Night config |
| `home/agents.nix` | skills links, `AGENTS.md`, Claude Code, Codex and Pi from llm-agents.nix with their settings, Pi's Tokyo Night theme |
| `home/moshi.nix` | the Moshi user service, see [Services](#services) |
| `home/mac/ssh.nix`, `git.nix` | SSH hosts and keys, Git identities and macOS ignores |
| `home/mac/terminals.nix`, `karabiner.nix` | Ghostty and Karabiner, as live links |
| `home/mac/window-management.nix` | JankyBorders, and skhd with its live `skhdrc` |
| `home/mac/packages.nix` | Mac-only tools (`doctl`, Mole as `mo`, document rendering for Neovim) |

`mkDarwin` and `mkHome` in `flake.nix` assemble each host from these directories.
A host's `home.nix` adds host-only modules on top of the shared home; each Mac's
imports `modules/home/mac`, dev-01's does not.

```mermaid
flowchart LR
  MD["modules/darwin"] --> MKD["mkDarwin"]
  HD["hosts/HOST/default.nix"] --> MKD
  MH["modules/home"] --> MKD
  MH --> MKH["mkHome"]
  HH["hosts/HOST/home.nix"] --> MKD
  MAC["modules/home/mac"] -->|"imported by Mac home.nix"| HH
  DH["hosts/dev-01/home.nix"] --> MKH
  MKD --> DC["darwinConfigurations: laptop"]
  MKH --> HC["homeConfigurations: dev-01"]
  HC --> DR["deploy.nodes.dev-01 (deploy-rs)"]
```

## Set up a new Mac

Needed once on a new machine, in this order:

1. Install Nix with the [Determinate installer](https://docs.determinate.systems/).
   Determinate Nix is a Nix distribution that manages its own daemon,
   configuration and garbage collection; this flake's `determinateNix.enable`
   makes nix-darwin leave Nix to it.

   Then give its daemon Full Disk Access, which Nix cannot declare: in System
   Settings, Privacy & Security, Full Disk Access, add
   `/usr/local/bin/determinate-nixd`, and restart it with
   `sudo launchctl kickstart -k system/systems.determinate.nix-daemon`. Without
   it, macOS stops the daemon deleting any app bundle in the store that has
   been run, and `nix store gc` aborts with `fchmodat … Operation not
   permitted`.
2. Install [Homebrew](https://brew.sh/). nix-darwin installs the casks and
   formulae listed in `modules/darwin/applications.nix`, but not Homebrew
   itself.
3. Clone this repository over HTTPS; SSH works only after the first switch:
   `git clone https://github.com/mich-murphy/nix-config.git ~/dev/nix-config`.
   The Macs link live files into `~/dev/nix-config`; keep it there.
4. Clone the two repositories Home Manager links to, also over HTTPS:

   ```sh
   git clone https://github.com/mich-murphy/skills.git ~/dev/skills
   git clone https://github.com/mich-murphy/neovim.git ~/.config/nvim
   ```

5. Set up the 1Password SSH agent. The private keys exist only in 1Password.
   This flake writes each key's public half to `~/.ssh/`, and the SSH config
   uses that file to pick the one key a host needs from the agent. Sign in to
   the 1Password app, then turn on Settings, Developer, "Use the SSH agent".
   The agent needs these items:

   | 1Password item | Public key file | Used for |
   | --- | --- | --- |
   | `GitHub - mich-murphy` | `github.pub` | github.com (personal account) |
   | `GitHub - BusinessCraft` | `github_bc.pub` | Git inside `~/businesscraft/` (work account) |
   | `dev-01` | `dev-01.pub` | the `dev-01` host |
   | `docker-host` | `docker-host.pub` | `docker-host` on the tailnet |
   | `truenas` | `truenas.pub` | `truenas`, reached through `docker-host` |

   A host is offered only the key its entry in `modules/home/mac/ssh.nix`
   names (GitHub's entries are in `git.nix`), and a host with no entry is
   offered none. To add a key, put its public half in `onePasswordPublicKeys`
   there and name it in the host's entry.
6. Check, then do the first switch. `darwin-rebuild` does not exist yet, so run
   it through `nix run`, and name the host: the machine's hostname does not match
   a flake name until this first activation has set it.

   ```sh
   cd ~/dev/nix-config
   nix flake check --no-write-lock-file
   sudo nix run nix-darwin/master#darwin-rebuild -- switch --flake .#laptop
   ```

   From then on `sudo darwin-rebuild switch --flake .` is enough. Log out and
   back in once, so that macOS loads the keyboard shortcuts and key repeat the
   switch wrote. Then move the clones from step 3 and 4 to SSH:

   ```sh
   git -C ~/dev/nix-config remote set-url origin git@github.com:mich-murphy/nix-config.git
   git -C ~/dev/skills remote set-url origin git@github.com:mich-murphy/skills.git
   git -C ~/.config/nvim remote set-url origin git@github.com:mich-murphy/neovim.git
   ```

   If a switch stops with "Existing file ... would be clobbered", something
   has written a file where Home Manager puts a link. Move that file aside and
   switch again.
7. Open a new shell, then install the commit hooks: `prek install`.
8. Install the Herdr plugin Nix does not install. `config/herdr/config.toml`
   binds keys to it and to Annotate, which a switch builds and registers
   with `herdr plugin link` (`modules/home/herdr.nix`). Do not
   `herdr plugin install` Annotate: Herdr refuses to install over the link.

   ```sh
   herdr plugin install -y lmilojevicc/herdr-splits.nvim --ref v0.5.4
   herdr plugin list   # annotate shows local:/nix/store/…
   ```

## Set up dev-01

dev-01 has no checkout of this repo. CI deploys it after each push to `main`;
a set-up Mac (above) can deploy it too. Items marked "assumed" could not be
checked from the Mac.

On dev-01:

1. Install Nix (assumed: Determinate, the same as on the Mac) and enable flakes.
2. Have the `dev` user with `/home/dev` and SSH access from the Mac with the
   `dev-01` 1Password key (the Mac's `dev-01` SSH alias names that user and
   key).
3. Clone the same skills and Neovim repositories to `~/dev/skills` and
   `~/.config/nvim` (they need GitHub access from the VM).
4. Create `~/.gitconfig-work`, the work identity that `hosts/dev-01/home.nix`
   includes (the machines repo's bootstrap is meant to write it), and the
   `~/businesscraft/.scratch` directory that `TMPDIR` points at.

dev-01 does not read the Namespace cache, and it builds nothing for a deploy:
it is sent a built profile (`remoteBuild = false`). `dev` is not a trusted Nix
user, so dev-01 adds a copied path only if a key in its system Nix
configuration signed it. The machines repo owns that configuration
(`etc/nix/nix.custom.conf`): Numtide's key, for the agent CLIs, and
`nix-config-1`, the key CI signs this repo's builds with on `main`. A profile
the Mac built from a branch or a dirty tree carries no such signature, so the
copy is refused. This governs what is copied in, not what `dev` may activate:
anything logged in as `dev` can still build and activate a profile on dev-01
itself.

CI's `deploy` job runs after both `build` jobs on `main`. It fetches the
profile without building anything (`--max-jobs 0`) and checks that every path
in it is signed by a trusted key or content-addressed, the rule dev-01 applies
too (`.github/deploy-dev-01.sh fetch`). Then, if that commit is
still the tip of `main`, it runs deploy-rs (`activate`), so a rerun of an
older run cannot roll back a newer deploy. It connects with the
`DEV_01_DEPLOY_KEY` secret, in the `dev-01` environment that only `main` may
use, and pins dev-01's host key. dev-01's `~/.ssh/authorized_keys` holds the
public half with `restrict`, which allows commands but no forwarding; the
machines repo's bootstrap keeps it there:

```text
restrict ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEUpYHixc/Ah8dLw67avNjJo+Frnl2an0pK+EiqXmjt3 nix-config-ci-deploy
```

To deploy from the Mac instead, for example while CI is down, check out a
commit whose `build` run on `main` has finished. Home Manager marks its small
files as not worth downloading, so the Mac builds them itself, and its copies
carry no CI signature; fetch the profile, add CI's signatures to anything the
Mac already built, then deploy:

1. Deploy the signed profile:

   ```sh
   profile=$(nix eval --raw .#deploy.nodes.dev-01.profiles.home.path)
   nix build --no-link --max-jobs 0 --option always-allow-substitutes true "$profile"
   nix store copy-sigs --recursive --substituter http://127.0.0.1:18765 \
     --substituter https://cache.nixos.org --substituter https://cache.numtide.com "$profile"
   deploy --skip-checks .#dev-01 -- --max-jobs 0
   ```

2. On dev-01, run the `herdr plugin install` command from step 8 above.

## Day to day

```mermaid
flowchart TD
  E["Edit on the Mac"] --> C["nix flake check"]
  C --> G["git commit"]
  G --> S["sudo darwin-rebuild switch --flake ."]
  G --> PR["pull request, merged to main"]
  PR --> B["CI build: every check, signed into the cache"]
  B --> D["CI deploy: fetch the signed profile, copy it to dev-01"]
  D --> A["activate on dev-01"]
  A --> K{"confirmed within 30 s?"}
  K -->|yes| OK["new generation stays"]
  K -->|no| RB["automatic rollback"]
  R["push to the skills or Neovim repo"] -.->|"no deploy involved"| P["git pull in that clone on dev-01"]
```

1. Edit on the Mac.
2. Verify: `nix flake check` (see [Verification](#verification)).
3. Commit. Both switch commands build the working tree as it is, so an
   uncommitted tree still builds, but the recorded revision then ends in
   `-dirty`.
4. Switch the Mac with `sudo darwin-rebuild switch --flake .`. The Nix
   daemon reads its substituters and keys only at start, so after a switch
   that changes them (`modules/darwin/nix.nix`), restart it with
   `sudo launchctl kickstart -k system/systems.determinate.nix-daemon`;
   until then it builds what the new cache would serve.
5. dev-01 deploys itself: merging to `main` runs the `build` job, then the
   `deploy` job, which activates the new profile on dev-01. If the deploy
   fails, rerun the latest run's deploy with `gh run rerun <run-id> --failed`;
   a rerun of an older run deploys nothing once `main` has moved on. GitHub
   keeps one pending run per group, so a burst of merges can cancel a pending
   run; the next run deploys the newer tip. Or deploy from the Mac (see
   [Set up dev-01](#set-up-dev-01)).
6. See what each host runs:

   | Host | Command |
   | --- | --- |
   | Mac | `darwin-version --configuration-revision` |
   | dev-01 | `ssh dev-01 cat .local/state/nix-config/revision` |

7. Roll back. On a Mac: `sudo darwin-rebuild switch --rollback`. On dev-01: a
   failed activation, or no confirmation within 30 seconds, rolls back by
   itself (deploy-rs "magic rollback"). To undo a deploy that went through,
   revert the commit on `main`, and CI deploys the revert. While CI is down,
   check out an earlier `main` commit whose `build` run finished and follow
   the Mac deploy steps in [Set up dev-01](#set-up-dev-01); a commit CI never
   built is refused by dev-01.

## Verification

Two commands are the local gate: `prek run --all-files` lints, and
`nix flake check` builds the Mac's checks. The commit hooks run prek at each
commit. The Mac never builds Linux; the pull request's `check` job does.

```mermaid
flowchart LR
  subgraph PK["prek at commit"]
    H1["builtin hygiene and syntax"]
    H2["markdownlint"]
    H3["nix format and nix lint"]
  end
  subgraph MAC["nix flake check on the Mac: aarch64-darwin"]
    M2["laptop: system, links, github-accounts"]
    M3["moshi-hook-version, plannotator-tui-version, namespace-devbox-version"]
  end
  subgraph LIN["check job on each pull request: x86_64-linux"]
    L1["dev-01-links, dev-01-deploy-profile"]
    L2["deploy-schema, deploy-activate"]
    L3["moshi-hook-version, plannotator-tui-version, namespace-devbox-version"]
  end
```

- `<host>-system` builds each Mac, including its Home Manager generation; `<host>-links` and
  `dev-01-links` confirm every live link points at a file that exists in this
  repo (or into the skills clone; dev-01 has no checkout).
- `<host>-github-accounts` checks with `ssh -G` that each GitHub host and
  remote selects the right account's key.
- `deploy-schema` and `deploy-activate` are deploy-rs's checks of the dev-01
  node. `dev-01-deploy-profile` is the profile deploy-rs copies to dev-01, so
  the `build` job on `main` builds and signs it.
- `moshi-hook-version` runs `moshi-hook --version` on Linux and, on the Macs,
  reads the version from the app bundle's `Info.plist` without running it.
- `plannotator-tui-version` builds plannotator-tui from source, with its
  tests, and runs `plannotator-tui --version`.
- `namespace-devbox-version` runs `devbox version`.

`prek run --all-files` is the only lint: nixfmt, nixf-diagnose, an ast-grep
rule that rejects Nix comments, markdownlint, zizmor for the workflows and
file hygiene hooks;
`prek install` (set-up step 7) makes it run on each commit.

The GitHub workflow runs on every pull request, on each push to `main` and on
demand. Pull requests run one Linux job, `check`: the commit hooks,
`nix flake check --all-systems --no-build` to evaluate every host, the Macs
included, and `nix flake check` to build the Linux checks. Branches of this
repository run it on a Namespace runner, which reads `main`'s signed builds
through the `nixcache` proxy, so only what the pull request changes is built.
Forks run it on GitHub's runner, which holds no Namespace token. Before a
merge, the Mac checks are built only by the local gate. Branch protection
requires `check` to pass before a pull request merges.

On a push to `main` the `build` job runs instead, on Namespace runners: one
macOS (`aarch64-darwin`) and one Linux (`x86_64-linux`). Each builds every
check for its system with `.github/push-cache.sh`, signs the closures with
`nix-config-1` and copies every path cache.nixos.org lacks to Namespace's Nix
cache, so the hosts download those paths rather than build them. A switch does
not wait for it; anything not yet cached builds locally.

## Live configuration

Some files are
[out-of-store links](https://nix-community.github.io/home-manager/usage/dotfiles.html):
symlinks into a working copy, so an edit applies without a switch. Where a host
has no checkout (`nixConfig.checkout = null` on dev-01), the same file comes from
the deployed generation and is read-only.

| File or tool | Mac | dev-01 | Edit it |
| --- | --- | --- | --- |
| Ghostty, Karabiner, skhd | live link into the checkout | not installed | on the Mac, in `config/` |
| Herdr, `AGENTS.md` | live link into the checkout | read-only, from the deployed generation | on the Mac, then deploy |
| Skills | live link into `~/dev/skills` | live link into `~/dev/skills` | in that clone; `git pull` on dev-01 |
| Neovim config | clone at `~/.config/nvim` | clone at `~/.config/nvim` | in that clone; `git pull` on dev-01 |
| Claude Code and Pi settings | regular files; a switch merges the keys declared in `modules/home/agents.nix` | same, on deploy | declared keys in Nix; the rest in the app |
| Codex `config.toml` | regular file; a switch merges the key declared in `modules/home/agents.nix` | same, on deploy | in the app |

Notes:

- A declared setting changed in the app is reset on the next switch; change it
  in Nix. If a switch stops with "changed during activation", an agent saved its
  settings at that moment; run it again.
- `lib.nixConfig.file "<path>"` in a module picks the live link or the store
  copy. If a Mac's checkout moves, set `nixConfig.checkout`.
- Herdr does not watch its config. After editing it, press `Ctrl+A`, then
  `Shift+R` in each running session.
- Karabiner owns its whole config directory; its generated backups and complex
  modifications are git-ignored.
- `config/wezterm/` is kept for possible future use; WezTerm is not installed
  or linked.
- The Dock's icons are declared in `darwin/system.nix`. One dragged in by hand
  lasts until the next switch.
- LinearMouse's settings are not in this repository; set them in the app on
  each Mac.
- yabai writes to `/tmp/yabai_<user>.err.log` and skhd to
  `~/Library/Logs/skhd/skhd.err.log`. Look there when a rule or a key binding
  does nothing.
- JankyBorders draws a 3 pt cyan border around the focused window. Its
  settings are in `home/mac/window-management.nix`; to try a value without a
  switch, run `borders <option>=<value>`, which updates the running instance
  until it restarts. It logs to `~/Library/Logs/borders/`. It is patched so
  that the border follows the rounder bottom corners macOS gives a window near
  the display's rounded corners, hides while Mission Control is open, and
  stays below a window's sheets and open panels; see
  [Patched packages](#patched-packages).
- Desktops: docked, the external display has 1 to 5 and the laptop one, which
  holds Teams and Slack; undocked, the laptop has 1 to 5 and Teams and Slack
  move to 3. Set the counts in Mission Control; yabai cannot add or remove
  desktops with SIP on. Alt+number switches desktop, Shift+Alt+number moves
  the focused window there and follows it, and Shift+Alt+H/J/K/L or N/P move
  it to another display. If a window will not move, yabai missed it when it opened;
  `launchctl kickstart -k gui/$(id -u)/org.nixos.yabai` restarts yabai.
- On macOS 27, Ctrl+3 and above stop switching desktops after a display
  connects or disconnects, until the Dock restarts. yabai restarts it two
  seconds after each change, so expect the Dock to flicker then. Shift+Alt+D
  restarts it by hand; windows stay where they are.

## Where packages go

| Put it in | When |
| --- | --- |
| `environment.systemPackages` | a tool for every user of the Mac, root included, or an app bundle that must sit in `/Applications/Nix Apps` |
| `home.packages` | a general command-line tool used in any directory |
| `programs.neovim.extraPackages` | only Neovim or Mason shells out to it (`go`, `python3` and, on the Macs, `nodejs` live here for Mason) |
| Homebrew | a closed-source Mac app, or one that needs a system extension, a privileged helper, a fixed `/Applications` path or macOS permissions that reset when its signature changes; or something nixpkgs lacks |
| a project's dev shell | it is that project's toolchain |

### Changing the application list

Homebrew cleanup runs on every switch and uninstalls, without `zap`, what
`modules/darwin/applications.nix` no longer declares. That includes App Store
apps installed by hand, despite what the `masApps` option text says
([nix-darwin issue 1860](https://github.com/nix-darwin/nix-darwin/issues/1860)).
Before switching, list what the working tree would remove:

```sh
nix eval --raw .#darwinConfigurations.laptop.config.homebrew.brewfile |
  HOMEBREW_NO_AUTO_UPDATE=1 brew bundle cleanup --file=-
```

Without `--force` it only lists; no output means nothing would be removed.

### Patched packages

The `jankyborders` entry in `overlays/modifications.nix` overrides nixpkgs'
JankyBorders with three patches in `overlays/patches/jankyborders/`, so its
version follows nixpkgs. Each patch is one `git format-patch` commit, and they
apply in this order on top of upstream's `v1.9.0` tag:

| Patch | Fixes |
| --- | --- |
| `screen-corner-radii.patch` | the border keeps the normal radius where macOS rounds a window's bottom corners near the display's rounded corners |
| `mission-control.patch` | on macOS 27 the borders stay on screen, unscaled, over Mission Control ([issue 209](https://github.com/FelixKratz/JankyBorders/issues/209)); based on [andrewcincotta/JankyBorders@0017b53](https://github.com/andrewcincotta/JankyBorders/commit/0017b53) |
| `attached-sheets.patch` | the border is drawn over a window's sheets and open panels |

cache.nixos.org has no build of the patched package, so it builds locally,
which takes a few seconds.

When a nixpkgs update bumps JankyBorders, evaluation stops at the version
assertion in `overlays/modifications.nix`. Check which of the fixes the new
release has, and delete those patches from the overlay and from
`overlays/patches/jankyborders`. Apply the rest to the new tag, resolve the
conflicts, export them again and update the asserted version (`v1.10.0`
stands for the new version):

```sh
patches=~/dev/nix-config/overlays/patches/jankyborders
git clone https://github.com/FelixKratz/JankyBorders && cd JankyBorders
git switch -c local-fixes v1.10.0
for p in screen-corner-radii mission-control attached-sheets; do
  git am -3 "$patches/$p.patch"   # on conflict: fix, git add, git am --continue
done
make && ./bin/borders --version
git format-patch -1 HEAD~2 --stdout --no-signature > "$patches/screen-corner-radii.patch"
git format-patch -1 HEAD~1 --stdout --no-signature > "$patches/mission-control.patch"
git format-patch -1 HEAD --stdout --no-signature > "$patches/attached-sheets.patch"
nix build ~/dev/nix-config#jankyborders
```

Once upstream has all three fixes, delete the `jankyborders` entry in
`overlays/modifications.nix` and `overlays/patches/jankyborders`.

`pkgs/plannotator-tui/package.nix` builds plannotator-tui, the review pane of
Herdr's Annotate plugin, with patches in `overlays/patches/plannotator-tui/`.
None of the first four is submitted upstream. Each patch is one `git format-patch` commit, and
they apply in this order on top of upstream's `v0.10.0` tag:

| Patch | Adds |
| --- | --- |
| `colour-roles.patch` | `[colors]` roles for inline code, code blocks, links and quotes, which upstream hardcodes |
| `code-highlighting.patch` | syntax highlighting of fenced code with the TextMate theme `[ui] code_theme` names; unset, nothing is highlighted |
| `mermaid-diagrams.patch` | fenced `mermaid` blocks drawn as text by [boxmaid](https://crates.io/crates/boxmaid), pinned to `=0.1.1`; `[ui] mermaid = false` turns it off, and a diagram boxmaid cannot draw shows its source |
| `border-role.patch` | a `[colors] border` role for table borders, unfocused note bubbles and the file-tree divider, which upstream draws in a fixed dark gray |
| `test-races.patch` | upstream's unreleased a059de2, trimmed to files present at `v0.10.0`: fixes "Text file busy" and other test races; drop it on the next release |

The first four are `cargoPatches`, not `patches`: two change `Cargo.lock`, and
the vendored dependencies are fetched after the patches apply. A change to a
patch's `Cargo.lock` hunk therefore changes `cargoHash` too.

To move to a new release (`v0.11.0` stands for it), apply the patches to the
new tag, resolve the conflicts, export them again, then update `version`,
`hash` and `cargoHash` together (build once with `lib.fakeHash` for each hash
and copy the one Nix reports):

```sh
patches=~/dev/nix-config/overlays/patches/plannotator-tui
git clone https://github.com/plannotator/plannotator-tui && cd plannotator-tui
git switch -c local-fixes v0.11.0
for p in colour-roles code-highlighting mermaid-diagrams border-role; do
  git am -3 "$patches/$p.patch"   # on conflict: fix, git add, git am --continue
done
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
git format-patch -1 HEAD~3 --stdout --no-signature > "$patches/colour-roles.patch"
git format-patch -1 HEAD~2 --stdout --no-signature > "$patches/code-highlighting.patch"
git format-patch -1 HEAD~1 --stdout --no-signature > "$patches/mermaid-diagrams.patch"
git format-patch -1 HEAD --stdout --no-signature > "$patches/border-role.patch"
nix build ~/dev/nix-config#plannotator-tui
```

Run the `cargo` commands inside
`nix shell nixpkgs#cargo nixpkgs#rustc nixpkgs#clippy nixpkgs#rustfmt`.

## Services

Moshi runs as a user service on every host (launchd on the Macs, systemd on
dev-01). The laptop also runs llama-server.

**Moshi** (`pkgs/moshi-hook`) is pinned to one release. To update it, change
`version` and both checksums from Moshi's
[Homebrew formula](https://github.com/rjyo/homebrew-moshi/blob/main/Formula/moshi-hook.rb).
Hooks written into each agent name a stable path (`/Applications/Nix
Apps/MoshiHook.app` on the Macs, `~/.local/libexec/moshi-hook` on Linux), so do
not change it. A switch reinstalls the hooks only when `moshi-hook status`
reports them out of date. The Macs run the service from the installed app
bundle, never from the store, because macOS makes a store bundle that has run
undeletable for garbage collection.

**llama-server** (`hosts/laptop/llama.nix`) serves local models to Pi through
llama.cpp's router on `127.0.0.1:9931`, which `LLAMA_BASE_URL` points Pi at.
llama.cpp comes from nixpkgs, and the router runs at login. Presets are in
`config/llama/models.ini`, with model paths relative to `~/models`. A loaded
model sleeps after 5 idle minutes and frees its memory; the idle router holds
about 0.1 GB. Select a model with `/model` in Pi, or load one with `/llama`.
The log is `~/Library/Logs/llama-server.log`. A switch that upgrades llama.cpp
restarts the router, which unloads any loaded model; other switches leave it
running. To restart it by hand, run
`launchctl kickstart -k gui/$(id -u)/org.nix-community.home.llama-server`.

The model files are not managed by Nix. To fetch them on a new laptop:

```sh
hf=https://huggingface.co/unsloth
fetch() { curl -fL --create-dirs -C - -o ~/models/$1 "$hf/$2?download=true"; }
fetch qwen3.6-35b-a3b/Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf Qwen3.6-35B-A3B-MTP-GGUF/resolve/main/Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf
fetch qwen3.6-35b-a3b/mmproj-F16.gguf Qwen3.6-35B-A3B-MTP-GGUF/resolve/main/mmproj-F16.gguf
```

## Remote Docker builds on dev-01

`hosts/dev-01/namespace.nix` adds `ns-buildx`, which runs
`nsc docker buildx` with a token that can only use Namespace's Docker
builders. dev-01 has no `nsc login`, so nothing there can write to
Namespace's caches. Create the token on the Mac, as a workspace admin, and
copy it over:

```sh
umask 077
tmp=$(mktemp -d)
nsc token create --name dev-01-docker-builder --expires_in 8760h \
  --grant '{"resource_type":"builder","resource_id":"*","actions":["ensure","access"]}' \
  --grant '{"resource_type":"tenant","resource_id":"*","actions":["get"]}' \
  --token_file "$tmp/builder.json"
ssh dev-01 'umask 077; mkdir -p ~/.config/namespace'
scp "$tmp/builder.json" dev-01:.config/namespace/builder.json
rm -rf "$tmp"
```

On dev-01, `ns-buildx` registers the `nsc-remote` builder without making it
the default; build with `docker buildx build --builder nsc-remote`. Any other
arguments go to `nsc docker buildx`, such as `ns-buildx cleanup --name
nsc-remote`.

Turborepo on dev-01 reads CI's cache through `turbo-cache-refresh`, which the
businesscraft setup owns, using a read-only `cache/turborepo` token.

## Notes

- Determinate Nix manages Nix itself and its garbage collection on the Macs
  (`determinateNix.enable`); nix-darwin's own Nix management is off.
- The user flake registry pins `nixpkgs` to this flake's input, so
  `nix shell nixpkgs#…` uses the same revision as the system instead of
  Determinate's default `nixpkgs-weekly`.
- The Mac reads CI's builds from Namespace's Nix cache through nixcache, a
  launchd agent on `127.0.0.1:18765` that uses your `nsc login` session. The
  session expires after about a month; renew it with `nsc login` and check it
  with `nsc auth check-login`. Without it the proxy cannot read the cache and
  the Mac builds what the cache would serve. The proxy runs with
  `-cache-upstream=local-only`, like CI's: CI skips uploading a NAR that
  cache.nixos.org already holds, so the proxy reads such NARs from
  cache.nixos.org, without copying them into our storage. The proxy logs to
  `~/Library/Logs/nixcache.log`.
- `nix develop` provides `nixfmt`, `nixf-diagnose`, `nvd` and `deploy`. For
  direnv, create a local `.envrc` containing `use flake` (globally ignored).
- `nvd diff /run/current-system ./result` after `darwin-rebuild build --flake .`
  shows what a switch would change.
