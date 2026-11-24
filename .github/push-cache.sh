#!/usr/bin/env bash
# Builds every check for one system, or copies built closures, signed with
# the key in NIX_CACHE_SIGNING_KEY, to a Nix binary cache. The workflow runs
# the two as separate steps, so no build or evaluation sees the key.
#
# Usage: .github/push-cache.sh build <system> <paths-file>
#        NIX_CACHE_SIGNING_KEY=<secret> .github/push-cache.sh push <cache-url> <paths-file>
#
# The key is written to a 0600 file, read by `nix copy` locally (never sent
# to the cache), and deleted on exit.
set -euo pipefail

case ${1:-} in
  build)
    system=$2
    paths_file=$3
    checks=$(nix eval --raw ".#checks.$system" \
      --apply 'c: builtins.concatStringsSep " " (builtins.attrNames c)')
    installables=()
    for name in $checks; do
      installables+=(".#checks.$system.$name")
    done
    echo "Building ${#installables[@]} checks for $system: $checks"
    nix build --no-link --print-out-paths --print-build-logs "${installables[@]}" >"$paths_file"
    if [ ! -s "$paths_file" ]; then
      echo "nix build printed no store paths" >&2
      exit 1
    fi
    ;;
  push)
    cache=$2
    paths_file=$3
    : "${NIX_CACHE_SIGNING_KEY:?must hold the secret signing key}"
    paths=()
    while IFS= read -r path; do
      paths+=("$path")
    done <"$paths_file"
    if [ "${#paths[@]}" -eq 0 ]; then
      echo "no store paths in $paths_file" >&2
      exit 1
    fi
    key_dir=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/nix-cache-key.XXXXXX")
    trap 'rm -rf "$key_dir"' EXIT
    (umask 077 && printf '%s\n' "$NIX_CACHE_SIGNING_KEY" >"$key_dir/key")
    unset NIX_CACHE_SIGNING_KEY
    echo "Copying ${#paths[@]} closures to $cache"
    nix copy --to "$cache?secret-key=$key_dir/key" "${paths[@]}"
    ;;
  *)
    echo "usage: $0 build <system> <paths-file> | push <cache-url> <paths-file>" >&2
    exit 2
    ;;
esac
