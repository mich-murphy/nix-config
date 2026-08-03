# Application Agent Telemetry Contract 1.0.0

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
- `tool.execute` for a content-free tool category and result;
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

Normal telemetry is metadata-only. Do not emit prompts, model messages,
reasoning, source, diffs, commands, file paths, tool arguments/results, request
or response bodies, environment variables, authorization data, or secrets.
Content capture requires a separately approved pipeline and storage policy.

The Collector must fail closed with an attribute allowlist, mask seeded secret
patterns, retry asynchronously, and expose queue, rejection, and export-failure
telemetry. An exporter outage never changes the agent task result.

## Outcomes and annotations

Join later CI, review, merge, revert, incident, and owner decisions by appending
an annotation conforming to `schemas/annotation.schema.json`. Never edit an old
annotation; append a superseding event that names its predecessor. Git is
authoritative for minimized evaluation cases, release manifests, and recorded
results. MLflow is the searchable operational and experiment record.
