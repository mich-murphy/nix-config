# Testing and Verification for Refactoring

## Contents

- [Build a Safety Net From Risk](#build-a-safety-net-from-risk)
- [Characterize Before Restructuring](#characterize-before-restructuring)
- [Keep Tests Refactor-Friendly](#keep-tests-refactor-friendly)
- [Use Red-Green-Refactor Deliberately](#use-red-green-refactor-deliberately)
- [Verify Properties Tests Cannot Establish](#verify-properties-tests-cannot-establish)
- [Handle a Red Baseline](#handle-a-red-baseline)

## Build a Safety Net From Risk

Choose observation points from the behavior and failure cost, not a coverage target.

| Risk | Useful evidence |
| --- | --- |
| Local calculation or policy | Focused example tests plus boundary cases |
| Many inputs share an invariant | Property tests with a credible domain and oracle |
| Module contract | Contract/component tests at the public seam |
| Database or serialized data | Migration, reader/writer, fixture, and rollback tests |
| External integration | Adapter contract tests plus a smaller number of real integration checks |
| User or operator flow | System/acceptance test and focused lower-level checks |
| Performance or resource contract | Representative benchmark, budget, and profiler evidence |
| Concurrency or distributed state | Invariant analysis, controlled schedules, fault injection, and runtime evidence |
| Security/privacy | Misuse cases, threat analysis, scanners where suitable, and specialist review |
| Deployment/reliability | Build/deploy checks, telemetry, staged rollout, and recovery exercise |

A green suite says only that its selected observations passed. State important
behaviors and consumers not covered.

## Characterize Before Restructuring

When code is unfamiliar or weakly tested:

1. Choose a stable behavior-level observation point near the requested change.
2. Capture representative success, failure, boundary, state, and side-effect cases.
3. Derive expected results independently where the contract is known.
4. Where the contract is unknown, label tests as characterization of current behavior.
5. Confirm the test can fail by making or observing a controlled mismatch when safe.
6. Refactor beneath that boundary in small steps.

Golden-master or snapshot tests are temporary risk controls when outputs are large and
the intended contract is not yet decomposed. Normalize unstable fields, review the
captured output, and replace overly broad snapshots with meaningful assertions as
understanding improves.

## Keep Tests Refactor-Friendly

Test meaningful behavior rather than private call order, field layout, helper names,
or incidental object graphs. A production refactor should normally leave behavioral
assertions unchanged.

Treat difficult setup, many unrelated failures, nondeterminism, and slow feedback as
design signals, then distinguish a leaky production boundary from a badly chosen test
boundary. Do not add interfaces, mocks, setters, or dependency-injection layers solely
to satisfy an isolated test. Preserve at least one integrated check where collaboration
semantics matter.

Control clocks, randomness, ports, files, and remote dependencies using established
repository seams. Keep tests deterministic, independent of order, and responsible for
cleanup. Do not hide intrinsic concurrency or distributed uncertainty with large
timeouts.

## Use Red-Green-Refactor Deliberately

For intentional behavior change:

1. Maintain a living list of success, failure, boundary, and regression examples.
2. Add one concrete test whose invocation defines a useful external interface.
3. Observe it fail for the intended missing behavior.
4. Make the smallest real production change that passes it and prior tests.
5. From green, improve one named responsibility, dependency, representation, or source
   of duplicated knowledge.
6. Run fast checks after each structural microstep.

If a bug is discovered during refactoring, stop. Preserve a reproduction, switch to
behavior mode for the fix, return to green, and then decide whether the original
refactor should resume.

Use examples to earn generalization. Do not write the entire test list before receiving
implementation feedback, copy actual output into expected values without review, or
retain test-specific branches after the general rule becomes visible.

## Verify Properties Tests Cannot Establish

Add a separate verifier for:

- module depth, information hiding, and later-change locality: maintainer walkthrough
  and design review;
- published compatibility: consumer inventory and mixed-version/migration exercise;
- performance: representative measurement and profiling;
- security/privacy: threat and adversarial review;
- operability: structured signals tied to failure questions;
- deployment safety: ordinary release path, stop conditions, rollback or forward
  repair; and
- product value/usability: user and outcome evidence.

Passing unit tests cannot approve these properties by proxy.

## Handle a Red Baseline

When baseline checks fail:

1. Record the exact command and failure.
2. Determine whether it is pre-existing, environment-specific, flaky, or caused by the
   proposed observation setup.
3. Do not weaken, skip, or rewrite the check merely to create a green baseline.
4. If a narrow unaffected verifier is trustworthy, continue only within its stated
   boundary and report the limitation.
5. Otherwise stop refactoring and ask for authority to repair the baseline or accept a
   risk-bearing restructuring.
