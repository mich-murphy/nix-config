# Application Agent Telemetry Contract 1.2.0

This contract extends OpenTelemetry GenAI semantic conventions with the
`app.agent.*` namespace. OpenTelemetry remains the transport and policy
boundary; backends are replaceable.

## Trace boundary

One trace represents one bounded user task. A harness session that receives
several tasks emits several traces and links them with
the recognized `session.id` attribute (`app.agent.session.id` is a compatibility
alias). Subagents use a child span when they remain within the
same task and a linked trace when they own an independently accepted task.

Use these record types and parent them beneath `agent.task`:

- `skill.activate` for selection, activation, and branch-specific resource use;
- `gen_ai.invoke_agent` and standard GenAI spans for model work;
- `tool.execute` for the tool category, input/output hashes, and status;
- `permission.wait` for a requested and completed permission decision;
- `validation.run` for tests, builds, lint, review, or policy checks; and
- `agent.final` for completion and the latest verifier-derived status; and
- `outcome.record` for delayed append-only CI, review, merge, revert, incident,
  owner, or user outcomes.

## Required task metadata

Emit the schema version plus harness/model/version, repository hash and base
revision, task and risk class, skill catalogue hash, requested and returned
model, semantic lane and effort, final status, duration, and an opaque outcome
reference. Emit skill package hashes, selection source, trigger, activation,
tool category/status, permission decision/policy, validation counts/status,
retry count, token counts, time to first output, permission wait, and human
rework when observed. Evaluation roots additionally require case ID, treatment,
repetition, mode, skill source/hash, prompt/tool/evaluator versions, and model
requested/returned. Return both the OTLP trace ID and deterministic MLflow
`tr-<trace-id>` identifier in each evaluation result. Use `not_observed` rather
than inventing a value.

Repository identifiers and outcome references are hashes or opaque IDs. Keep
session IDs, commits, users, repository identifiers, and paths out of metric
labels. They may remain trace-only attributes when policy permits.

## Content policy

All harnesses default to metadata-only capture. Store content hashes and stable
case or artifact references, not prompts, assistant responses, source, diffs,
commands, file paths, tool arguments, or tool results. Structured GenAI input
and output fields may carry hash-only metadata when
`app.agent.content.capture=metadata`. Rich input/output content requires an
explicitly approved resource attribute and a documented purpose and expiry.
Tool arguments/results, hidden reasoning, and raw provider bodies are never
retained by the standard pipeline.

Never retain environment-variable collections, authorization or proxy
authorization headers, cookies, passwords, API keys, access or refresh tokens,
private keys, or other credentials. The Collector must mask
credential-shaped attributes and seeded secret patterns in retained values,
retry asynchronously, and expose queue, rejection, and export-failure
telemetry. Remove or hash identity attributes before storage. Rich content is
trace-only and must never become a metric label. An exporter outage never
changes the agent task result, but evaluation validity records the telemetry
failure separately from the model task result.

Harnesses may expose different data over their lifecycle APIs. Prefer one
application-owned task trace over native low-level fragment traces, and mark
unavailable metadata as not observed rather than reconstructing it from
unstable internal storage.

## Outcomes and annotations

`completed` means the harness stopped; it is not acceptance. Emit `accepted` or
`failed` only when verifier provenance is present. Join later CI, review, merge,
revert, incident, and owner decisions by appending
an annotation conforming to `schemas/annotation.schema.json`. Never edit an old
annotation; append a superseding event that names its predecessor. Git is
authoritative for minimized evaluation definitions and compact release
manifests. MLflow is authoritative for generated runs, traces, assessments,
metrics, and minimized result artifacts.
