# Application Agent Telemetry Contract 1.1.0

This contract extends OpenTelemetry GenAI semantic conventions with the
`app.agent.*` namespace. OpenTelemetry remains the transport and policy
boundary; backends are replaceable.

## Trace boundary

One trace represents one bounded user task. A harness session that receives
several tasks emits several traces and links them with
`app.agent.session.id`. Subagents use a child span when they remain within the
same task and a linked trace when they own an independently accepted task.

Use these record types and parent them beneath `agent.task`:

- `skill.activate` for selection, activation, and branch-specific resource use;
- `gen_ai.invoke_agent` and standard GenAI spans for model work;
- `tool.execute` for the tool category, arguments, result, and status;
- `permission.wait` for a requested and completed permission decision;
- `validation.run` for tests, builds, lint, review, or policy checks; and
- `outcome.record` for append-only CI, review, merge, revert, incident, owner,
  or user outcomes.

## Required task metadata

Emit the schema version plus harness/model/version, repository hash and base
revision, task and risk class, skill catalogue hash, requested and returned
model, semantic lane and effort, final status, duration, and an opaque outcome
reference. Emit skill package hashes, selection source, trigger, activation,
tool category/status, permission decision/policy, validation counts/status,
retry count, token counts, time to first output, permission wait, and human
rework when observed. Use `not_observed` rather than inventing a value.

Repository identifiers and outcome references are hashes or opaque IDs. Keep
session IDs, commits, users, repository identifiers, and paths out of metric
labels. They may remain trace-only attributes when policy permits.

## Content policy

Operational traces may include prompts, assistant responses, source, diffs,
commands, file paths, and tool arguments and results in the approved self-hosted
pipeline. Record structured GenAI content in `gen_ai.input.messages`,
`gen_ai.output.messages`, `gen_ai.tool.call.arguments`, and
`gen_ai.tool.call.result` when the harness exposes it. Do not record hidden
reasoning or raw provider request and response bodies.

Never retain environment-variable collections, authorization or proxy
authorization headers, cookies, passwords, API keys, access or refresh tokens,
private keys, or other credentials. The Collector must mask
credential-shaped attributes and seeded secret patterns in retained values,
retry asynchronously, and expose queue, rejection, and export-failure
telemetry. Rich content is trace-only and must never become a metric label. An
exporter outage never changes the agent task result.

Harnesses may expose different content over their trace signal. Record the
richest native trace available without scraping private session databases or
adding launch wrappers solely for telemetry. Mark unavailable content as not
observed rather than reconstructing it from unstable internal storage.

## Outcomes and annotations

Join later CI, review, merge, revert, incident, and owner decisions by appending
an annotation conforming to `schemas/annotation.schema.json`. Never edit an old
annotation; append a superseding event that names its predecessor. Git is
authoritative for minimized evaluation cases, release manifests, and recorded
results. MLflow is the searchable operational and experiment record.
