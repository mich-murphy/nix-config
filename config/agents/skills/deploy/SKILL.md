---
name: deploy
description: Deploy one release-ready immutable software candidate through the repository's established deployment path, observe declared health signals, and stop, roll back, or hand off forward repair when rollout conditions fail. Use only when the user explicitly invokes $deploy with a target environment and deployment authority. Do not use for implementation, release-readiness review, CI-only verification, implicit production mutation, or a candidate lacking identity, recovery, and health evidence.
---

# Deploy and Verify a Release-Ready Candidate

Treat deployment as a controlled production mutation. Use the repository's
existing path and keep final authority with the user or named operator.

## Establish Authority and Inputs

Read repository instructions and [preflight.md](references/preflight.md).
Require:

- immutable candidate identity and release-ready evidence;
- exact target environment and exposure scope;
- explicit authority for deployment and any release activation;
- canonical deployment command or runbook;
- compatibility and migration sequence;
- health and user-impact signals;
- rollout stop conditions and observation window;
- tested rollback or approved forward-repair procedure; and
- an owner able to act on alerts.

If any item is missing, perform no mutation. Return the missing evidence and
smallest required handoff. Do not infer production authority from permission to
build, review, merge, or prepare a release.

## Run Preflight

1. Confirm the candidate identity still matches release-ready evidence.
2. Confirm required CI, artifacts, signatures, approvals, and environment
   prerequisites.
3. Check that configuration, secrets, migrations, and dependencies target the
   intended environment without exposing sensitive values.
4. Confirm the ordinary deployment and recovery paths are available.
5. Establish baseline health and current version markers.
6. State the exact commands, expected transitions, stop conditions, and
   recovery action before execution.

Stop on a stale candidate, broken pipeline, unhealthy baseline, ambiguous
target, missing approval, irreversible migration without an approved recovery
strategy, or telemetry that cannot distinguish success from failure.

## Deploy Through the Normal Path

Read [rollout-and-recovery.md](references/rollout-and-recovery.md).

1. Promote the same immutable candidate across environments where supported.
2. Use the repository's automated, repeatable deployment path.
3. Separate deployment from user exposure when the system supports progressive
   release.
4. Record command, actor, target, candidate, deployment marker, timestamps, and
   result without recording secrets.
5. Observe each rollout step before increasing exposure.
6. Stop immediately when a declared condition fails.

Do not improvise a new deployment mechanism, bypass an approval, edit
production manually, or expand exposure to make a partial success appear
complete.

## Verify Production Behavior

Inspect user-visible health, service objectives, dependencies, saturation,
security signals, business signals, logs, metrics, traces, and declared smoke
checks as relevant. Correlate observations with the candidate and deployment
marker.

Command success is not deployment success. Continue only when the declared
signals remain acceptable for the observation window.

## Stop or Recover

On failure:

1. stop further rollout or exposure;
2. preserve timestamps, version identity, signals, and command evidence;
3. follow the approved rollback or forward-repair procedure;
4. verify recovery with the same user-impact signals;
5. escalate when recovery is destructive, ambiguous, unavailable, or outside
   authority; and
6. do not freelance during an active incident.

Never choose between rollback and forward repair when the plan leaves that
consequential decision unresolved.

## Report the Outcome

Use [deployment-record.json](assets/deployment-record.json). Report one status:

- `completed`: deployment and required exposure are healthy;
- `stopped`: rollout halted before unacceptable exposure;
- `rolled-back`: prior candidate restored and recovery verified;
- `forward-repair-required`: failure contained but approved repair remains;
- `blocked`: preflight or authority prevented mutation.

Include candidate, target, commands, approvals, observations, exposure,
recovery, remaining risks, and next owner. Never describe deployed as released
when user exposure was not activated, or command completion as healthy without
observed evidence.

## Reference Route

- [preflight.md](references/preflight.md): authority, candidate, pipeline,
  compatibility, telemetry, and recovery gate.
- [rollout-and-recovery.md](references/rollout-and-recovery.md): progressive
  exposure, stop conditions, observation, and recovery.
- [sources.md](references/sources.md): evidence posture and provenance.
