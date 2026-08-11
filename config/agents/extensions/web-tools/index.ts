import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { StringEnum, Type } from "@earendil-works/pi-ai";
import {
  DEFAULT_MAX_BYTES,
  DEFAULT_MAX_LINES,
  defineTool,
  formatSize,
  truncateHead,
  withFileMutationQueue,
  type ExtensionAPI,
} from "@earendil-works/pi-coding-agent";
import { Text } from "@earendil-works/pi-tui";
import {
  MAX_FETCH_BYTES,
  MAX_SEARCH_BYTES,
  type SearchProvider,
  webFetch,
  webSearch,
} from "./core.js";

interface OutputDetails {
  provider?: SearchProvider;
  url?: string;
  contentType?: string;
  format?: string;
  truncated: boolean;
  fullOutputPath?: string;
}

function enabled(value: string | undefined): boolean {
  return value !== undefined && ["1", "true", "yes", "on"].includes(value.toLowerCase());
}

function configuredProvider(): SearchProvider | undefined {
  const value = process.env.PI_WEBSEARCH_PROVIDER ?? process.env.OPENCODE_WEBSEARCH_PROVIDER;
  return value === "exa" || value === "parallel" ? value : undefined;
}

async function boundOutput(output: string, prefix: string): Promise<{
  text: string;
  truncated: boolean;
  fullOutputPath?: string;
}> {
  const truncation = truncateHead(output, {
    maxLines: DEFAULT_MAX_LINES,
    maxBytes: DEFAULT_MAX_BYTES,
  });
  if (!truncation.truncated) return { text: output, truncated: false };

  const directory = await mkdtemp(join(tmpdir(), `${prefix}-`));
  const fullOutputPath = join(directory, "output.txt");
  await withFileMutationQueue(fullOutputPath, () =>
    writeFile(fullOutputPath, output, { encoding: "utf8", mode: 0o600 }),
  );
  const notice =
    `\n\n[Output truncated: showing ${truncation.outputLines} of ${truncation.totalLines} lines ` +
    `(${formatSize(truncation.outputBytes)} of ${formatSize(truncation.totalBytes)}). ` +
    `Full output saved to: ${fullOutputPath}]`;
  return { text: truncation.content + notice, truncated: true, fullOutputPath };
}

const webFetchTool = defineTool({
  name: "webfetch",
  label: "Web Fetch",
  description:
    `Fetch an HTTP or HTTPS URL without running JavaScript. Converts HTML to markdown or text, ` +
    `supports images, rejects responses over ${formatSize(MAX_FETCH_BYTES)}, and truncates model text ` +
    `to ${DEFAULT_MAX_LINES} lines or ${formatSize(DEFAULT_MAX_BYTES)} while saving the full text to a temporary file.`,
  promptSnippet: "Fetch and convert content from an HTTP or HTTPS URL",
  promptGuidelines: [
    "Use webfetch to read a known URL; it is a direct HTTP client and does not execute JavaScript like a browser.",
  ],
  parameters: Type.Object({
    url: Type.String({ description: "HTTP or HTTPS URL to fetch" }),
    format: Type.Optional(
      StringEnum(["text", "markdown", "html"] as const, {
        description: "Output format (default: markdown)",
      }),
    ),
    timeout: Type.Optional(
      Type.Number({
        exclusiveMinimum: 0,
        maximum: 120,
        description: "Timeout in seconds (default: 30, maximum: 120)",
      }),
    ),
  }),
  async execute(_toolCallId, params, signal) {
    try {
      const output = await webFetch(params, { signal });
      const bounded = await boundOutput(output.output, "pi-webfetch");
      const details: OutputDetails = {
        url: output.url,
        contentType: output.contentType,
        format: output.format,
        truncated: bounded.truncated,
        fullOutputPath: bounded.fullOutputPath,
      };
      return {
        content: [
          { type: "text" as const, text: bounded.text },
          ...(output.image
            ? [{ type: "image" as const, data: output.image.data, mimeType: output.image.mimeType }]
            : []),
        ],
        details,
      };
    } catch {
      throw new Error(`Unable to fetch ${params.url}`);
    }
  },
  renderCall(args, theme) {
    return new Text(
      theme.fg("toolTitle", theme.bold("Web Fetch ")) + theme.fg("muted", args.url),
      0,
      0,
    );
  },
  renderResult(result, { isPartial }, theme) {
    if (isPartial) return new Text(theme.fg("warning", "Fetching..."), 0, 0);
    const details = result.details as OutputDetails | undefined;
    let text = theme.fg("success", details?.contentType || "Fetched");
    if (details?.truncated) text += theme.fg("warning", " (truncated)");
    return new Text(text, 0, 0);
  },
});

