import type { ImageContent, TextContent } from "@earendil-works/pi-ai";

type ContentBlock = TextContent | ImageContent;

export type SuspiciousOutputKind = "binary" | "base64";

const MIN_SUSPICIOUS_CHARACTERS = 4_096;
const BASE64_ALPHABET = /^[A-Za-z0-9+/]+={0,2}$/;

function suspiciousOutputKind(text: string): SuspiciousOutputKind | undefined {
  if (text.length < MIN_SUSPICIOUS_CHARACTERS) return undefined;

  let controlCharacters = 0;
  for (const character of text) {
    const codePoint = character.charCodeAt(0);
    if (character === "\ufffd" || codePoint === 0 || (codePoint < 32 && character !== "\n" && character !== "\r" && character !== "\t")) {
      controlCharacters += 1;
    }
  }
  if (controlCharacters >= 8 && controlCharacters / text.length >= 0.002) return "binary";

  const compact = text.replace(/\s/g, "");
  if (compact.length >= 8_192 && compact.length / text.length >= 0.95 && BASE64_ALPHABET.test(compact)) return "base64";
  return undefined;
}

function quarantineNotice(kind: SuspiciousOutputKind, characters: number): string {
  const label = kind === "binary" ? "Binary-like" : "Base64-like";
  const advice =
    kind === "binary"
      ? "Use file, otool, or strings instead of cat for executables."
      : "Write encoded data to a file and inspect its type or metadata instead of printing it.";
  return `[${label} bash output quarantined before it entered model context (${characters.toLocaleString("en-US")} characters). ${advice} If similar output was recorded before this guard loaded, compact or start a new session.]`;
}

export function sanitizeBashContent(content: readonly ContentBlock[]): {
  content: ContentBlock[];
  detected: SuspiciousOutputKind | undefined;
} {
  for (const block of content) {
    if (block.type !== "text" || typeof block.text !== "string") continue;
    const detected = suspiciousOutputKind(block.text);
    if (detected) {
      return {
        content: [{ type: "text", text: quarantineNotice(detected, block.text.length) }],
        detected,
      };
    }
  }
  return { content: content as ContentBlock[], detected: undefined };
}

export function inspectBashCommand(command: string): string | undefined {
  if (!/\bcat\b[\s\S]*(?:\$\(\s*(?:which|command\s+-v)\b|`\s*(?:which|command\s+-v)\b)/i.test(command)) {
    return undefined;
  }
  return 'Refusing to pipe a discovered executable through cat. Inspect it with file "$(which COMMAND)", otool, or strings "$(which COMMAND)" | head instead.';
}

export function sanitizeContextMessages<T>(messages: readonly T[]): T[] {
  return messages.map((message) => {
    if (typeof message !== "object" || message === null) return message;
    const fields = message as Record<string, unknown>;
    if (fields.role !== "toolResult" || fields.toolName !== "bash" || !Array.isArray(fields.content)) return message;
    const sanitized = sanitizeBashContent(fields.content as ContentBlock[]);
    if (!sanitized.detected) return message;
    return { ...fields, content: sanitized.content } as T;
  });
}
