#!/bin/sh
set -eu
controller=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$controller/env.sh"
NO_MISTAKES_CONTROLLER=$controller
export NO_MISTAKES_CONTROLLER
exec cargo run --locked --manifest-path "$controller/Cargo.toml" -p xtask -- quality "$@"
