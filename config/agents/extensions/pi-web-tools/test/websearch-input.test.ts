import test from "node:test";
import assert from "node:assert/strict";
import { parseWebSearchToolParams } from "../websearch-input.ts";
import type { WebToolsSettings } from "../types.ts";

const testSearchSettings: WebToolsSettings["search"] = {
	enabled: true,
	providers: ["exa"],
	timeoutSeconds: 25,
	defaultMaxResults: 8,
	defaultDepth: "auto",
};

test("parseWebSearchToolParams trims query and applies defaults", () => {
	const result = parseWebSearchToolParams({ query: "  example docs  " }, testSearchSettings);

	assert.equal(result._tag, "ok");
	assert.equal(result.value.query, "example docs");
	assert.equal(result.value.maxResults, 8);
	assert.equal(result.value.depth, "auto");
	assert.equal(result.value.timeoutSeconds, 25);
});

test("parseWebSearchToolParams accepts deep and clamps maxResults", () => {
	const low = parseWebSearchToolParams({ query: "example", maxResults: 0, depth: "deep" }, testSearchSettings);
	const high = parseWebSearchToolParams({ query: "example", maxResults: 999 }, testSearchSettings);
	const clampedDefault = parseWebSearchToolParams(
		{ query: "example" },
		{ ...testSearchSettings, defaultMaxResults: 999 },
	);

	assert.equal(low._tag, "ok");
	assert.equal(low.value.depth, "deep");
	assert.equal(low.value.maxResults, 1);
	assert.equal(high._tag, "ok");
	assert.equal(high.value.maxResults, 20);
	assert.equal(clampedDefault._tag, "ok");
	assert.equal(clampedDefault.value.maxResults, 20);
});

test("parseWebSearchToolParams normalizes recency and domain policy", () => {
	const result = parseWebSearchToolParams({
		query: "example",
		recency: "week",
		domains: ["Docs.Example.com", "-old.example.com", "docs.example.com"],
	}, testSearchSettings);

	assert.equal(result._tag, "ok");
	assert.equal(result.value.recency, "week");
	assert.deepEqual(result.value.allowedDomains, ["docs.example.com"]);
	assert.deepEqual(result.value.blockedDomains, ["old.example.com"]);
});

test("parseWebSearchToolParams rejects invalid boundary input", () => {
	assert.deepEqual(parseWebSearchToolParams({ query: "   " }, testSearchSettings), {
		_tag: "err",
		error: { _tag: "EmptySearchQuery" },
	});
	assert.deepEqual(parseWebSearchToolParams({ query: "example", depth: "slow" }, testSearchSettings), {
		_tag: "err",
		error: { _tag: "InvalidToolField", field: "depth", message: "Expected one of: auto, fast, deep" },
	});
	assert.deepEqual(parseWebSearchToolParams({ query: "example", maxResults: "8" }, testSearchSettings), {
		_tag: "err",
		error: { _tag: "InvalidToolField", field: "maxResults", message: "Expected a finite number" },
	});
	assert.deepEqual(parseWebSearchToolParams({ query: "example", timeout: 1 }, testSearchSettings), {
		_tag: "err",
		error: { _tag: "UnknownToolField", field: "timeout" },
	});
	assert.deepEqual(parseWebSearchToolParams({ query: "example", domains: ["http://example.com"] }, testSearchSettings), {
		_tag: "err",
		error: { _tag: "InvalidToolField", field: "domains", message: "Invalid hostname: \"http://example.com\"" },
	});
});
