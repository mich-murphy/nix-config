#!/usr/bin/env bash
# Starts or stops Namespace's nixcache proxy on http://127.0.0.1:18765, on a
# Namespace runner, whose workload token authorises it.
#
# Usage: .github/nixcache.sh start|stop
#
# local-only: public paths are answered from cache.nixos.org (so `nix copy`
# skips them) without being mirrored into our cache, while our own signed
# uploads are still written and read.
set -euo pipefail

dir=${RUNNER_TEMP:-${TMPDIR:-/tmp}}

case ${1:-} in
  start)
    nix build .#nixcache --out-link "$dir/nixcache" --option substituters https://cache.nixos.org
    nohup "$dir/nixcache/bin/nixcache" -foreground \
      -cache-upstream local-only \
      -cache_dir "$dir/nixcache-dir" \
      >"$dir/nixcache.log" 2>&1 &
    echo $! >"$dir/nixcache.pid"
    for _ in $(seq 30); do
      curl -fsS http://127.0.0.1:18765/nix-cache-info && exit 0
      sleep 1
    done
    cat "$dir/nixcache.log"
    exit 1
    ;;
  stop)
    if [ -f "$dir/nixcache.pid" ]; then
      kill -TERM "$(cat "$dir/nixcache.pid")" || true
    fi
    tail -n 50 "$dir/nixcache.log" || true
    ;;
  *)
    echo "usage: $0 start|stop" >&2
    exit 2
    ;;
esac
