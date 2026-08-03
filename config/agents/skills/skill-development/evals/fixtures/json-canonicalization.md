# Reviewed workflow evidence

Source: five formatter-review failures from 2026-06.

The operation is fully deterministic: parse JSON while rejecting duplicate
keys at every object depth, sort object keys lexicographically, preserve array
order and values, emit UTF-8 with one trailing newline, and refuse to overwrite
the source when validation fails. Acceptance is executable: invalid or
duplicate-key input exits non-zero without changing the source; valid output
parses; a second run is byte-identical.
