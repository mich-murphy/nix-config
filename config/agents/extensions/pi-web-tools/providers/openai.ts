import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import { err, ok, type Result } from "../result.ts";
import type { NormalizedSearchResult, SearchProvider, SearchProviderError, SearchProviderRequest, SearchProviderSuccess } from "./types.ts";

const API_RESPONSES_URL = "https://api.openai.com/v1/responses";
const CODEX_RESPONSES_URL = "https://chatgpt.com/backend-api/codex/responses";
const MAX_RESPONSE_BYTES = 1 * 1024 * 1024;
const PROVIDERS = ["openai-codex", "openai"] as const;

type ProviderHeaders = Record<string, string | null>;

interface ResolvedAuth {
	readonly provider: (typeof PROVIDERS)[number];
	readonly apiKey: string;
	readonly model: string;
	readonly headers: ProviderHeaders;
}

/** OpenAI Responses web-search adapter. Pi's existing provider authentication remains the credential owner. */
export class OpenAISearchProvider implements SearchProvider {
	readonly name = "openai" as const;

	constructor(
		private readonly context: ExtensionContext,
		private readonly fetchResponse: typeof fetch = fetch,
	) {}

	async search(
		input: SearchProviderRequest,
		options: { readonly signal?: AbortSignal } = {},
	): Promise<Result<SearchProviderSuccess, SearchProviderError>> {
		const auth = await resolveAuth(this.context);
		if (!auth) {
			return err({ _tag: "SearchProviderUnavailable", provider: this.name, cause: new Error("No Pi OpenAI authentication") });
		}

		const headers: Record<string, string> = {
			...toRequestHeaders(auth.headers),
			Authorization: `Bearer ${auth.apiKey}`,
			"Content-Type": "application/json",
			"OpenAI-Beta": "responses=experimental",
		};
		const decodedJwt = decodeJwt(auth.apiKey);
		const codex = auth.provider === "openai-codex" || isCodexJwt(decodedJwt);
		if (codex) {
			const accountId = extractAccountId(decodedJwt);
			if (accountId) headers["chatgpt-account-id"] = accountId;
			headers.originator = "pi";
		}

		try {
			const response = await this.fetchResponse(codex ? CODEX_RESPONSES_URL : API_RESPONSES_URL, {
				method: "POST",
				headers,
				redirect: "error",
				body: JSON.stringify({
					model: auth.model,
					instructions: buildInstructions(input),
					input: [{ role: "user", content: [{ type: "input_text", text: input.query }] }],
					tools: [buildSearchTool(input)],
					include: ["web_search_call.action.sources"],
					store: false,
					stream: true,
					tool_choice: "required",
					parallel_tool_calls: true,
				}),
				signal: options.signal,
			});
			if (!response.ok) {
				await response.body?.cancel().catch(() => undefined);
				return err({ _tag: "SearchProviderStatusRejected", provider: this.name, status: response.status });
			}
			const declaredLength = Number(response.headers.get("content-length"));
			if (Number.isFinite(declaredLength) && declaredLength > MAX_RESPONSE_BYTES) {
				await response.body?.cancel().catch(() => undefined);
				return err({ _tag: "SearchProviderResponseTooLarge", provider: this.name, maxBytes: MAX_RESPONSE_BYTES });
			}
			const text = await readBoundedText(response, MAX_RESPONSE_BYTES);
			const output = parseResponseOutput(text);
			const results = extractResults(output).slice(0, input.maxResults);
			if (results.length === 0) return err({ _tag: "SearchProviderNoRecognizedResults", provider: this.name });
			return ok({ provider: this.name, results });
		} catch (cause: unknown) {
			if (options.signal?.aborted) return err({ _tag: "SearchProviderCancelled", provider: this.name, cause });
			if (cause instanceof ResponseTooLargeError) {
				return err({ _tag: "SearchProviderResponseTooLarge", provider: this.name, maxBytes: MAX_RESPONSE_BYTES });
			}
			return err({ _tag: "SearchProviderUnavailable", provider: this.name, cause });
		}
	}
}

async function resolveAuth(context: ExtensionContext): Promise<ResolvedAuth | undefined> {
	let models: ReturnType<typeof context.modelRegistry.getAll>;
	try {
		models = context.modelRegistry.getAll();
	} catch {
		return undefined;
	}
	for (const provider of PROVIDERS) {
		const model = pickModel(models.filter((candidate) => candidate.provider === provider));
		if (!model) continue;
		try {
			const resolved = await context.modelRegistry.getApiKeyAndHeaders(model);
			if (resolved.ok && resolved.apiKey) {
				return { provider, apiKey: resolved.apiKey, model: model.id, headers: resolved.headers ?? {} };
			}
		} catch {
			// Try the next Pi-owned provider.
		}
	}
	return undefined;
}

function pickModel<T extends { readonly id: string }>(models: readonly T[]): T | undefined {
	return [...models]
		.filter((model) => !model.id.split("-").some((segment) => segment === "pro" || segment === "ultra"))
		.sort((left, right) => right.id.localeCompare(left.id, undefined, { numeric: true }))[0];
}

function buildInstructions(input: SearchProviderRequest): string {
	const allowedDomains = input.allowedDomains ?? [];
	const blockedDomains = input.blockedDomains ?? [];
	const lines = [
		"Search the public web. Return evidence from distinct sources with precise URL citations.",
		`Use at most ${input.maxResults} sources.`,
	];
	if (input.recency) lines.push(`Prefer sources from the past ${input.recency}.`);
	if (allowedDomains.length > 0) lines.push(`Only use these domains: ${allowedDomains.join(", ")}.`);
	if (blockedDomains.length > 0) lines.push(`Do not use these domains: ${blockedDomains.join(", ")}.`);
	return lines.join(" ");
}

