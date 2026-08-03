# TDD Boundaries

TDD owns predictable behavior, local interface feedback, and green-state
implementation design. It does not own unsettled requirements or system
architecture.

| Decision | TDD contribution | Required additional evidence |
| --- | --- | --- |
| Local behavior and interface | Drive with examples | Caller or domain validation when intent is uncertain |
| Internal responsibility and representation | Explore through green refactoring | Design review and later-change scenarios |
| Public API or stored data | Specify compatibility examples | Consumer inventory, versioning, migration, rollback |
| Performance and capacity | Preserve functional behavior | Representative benchmarks, profiling, budgets |
| Security and privacy | Exercise known controls and misuse cases | Threat analysis and security review |
| Concurrency and distributed state | Reproduce known schedules and failures | Invariant analysis, fault injection, runtime evidence |
| Deployment and reliability | Check buildable or deployable artifacts | Telemetry, rollout, recovery, production checks |
| Product value and usability | Preserve implemented behavior | User and outcome evaluation |

Stop TDD when expected outputs cannot be predicted, important examples cannot
be discovered, tests cannot remain timely and trustworthy, or the implementation
requires an unapproved consequential decision. Combine suitable predictable
components with simulation, property checks, exploration, formal analysis,
monitoring, or human evaluation rather than forcing the whole problem through
unit tests.
