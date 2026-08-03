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

const SCHEMA_VERSION = "1.1.0";
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

function inputMessages(prompt: string): string {
  return jsonAttribute([{ role: "user", parts: [{ type: "text", content: prompt }] }]);
}

function outputMessages(message: unknown): string {
  const value = message as { content?: Array<Record<string, unknown>>; stopReason?: string };
  const parts: Array<Record<string, unknown>> = [];
  for (const item of value.content ?? []) {
    if (item.type === "text" && typeof item.text === "string") {
      parts.push({ type: "text", content: item.text });
    }
    if (item.type === "toolCall" && typeof item.name === "string") {
      parts.push({
        type: "tool_call",
        ...(typeof item.id === "string" ? { id: item.id } : {}),
        name: item.name,
        arguments: item.arguments ?? {},
      });
    }
  }
  return jsonAttribute([{
    role: "assistant",
    parts,
    ...(typeof value.stopReason === "string" ? { finish_reason: value.stopReason } : {}),
  }]);
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
  let pendingPrompt = "";
  const tools = new Map<string, Span>();

  const resource = (): Attributes => ({
    "service.name": "pi-coding-agent",
    "service.version": process.env.PI_VERSION ?? "not_observed",
    "app.agent.schema.version": SCHEMA_VERSION,
    "app.agent.harness.name": "pi",
    "app.agent.harness.version": process.env.PI_VERSION ?? "not_observed",
    "app.agent.repository.hash": process.env.APP_AGENT_REPOSITORY_HASH ?? "not_observed",
    "app.agent.repository.base_revision": process.env.APP_AGENT_BASE_REVISION ?? "not_observed",
    "app.agent.skill.catalogue_hash": process.env.APP_AGENT_SKILL_CATALOGUE_HASH ?? "not_observed",
  });

  pi.on("session_start", async (_event, ctx) => {
    const file = ctx.sessionManager.getSessionFile();
    sessionId = hash(file ?? sessionId);
    await flush();
  });

  pi.on("before_agent_start", (event) => {
    pendingPrompt = event.prompt;
  });

  pi.on("agent_start", async (_event, ctx) => {
    const traceId = hex(16);
    task = {
      traceId,
      spanId: hex(8),
      name: "agent.task",
      startTimeUnixNano: now(),
      attributes: {
        "app.agent.record.type": "task",
        "app.agent.task.id": traceId,
        "app.agent.session.id": sessionId,
        "app.agent.task.class": process.env.APP_AGENT_TASK_CLASS ?? "not_observed",
        "app.agent.risk.class": process.env.APP_AGENT_RISK_CLASS ?? "not_observed",
        "app.agent.model.requested": ctx.model?.id ?? "not_observed",
        "app.agent.model.returned": ctx.model?.id ?? "not_observed",
        "app.agent.model.effort": ctx.thinkingLevel ?? "not_observed",
        "app.agent.permission.decision": "not_observed",
        "gen_ai.input.messages": inputMessages(pendingPrompt),
      },
    };
    pendingPrompt = "";
    taskStartedAtMs = Date.now();
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
      attributes: { "gen_ai.operation.name": "invoke_agent", "app.agent.retry.count": event.turnIndex },
    };
  });

  pi.on("tool_execution_start", (event) => {
    if (!task) return;
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
        "gen_ai.tool.call.arguments": jsonAttribute(event.args),
      },
    });
  });

  pi.on("tool_execution_end", (event) => {
    const span = tools.get(event.toolCallId);
    if (!span) return;
    span.endTimeUnixNano = now();
    span.status = event.isError ? "error" : "ok";
    span.attributes["app.agent.tool.status"] = event.isError ? "error" : "ok";
    span.attributes["gen_ai.tool.call.result"] = jsonAttribute(event.result);
    spans.push(span);
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
      currentTurn.attributes["gen_ai.output.messages"] = outputMessages(event.message);
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

  pi.on("agent_end", async (event) => {
    if (!task) return;
    const finalAssistant = [...event.messages].reverse().find((message) => message.role === "assistant");
    if (finalAssistant) task.attributes["gen_ai.output.messages"] = outputMessages(finalAssistant);
    task.endTimeUnixNano = now();
    task.status = "ok";
    task.attributes["app.agent.final.status"] = "accepted";
    task.attributes["app.agent.duration_ms"] = Date.now() - taskStartedAtMs;
    task.attributes["app.agent.outcome.reference"] = process.env.APP_AGENT_OUTCOME_REFERENCE ?? "not_observed";
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
