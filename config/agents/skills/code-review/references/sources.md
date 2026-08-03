# Evidence and Provenance

This package incorporates the relevant synthesis directly. Its primary sources
are:

- Google, [Code Review Developer Guide](https://google.github.io/eng-practices/review/),
  for code health, design, functionality, complexity, tests, and documentation.
- Kent Beck, [*Test-Driven Development: By Example* official sample](https://www.informit.com/content/images/9780321146533/samplepages/0321146530.pdf)
  and [Canon TDD](https://newsletter.kentbeck.com/p/canon-tdd), for meaningful
  behavioral evidence and the red-green-refactor boundary.
- Chris Riccomini and Dmitriy Ryaboy,
  [*The Missing README*](https://themissingreadme.com/), for brownfield change,
  review, compatibility, and operational concerns.
- Kun Chen's pinned
  [no-mistakes workflow](https://github.com/kunchenguid/no-mistakes/blob/4a692bd336c37e9ac36761ee82e558865402abba/README.md),
  as a practitioner report on fresh-context validation rather than causal
  evidence.

Official and practitioner guidance establishes review contracts and mechanisms;
it does not prove that this exact agent reviewer improves outcomes. Trial it
locally. Measure material findings confirmed by maintainers, false positives,
missed defects, rework, later change locality, review time, and reviewer burden.
