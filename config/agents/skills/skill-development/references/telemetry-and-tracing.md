# Telemetry and Tracing

Use OpenTelemetry APIs and OTLP as the interoperable boundary when live export
is justified. Put a Collector under team control for redaction, filtering,
sampling, enrichment, retries, and backend fan-out. Pin both the OpenTelemetry
GenAI convention version and the application schema version.

Start with one bounded task per trace and metadata-only capture. A useful tree
is:

```text
agent.task
├── skill.activate
│   ├── skill.read_resource
│   └── skill.run_script
├── gen_ai.invoke_agent
│   └── gen_ai.execute_tool
├── validation.run
└── evaluator.run
```

Record the lifecycle `offered → selected → activated → expanded → executed →
evaluated`. Include stable task, session, case, and trace identifiers; package
hash; skill source and invocation route; harness, model, effort, prompt, tool,
and evaluator versions; timestamps; status; retries; latency; token classes;
cost; permission waits; validation; and final accepted or delayed outcome.

Never use prompt, response, source, tool payload, credentials, environment
variables, or high-cardinality identifiers as metric labels. Keep content
capture disabled by default. If diagnostic content is approved, isolate its
storage, access, retention, redaction, and deletion policy and test the normal
pipeline with seeded canary secrets.

Operational telemetry is append-only evidence of what happened. The evaluation
registry is a separate, immutable plane containing minimized task fixtures,
clean snapshots, private verifiers, labels, splits, and source lineage. Promote
a trace only after human review makes it reproducible and removes sensitive or
irrelevant content.

Activation is adoption telemetry, not quality. Join traces to executable or
human-reviewed outcomes and calculate tokens, cost, latency, and rework per
accepted task. Preserve invalid telemetry separately from task failure.
