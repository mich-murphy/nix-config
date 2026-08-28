import type { Context, ImageContent, Message, TextContent } from "@earendil-works/pi-ai";

/** Image bytes extracted from one transcript entry. */
export interface ImageAttachment {
  /** Base64-encoded image bytes. */
  readonly data: string;
  /** Image media type supplied by Pi. */
  readonly mediaType: string;
}

/** One deterministic JSONL transcript entry and its extracted image payloads. */
export interface TranscriptEntry {
  /** Serialized JSONL text. */
  readonly text: string;
  /** Images referenced by this entry. */
  readonly images: ReadonlyArray<ImageAttachment>;
}

interface SerializedMessage {
  readonly json: object;
  readonly images: ReadonlyArray<ImageAttachment>;
}

function serializeImageBlocks(
  content: ReadonlyArray<TextContent | ImageContent>,
): { readonly content: ReadonlyArray<object>; readonly images: ReadonlyArray<ImageAttachment> } {
  const images: ImageAttachment[] = [];
  const serialized = content.map((block) => {
    if (block.type === "text") return { type: "text", text: block.text };
    images.push({ data: block.data, mediaType: block.mimeType });
    return { type: "image", mediaType: block.mimeType, imageRef: images.length - 1 };
  });
  return { content: serialized, images };
}

function serializeMessage(message: Message): SerializedMessage | undefined {
  if (message.role === "user") {
    if (typeof message.content === "string") {
      return { json: { role: "user", content: [{ type: "text", text: message.content }] }, images: [] };
    }
    const { content, images } = serializeImageBlocks(message.content);
    return { json: { role: "user", content }, images };
  }

  if (message.role === "assistant") {
    const content: object[] = [];
    for (const block of message.content) {
      if (block.type === "text") content.push({ type: "text", text: block.text });
      if (block.type === "toolCall") {
        content.push({ type: "toolCall", id: block.id, name: block.name, arguments: block.arguments });
      }
    }
    return content.length > 0 ? { json: { role: "assistant", content }, images: [] } : undefined;
  }

  const { content, images } = serializeImageBlocks(message.content);
  return {
    json: {
      role: "toolResult",
      toolCallId: message.toolCallId,
      toolName: message.toolName,
      isError: message.isError,
      content,
    },
    images,
  };
}

/**
 * Serialize Pi messages into stable transcript entries.
 *
 * @param context - Current Pi provider context.
 * @returns Entries in conversation order.
 */
export function serializeConversationEntries(context: Context): ReadonlyArray<TranscriptEntry> {
  const entries: TranscriptEntry[] = [];
  for (const message of context.messages) {
    const serialized = serializeMessage(message);
    if (serialized !== undefined) entries.push({ text: JSON.stringify(serialized.json), images: serialized.images });
  }
  return entries;
}
