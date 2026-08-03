# Corpus Evidence Routes

Use the repository corpus as the source of truth for material recommendations.
Open only the route needed for the current decision, then inspect its matching
`evidence_claims` entry. Paths below are relative to this file.

| Decision | Corpus source | Claim and evidence boundary |
| --- | --- | --- |
| Classify and weight evidence | [Evidence Hierarchy](https://github.com/mich-murphy/agentic-workflow-research/blob/main/docs/foundations-and-synthesis/evidence-hierarchy.md) | `claim-level-assessment`, authority/methodological-standard, high certainty, adopt. Assess claims rather than assigning one grade to a document. |
| Compare official skill-creation guidance | [Official Skill-Creation Guidance Compared](https://github.com/mich-murphy/agentic-workflow-research/blob/main/docs/skills-and-instructions/official-skill-creation-comparison.md) | `official-guidance` is high-certainty current behavior; `actionable-synthesis` is low-certainty inference and should be trialled locally. |
| Design skill evaluation and tracing | [Tracing, Tracer Bullets, and AI Skills](https://github.com/mich-murphy/agentic-workflow-research/blob/main/docs/foundations-and-synthesis/tracing-tracer-bullets-and-ai-skills.md) | `counterfactual-skill-trace-auditing` is a low-certainty controlled benchmark; use paired traces diagnostically, not as the primary outcome. |
| Instrument agents and skills | [AI Agent Observability](https://github.com/mich-murphy/agentic-workflow-research/blob/main/docs/tools-models-and-orchestration/ai-agent-observability.md) | Adopt the high-certainty OpenTelemetry boundary; task-centered traces, paired skill evaluation, scorecards, and release gates are moderate or low-certainty synthesis requiring local validation. |
| Choose model and effort | [Model and Effort Selection](https://github.com/mich-murphy/agentic-workflow-research/blob/main/docs/tools-models-and-orchestration/official-anthropic-openai-model-guidance.md) | Current model maps are high-certainty vendor behavior; task-to-route mappings and durable skill routing are moderate-certainty synthesis and should be trialled locally. |

Apply these non-compensatory limits:

- Official documentation establishes interfaces and current product behavior,
  not improved outcomes.
- A single benchmark is at most moderate certainty for its measured task and
  normally low certainty for a generalized workflow recommendation.
- Practitioner convergence establishes current practice, not causal benefit.
- A local candidate can be called better only on its tested task, model,
  effort, harness, tools, permissions, version, and acceptance criteria.
- Recheck model IDs, harness behavior, telemetry schemas, and product features
  before changing a production route.
