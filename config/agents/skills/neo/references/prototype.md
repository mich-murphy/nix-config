# Prototype Routing

Use a prototype to answer one unresolved design question. Keep the evidence,
not production code.

## Deterministic Categories

- **Visual:** interaction, navigation, comprehension, information hierarchy,
  visual states, accessibility, or usability.
- **Logical:** state transitions, domain rules, algorithms, data behavior,
  integration feasibility, latency, concurrency, or failure mechanics.
- **None:** repository evidence, an executable example, or discussion can
  answer the question more cheaply.
- **Tracer:** the direction is understood; build a production-quality
  end-to-end slice to prove integration.

The LLM proposes the uncertainty category and the user confirms it. The CLI
records the confirmed route.

## Prototype Brief

Specify:

- one question and why discussion cannot settle it;
- fidelity and artifact type;
- inputs, constraints, and intentionally omitted production qualities;
- evaluator or audience;
- evidence that will answer the question;
- time boundary and stop condition;
- disposal/isolation condition; and
- how the result updates a product, architecture, or program decision.

Run the prototype in a separate context or branch. Do not copy its shortcuts
into production. If it earns a future, begin again from a reviewed tracer
slice.

## Evidence Posture

The prototype/tracer distinction is established in classic practitioner
literature and reinforced by current skill workflows. The exact classification
rules are a local deterministic synthesis to evaluate.
