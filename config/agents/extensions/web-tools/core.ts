import { Parser } from "htmlparser2";
import TurndownService from "turndown";

export const MAX_FETCH_BYTES = 5 * 1024 * 1024;
export const MAX_SEARCH_BYTES = 256 * 1024;
export const SEARCH_TIMEOUT_MS = 25_000;

const CHROME_USER_AGENT =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 " +
  "(KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";

export type FetchFormat = "text" | "markdown" | "html";
export type SearchProvider = "exa" | "parallel";

export interface WebFetchInput {
  url: string;
  format?: FetchFormat;
  timeout?: number;
}

export interface WebFetchOutput {
  url: string;
  contentType: string;
  format: FetchFormat;
  output: string;
  image?: { data: string; mimeType: string };
}

export interface WebSearchInput {
  query: string;
  numResults?: number;
  livecrawl?: "fallback" | "preferred";
  type?: "auto" | "fast" | "deep";
  contextMaxCharacters?: number;
}

export interface WebSearchOutput {
  provider: SearchProvider;
  text: string;
}

export interface WebSearchConfig {
  provider?: SearchProvider;
  enableExa?: boolean;
  enableParallel?: boolean;
  exaApiKey?: string;
  parallelApiKey?: string;
}

export type FetchLike = (
  input: string | URL | Request,
  init?: RequestInit,
) => Promise<Response>;

interface RequestOptions {
  fetch?: FetchLike;
  signal?: AbortSignal;
}

function validateHttpUrl(value: string): URL {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    throw new Error(`Invalid URL: ${value}`);
  }
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new Error(`Unsupported URL protocol: ${url.protocol}`);
  }
  return url;
}

function acceptHeader(format: FetchFormat): string {
  if (format === "markdown") {
    return "text/markdown, text/x-markdown;q=0.9, text/plain;q=0.8, text/html;q=0.7, */*;q=0.1";
  }
  if (format === "text") {
    return "text/plain, text/markdown;q=0.9, text/html;q=0.8, */*;q=0.1";
  }
  return "text/html, application/xhtml+xml;q=0.9, text/plain;q=0.8, text/markdown;q=0.7, */*;q=0.1";
}

function requestHeaders(format: FetchFormat, userAgent: string): Record<string, string> {
  return {
    Accept: acceptHeader(format),
    "Accept-Language": "en-US,en;q=0.9",
    "User-Agent": userAgent,
  };
}

function composeSignal(signal: AbortSignal | undefined, timeoutMs: number): AbortSignal {
  const timeout = AbortSignal.timeout(timeoutMs);
  return signal ? AbortSignal.any([signal, timeout]) : timeout;
}

async function collectBoundedBody(response: Response, maxBytes: number): Promise<Uint8Array> {
  const declared = response.headers.get("content-length");
  if (declared !== null) {
    const length = Number(declared);
    if (!Number.isSafeInteger(length) || length < 0) {
      throw new Error("Invalid Content-Length header");
    }
    if (length > maxBytes) throw new Error(`Response exceeds ${maxBytes} bytes`);
  }

  if (!response.body) return new Uint8Array();
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      total += value.byteLength;
      if (total > maxBytes) {
        await reader.cancel("response too large");
        throw new Error(`Response exceeds ${maxBytes} bytes`);
      }
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }

  const body = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    body.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return body;
}

function normalizedMime(response: Response): string {
  return (response.headers.get("content-type") ?? "").split(";", 1)[0]!.trim().toLowerCase();
}

function isImageMime(mime: string): boolean {
  return mime.startsWith("image/") && mime !== "image/svg+xml" && mime !== "image/vnd.fastbidsheet";
}

function isTextMime(mime: string): boolean {
  return (
    mime === "" ||
    mime.startsWith("text/") ||
    mime === "application/json" ||
    mime === "application/xml" ||
    mime === "application/javascript" ||
    mime.endsWith("+json") ||
    mime.endsWith("+xml")
  );
}

function htmlToText(html: string): string {
  const suppressed = new Set(["script", "style", "noscript", "iframe", "object", "embed"]);
  let ignoredDepth = 0;
  let output = "";
  const parser = new Parser({
    onopentag(name) {
      if (suppressed.has(name.toLowerCase())) ignoredDepth += 1;
    },
    ontext(text) {
      if (ignoredDepth === 0) output += text;
    },
    onclosetag(name) {
      if (suppressed.has(name.toLowerCase()) && ignoredDepth > 0) ignoredDepth -= 1;
    },
  });
  parser.write(html);
  parser.end();
  return output.trim();
}

function htmlToMarkdown(html: string): string {
  const turndown = new TurndownService({
    headingStyle: "atx",
    hr: "---",
    bulletListMarker: "-",
    codeBlockStyle: "fenced",
    emDelimiter: "*",
  });
  turndown.remove(["script", "style", "meta", "link"]);
  return turndown.turndown(html);
}

function stableProvider(sessionId: string): SearchProvider {
  let hash = 2166136261;
  for (const byte of new TextEncoder().encode(sessionId)) {
    hash ^= byte;
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0) % 2 === 0 ? "exa" : "parallel";
}

export function selectSearchProvider(config: WebSearchConfig, sessionId: string): SearchProvider {
  if (config.provider) return config.provider;
  if (config.enableParallel) return "parallel";
  if (config.enableExa) return "exa";
  return stableProvider(sessionId);
}

