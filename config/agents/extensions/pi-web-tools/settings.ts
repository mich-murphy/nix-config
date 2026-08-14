import {
	parsePublicHttpUrl,
	type PublicHttpUrl,
	type SearchDepth,
	type SearchProviderName,
	type WebFetchFormat,
	type WebToolsSettings,
} from "./types.ts";

export const WEB_FETCH_FORMATS = ["markdown", "text", "html"] as const satisfies readonly WebFetchFormat[];
export const SEARCH_DEPTHS = ["auto", "fast", "deep"] as const satisfies readonly SearchDepth[];
export const SEARCH_PROVIDERS = ["openai", "exa"] as const satisfies readonly SearchProviderName[];

export const FETCH_TIMEOUT_SECONDS = {
	default: 30,
	min: 1,
	max: 120,
} as const;

export const SEARCH_TIMEOUT_SECONDS = {
	default: 25,
	min: 1,
	max: 120,
} as const;

export const SEARCH_MAX_RESULTS = {
	default: 8,
	min: 1,
	max: 20,
} as const;

export const FETCH_MAX_RESPONSE_BYTES = 5 * 1024 * 1024;
export const FETCH_MAX_REDIRECTS = 5;

export type ToolInputParseError =
	| { readonly _tag: "InvalidToolInput"; readonly message: string }
	| { readonly _tag: "InvalidToolField"; readonly field: string; readonly message: string }
	| { readonly _tag: "UnknownToolField"; readonly field: string };

const DEFAULTS = {
	fetchDefaultFormat: "markdown",
	fetchTimeoutSeconds: FETCH_TIMEOUT_SECONDS.default,
	fetchMaxResponseBytes: FETCH_MAX_RESPONSE_BYTES,
	fetchBlockPrivateHosts: true,
	fetchMaxRedirects: FETCH_MAX_REDIRECTS,
	fetchFallbackUserAgent: "opencode",
	searchProviders: ["openai", "exa"] as const,
	searchTimeoutSeconds: SEARCH_TIMEOUT_SECONDS.default,
	searchDefaultMaxResults: SEARCH_MAX_RESULTS.default,
	searchDefaultDepth: "auto",
} as const;

/** Clamp a finite number to an inclusive integer range. */
export function clampInteger(
	value: number,
	bounds: { readonly min: number; readonly max: number; readonly fallback: number },
): number {
	if (!Number.isFinite(value)) {
		return bounds.fallback;
	}

	return Math.max(bounds.min, Math.min(bounds.max, Math.round(value)));
}

export function parseOnOff(value: string | undefined, fallback: boolean): boolean {
	if (!value) return fallback;
	const normalized = value.trim().toLowerCase();
	if (normalized === "on") return true;
	if (normalized === "off") return false;
	return fallback;
}

export function parseIntegerSetting(
	value: string | undefined,
	fallback: number,
	options: { min?: number; max?: number } = {},
): number {
	const parsed = Number.parseInt(value?.trim() ?? "", 10);
	if (!Number.isFinite(parsed)) return fallback;
	if (options.min !== undefined && parsed < options.min) return fallback;
	if (options.max !== undefined && parsed > options.max) return fallback;
	return parsed;
}

export function parseEnumSetting<T extends string>(
	value: string | undefined,
	allowed: readonly T[],
	fallback: T,
): T {
	if (!value) return fallback;
	const normalized = value.trim() as T;
	return allowed.includes(normalized) ? normalized : fallback;
}

export const EXA_ENDPOINT_ENVIRONMENT_VARIABLE = "PI_WEB_TOOLS_EXA_ENDPOINT";
export const DEFAULT_EXA_ENDPOINT = "https://mcp.exa.ai/mcp";

/** Return web fetch settings that do not depend on search provider configuration. */
export function getWebFetchSettings(): WebToolsSettings["fetch"] {
	return {
		defaultFormat: DEFAULTS.fetchDefaultFormat,
		timeoutSeconds: DEFAULTS.fetchTimeoutSeconds,
		maxResponseBytes: DEFAULTS.fetchMaxResponseBytes,
		blockPrivateHosts: DEFAULTS.fetchBlockPrivateHosts,
		maxRedirects: DEFAULTS.fetchMaxRedirects,
		fallbackUserAgent: DEFAULTS.fetchFallbackUserAgent,
	};
}

/** Resolve the bounded provider route. Pi OpenAI auth is primary; Exa-compatible MCP is fallback. */
export function getWebSearchSettings(
	environment: Readonly<Record<string, string | undefined>> = process.env,
): WebToolsSettings["search"] {
	const endpoint = environment[EXA_ENDPOINT_ENVIRONMENT_VARIABLE]?.trim() || DEFAULT_EXA_ENDPOINT;
	return {
		enabled: true,
		providers: DEFAULTS.searchProviders,
		endpoint: parseExaEndpoint(endpoint),
		timeoutSeconds: DEFAULTS.searchTimeoutSeconds,
		defaultMaxResults: DEFAULTS.searchDefaultMaxResults,
		defaultDepth: DEFAULTS.searchDefaultDepth,
	};
}

function parseExaEndpoint(input: string): PublicHttpUrl {
	const parsed = parsePublicHttpUrl(input);
	if (parsed._tag === "err") {
		throw new Error(
			`Pi Web Tools configuration error: ${EXA_ENDPOINT_ENVIRONMENT_VARIABLE} must be a public HTTP or HTTPS URL`,
		);
	}
	return parsed.value;
}
