# TDD Mechanics

## Behavioral Analysis Before Code

Maintain a living list of examples and questions. The list protects focus and
captures discoveries; it is neither a batch of tests nor proof that the
behavior is completely specified.

Choose the next example by learning value. A good example:

- matters to a caller;
- varies one meaningful dimension;
- rejects a plausible wrong behavior;
- makes one interface decision visible;
- is small enough for fast feedback; and
- moves toward an integrated outcome rather than an isolated implementation
  layer.

## Adaptive Step Size

Use the feedback interval, not a line-count rule:

- Take a coherent obvious step when the result is local and quickly checked.
- Split the example when several changes could explain its result.
- Move to a nearer observation point when setup is slow, while retaining the
  slower integrated verifier.
- Select a discriminating example when the design is unclear.
- Stop and reconsider when tiny steps only accumulate special cases.

## Green Tactics

**Obvious implementation:** implement the general rule directly when it is
clear and the test will quickly reject a mistake.

**Fake it:** use a deliberately narrow value to reach green when it clarifies
the next decision. Do not leave a growing chain of test-specific branches.

**Triangulate:** when the general rule is uncertain, add an example along the
relevant axis. Name the current simplifying assumption, break it with the
example, and generalize only as far as the examples justify.

## Phase Discipline

- Red decides behavior and its observation.
- Green supplies the smallest understandable real implementation.
- Refactor improves implementation design while behavior remains fixed.

Do not hide a feature, bug fix, optimization, migration, or architecture change
inside the refactoring phase.
