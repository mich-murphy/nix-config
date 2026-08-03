# Rollout, Observation, and Recovery

Prefer the smallest exposure that can reveal real behavior without exceeding
the accepted risk. Use environment progression, canaries, cohorts, feature
flags, or staged traffic only when the system already supports them.

At each step:

1. record candidate, target, deployment marker, and exposure;
2. execute the established command;
3. wait only for the declared system transition or observation window;
4. compare health with baseline and stop thresholds;
5. continue, stop, or recover explicitly.

Use user-visible health and service symptoms before large collections of
unowned infrastructure signals. Preserve dependency, saturation, security, and
business signals needed to distinguish likely causes.

Rollback restores a known prior state. Forward repair advances to another
approved candidate. Choose only according to the runbook or accountable owner.
Account for data and mixed-version behavior: reverting code may not revert a
schema, event, or external side effect.

After recovery, verify the user-impact signal rather than assuming the recovery
command worked. Preserve evidence for incident review and convert the failure
into a durable test, control, runbook, or planning correction.
