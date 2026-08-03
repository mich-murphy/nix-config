# Security and Provenance Audit

List every file and explain why it ships. Reject symlinks that escape the
package, hidden executables, embedded credentials, generated caches, unused
dependencies, and undocumented network access.

For each script or tool, inspect inputs, output paths, overwrite/delete
behavior, command construction, environment inheritance, permissions, network
destinations, failure mode, and verifier. Prefer fail-closed validation and
explicit confirmation for destructive or irreversible actions.

For third-party code, assets, examples, and facts, retain the source URL,
version or commit, license, retrieval date, local modifications, and integrity
hash. An official source establishes interface behavior, not outcome uplift.

Before release, run package structure and unit checks, all-harness evaluation,
secret-seeded content-leak checks, permission and destructive-action boundary
cases, and a clean installation test. Record unverified behavior as a
limitation rather than a pass.
