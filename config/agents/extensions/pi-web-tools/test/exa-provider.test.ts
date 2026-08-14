import test from "node:test";
import assert from "node:assert/strict";
import { ok, type Result } from "../result.ts";
import { parsePublicHttpUrl, parseSearchQuery } from "../types.ts";
import {
	ExaSearchProvider,
	FetchHttpTextClient,
	type HttpClientError,
	type HttpJsonRequest,
	type HttpTextClient,
	type HttpTextResponse,
} from "../providers/exa.ts";

const LEGACY_PROVIDER_TEXT = [
	"Title: Example Domain",
	"URL: https://example.com/",
	"Text: Example Domain",
	"",
	"Documentation-safe example domain.",
].join("\n");

class RecordingHttpTextClient implements HttpTextClient {
	readonly requests: HttpJsonRequest[] = [];

	constructor(private readonly response: Result<HttpTextResponse, HttpClientError>) {}

	async postJson(
		request: HttpJsonRequest,
		_options?: { readonly signal?: AbortSignal },
	): Promise<Result<HttpTextResponse, HttpClientError>> {
		this.requests.push(request);
		return this.response;
	}
}

test("ExaSearchProvider uses the configured MCP endpoint without extension-owned credentials", async () => {
	const http = new RecordingHttpTextClient(
		ok({
			status: 200,
			statusText: "OK",
			headers: new Headers({ "content-type": "application/json" }),
			bodyText: JSON.stringify({ result: { content: [{ type: "text", text: LEGACY_PROVIDER_TEXT }] } }),
			bytes: 123,
		}),
	);
	const endpoint = parsePublicHttpUrl("https://gateway.example.test/exa/mcp?tenant=research");
	const query = parseSearchQuery("example");
	assert.equal(endpoint._tag, "ok");
	assert.equal(query._tag, "ok");

	const provider = new ExaSearchProvider(endpoint.value, http);
	const result = await provider.search({ query: query.value, maxResults: 5, depth: "deep" });

	assert.equal(result._tag, "ok");
	assert.equal(result.value.provider, "exa");
	assert.equal(result.value.results.length, 1);
	assert.equal(http.requests[0]?.url, "https://gateway.example.test/exa/mcp?tenant=research");
	assert.deepEqual(http.requests[0]?.headers, {
		accept: "application/json, text/event-stream",
		"content-type": "application/json",
	});
	const requestBody = http.requests[0]?.body;
	assert.ok(isEncodedExaRequest(requestBody));
	assert.deepEqual(requestBody.params.arguments, { query: "example", numResults: 5 });
});

test("FetchHttpTextClient refuses redirects for configured MCP endpoints", async () => {
	let capturedInit: RequestInit | undefined;
	const client = new FetchHttpTextClient(async (_input, init) => {
		capturedInit = init;
		return new Response("{}", { status: 200, headers: { "content-type": "application/json" } });
	});
	const endpoint = "https://mcp.exa.ai/mcp" as HttpJsonRequest["url"];

	const result = await client.postJson({
		url: endpoint,
		headers: { "x-request-id": "test" },
		body: {},
		maxResponseBytes: 1024,
	});

	assert.equal(result._tag, "ok");
	assert.equal(capturedInit?.redirect, "error");
});

test("ExaSearchProvider returns safe provider errors", async () => {
	const http = new RecordingHttpTextClient(
		ok({
			status: 200,
			statusText: "OK",
			headers: new Headers({ "content-type": "text/event-stream" }),
			bodyText: `event: message\ndata: ${JSON.stringify({ result: { isError: true, content: [{ type: "text", text: "raw provider details" }] } })}\n\n`,
			bytes: 123,
		}),
	);
	const endpoint = parsePublicHttpUrl("https://mcp.exa.ai/mcp");
	const query = parseSearchQuery("example");
	assert.equal(endpoint._tag, "ok");
	assert.equal(query._tag, "ok");

	const provider = new ExaSearchProvider(endpoint.value, http);
	const result = await provider.search({ query: query.value, maxResults: 5, depth: "fast" });

	assert.deepEqual(result, {
		_tag: "err",
		error: { _tag: "SearchProviderReturnedError", provider: "exa", safeMessage: "Search provider returned an error" },
	});
});

test("ExaSearchProvider enforces domain policy on provider results", async () => {
	const text = [
		"Title: Allowed",
		"URL: https://docs.example.com/page",
		"Text: allowed",
		"",
		"Title: Blocked",
		"URL: https://other.test/page",
		"Text: blocked",
	].join("\n");
	const http = new RecordingHttpTextClient(ok({
		status: 200,
		statusText: "OK",
		headers: new Headers({ "content-type": "application/json" }),
		bodyText: JSON.stringify({ result: { content: [{ type: "text", text }] } }),
		bytes: 123,
	}));
	const endpoint = parsePublicHttpUrl("https://mcp.exa.ai/mcp");
	const query = parseSearchQuery("example");
	assert.equal(endpoint._tag, "ok");
	assert.equal(query._tag, "ok");

	const provider = new ExaSearchProvider(endpoint.value, http);
	const result = await provider.search({
		query: query.value,
		maxResults: 5,
		depth: "auto",
		allowedDomains: ["example.com"],
	});

	assert.equal(result._tag, "ok");
	assert.deepEqual(result.value.results.map((item) => item.url), ["https://docs.example.com/page"]);
});

function isEncodedExaRequest(value: unknown): value is { readonly params: { readonly arguments: Record<string, unknown> } } {
	return (
		typeof value === "object" &&
		value !== null &&
		"params" in value &&
		typeof value.params === "object" &&
		value.params !== null &&
		"arguments" in value.params &&
		typeof value.params.arguments === "object" &&
		value.params.arguments !== null
	);
}
