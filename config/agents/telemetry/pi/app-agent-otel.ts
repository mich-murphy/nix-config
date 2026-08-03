import { createHash, randomBytes } from "node:crypto";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

type Attributes = Record<string, string | number | boolean>;
type Span = {
  traceId: string;
  spanId: string;
  parentSpanId?: string;
  name: string;
  startTimeUnixNano: string;
  endTimeUnixNano?: string;
  attributes: Attributes;
  status?: "ok" | "error";
};

const SCHEMA_VERSION = "1.2.0";
const MAX_PENDING_EXPORTS = 64;
const MAX_CONTENT_LENGTH = 61_440;
const endpoint = process.env.OTEL_EXPORTER_OTLP_TRACES_ENDPOINT
  ?? process.env.OTEL_EXPORTER_OTLP_ENDPOINT
  ?? "http://docker-host:4318/v1/traces";
const pending: unknown[] = [];

const hex = (bytes: number) => randomBytes(bytes).toString("hex");
const now = () => (BigInt(Date.now()) * 1_000_000n).toString();
const hash = (value: string) => createHash("sha256").update(value).digest("hex");

function truncate(value: string): string {
  if (value.length <= MAX_CONTENT_LENGTH) return value;
  const marker = `[TRUNCATED ${value.length - MAX_CONTENT_LENGTH} CHARS]`;
  return `${value.slice(0, MAX_CONTENT_LENGTH - marker.length)}${marker}`;
}

function jsonAttribute(value: unknown): string {
  try {
    return truncate(JSON.stringify(value, (_key, item) => typeof item === "bigint" ? item.toString() : item) ?? "null");
  } catch {
    return truncate(String(value));
  }
}

function metadataMessages(role: string, values: Record<string, unknown>): string {
  return jsonAttribute([{ role, parts: [{ type: "text", content: JSON.stringify(values) }] }]);
}

function outputMetadata(message: unknown): string {
  const value = message as { content?: Array<Record<string, unknown>>; stopReason?: string };
  const contentHashes: string[] = [];
  for (const item of value.content ?? []) {
    if (item.type === "text" && typeof item.text === "string") {
      contentHashes.push(hash(item.text));
    }
  }
  return metadataMessages("assistant", {
    content_hashes: contentHashes,
    finish_reason: value.stopReason ?? "not_observed",
  });
}

function verifiedOutcome(): { status: string; verifier: string } {
  const requested = process.env.APP_AGENT_VERIFIED_OUTCOME ?? "completed";
  const verifier = process.env.APP_AGENT_VERIFIER_PROVENANCE ?? "not_observed";
  const supported = ["completed", "accepted", "failed", "delayed", "cancelled"];
  if (!supported.includes(requested)) return { status: "completed", verifier };
  if (["accepted", "failed"].includes(requested) && verifier === "not_observed") {
    return { status: "completed", verifier };
  }
  return { status: requested, verifier };
}

function validationType(name: string, args: unknown): string | undefined {
  if (name !== "bash") return undefined;
  const command = String((args as { command?: unknown })?.command ?? "").toLowerCase();
  if (/\b(pytest|unittest|cargo test|go test|nix flake check)\b/.test(command)) return "test";
  if (/\b(lint|ruff|clippy|shellcheck|markdownlint)\b/.test(command)) return "lint";
  if (/\b(build|docker compose config)\b/.test(command)) return "build";
  return undefined;
}

function skillReference(args: unknown): { name: string; stage: string } | undefined {
  const text = jsonAttribute(args);
  const match = text.match(/(?:^|\/)skills\/([a-z][a-z0-9-]{1,80})\/(SKILL\.md|references\/|scripts\/)/);
  if (!match) return undefined;
  return {
    name: match[1],
    stage: match[2] === "SKILL.md" ? "activated" : match[2] === "references/" ? "expanded" : "executed",
  };
}

function otlpValue(value: string | number | boolean): Record<string, unknown> {
  if (typeof value === "boolean") return { boolValue: value };
  if (typeof value === "number") return Number.isInteger(value)
    ? { intValue: String(value) }
    : { doubleValue: value };
  return { stringValue: value };
}

function headers(): Record<string, string> {
  const result: Record<string, string> = { "content-type": "application/json" };
  for (const item of (process.env.OTEL_EXPORTER_OTLP_HEADERS ?? "").split(",")) {
    const separator = item.indexOf("=");
    if (separator > 0) result[item.slice(0, separator).trim()] = item.slice(separator + 1).trim();
  }
  return result;
}