function buildSearchTool(input: SearchProviderRequest): Record<string, unknown> {
	const allowedDomains = input.allowedDomains ?? [];
	const blockedDomains = input.blockedDomains ?? [];
	if (allowedDomains.length === 0 && blockedDomains.length === 0) return { type: "web_search" };
	return {
		type: "web_search",
		filters: {
			...(allowedDomains.length > 0 ? { allowed_domains: allowedDomains } : {}),
			...(blockedDomains.length > 0 ? { blocked_domains: blockedDomains } : {}),
		},
	};
}

function parseResponseOutput(text: string): unknown[] {
	const trimmed = text.trim();
	if (trimmed.startsWith("{")) {
		const parsed: unknown = JSON.parse(trimmed);
		return isRecord(parsed) && Array.isArray(parsed.output) ? parsed.output : [];
	}
	const items: unknown[] = [];
	let completed: unknown[] | undefined;
	for (const line of text.split("\n")) {
		if (!line.startsWith("data: ")) continue;
		const data = line.slice(6).trim();
		if (!data || data === "[DONE]") continue;
		try {
			const event: unknown = JSON.parse(data);
			if (!isRecord(event)) continue;
			if (event.type === "response.output_item.done" && event.item) items.push(event.item);
			if ((event.type === "response.done" || event.type === "response.completed") && isRecord(event.response)) {
				if (Array.isArray(event.response.output)) completed = event.response.output;
			}
		} catch {
			// Ignore non-JSON SSE keepalive lines.
		}
	}
	return completed?.length ? completed : items;
}

function extractResults(output: unknown[]): NormalizedSearchResult[] {
	const results: NormalizedSearchResult[] = [];
	const seen = new Set<string>();
	const add = (url: unknown, title: unknown, snippet?: string) => {
		if (typeof url !== "string") return;
		let parsed: URL;
		try { parsed = new URL(url); } catch { return; }
		if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return;
		parsed.searchParams.delete("utm_source");
		const normalized = parsed.toString();
		if (seen.has(normalized)) return;
		seen.add(normalized);
		results.push({ title: typeof title === "string" && title.trim() ? title.trim() : normalized, url: normalized as NormalizedSearchResult["url"], ...(snippet ? { snippet } : {}) });
	};

	for (const item of output) {
		if (!isRecord(item)) continue;
		if (item.type === "message" && Array.isArray(item.content)) {
			for (const part of item.content) {
				if (!isRecord(part)) continue;
				const text = typeof part.text === "string" ? part.text : "";
				if (!Array.isArray(part.annotations)) continue;
				for (const annotation of part.annotations) {
					if (!isRecord(annotation) || annotation.type !== "url_citation") continue;
					add(annotation.url, annotation.title, citationSnippet(text, annotation.start_index, annotation.end_index));
				}
			}
		}
		if (item.type === "web_search_call") {
			const action = isRecord(item.action) ? item.action : {};
			for (const group of [action.sources, item.sources, item.results]) {
				if (!Array.isArray(group)) continue;
				for (const source of group) {
					if (!isRecord(source)) continue;
					add(source.url ?? source.source_website_url, source.title ?? source.caption);
				}
			}
		}
	}
	return results;
}

function citationSnippet(text: string, start: unknown, end: unknown): string | undefined {
	if (typeof start !== "number" || typeof end !== "number") return undefined;
	const value = text.slice(Math.max(0, start - 100), Math.min(text.length, end + 100)).replace(/\s+/g, " ").trim();
	return value ? value.slice(0, 300) : undefined;
}

async function readBoundedText(response: Response, maxBytes: number): Promise<string> {
	if (!response.body) return "";
	const reader = response.body.getReader();
	const chunks: Uint8Array[] = [];
	let bytes = 0;
	while (true) {
		const { done, value } = await reader.read();
		if (done) break;
		if (!value) continue;
		bytes += value.byteLength;
		if (bytes > maxBytes) {
			await reader.cancel().catch(() => undefined);
			throw new ResponseTooLargeError();
		}
		chunks.push(value);
	}
	return new TextDecoder().decode(Buffer.concat(chunks.map((chunk) => Buffer.from(chunk))));
}

class ResponseTooLargeError extends Error {}

function toRequestHeaders(headers: ProviderHeaders): Record<string, string> {
	return Object.fromEntries(Object.entries(headers).filter((entry): entry is [string, string] => entry[1] !== null));
}

function decodeJwt(token: string): Record<string, unknown> | undefined {
	const part = token.split(".")[1];
	if (!part) return undefined;
	try {
		const parsed: unknown = JSON.parse(Buffer.from(part, "base64url").toString("utf8"));
		return isRecord(parsed) ? parsed : undefined;
	} catch {
		return undefined;
	}
}

function isCodexJwt(decodedJwt: Record<string, unknown> | undefined): boolean {
	return Boolean(decodedJwt?.["https://api.openai.com/auth"]);
}

function extractAccountId(decodedJwt: Record<string, unknown> | undefined): string | undefined {
	const auth = decodedJwt?.["https://api.openai.com/auth"];
	if (!isRecord(auth)) return undefined;
	const value = auth.chatgpt_account_id;
	return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}
