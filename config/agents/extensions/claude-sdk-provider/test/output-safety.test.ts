import { describe, expect, test } from "bun:test";
import { inspectBashCommand, sanitizeBashContent, sanitizeContextMessages } from "../output-safety";

describe("bash output safety", () => {
  test("blocks the executable-dump pattern that poisoned the investigated session", () => {
    expect(inspectBashCommand("cat $(which plannotator) 2>/dev/null | head -100")).toContain(
      'file "$(which COMMAND)"',
    );
  });

  test("quarantines binary output before it enters the transcript", () => {
    const binary = `Mach-O\u0000\u0001\u0002${"\u0000binary".repeat(5_000)}`;
    const result = sanitizeBashContent([{ type: "text", text: binary }]);

    expect(result.detected).toBe("binary");
    const notice = result.content[0];
    expect(notice?.type).toBe("text");
    if (notice?.type !== "text") throw new Error("missing quarantine notice");
    expect(notice.text).toContain("Binary-like bash output quarantined");
    expect(notice.text.length).toBeLessThan(1_000);
    expect(notice.text).toContain("Use file, otool, or strings");
  });

  test("quarantines long base64 output", () => {
    const base64 = "iVBORw0KGgo" + "A".repeat(20_000);
    const result = sanitizeBashContent([{ type: "text", text: base64 }]);

    expect(result.detected).toBe("base64");
    const notice = result.content[0];
    if (notice?.type !== "text") throw new Error("missing quarantine notice");
    expect(notice.text).toContain("Base64-like bash output quarantined");
    expect(notice.text.length).toBeLessThan(1_000);
  });

  test("leaves normal command output unchanged", () => {
    const content = [{ type: "text" as const, text: "src/index.ts\nREADME.md\n3 files changed" }];
    expect(sanitizeBashContent(content)).toEqual({ content, detected: undefined });
  });

  test("removes already-recorded suspicious bash output from provider context", () => {
    const messages = [
      { role: "user", content: "inspect it" },
      {
        role: "toolResult",
        toolName: "bash",
        toolCallId: "call-1",
        isError: false,
        content: [{ type: "text", text: `header${"\u0000payload".repeat(5_000)}` }],
      },
    ];

    const sanitized = sanitizeContextMessages(messages);
    const toolResult = sanitized[1] as unknown as { content: Array<{ type: string; text: string }> };
    expect(toolResult.content[0]?.text).toContain("Binary-like bash output quarantined");
    expect(toolResult.content[0]?.text?.length).toBeLessThan(1_000);
  });
});
