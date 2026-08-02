# Sources and Evidence Boundary

## Evidence Boundary

The workflow combines expert demonstrations, expert opinion, implementation reports,
official documentation, and repository synthesis. These sources provide strong
vocabulary and inspectable procedures, but do not prove that one fixed workflow or
pattern improves maintainability for every team. Treat most design recommendations as
local trials and verify them through accepted behavior, review discoveries, later
change locality, rework, and maintainer comprehension.

Model names and product defaults are especially time-sensitive. Semantic routing is
more durable than exact identifiers.

## Primary Practice Sources

- Martin Fowler, [*Refactoring*, second edition](https://martinfowler.com/books/refactoring.html),
  [free first chapter](https://www.thoughtworks.com/content/dam/thoughtworks/documents/books/bk_Refactoring2-free-chapter_en.pdf),
  [Catalog of Refactorings](https://refactoring.com/catalog/),
  [Workflows of Refactoring](https://martinfowler.com/articles/workflowsOfRefactoring/),
  [Changing Interfaces](https://martinfowler.com/bliki/IsChangingInterfacesRefactoring.html),
  and [Branch by Abstraction](https://martinfowler.com/bliki/BranchByAbstraction.html).
- John Ousterhout, [*A Philosophy of Software Design*](https://web.stanford.edu/~ouster/cgi-bin/aposd.php),
  [modular design notes](https://web.stanford.edu/~ouster/cgi-bin/cs190-winter18/lecture.php%3Ftopic%3DmodularDesign),
  and [code-review method](https://web.stanford.edu/~ouster/cs190-winter24/lectures/codeReview1/).
- Kent Beck, [*Test-Driven Development: By Example* official sample](https://www.informit.com/content/images/9780321146533/samplepages/0321146530.pdf),
  [Canon TDD](https://newsletter.kentbeck.com/p/canon-tdd), and
  [TDD prerequisites](https://newsletter.kentbeck.com/p/tdd-prerequisites).
- David Thomas and Andrew Hunt,
  [*The Pragmatic Programmer* official tips](https://pragprog.com/tips/), including
  DRY, orthogonality, reversibility, contracts, tracer bullets, property tests, and
  routine refactoring.
- Michael Feathers, [*Working Effectively with Legacy Code*](https://www.pearson.com/en-us/subject-catalog/p/working-effectively-with-legacy-code/P200000009149),
  for locating change/test points and creating seams before changing legacy code.
- David Parnas, “On the Criteria To Be Used in Decomposing Systems into Modules,”
  for decomposing around hidden change decisions rather than processing stages.
- Barbara Liskov and Stephen Zilles, “Programming with Abstract Data Types,” for
  separating behavior from representation through explicit abstraction boundaries.
- Erich Gamma, Richard Helm, Ralph Johnson, and John Vlissides, *Design Patterns*, as
  a pattern vocabulary; use patterns only in response to a demonstrated design
  pressure.
- Chris Riccomini and Dmitriy Ryaboy, *The Missing README*, for brownfield lifecycle,
  risk-based testing, review, compatibility, and incremental replacement.
- Mark Richards and Neal Ford, *Fundamentals of Software Architecture*, for explicit
  trade-offs, risk review, fitness checks, and rejecting pattern-first architecture.
- Google, [Code Review Developer Guide](https://google.github.io/eng-practices/review/),
  for reviewing design, functionality, complexity, tests, naming, comments, style,
  and documentation while favoring continuous code-health improvement over perfection.

## Model and Skill Sources

- Anthropic, [model selection](https://platform.claude.com/docs/en/about-claude/models/choosing-a-model)
  and [effort](https://platform.claude.com/docs/en/build-with-claude/effort).
- OpenAI, [latest model guide](https://developers.openai.com/api/docs/guides/latest-model)
  and [Codex model guide](https://learn.chatgpt.com/docs/models).
- Agent Skills, [portable specification](https://agentskills.io/specification).

## Interpretation Rules

- A catalog entry supplies mechanics and vocabulary, not proof that it is the right
  design.
- A passing test supports only the behavior it observes.
- Official model guidance describes intended product roles, not cross-vendor
  superiority.
- Practitioner convergence is useful for tactics and failure modes, not causal claims.
- Prefer the repository's existing conventions and direct evidence over generic advice.
- Report inference, unknown consumers, and unverified behavior explicitly.
