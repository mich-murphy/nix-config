import test from "node:test";
import assert from "node:assert/strict";
import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import { OpenAISearchProvider } from "../providers/openai.ts";
import { parseSearchQuery, type PublicHttpUrl } from "../types.ts";
import type { SearchProviderRequest } from "../providers/types.ts";

const query = mustParseQuery("official documentation");

function request(overrides: Partial<SearchProviderRequest> = {}): SearchProviderRequest {
	return { query, maxResults: 5, depth: "auto", allowedDomains: [], blockedDomains: [], ...overrides };
}

test("OpenAISearchProvider parses JSON Responses output and URL citations", async () => {
	const provider = new OpenAISearchProvider(
		contextWithAuth("openai", "api-key"),
		async () => Response.json(jsonResponse("https://docs.example.com/guide?utm_source=openai", "Guide")),
	);

	const result = await provider.search(request());

	assert.equal(result._tag, "ok");
	assert.deepEqual(result.value, {
		provider: "openai",
		results: [{
			title: "Guide",
			url: "https://docs.example.com/guide" as PublicHttpUrl,
			snippet: "The official guide explains setup.",
		}],
	});
});

test("OpenAISearchProvider parses streamed Responses events", async () => {
	const body = [
		`data: ${JSON.stringify({ type: "response.output_item.done", item: searchCall("https://example.com/stream", "Stream result") })}`,
		"data: [DONE]",
		"",
	].join("\n");
	const provider = new OpenAISearchProvider(
		contextWithAuth("openai", "api-key"),
		async () => new Response(body, { status: 200, headers: { "content-type": "text/event-stream" } }),
	);

	const result = await provider.search(request());

	assert.equal(result._tag, "ok");
	assert.equal(result.value.results[0]?.url, "https://example.com/stream");
});

test("OpenAISearchProvider uses API auth and forwards domain filters", async () => {
	let capturedUrl = "";
	let capturedInit: RequestInit | undefined;
	const provider = new OpenAISearchProvider(
		contextWithAuth("openai", "api-key", { "x-provider-header": "present" }),
		async (input, init) => {
			capturedUrl = String(input);
			capturedInit = init;
			return Response.json({ output: [searchCall("https://docs.example.com", "Docs")] });
		},
	);

	await provider.search(request({
		recency: "week",
		allowedDomains: ["docs.example.com"],
		blockedDomains: ["old.example.com"],
	}));

	assert.equal(capturedUrl, "https://api.openai.com/v1/responses");
	assert.equal((capturedInit?.headers as Record<string, string>).Authorization, "Bearer api-key");
	assert.equal((capturedInit?.headers as Record<string, string>)["x-provider-header"], "present");
	assert.equal(capturedInit?.redirect, "error");
	const body = JSON.parse(String(capturedInit?.body)) as { instructions: string; tools: Array<{ filters: unknown }> };
	assert.match(body.instructions, /past week/);
	assert.deepEqual(body.tools[0]?.filters, {
		allowed_domains: ["docs.example.com"],
		blocked_domains: ["old.example.com"],
	});
});

test("OpenAISearchProvider uses Codex auth, endpoint, and account header", async () => {
	let capturedUrl = "";
	let capturedHeaders: Record<string, string> = {};
	const token = codexJwt("account-123");
	const provider = new OpenAISearchProvider(
		contextWithAuth("openai-codex", token),
		async (input, init) => {
			capturedUrl = String(input);
			capturedHeaders = init?.headers as Record<string, string>;
			return Response.json({ output: [searchCall("https://example.com", "Example")] });
		},
	);

	await provider.search(request());

	assert.equal(capturedUrl, "https://chatgpt.com/backend-api/codex/responses");
	assert.equal(capturedHeaders.Authorization, `Bearer ${token}`);
	assert.equal(capturedHeaders["chatgpt-account-id"], "account-123");
	assert.equal(capturedHeaders.originator, "pi");
});

function contextWithAuth(provider: "openai" | "openai-codex", apiKey: string, headers: Record<string, string> = {}): ExtensionContext {
	const model = { provider, id: provider === "openai" ? "gpt-5.6-terra" : "gpt-5.6-sol" };
	return {
		modelRegistry: {
			getAll: () => [model],
			getApiKeyAndHeaders: async () => ({ ok: true, apiKey, headers }),
		},
	} as unknown as ExtensionContext;
}

function jsonResponse(url: string, title: string) {
	const text = "The official guide explains setup.";
	return {
		output: [{
			type: "message",
			content: [{ type: "output_text", text, annotations: [{ type: "url_citation", url, title, start_index: 0, end_index: text.length }] }],
		}],
	};
}

function searchCall(url: string, title: string) {
	return { type: "web_search_call", action: { sources: [{ url, title }] } };
}

function codexJwt(accountId: string): string {
	const payload = Buffer.from(JSON.stringify({ "https://api.openai.com/auth": { chatgpt_account_id: accountId } })).toString("base64url");
	return `header.${payload}.signature`;
}

function mustParseQuery(value: string) {
	const parsed = parseSearchQuery(value);
	if (parsed._tag === "err") throw new Error("Invalid test query");
	return parsed.value;
}
