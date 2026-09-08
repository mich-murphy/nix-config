#!/bin/sh
set -eu
controller=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$controller/env.sh"
exec cargo run --locked --manifest-path "$controller/Cargo.toml" -p xtask -- quality "$@"
