# Evidence and Container Choice

Create a skill from an inspected successful workflow or a recurring reviewed
failure. Evidence may be an accepted task trace, repeated review feedback, an
incident, or a stable manual procedure. Record source lineage and preserve the
smallest redacted reproduction. Frequency matters for convenience failures;
severity can justify a single security or data-loss regression.

Choose the narrowest control:

| Need | Container |
| --- | --- |
| Stable reusable judgment or procedure | Skill |
| Deep, broad, or volatile facts | Retrieval source |
| Deterministic transformation or invariant | Script or tool |
| Mechanical lifecycle enforcement | Hook or policy |
| Repository-local convention | `AGENTS.md` |
| One task's constraint | Prompt or task context |

Do not use prose to simulate a deterministic validator. Do not copy a volatile
manual into a skill. Combine containers when the skill selects a branch and a
script safely executes its mechanics.
