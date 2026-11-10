# `nix run .#deploy` and `nix run .#status`, run from the Mac. Each host runs
# fleet-host itself: this one directly, every other one over a single SSH
# session. With no arguments every host in the flake is included.
#
# Usage: deploy|status [host...]
# The flake sets FLEET_ACTION, FLEET_HOSTS (one "name kind configuration
# checkout skills" line per host) and FLEET_HOST_SCRIPT.
this_host=$(hostname -s)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
script=$(base64 <"$FLEET_HOST_SCRIPT" | tr -d '\n')

# Whether the first argument is one of the others.
is_one_of() {
  local item=$1 candidate
  shift
  for candidate in "$@"; do
    if [[ $candidate == "$item" ]]; then return 0; fi
  done
  return 1
}

mapfile -t known < <(cut -f1 "$FLEET_HOSTS")
for host in "$@"; do
  if ! is_one_of "$host" "${known[@]}"; then
    echo "unknown host $host; hosts: ${known[*]}" >&2
    exit 2
  fi
done

run_host() {
  local name=$1 kind=$2
  shift
  if [[ $name == "$this_host" ]]; then
    "$FLEET_HOST_SCRIPT" "$FLEET_ACTION" "$@"
    return
  fi
  local tty=(-n)
  # darwin-rebuild needs sudo, which asks for a password on a terminal.
  if [[ $FLEET_ACTION == deploy && $kind == darwin && -t 0 ]]; then tty=(-t); fi
  # bash -c rather than bash -s, so commands that read stdin cannot eat the
  # script. The remote login shell may be fish; this quoting suits both.
  ssh "${tty[@]}" -o BatchMode=yes -o ConnectTimeout=15 -o ServerAliveInterval=10 "$name" \
    "bash -c 'eval \"\$(echo $script | base64 -d)\"' fleet-host $FLEET_ACTION $*"
}

reached=()
failed=()
while IFS=$'\t' read -r -u 3 name kind config checkout skills; do
  if [[ $# -gt 0 ]] && ! is_one_of "$name" "$@"; then continue; fi
  printf '== %s\n' "$name"
  {
    code=0
    run_host "$name" "$kind" "$config" "$checkout" "$skills" || code=$?
    echo "@exit $code"
  } | tee "$work/$name" | grep --line-buffered -v '^@' || true
  code=$(sed -n 's/^@exit //p' "$work/$name")
  case $code in
    0) reached+=("$name") ;;
    255)
      echo "  unreachable: ssh $name failed"
      failed+=("$name")
      ;;
    *)
      reached+=("$name")
      failed+=("$name")
      ;;
  esac
done 3<"$FLEET_HOSTS"

if [[ $FLEET_ACTION == status && ${#reached[@]} -gt 0 ]]; then
  printf '\n== versions\n  %-14s' ""
  printf ' %-12s' "${reached[@]}"
  echo
  for tool in $(cd "$work" && awk '$1 == "@version" && !seen[$2]++ { print $2 }' "${reached[@]}"); do
    printf '  %-14s' "$tool"
    versions=()
    for host in "${reached[@]}"; do
      version=$(awk -v tool="$tool" '$1 == "@version" && $2 == tool { print $3 }' "$work/$host")
      versions+=("${version:--}")
      printf ' %-12s' "${version:--}"
    done
    if [[ $(printf '%s\n' "${versions[@]}" | sort -u | wc -l) -gt 1 ]]; then printf ' differs'; fi
    echo
  done
fi

if [[ ${#failed[@]} -gt 0 ]]; then
  printf '\npending, failed or unreachable: %s\n' "${failed[*]}"
  exit 1
fi
