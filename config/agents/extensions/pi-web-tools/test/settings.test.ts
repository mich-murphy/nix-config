import test from "node:test";
import assert from "node:assert/strict";
import {
	EXA_ENDPOINT_ENVIRONMENT_VARIABLE,
	getWebFetchSettings,
	getWebSearchSettings,
	parseEnumSetting,
	parseIntegerSetting,
	parseOnOff,
} from "../settings.ts";

test("parseOnOff accepts on/off and falls back safely", () => {
	assert.equal(parseOnOff("on", false), true);
	assert.equal(parseOnOff("off", true), false);
	assert.equal(parseOnOff("bogus", true), true);
	assert.equal(parseOnOff(undefined, false), false);
});

test("parseIntegerSetting validates integer ranges", () => {
	assert.equal(parseIntegerSetting("30", 10, { min: 1, max: 120 }), 30);
	assert.equal(parseIntegerSetting("0", 10, { min: 1, max: 120 }), 10);
	assert.equal(parseIntegerSetting("121", 10, { min: 1, max: 120 }), 10);
	assert.equal(parseIntegerSetting("not-a-number", 10, { min: 1, max: 120 }), 10);
});

test("parseEnumSetting validates allowed values", () => {
	assert.equal(parseEnumSetting("markdown", ["markdown", "text", "html"], "text"), "markdown");
	assert.equal(parseEnumSetting("pdf", ["markdown", "text", "html"], "text"), "text");
	assert.equal(parseEnumSetting(undefined, ["markdown", "text", "html"], "text"), "text");
});

test("web fetch settings do not require Exa configuration", () => {
	assert.equal(getWebFetchSettings().defaultFormat, "markdown");
});

test("web search settings enable Exa with a validated MCP endpoint", () => {
	const settings = getWebSearchSettings({
		[EXA_ENDPOINT_ENVIRONMENT_VARIABLE]: "https://mcp.exa.ai/mcp",
	});

	assert.deepEqual(settings.providers, ["openai", "exa"]);
	assert.equal(settings.endpoint, "https://mcp.exa.ai/mcp");
});

test("web search defaults to Exa's standard hosted MCP endpoint", () => {
	const settings = getWebSearchSettings({});
	assert.deepEqual(settings.providers, ["openai", "exa"]);
	assert.equal(settings.endpoint, "https://mcp.exa.ai/mcp");
});

test("web search treats an empty Exa endpoint override as unset", () => {
	const settings = getWebSearchSettings({ [EXA_ENDPOINT_ENVIRONMENT_VARIABLE]: "  " });
	assert.deepEqual(settings.providers, ["openai", "exa"]);
	assert.equal(settings.endpoint, "https://mcp.exa.ai/mcp");
});

test("web search rejects an invalid Exa endpoint without exposing its value", () => {
	assert.throws(
		() => getWebSearchSettings({ [EXA_ENDPOINT_ENVIRONMENT_VARIABLE]: "not-a-url" }),
		new Error(
			`Pi Web Tools configuration error: ${EXA_ENDPOINT_ENVIRONMENT_VARIABLE} must be a public HTTP or HTTPS URL`,
		),
	);
});
