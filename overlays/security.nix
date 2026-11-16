# Security fixes ahead of nixpkgs. Empty unless an incident is open.
#
# Try these in order, cheapest first (DESIGN.md, Patches):
#   1. Update nixpkgs to a revision that has the fix. Nothing builds locally.
#   2. Override only the exposed programs, for example
#      `curl = prev.curl.override { openssl = patchedOpenssl; };`.
#   3. Replace the library here for everything. Most of the closure rebuilds,
#      so let CI build and cache it before any host switches, and keep build
#      tools such as gitMinimal on the unpatched package.
#
# Each entry names its advisory and has a removal item in TODO.md.
_final: _prev: { }
