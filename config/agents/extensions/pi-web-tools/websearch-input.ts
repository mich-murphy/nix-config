import { err, ok, type Result } from "./result.ts";
import { SEARCH_DEPTHS, SEARCH_MAX_RESULTS, SEARCH_TIMEOUT_SECONDS, clampInteger, type ToolInputParseError } from "./settings.ts";
import { parseSearchQuery, type ParseSearchQueryError, type SearchDepth, type SearchQuery, type WebToolsSettings } from "./types.ts";

export type SearchRecency = "day" | "week" | "month" | "year";

export interface RawWebSearchToolParams {
	readonly query: string;
	readonly maxResults?: number;
	readonly depth?: SearchDepth;
	readonly recency?: SearchRecency;
	readonly domains?: readonly string[];
}

export interface WebSearchToolInput {
	readonly query: SearchQuery;
	readonly maxResults: number;
	readonly depth: SearchDepth;
	readonly timeoutSeconds: number;
	readonly recency?: SearchRecency;
	readonly allowedDomains: readonly string[];
	readonly blockedDomains: readonly string[];
}

/** Parse raw Pi websearch params into the small search-module interface. */
export function parseWebSearchToolParams(
	raw: unknown,
	settings: WebToolsSettings["search"],
): Result<WebSearchToolInput, ToolInputParseError | ParseSearchQueryError> {
	if (!isPlainObject(raw)) return err({ _tag: "InvalidToolInput", message: "Expected an object" });
	for (const key of Object.keys(raw)) {
		if (!["query", "maxResults", "depth", "recency", "domains"].includes(key)) return err({ _tag: "UnknownToolField", field: key });
	}
	if (typeof raw.query !== "string") return err({ _tag: "InvalidToolField", field: "query", message: "Expected a string" });
	const query = parseSearchQuery(raw.query);
	if (query._tag === "err") return query;

	let maxResults = clampInteger(settings.defaultMaxResults, { min: SEARCH_MAX_RESULTS.min, max: SEARCH_MAX_RESULTS.max, fallback: SEARCH_MAX_RESULTS.default });
	if (raw.maxResults !== undefined) {
		if (typeof raw.maxResults !== "number" || !Number.isFinite(raw.maxResults)) return err({ _tag: "InvalidToolField", field: "maxResults", message: "Expected a finite number" });
		maxResults = clampInteger(raw.maxResults, { min: SEARCH_MAX_RESULTS.min, max: SEARCH_MAX_RESULTS.max, fallback: SEARCH_MAX_RESULTS.default });
	}

	let depth = settings.defaultDepth;
	if (raw.depth !== undefined) {
		if (typeof raw.depth !== "string" || !includes(SEARCH_DEPTHS, raw.depth)) return err({ _tag: "InvalidToolField", field: "depth", message: "Expected one of: auto, fast, deep" });
		depth = raw.depth as SearchDepth;
	}

	let recency: SearchRecency | undefined;
	if (raw.recency !== undefined) {
		if (typeof raw.recency !== "string" || !includes(["day", "week", "month", "year"] as const, raw.recency)) return err({ _tag: "InvalidToolField", field: "recency", message: "Expected one of: day, week, month, year" });
		recency = raw.recency as SearchRecency;
	}

	const domains = parseDomains(raw.domains);
	if (domains._tag === "err") return domains;
	const timeoutSeconds = clampInteger(settings.timeoutSeconds, { min: SEARCH_TIMEOUT_SECONDS.min, max: SEARCH_TIMEOUT_SECONDS.max, fallback: SEARCH_TIMEOUT_SECONDS.default });
	return ok({ query: query.value, maxResults, depth, timeoutSeconds, ...(recency ? { recency } : {}), ...domains.value });
}

function parseDomains(value: unknown): Result<{ allowedDomains: string[]; blockedDomains: string[] }, ToolInputParseError> {
	if (value === undefined) return ok({ allowedDomains: [], blockedDomains: [] });
	if (!Array.isArray(value) || value.length > 100) return err({ _tag: "InvalidToolField", field: "domains", message: "Expected an array of at most 100 hostnames" });
	const allowedDomains: string[] = [];
	const blockedDomains: string[] = [];
	for (const entry of value) {
		if (typeof entry !== "string") return err({ _tag: "InvalidToolField", field: "domains", message: "Expected only hostname strings" });
		const blocked = entry.trim().startsWith("-");
		const hostname = normalizeDomain(blocked ? entry.trim().slice(1) : entry);
		if (!hostname) return err({ _tag: "InvalidToolField", field: "domains", message: `Invalid hostname: ${JSON.stringify(entry)}` });
		const target = blocked ? blockedDomains : allowedDomains;
		if (!target.includes(hostname)) target.push(hostname);
	}
	return ok({ allowedDomains, blockedDomains });
}

function normalizeDomain(value: string): string | undefined {
	const trimmed = value.trim().toLowerCase().replace(/^\.+|\.+$/g, "");
	if (!trimmed || trimmed.length > 253 || /[\s\\/?:#@]/.test(trimmed)) return undefined;
	return /^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$/i.test(trimmed) ? trimmed : undefined;
}

function includes(values: readonly string[], value: string): boolean { return values.includes(value); }
function isPlainObject(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null && !Array.isArray(value); }
