#!/bin/sh
_cache_home=${XDG_CACHE_HOME:-"${HOME:?HOME or XDG_CACHE_HOME is required}/.cache"}
_cache_root=$_cache_home/no-mistakes
export CARGO_TARGET_DIR=$_cache_root/target
export PATH=$_cache_root/toolchain/bin:$PATH
unset _cache_home _cache_root
