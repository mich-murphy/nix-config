# Deployment Preflight

## Candidate

- Release-ready evidence identifies the exact revision and artifact.
- The candidate has not changed since verification and review.
- Required CI, provenance, signatures, and approvals are complete.
- The same immutable artifact will be promoted where feasible.

## Target and Authority

- Environment, region, tenant, service, and exposure are explicit.
- The user or named operator authorized the production mutation.
- Separate release activation is authorized when deployment and release differ.
- Approval and rollback rights are available to the acting identity.

## Compatibility and Data

- Old and new versions can coexist as required.
- Schema, event, API, and configuration sequencing is explicit.
- Migration is observable, restartable or recoverable where required.
- Destructive or irreversible steps have explicit human approval.

## Pipeline and Recovery

- Canonical deployment and rollback or forward-repair paths are known.
- Credentials, artifact stores, dependencies, and control planes are healthy.
- Current production health and version provide a baseline.
- Recovery has a verifier, owner, and authority.

## Observation

- Deployment markers identify candidate and time.
- User-impact and reliability signals define success.
- Alerts are actionable and have an owner.
- Stop thresholds and observation window are declared.

A tool's presence does not establish deployability. Stop when the actual normal
path or feedback signal is broken.
