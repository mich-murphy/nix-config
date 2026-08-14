import test from "node:test";
import assert from "node:assert/strict";
import { err, ok, type Result } from "../result.ts";
import { SearchWeb } from "../search-web.ts";
import { createWebSearchTool } from "../websearch.ts";
import type { WebToolsSettings } from "../types.ts";
import type { ToolOutputStore, ToolOutputStoreError } from "../tool-output.ts";
import type { SearchProvider, SearchProviderError, SearchProviderRequest, SearchProviderSuccess } from "../providers/types.ts";

const settings: WebToolsSettings = {
	fetch: {
		defaultFormat: "markdown",
		timeoutSeconds: 30,
		maxResponseBytes: 5 * 1024 * 1024,
		blockPrivateHosts: true,
		maxRedirects: 5,
		fallbackUserAgent: "opencode",
	},
	search: {
		enabled: true,
		providers: ["exa"],
		timeoutSeconds: 25,
		defaultMaxResults: 8,
		defaultDepth: "auto",
	},
};

class FakeProvider implements SearchProvider {
	readonly name = "exa" as const;

	constructor(private readonly response: Result<SearchProviderSuccess, SearchProviderError>) {}

	async search(
		_input: SearchProviderRequest,
		_options?: { readonly signal?: AbortSignal },
	): Promise<Result<SearchProviderSuccess, SearchProviderError>> {
		return this.response;
	}
}

class UnusedOutputStore implements ToolOutputStore {
	async writeTextFile(
		_prefix: string,
		_fileName: string,
		_content: string,
	): Promise<Result<string, ToolOutputStoreError>> {
		return ok("/tmp/unused.txt");
	}
}

test("websearch execute throws safe message for provider protocol failures", async () => {
	const searchWeb = new SearchWeb({
		settings: settings.search,
		provider: new FakeProvider(
			err({
				_tag: "SearchProviderProtocolInvalid",
				provider: "exa",
				reason: "missing result content raw details",
			}),
		),
	});
	const tool = createWebSearchTool({ settings: settings.search, searchWeb, outputStore: new UnusedOutputStore() });

	await assert.rejects(
		tool.execute("id", { query: "example" }),
		/Search provider returned an invalid response/,
	);
});
