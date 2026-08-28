import type { ImageContent, TextContent } from "@earendil-works/pi-ai";

type ContentBlock = TextContent | ImageContent;

/** Suspicious shell-output category recognized by the quarantine guard. */
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

/** Result of inspecting shell tool content. */
export interface SanitizedBashContent {
  /** Original content or a short quarantine notice. */
  readonly content: ContentBlock[];
  /** Detected category, or `undefined` when the content is safe. */
  readonly detected: SuspiciousOutputKind | undefined;
}

/**
 * Replace suspicious shell output with a bounded quarantine notice.
 *
 * @param content - Parsed Pi text and image content blocks.
 * @returns The original readonly content or replacement notice.
 */
export function sanitizeBashContent(content: ReadonlyArray<ContentBlock>): SanitizedBashContent {
  for (const block of content) {
    if (block.type !== "text") continue;
    const detected = suspiciousOutputKind(block.text);
    if (detected) {
      return {
        content: [{ type: "text", text: quarantineNotice(detected, block.text.length) }],
        detected,
      };
    }
  }
  return { content: [...content], detected: undefined };
}

/**
 * Detect a command that prints a discovered executable through `cat`.
 *
 * @param command - Shell command submitted to Pi's bash tool.
 * @returns A blocking explanation when the unsafe pattern is present.
 */
export function inspectBashCommand(command: string): string | undefined {
  if (!/\bcat\b[\s\S]*(?:\$\(\s*(?:which|command\s+-v)\b|`\s*(?:which|command\s+-v)\b)/i.test(command)) {
    return undefined;
  }
  return 'Refusing to pipe a discovered executable through cat. Inspect it with file "$(which COMMAND)", otool, or strings "$(which COMMAND)" | head instead.';
}

function isContentBlock(value: unknown): value is ContentBlock {
  if (typeof value !== "object" || value === null) return false;
  // SAFETY: The object check permits property reads while every consumed property remains unknown until checked below.
  const block = value as Record<string, unknown>;
  if (block.type === "text") return typeof block.text === "string";
  return block.type === "image" && typeof block.data === "string" && typeof block.mimeType === "string";
}

/**
 * Quarantine suspicious bash results already stored in a Pi context.
 *
 * @param messages - Pi context messages. The generic preserves the caller's concrete message union.
 * @returns A new message array with only suspicious bash results replaced.
 */
export function sanitizeContextMessages<T>(messages: ReadonlyArray<T>): T[] {
  return messages.map((message) => {
    if (typeof message !== "object" || message === null) return message;
    // SAFETY: This is an interop boundary over Pi's message union. Fields are checked before use and the original value is returned when the shape does not match.
    const fields = message as Record<string, unknown>;
    if (fields.role !== "toolResult" || fields.toolName !== "bash" || !Array.isArray(fields.content)) return message;
    if (!fields.content.every(isContentBlock)) return message;
    const sanitized = sanitizeBashContent(fields.content);
    if (!sanitized.detected) return message;
    // SAFETY: The replacement preserves every field of T and replaces tool-result content with valid Pi content blocks after the role and content checks above.
    return { ...fields, content: [...sanitized.content] } as T;
  });
}