function payload(spans: Span[], resource: Attributes): unknown {
  return {
    resourceSpans: [{
      resource: { attributes: Object.entries(resource).map(([key, value]) => ({ key, value: otlpValue(value) })) },
      scopeSpans: [{
        scope: { name: "app.agent.pi", version: SCHEMA_VERSION },
        spans: spans.map((span) => ({
          traceId: span.traceId,
          spanId: span.spanId,
          ...(span.parentSpanId ? { parentSpanId: span.parentSpanId } : {}),
          name: span.name,
          kind: 1,
          startTimeUnixNano: span.startTimeUnixNano,
          endTimeUnixNano: span.endTimeUnixNano ?? now(),
          attributes: Object.entries(span.attributes).map(([key, value]) => ({ key, value: otlpValue(value) })),
          status: { code: span.status === "error" ? 2 : 1 },
        })),
      }],
    }],
  };
}

async function flush(item?: unknown): Promise<void> {
  if (!endpoint) return;
  if (item) {
    if (pending.length >= MAX_PENDING_EXPORTS) {
      pending.shift();
      console.error("app-agent-otel: export queue full; oldest trace dropped");
    }
    pending.push(item);
  }
  while (pending.length > 0) {
    try {
      const response = await fetch(endpoint, {
        method: "POST",
        headers: headers(),
        body: JSON.stringify(pending[0]),
        signal: AbortSignal.timeout(3000),
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      pending.shift();
    } catch (error) {
      console.error(`app-agent-otel: export failed without affecting task: ${String(error)}`);
      return;
    }
  }
}

function toolType(name: string): string {
  if (name === "bash") return "shell";
  if (["read", "edit", "write"].includes(name)) return "filesystem";
  if (["grep", "find", "ls"].includes(name)) return "search";
  return name.startsWith("mcp_") ? "mcp" : "other";
}

export default function appAgentOtel(pi: ExtensionAPI) {
  let sessionId = hex(16);
  let task: Span | undefined;
  let spans: Span[] = [];
  let currentTurn: Span | undefined;
  let taskStartedAtMs = 0;
  let taskCostUsd = 0;
  let costObserved = false;
  let pendingPrompt = "";
  const tools = new Map<string, Span>();
  const skills = new Map<string, Set<string>>();
  const skillHashes = new Map<string, string>();

  const resource = (): Attributes => ({
    "service.name": "pi-coding-agent",
    "service.version": process.env.PI_VERSION ?? "not_observed",
    "app.agent.schema.version": SCHEMA_VERSION,
    "app.agent.harness.name": "pi",
    "app.agent.harness.version": process.env.PI_VERSION ?? "not_observed",
    "app.agent.repository.hash": process.env.APP_AGENT_REPOSITORY_HASH ?? "not_observed",
    "app.agent.repository.base_revision": process.env.APP_AGENT_BASE_REVISION ?? "not_observed",
    "app.agent.skill.catalogue_hash": process.env.APP_AGENT_SKILL_CATALOGUE_HASH ?? "not_observed",
    "app.agent.trace.kind": process.env.APP_AGENT_TRACE_KIND ?? "operational",
    "deployment.environment.name": process.env.APP_AGENT_TRACE_KIND === "evaluation" ? "evaluation" : "local",
  });

  pi.on("session_start", async (_event, ctx) => {
    const file = ctx.sessionManager.getSessionFile();
    sessionId = hash(file ?? sessionId);
    await flush();
  });

  pi.on("before_agent_start", (event) => {
    pendingPrompt = event.prompt;
    skills.clear();
    skillHashes.clear();
    for (const match of event.prompt.matchAll(/(?:^|\s)[$/]([a-z][a-z0-9-]{1,80})\b/g)) {
      skills.set(match[1], new Set(["offered", "selected"]));
    }
    for (const skill of event.systemPromptOptions.skills ?? []) {
      skills.set(skill.name, new Set(["offered"]));
      skillHashes.set(skill.name, hash(skill.content));
    }
  });

  pi.on("agent_start", async (_event, ctx) => {
    if (task) return;
    const traceId = hex(16);
    task = {
      traceId,
      spanId: hex(8),
      name: "agent.task",
      startTimeUnixNano: now(),
      attributes: {
        "app.agent.record.type": "task",
        "app.agent.task.id": traceId,
        "session.id": sessionId,
        "app.agent.session.id": sessionId,
        "app.agent.task.class": process.env.APP_AGENT_TASK_CLASS ?? "not_observed",
        "app.agent.risk.class": process.env.APP_AGENT_RISK_CLASS ?? "not_observed",
        "app.agent.skill.catalogue_hash": process.env.APP_AGENT_SKILL_CATALOGUE_HASH ?? "not_observed",
        "app.agent.model.requested": ctx.model?.id ?? "not_observed",
        "app.agent.model.returned": ctx.model?.id ?? "not_observed",
        "app.agent.model.effort": ctx.thinkingLevel ?? "not_observed",
        "app.agent.permission.decision": "not_observed",
        "app.agent.content.capture": "metadata",
        "gen_ai.input.messages": metadataMessages("user", { prompt_hash: hash(pendingPrompt) }),
      },
    };
    pendingPrompt = "";
    taskStartedAtMs = Date.now();
    taskCostUsd = 0;
    costObserved = false;
    spans = [];
  });

  pi.on("turn_start", (event) => {
    if (!task) return;
    currentTurn = {
      traceId: task.traceId,
      spanId: hex(8),
      parentSpanId: task.spanId,
      name: "gen_ai.invoke_agent",
      startTimeUnixNano: String(BigInt(event.timestamp) * 1_000_000n),
      attributes: {
        "gen_ai.operation.name": "invoke_agent",
        "app.agent.retry.count": event.turnIndex,
        "app.agent.content.capture": "metadata",
      },
    };
  });

  pi.on("tool_execution_start", (event) => {
    if (!task) return;
    const referenced = skillReference(event.args);
    if (referenced) {
      const stages = skills.get(referenced.name) ?? new Set<string>();
      for (const stage of ["offered", "selected", "activated"]) stages.add(stage);
      if (referenced.stage === "expanded" || referenced.stage === "executed") stages.add("expanded");
      if (referenced.stage === "executed") stages.add("executed");
      skills.set(referenced.name, stages);
    }
    tools.set(event.toolCallId, {
      traceId: task.traceId,
      spanId: hex(8),
      parentSpanId: currentTurn?.spanId ?? task.spanId,
      name: "tool.execute",
      startTimeUnixNano: now(),
      attributes: {
        "app.agent.record.type": "tool",
        "app.agent.tool.type": toolType(event.toolName),
        "gen_ai.operation.name": "execute_tool",
        "gen_ai.tool.name": event.toolName,
        "gen_ai.tool.call.id": event.toolCallId,
        "app.agent.tool.input_hash": hash(jsonAttribute(event.args)),
        ...(validationType(event.toolName, event.args)
          ? { "app.agent.validation.type": validationType(event.toolName, event.args) as string }
          : {}),
      },
    });
  });

  pi.on("tool_execution_end", (event) => {
    const span = tools.get(event.toolCallId);
    if (!span) return;
    span.endTimeUnixNano = now();
    span.status = event.isError ? "error" : "ok";
    span.attributes["app.agent.tool.status"] = event.isError ? "error" : "ok";
    span.attributes["app.agent.tool.output_hash"] = hash(jsonAttribute(event.result));
    spans.push(span);
    const validation = span.attributes["app.agent.validation.type"];
    if (typeof validation === "string") {
      spans.push({
        traceId: span.traceId,
        spanId: hex(8),
        parentSpanId: task?.spanId,
        name: "validation.run",
        startTimeUnixNano: span.startTimeUnixNano,
        endTimeUnixNano: span.endTimeUnixNano,
        status: span.status,
        attributes: {
          "app.agent.record.type": "validation",
          "app.agent.validation.type": validation,
          "app.agent.validation.status": event.isError ? "fail" : "pass",
          "app.agent.validation.provenance": `tool:${event.toolCallId}`,
        },
      });
    }
    tools.delete(event.toolCallId);
  });

  pi.on("turn_end", (event) => {
    if (!currentTurn) return;
    currentTurn.endTimeUnixNano = now();
    if (event.message.role === "assistant") {
      const usage = event.message.usage;
      currentTurn.attributes["gen_ai.provider.name"] = event.message.provider;
      currentTurn.attributes["gen_ai.request.model"] = event.message.model;
      currentTurn.attributes["gen_ai.response.model"] = event.message.responseModel ?? event.message.model;
      currentTurn.attributes["gen_ai.usage.input_tokens"] = usage.input;
      currentTurn.attributes["gen_ai.usage.output_tokens"] = usage.output;
      currentTurn.attributes["app.agent.tokens.cached"] = usage.cacheRead;
      currentTurn.attributes["app.agent.tokens.reasoning"] = usage.reasoning ?? 0;
      if (typeof usage.cost?.total === "number") {
        currentTurn.attributes["app.agent.cost.usd"] = usage.cost.total;
        taskCostUsd += usage.cost.total;
        costObserved = true;
      }
      currentTurn.attributes["gen_ai.output.messages"] = outputMetadata(event.message);
    }
    spans.push(currentTurn);
    currentTurn = undefined;
  });

  pi.events.on("app.agent.skill", (event: Attributes) => {
    if (!task) return;
    spans.push({
      traceId: task.traceId,
      spanId: hex(8),
      parentSpanId: task.spanId,
      name: "skill.activate",
      startTimeUnixNano: now(),
      attributes: { "app.agent.record.type": "skill", ...event },
    });
  });

  pi.events.on("app.agent.validation", (event: Attributes) => {
    if (!task) return;
    spans.push({
      traceId: task.traceId,
      spanId: hex(8),
      parentSpanId: task.spanId,
      name: "validation.run",
      startTimeUnixNano: now(),
      attributes: { "app.agent.record.type": "validation", ...event },
    });
  });

  pi.on("agent_end", (event) => {
    if (!task) return;
    const finalAssistant = [...event.messages].reverse().find((message) => message.role === "assistant");
    if (finalAssistant) task.attributes["gen_ai.output.messages"] = outputMetadata(finalAssistant);
  });

  pi.on("agent_settled", async () => {
    if (!task) return;
    for (const [name, observedStages] of skills.entries()) {
      observedStages.add("evaluated");
      for (const stage of observedStages) {
        spans.push({
          traceId: task.traceId,
          spanId: hex(8),
          parentSpanId: task.spanId,
          name: stage === "activated" ? "skill.activate" : "skill.lifecycle",
          startTimeUnixNano: task.startTimeUnixNano,
          endTimeUnixNano: now(),
          attributes: {
            "app.agent.record.type": "skill",
            "app.agent.skill.name": name,
            "app.agent.skill.package_hash": skillHashes.get(name)
              ?? process.env.APP_AGENT_SKILL_PACKAGE_HASH
              ?? "not_observed",
            "app.agent.skill.activation": stage,
            "app.agent.skill.selection": "observed",
          },
        });
      }
    }
    const outcome = verifiedOutcome();
    task.endTimeUnixNano = now();
    task.status = ["failed", "cancelled"].includes(outcome.status) ? "error" : "ok";
    task.attributes["app.agent.final.status"] = outcome.status;
    task.attributes["app.agent.outcome.status"] = outcome.status;
    task.attributes["app.agent.outcome.verifier"] = outcome.verifier;
    task.attributes["app.agent.cost.status"] = costObserved ? "observed" : "not_observed";
    if (costObserved) task.attributes["app.agent.cost.usd"] = taskCostUsd;
    task.attributes["app.agent.duration_ms"] = Date.now() - taskStartedAtMs;
    task.attributes["app.agent.outcome.reference"] = process.env.APP_AGENT_OUTCOME_REFERENCE ?? "not_observed";
    spans.push({
      traceId: task.traceId,
      spanId: hex(8),
      parentSpanId: task.spanId,
      name: "agent.final",
      startTimeUnixNano: task.endTimeUnixNano,
      endTimeUnixNano: task.endTimeUnixNano,
      status: task.status,
      attributes: {
        "app.agent.record.type": "outcome",
        "app.agent.final.status": outcome.status,
        "app.agent.outcome.status": outcome.status,
        "app.agent.outcome.verifier": outcome.verifier,
      },
    });
    await flush(payload([task, ...spans], resource()));
    task = undefined;
    spans = [];
  });

  pi.on("session_shutdown", async () => {
    if (task) {
      task.endTimeUnixNano = now();
      task.status = "error";
      task.attributes["app.agent.final.status"] = "cancelled";
      task.attributes["app.agent.duration_ms"] = Date.now() - taskStartedAtMs;
      await flush(payload([task, ...spans], resource()));
      task = undefined;
    }
    await flush();
  });
}
