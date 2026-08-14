import test from "node:test";
import assert from "node:assert/strict";
import { FallbackSearchProvider } from "../providers/fallback.ts";
import { err, ok, type Result } from "../result.ts";
import { parsePublicHttpUrl, parseSearchQuery } from "../types.ts";
import type { SearchProvider, SearchProviderError, SearchProviderRequest, SearchProviderSuccess } from "../providers/types.ts";

const input: SearchProviderRequest = {
	query: mustParseQuery("example"),
	maxResults: 5,
	depth: "auto",
	allowedDomains: [],
	blockedDomains: [],
};

test("FallbackSearchProvider tries the next adapter after an expected provider failure", async () => {
	const calls: string[] = [];
	const first = provider("openai", calls, err({ _tag: "SearchProviderStatusRejected", provider: "openai", status: 503 }));
	const expected = [{ title: "Example", url: mustParseUrl("https://example.com") }];
	const second = provider("exa", calls, ok({ provider: "exa", results: expected }));

	const result = await new FallbackSearchProvider([first, second]).search(input);

	assert.deepEqual(result, ok({ provider: "exa", results: expected }));
	assert.deepEqual(calls, ["openai", "exa"]);
});

test("FallbackSearchProvider treats cancellation as terminal", async () => {
	const calls: string[] = [];
	const cancelled = err({ _tag: "SearchProviderCancelled", provider: "openai", cause: new Error("cancelled") } as const);
	const first = provider("openai", calls, cancelled);
	const second = provider("exa", calls, ok({ provider: "exa", results: [] }));

	const result = await new FallbackSearchProvider([first, second]).search(input);

	assert.equal(result._tag, "err");
	assert.equal(result.error._tag, "SearchProviderCancelled");
	assert.deepEqual(calls, ["openai"]);
});

function provider(
	name: "openai" | "exa",
	calls: string[],
	response: Result<SearchProviderSuccess, SearchProviderError>,
): SearchProvider {
	return {
		name,
		async search() {
			calls.push(name);
			return response;
		},
	};
}

function mustParseQuery(value: string) {
	const parsed = parseSearchQuery(value);
	if (parsed._tag === "err") throw new Error("Invalid test query");
	return parsed.value;
}

function mustParseUrl(value: string) {
	const parsed = parsePublicHttpUrl(value);
	if (parsed._tag === "err") throw new Error("Invalid test URL");
	return parsed.value;
}
