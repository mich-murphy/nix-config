# Release Readiness

A candidate is release-ready only when all affected dimensions pass or are
explicitly marked not affected with evidence.

## Blocking Gate

- The plan and readiness record are unchanged.
- Every acceptance criterion is implemented or explicitly rejected upstream.
- Meaningful TDD evidence exists where predictable behavior changed.
- Focused and relevant prior tests pass.
- Repository-required build, type, lint, integration, security, documentation,
  and artifact checks pass.
- Tests were not deleted, weakened, bypassed, or coupled to actual output.
- The diff is inside scope and preserves unrelated user work.
- Compatibility, migration, data, public-contract, and operational obligations
  are satisfied.
- A fresh independent review of the current candidate has no blocking finding.

## Claim Boundary

Release-ready means the candidate has passed the available pre-release gate.
It does not mean:

- deployed, released, published, merged, or exposed to users;
- free of defects;
- secure or reliable beyond the evidence inspected;
- compatible with consumers that were not identified; or
- operationally healthy in an environment where it has not run.

Hand deployment to the separate explicit `$deploy` skill.
