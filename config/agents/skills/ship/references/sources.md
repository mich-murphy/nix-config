# Evidence and Provenance

The workflow is a local-trial synthesis, not a universally proven development
method.

This package incorporates the workflow synthesis directly. Its primary sources
are:

- Kent Beck, [*Test-Driven Development: By Example* official sample](https://www.informit.com/content/images/9780321146533/samplepages/0321146530.pdf)
  and [Canon TDD](https://newsletter.kentbeck.com/p/canon-tdd), for small
  behavior-led implementation steps.
- Martin Fowler, [Workflows of Refactoring](https://martinfowler.com/articles/workflowsOfRefactoring/)
  and [Changing Interfaces](https://martinfowler.com/bliki/IsChangingInterfacesRefactoring.html),
  for behavior-preserving microsteps and published-interface limits.
- Google, [Code Review Developer Guide](https://google.github.io/eng-practices/review/),
  for independent code-health review.
- Chris Riccomini and Dmitriy Ryaboy,
  [*The Missing README*](https://themissingreadme.com/), for professional
  lifecycle, compatibility, and operational review.
- Kun Chen's pinned
  [no-mistakes workflow](https://github.com/kunchenguid/no-mistakes/blob/4a692bd336c37e9ac36761ee82e558865402abba/README.md),
  as an implementation report on fresh-context validation, not causal evidence.

Measure acceptance, review rework, escaped defects, later change locality,
human burden, and cost per accepted task. Do not treat prompt adherence,
coverage, lines changed, or pull-request count as quality.