function validateSearchInput(input: WebSearchInput): void {
  if (input.query.trim().length === 0) throw new Error("query must not be empty");
  if (
    input.numResults !== undefined &&
    (!Number.isInteger(input.numResults) || input.numResults <= 0 || input.numResults > 20)
  ) {
    throw new Error("numResults must be a positive integer no greater than 20");
  }
  if (
    input.contextMaxCharacters !== undefined &&
    (!Number.isInteger(input.contextMaxCharacters) ||
      input.contextMaxCharacters <= 0 ||
      input.contextMaxCharacters > 50_000)
  ) {
    throw new Error("contextMaxCharacters must be a positive integer no greater than 50000");
  }
}

function firstMcpText(payload: unknown): string | undefined {
  if (!payload || typeof payload !== "object") return undefined;
  const result = (payload as { result?: unknown }).result;
  if (!result || typeof result !== "object") return undefined;
  const content = (result as { content?: unknown }).content;
  if (!Array.isArray(content)) return undefined;
  for (const item of content) {
    if (!item || typeof item !== "object") continue;
    const text = (item as { text?: unknown }).text;
    if (typeof text === "string" && text.length > 0) return text;
  }
  return undefined;
}

function parseMcpText(body: string): string | undefined {
  try {
    return firstMcpText(JSON.parse(body));
  } catch {
    for (const line of body.split(/\r?\n/)) {
      const match = /^data:\s?(.*)$/.exec(line);
      if (!match || match[1] === "[DONE]") continue;
      try {
        const text = firstMcpText(JSON.parse(match[1]!));
        if (text) return text;
      } catch {
        // Ignore keep-alives and non-JSON SSE frames.
      }
    }
    return undefined;
  }
}

export async function webSearch(
  input: WebSearchInput,
  config: WebSearchConfig,
  sessionId: string,
  options: RequestOptions = {},
): Promise<WebSearchOutput> {
  validateSearchInput(input);
  const provider = selectSearchProvider(config, sessionId);
  const headers: Record<string, string> = {
    Accept: "application/json, text/event-stream",
    "Content-Type": "application/json",
  };

  let endpoint: string;
  let name: string;
  let args: Record<string, unknown>;
  if (provider === "exa") {
    const url = new URL("https://mcp.exa.ai/mcp");
    if (config.exaApiKey) url.searchParams.set("exaApiKey", config.exaApiKey);
    endpoint = url.toString();
    name = "web_search_exa";
    args = {
      query: input.query,
      type: input.type ?? "auto",
      numResults: input.numResults ?? 8,
      livecrawl: input.livecrawl ?? "fallback",
      ...(input.contextMaxCharacters === undefined
        ? {}
        : { contextMaxCharacters: input.contextMaxCharacters }),
    };
  } else {
    endpoint = "https://search.parallel.ai/mcp";
    name = "web_search";
    args = {
      objective: input.query,
      search_queries: [input.query],
      session_id: sessionId,
    };
    headers["User-Agent"] = "pi-web-tools/0.1.0";
    if (config.parallelApiKey) headers.Authorization = `Bearer ${config.parallelApiKey}`;
  }

  const response = await (options.fetch ?? fetch)(endpoint, {
    method: "POST",
    headers,
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "tools/call",
      params: { name, arguments: args },
    }),
    signal: composeSignal(options.signal, SEARCH_TIMEOUT_MS),
  });
  if (!response.ok) throw new Error(`Search provider returned HTTP ${response.status}`);

  const body = new TextDecoder().decode(await collectBoundedBody(response, MAX_SEARCH_BYTES));
  return {
    provider,
    text: parseMcpText(body) ?? "No search results found. Please try a different query.",
  };
}

export async function webFetch(
  input: WebFetchInput,
  options: RequestOptions = {},
): Promise<WebFetchOutput> {
  const url = validateHttpUrl(input.url);
  const format = input.format ?? "markdown";
  const timeout = input.timeout ?? 30;
  if (!Number.isFinite(timeout) || timeout <= 0 || timeout > 120) {
    throw new Error("timeout must be greater than 0 and at most 120 seconds");
  }

  const fetchImpl = options.fetch ?? fetch;
  const signal = composeSignal(options.signal, timeout * 1000);
  const send = (userAgent: string) =>
    fetchImpl(url, {
      method: "GET",
      headers: requestHeaders(format, userAgent),
      redirect: "follow",
      signal,
    });

  let response = await send(CHROME_USER_AGENT);
  if (response.status === 403 && response.headers.get("cf-mitigated") === "challenge") {
    response = await send("pi");
  }
  if (!response.ok) throw new Error(`HTTP ${response.status} ${response.statusText}`.trim());

  const body = await collectBoundedBody(response, MAX_FETCH_BYTES);
  const contentType = normalizedMime(response);
  if (isImageMime(contentType)) {
    return {
      url: input.url,
      contentType,
      format,
      output: "Image fetched successfully",
      image: { data: Buffer.from(body).toString("base64"), mimeType: contentType },
    };
  }
  if (!isTextMime(contentType)) {
    throw new Error(`Unsupported content type: ${contentType || "unknown"}`);
  }

  const decoded = new TextDecoder().decode(body);
  const isHtml = contentType === "text/html" || contentType === "application/xhtml+xml";
  const output =
    isHtml && format === "markdown"
      ? htmlToMarkdown(decoded)
      : isHtml && format === "text"
        ? htmlToText(decoded)
        : decoded;

  return { url: input.url, contentType, format, output };
}
