import { err, type Result } from "../result.ts";
import type { SearchProviderName } from "../types.ts";
import type { SearchProvider, SearchProviderError, SearchProviderRequest, SearchProviderSuccess } from "./types.ts";

/** Ordered search adapter. Cancellation is terminal; availability/protocol/status failures try the next configured provider. */
export class FallbackSearchProvider implements SearchProvider {
	readonly name: SearchProviderName;

	constructor(private readonly providers: readonly SearchProvider[]) {
		if (providers.length === 0) throw new Error("At least one search provider is required");
		this.name = providers[0].name;
	}

	async search(
		input: SearchProviderRequest,
		options: { readonly signal?: AbortSignal } = {},
	): Promise<Result<SearchProviderSuccess, SearchProviderError>> {
		let lastError: SearchProviderError | undefined;
		for (const provider of this.providers) {
			const result = await provider.search(input, options);
			if (result._tag === "ok") return result;
			lastError = result.error;
			if (result.error._tag === "SearchProviderCancelled") return result;
		}
		return err(lastError ?? { _tag: "SearchProviderUnavailable", provider: this.name, cause: new Error("No configured provider") });
	}
}