const webSearchTool = defineTool({
  name: "websearch",
  label: "Web Search",
  description:
    `Search the web through Exa or Parallel's hosted MCP endpoint. Search responses are limited to ` +
    `${formatSize(MAX_SEARCH_BYTES)} and returned model text is truncated to ${DEFAULT_MAX_LINES} lines or ` +
    `${formatSize(DEFAULT_MAX_BYTES)} while saving the full text to a temporary file.`,
  promptSnippet: "Search the web for current information using Exa or Parallel",
  promptGuidelines: [
    "Use websearch for current or externally sourced information, then use webfetch when a specific result page needs closer reading.",
  ],
  parameters: Type.Object({
    query: Type.String({ minLength: 1, description: "Search query" }),
    numResults: Type.Optional(
      Type.Integer({ minimum: 1, maximum: 20, description: "Number of results (default: 8)" }),
    ),
    livecrawl: Type.Optional(
      StringEnum(["fallback", "preferred"] as const, {
        description: "Whether live crawling is preferred (default: fallback)",
      }),
    ),
    type: Type.Optional(
      StringEnum(["auto", "fast", "deep"] as const, {
        description: "Search depth (default: auto)",
      }),
    ),
    contextMaxCharacters: Type.Optional(
      Type.Integer({
        minimum: 1,
        maximum: 50_000,
        description: "Maximum provider context characters (maximum: 50000)",
      }),
    ),
  }),
  async execute(_toolCallId, params, signal, _onUpdate, ctx) {
    try {
      const output = await webSearch(
        params,
        {
          provider: configuredProvider(),
          enableExa:
            enabled(process.env.OPENCODE_EXPERIMENTAL) ||
            enabled(process.env.OPENCODE_ENABLE_EXA) ||
            enabled(process.env.OPENCODE_EXPERIMENTAL_EXA),
          enableParallel:
            enabled(process.env.OPENCODE_ENABLE_PARALLEL) ||
            enabled(process.env.OPENCODE_EXPERIMENTAL_PARALLEL),
          exaApiKey: process.env.EXA_API_KEY,
          parallelApiKey: process.env.PARALLEL_API_KEY,
        },
        ctx.sessionManager.getSessionId(),
        { signal },
      );
      const bounded = await boundOutput(output.text, "pi-websearch");
      const details: OutputDetails = {
        provider: output.provider,
        truncated: bounded.truncated,
        fullOutputPath: bounded.fullOutputPath,
      };
      return { content: [{ type: "text" as const, text: bounded.text }], details };
    } catch {
      throw new Error(`Unable to search the web for ${params.query}`);
    }
  },
  renderCall(args, theme) {
    return new Text(
      theme.fg("toolTitle", theme.bold("Web Search ")) + theme.fg("muted", `"${args.query}"`),
      0,
      0,
    );
  },
  renderResult(result, { isPartial }, theme) {
    if (isPartial) return new Text(theme.fg("warning", "Searching..."), 0, 0);
    const details = result.details as OutputDetails | undefined;
    let text = theme.fg("success", details?.provider ? `${details.provider} results` : "Search complete");
    if (details?.truncated) text += theme.fg("warning", " (truncated)");
    return new Text(text, 0, 0);
  },
});

export default function webToolsExtension(pi: ExtensionAPI): void {
  pi.registerTool(webFetchTool);
  pi.registerTool(webSearchTool);
}
