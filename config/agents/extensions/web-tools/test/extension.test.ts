import { describe, expect, test } from "bun:test";
import type { ExtensionAPI, ToolDefinition } from "@earendil-works/pi-coding-agent";
import webToolsExtension from "../index.js";

describe("web tools extension", () => {
  test("registers webfetch and websearch as Pi tools", () => {
    const tools: ToolDefinition[] = [];
    const pi = {
      registerTool(tool: ToolDefinition) {
        tools.push(tool);
      },
    } as unknown as ExtensionAPI;

    webToolsExtension(pi);

    expect(tools.map((tool) => tool.name)).toEqual(["webfetch", "websearch"]);
    expect(tools.every((tool) => tool.promptSnippet)).toBe(true);
  });
});
