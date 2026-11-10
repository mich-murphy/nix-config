# One host's part of `nix run .#deploy` and `nix run .#status`. It runs on the
# host itself (other hosts receive it through `ssh <host> bash -s`), so it uses
# only that host's own git, jq and switch commands.
#
# Usage: fleet-host deploy|status darwin|home <configuration> <checkout> <skills>
action=$1
kind=$2
config=$3
checkout=$4
skills=$5
nvim=${XDG_CONFIG_HOME:-$HOME/.config}/nvim
state=${XDG_STATE_HOME:-$HOME/.local/state}
generation=$state/home-manager/gcroots/current-home
failed=0

report() { printf '  %-14s %s\n' "$1" "$2"; }
pending() {
  failed=1
  report "$1" "pending: $2"
}
short() {
  case $1 in
    *-dirty) printf '%s-dirty' "${1:0:7}" ;;
    *) printf '%s' "${1:0:7}" ;;
  esac
}

# The branch a checkout publishes, from origin's default branch.
published_branch() {
  local ref
  ref=$(git -C "$1" symbolic-ref -q --short refs/remotes/origin/HEAD) || ref=origin/main
  printf '%s' "${ref#origin/}"
}

local_changes() {
  local count
  count=$(git -C "$1" status --porcelain | wc -l)
  count=${count// /}
  if [[ $count != 0 ]]; then printf ', %s local changes' "$count"; fi
}

first_error() {
  local line
  while IFS= read -r line; do
    case $line in error:* | fatal:*)
      printf '%s' "$line"
      return
      ;;
    esac
  done <<<"$1"
  printf '%s' "${1%%$'\n'*}"
}

# Fast-forward a checkout to its published branch. Local edits stay. When Git
# refuses because they conflict, or the checkout has unpushed commits or is on
# another branch, the checkout is left as it is and reported pending.
deploy_repo() {
  local label=$1 dir=$2 branch current before after ahead error
  if [[ ! -d $dir/.git ]]; then
    pending "$label" "no checkout at $dir"
    return 1
  fi
  branch=$(published_branch "$dir")
  current=$(git -C "$dir" branch --show-current)
  if [[ $current != "$branch" ]]; then
    pending "$label" "on ${current:-a detached HEAD}, not $branch"
    return 1
  fi
  before=$(git -C "$dir" rev-parse HEAD)
  if ! error=$(git -C "$dir" pull --quiet --ff-only --no-rebase origin "$branch" 2>&1); then
    pending "$label" "git pull --ff-only failed: $(first_error "$error")"
    return 1
  fi
  ahead=$(git -C "$dir" rev-list --count "origin/$branch..HEAD")
  if [[ $ahead != 0 ]]; then
    pending "$label" "$ahead commits not pushed to origin/$branch"
    return 1
  fi
  after=$(git -C "$dir" rev-parse HEAD)
  if [[ $before == "$after" ]]; then
    report "$label" "up to date at $(short "$after")$(local_changes "$dir")"
  else
    report "$label" "$(short "$before") -> $(short "$after")$(local_changes "$dir")"
  fi
}

# Build and activate the checkout's commit. ?rev= builds the committed tree, so
# local edits never deploy.
switch_host() {
  local rev flake log active
  rev=$(git -C "$checkout" rev-parse HEAD)
  active=$(cat "$state/nix-config/revision" 2>/dev/null) || active=unknown
  if [[ $active == "$rev" ]]; then
    report switch "already active at $(short "$rev")"
    return
  fi
  flake="git+file://$checkout?rev=$rev#$config"
  log=$state/nix-config/deploy.log
  mkdir -p "${log%/*}"
  report switch "building $(short "$rev"), log in $log"
  if [[ $kind == darwin ]]; then
    set -- sudo /run/current-system/sw/bin/darwin-rebuild switch --flake "$flake"
  else
    set -- home-manager switch -b backup --flake "$flake"
  fi
  if ! "$@" >"$log" 2>&1; then
    pending switch "$1 failed; the last lines of its log:"
    tail -n 5 "$log" | sed 's/^/      /'
    return
  fi
  active=$(cat "$state/nix-config/revision" 2>/dev/null) || active=unknown
  if [[ $active == "$rev" ]]; then
    report switch "active at $(short "$rev")"
  else
    pending switch "the switch finished but the active revision is $(short "$active")"
  fi
}

