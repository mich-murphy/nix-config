#!/usr/bin/env bash
set -euo pipefail

root=${RUNNER_TEMP:-${TMPDIR:-/tmp}}
link=$root/dev-01-profile
profile_attr=.#deploy.nodes.dev-01.profiles.home.path
host=bsncraft-dev-01.australiaeast.cloudapp.azure.com
host_key="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOUCugJu/n0HygdLoC1mtaYvVuJN92ZVIn164lBOVjoc"

fetch() {
  nix build --out-link "$link" --max-jobs 0 "$profile_attr"
  echo "Checking signatures in the closure of $(readlink "$link")"
  nix store verify --no-contents --recursive --sigs-needed 1 "$link"
}

activate() (
  : "${DEV_01_DEPLOY_KEY:?must hold the deploy SSH private key}"
  : "${GITHUB_SHA:?must name the commit being deployed}"
  : "${GITHUB_REPOSITORY:?must name the repository}"

  local expected
  expected=$(nix eval --raw "$profile_attr")
  if [ "$(readlink "$link" || true)" != "$expected" ]; then
    echo "run '$0 fetch' for this commit first" >&2
    exit 1
  fi

  local tip
  tip=$(git ls-remote "https://github.com/$GITHUB_REPOSITORY" refs/heads/main | cut -f1)
  if [ "$tip" != "$GITHUB_SHA" ]; then
    echo "::notice::main is now at $tip, not $GITHUB_SHA; its own run deploys it"
    exit 0
  fi

  local dir
  dir=$(mktemp -d "$root/dev-01-ssh.XXXXXX")
  trap 'rm -rf "$dir"' EXIT
  case $dir in
    *[[:space:]]*)
      echo "deploy-rs splits --ssh-opts on spaces; $dir has one" >&2
      exit 1
      ;;
  esac
  (umask 077 && printf '%s\n' "$DEV_01_DEPLOY_KEY" >"$dir/key")
  unset DEV_01_DEPLOY_KEY
  printf '%s %s\n' "$host" "$host_key" >"$dir/known_hosts"
  cat >"$dir/config" <<EOF
Host dev-01
  HostName $host
  User dev
  IdentityFile $dir/key
  IdentitiesOnly yes
  IdentityAgent none
  UserKnownHostsFile $dir/known_hosts
  StrictHostKeyChecking yes
  BatchMode yes
EOF

  nix shell --inputs-from . nixpkgs#deploy-rs --command \
    deploy --skip-checks --ssh-opts "-F $dir/config" .#dev-01 -- --max-jobs 0
)

case ${1:-} in
  fetch) fetch ;;
  activate) activate ;;
  *)
    echo "usage: $0 fetch|activate" >&2
    exit 2
    ;;
esac