status_repo() {
  local label=$1 dir=$2 branch current ahead behind note=""
  local parts=()
  if [[ ! -d $dir/.git ]]; then
    report "$label" "no checkout at $dir"
    return
  fi
  branch=$(published_branch "$dir")
  git -C "$dir" fetch --quiet origin 2>/dev/null || note=" (fetch failed; last fetched state)"
  current=$(git -C "$dir" branch --show-current)
  if ! read -r ahead behind < <(git -C "$dir" rev-list --left-right --count "HEAD...origin/$branch" 2>/dev/null); then
    report "$label" "$(short "$(git -C "$dir" rev-parse HEAD)") (no origin/$branch to compare)"
    return
  fi
  if [[ $current != "$branch" ]]; then parts+=("on ${current:-a detached HEAD}"); fi
  if [[ $behind != 0 ]]; then parts+=("$behind behind origin/$branch"); fi
  if [[ $ahead != 0 ]]; then parts+=("$ahead not pushed"); fi
  if [[ ${#parts[@]} == 0 ]]; then parts=(published); fi
  report "$label" "$(short "$(git -C "$dir" rev-parse HEAD)" ) ${parts[*]}$(local_changes "$dir")$note"
}

status_active() {
  local active head
  active=$(cat "$state/nix-config/revision" 2>/dev/null) || active=unknown
  head=$(git -C "$checkout" rev-parse HEAD 2>/dev/null) || head=unknown
  if [[ $active == "$head" ]]; then
    report active "$(short "$active"), the checkout's commit"
  else
    report active "$(short "$active"); the checkout is at $(short "$head"), so a deploy is pending"
  fi
}

# Pi installs git packages as clones; `pi update --extensions` adopts updates.
status_pi_packages() {
  local dir upstream head latest
  for dir in "$HOME"/.pi/agent/git/*/*/*; do
    [[ -d $dir/.git ]] || continue
    head=$(git -C "$dir" rev-parse HEAD)
    if ! upstream=$(git -C "$dir" rev-parse --abbrev-ref '@{upstream}' 2>/dev/null); then
      report "${dir##*/}" "pinned at $(short "$head")"
      continue
    fi
    latest=$(git -C "$dir" ls-remote origin "refs/heads/${upstream#origin/}" 2>/dev/null | cut -f1) || latest=""
    if [[ -z $latest ]]; then
      report "${dir##*/}" "$(short "$head") (could not reach origin)"
    elif [[ $latest == "$head" ]]; then
      report "${dir##*/}" "$(short "$head"), latest"
    else
      report "${dir##*/}" "$(short "$head"); ${upstream#origin/} is at $(short "$latest"), run pi update --extensions"
    fi
  done
}

# Every file Home Manager manages should still be its link into the active
# generation; an application that saves a regular file over it breaks the link.
status_links() {
  local files path target count=0
  local replaced=()
  if ! files=$(readlink -f "$generation/home-files"); then
    report links "no active Home Manager generation"
    return
  fi
  while IFS= read -r -d '' path; do
    count=$((count + 1))
    target=$(readlink "$HOME/${path#"$files"/}" || true)
    if [[ $target != "$path" ]]; then replaced+=("${path#"$files"/}"); fi
  done < <(find "$files" -mindepth 1 \( -type l -o -type f \) -print0)
  if [[ ${#replaced[@]} == 0 ]]; then
    report links "all $count managed files linked"
  else
    report links "${#replaced[@]} of $count no longer linked: ${replaced[*]}"
  fi
}

# Declared settings keys whose live value differs; the next switch resets them.
status_settings() {
  local label=$1 live=$2 name=$3 static keys
  if ! static=$(grep -o -m1 -E "/nix/store/[a-z0-9]+-$name" "$generation/activate" 2>/dev/null); then
    return
  fi
  if [[ ! -f $live ]]; then
    report "$label" "$live is missing"
    return
  fi
  if ! keys=$(jq -nr --slurpfile live "$live" --slurpfile static "$static" '
    $live[0] as $l | $static[0] | to_entries[]
    | select(if .key == "packages" then (.value - ($l.packages // [])) != []
             else ($l * {(.key): .value})[.key] != $l[.key] end)
    | .key' | paste -sd ' ' -); then
    report "$label" "could not read $live"
    return
  fi
  if [[ -z $keys ]]; then
    report "$label" "declared settings applied"
  else
    report "$label" "changed since the last switch: $keys"
  fi
}

# Machine-readable lines for the versions table; not shown as they are.
status_versions() {
  local tool output version
  for tool in claude codex pi herdr moshi-hook; do
    version=missing
    if command -v "$tool" >/dev/null; then
      output=$("$tool" --version 2>/dev/null) || true
      version=unknown
      if [[ $output =~ [0-9]+(\.[0-9]+)+ ]]; then version=${BASH_REMATCH[0]}; fi
    fi
    printf '@version %s %s\n' "$tool" "$version"
  done
}

case $action in
  deploy)
    deploy_repo neovim "$nvim" || true
    deploy_repo skills "$skills" || true
    if deploy_repo nix-config "$checkout"; then
      switch_host
    else
      report switch "skipped until nix-config is no longer pending"
    fi
    exit "$failed"
    ;;
  status)
    status_repo nix-config "$checkout"
    status_active
    status_repo neovim "$nvim"
    status_repo skills "$skills"
    status_pi_packages
    status_links
    status_settings claude "$HOME/.claude/settings.json" claude-code-settings.json
    status_settings pi "$HOME/.pi/agent/settings.json" pi-coding-agent-settings.json
    status_versions
    ;;
  *)
    echo "usage: fleet-host deploy|status darwin|home <configuration> <checkout> <skills>" >&2
    exit 2
    ;;
esac
