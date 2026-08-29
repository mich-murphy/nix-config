import { CustomEditor, type ExtensionAPI } from "@earendil-works/pi-coding-agent";
import {
  matchesKey,
  parseKey,
  truncateToWidth,
  visibleWidth,
} from "@earendil-works/pi-tui";

const INPUT_LEFT = "\x1b[D";
const INPUT_RIGHT = "\x1b[C";
const INPUT_UP = "\x1b[A";
const INPUT_DOWN = "\x1b[B";
const INPUT_DELETE = "\x1b[3~";
const INPUT_WORD_LEFT = "\x1b[1;3D";
const INPUT_WORD_RIGHT = "\x1b[1;3C";
const INPUT_DELETE_WORD_RIGHT = "\x1b[3;3~";
const INPUT_LINE_START = "\x01";
const INPUT_LINE_END = "\x05";
const INPUT_DELETE_TO_LINE_END = "\x0b";
const INPUT_UNDO = "\x1f";

type PendingCommand = "none" | "delete" | "delete-inner" | "change" | "change-inner";
type ActivePendingCommand = Exclude<PendingCommand, "none">;
type VimState =
  | { readonly mode: "insert" }
  | { readonly mode: "normal"; readonly pending: PendingCommand };
type CharacterClass = "punctuation" | "whitespace" | "word";

function characterClass(value: string): CharacterClass {
  if (/\s/u.test(value)) return "whitespace";
  if (/[\p{L}\p{N}_]/u.test(value)) return "word";
  return "punctuation";
}

/** Pi's main prompt editor with a deliberately small set of Vim bindings. */
class VimEditor extends CustomEditor {
  private vimState: VimState = { mode: "insert" };

  override handleInput(data: string): void {
    if (matchesKey(data, "escape")) {
      this.handleEscape(data);
      return;
    }

    if (this.vimState.mode === "insert") {
      super.handleInput(data);
      return;
    }

    if (this.vimState.pending !== "none") {
      this.handlePending(this.vimState.pending, parseKey(data) ?? data);
      return;
    }

    const key = parseKey(data) ?? data;
    switch (key) {
      case "i":
        this.enterInsertMode();
        return;
      case "a":
        super.handleInput(INPUT_RIGHT);
        this.enterInsertMode();
        return;
      case "A":
        super.handleInput(INPUT_LINE_END);
        this.enterInsertMode();
        return;
      case "I":
        super.handleInput(INPUT_LINE_START);
        this.enterInsertMode();
        return;
      case "o":
        this.openLineBelow();
        return;
      case "O":
        this.openLineAbove();
        return;
      case "h":
        super.handleInput(INPUT_LEFT);
        return;
      case "j":
        super.handleInput(INPUT_DOWN);
        this.clampNormalCursor();
        return;
      case "k":
        super.handleInput(INPUT_UP);
        this.clampNormalCursor();
        return;
      case "l":
        this.moveNormalCursorRight();
        return;
      case "w":
        this.moveWordForward();
        return;
      case "b":
        super.handleInput(INPUT_WORD_LEFT);
        this.clampNormalCursor();
        return;
      case "0":
        super.handleInput(INPUT_LINE_START);
        return;
      case "$":
        super.handleInput(INPUT_LINE_END);
        this.clampNormalCursor();
        return;
      case "x":
        super.handleInput(INPUT_DELETE);
        this.clampNormalCursor();
        return;
      case "D":
        super.handleInput(INPUT_DELETE_TO_LINE_END);
        this.clampNormalCursor();
        return;
      case "C":
        super.handleInput(INPUT_DELETE_TO_LINE_END);
        this.enterInsertMode();
        return;
      case "u":
        super.handleInput(INPUT_UNDO);
        this.clampNormalCursor();
        return;
      case "d":
        this.vimState = { mode: "normal", pending: "delete" };
        return;
      case "c":
        this.vimState = { mode: "normal", pending: "change" };
        return;
      case "enter": {
        const textBeforeSubmit = this.getText();
        super.handleInput(data);
        if (textBeforeSubmit.length > 0 && this.getText().length === 0) {
          this.enterInsertMode();
        }
        return;
      }
      default:
        this.handleUnhandledNormalInput(data, key);
    }
  }

  override render(width: number): string[] {
    const lines = super.render(width);
    const lastIndex = lines.length - 1;
    const lastLine = lines[lastIndex];
    if (lastLine === undefined) return lines;

    const pending = this.vimState.mode === "normal" && this.vimState.pending !== "none"
      ? ` ${this.pendingLabel(this.vimState.pending)}`
      : "";
    const label = this.vimState.mode === "insert" ? " INSERT " : ` NORMAL${pending} `;
    if (visibleWidth(lastLine) < label.length) return lines;

    lines[lastIndex] = truncateToWidth(lastLine, width - label.length, "") + label;
    return lines;
  }

  private handleEscape(data: string): void {
    if (this.vimState.mode === "insert") {
      this.vimState = { mode: "normal", pending: "none" };
      this.clampNormalCursor();
      return;
    }
    if (this.vimState.pending !== "none") {
      this.vimState = { mode: "normal", pending: "none" };
      return;
    }
    super.handleInput(data);
  }

  private handlePending(pending: ActivePendingCommand, key: string): void {
    if (pending === "delete") {
      if (key === "i") {
        this.vimState = { mode: "normal", pending: "delete-inner" };
        return;
      }
      this.vimState = { mode: "normal", pending: "none" };
      if (key === "d") this.deleteLine();
      else if (key === "w") this.deleteVimWordForward();
      return;
    }
    if (pending === "change") {
      if (key === "i") {
        this.vimState = { mode: "normal", pending: "change-inner" };
        return;
      }
      this.vimState = { mode: "normal", pending: "none" };
      if (key === "c") this.changeLine();
      else if (key === "w") this.changeWordForward();
      return;
    }

    this.vimState = { mode: "normal", pending: "none" };
    if (key !== "w") return;
    if (pending === "delete-inner") {
      this.deleteInnerWord();
      return;
    }
    this.changeInnerWord();
  }

  private deleteLine(): void {
    super.handleInput(INPUT_LINE_START);
    super.handleInput(INPUT_DELETE_TO_LINE_END);
    super.handleInput(INPUT_DELETE);
    this.clampNormalCursor();
  }

  private changeLine(): void {
    super.handleInput(INPUT_LINE_START);
    super.handleInput(INPUT_DELETE_TO_LINE_END);
    this.enterInsertMode();
  }

  private deleteVimWordForward(): void {
    super.handleInput(INPUT_DELETE_WORD_RIGHT);
    this.clampNormalCursor();
  }

  private changeWordForward(): void {
    super.handleInput(INPUT_DELETE_WORD_RIGHT);
    this.enterInsertMode();
  }

  private deleteInnerWord(): void {
    this.deleteInnerWordContent();
    this.clampNormalCursor();
  }

  private changeInnerWord(): void {
    this.deleteInnerWordContent();
    this.enterInsertMode();
  }

  private deleteInnerWordContent(): void {
    const { line, col } = this.getCursor();
    const text = this.getLines()[line] ?? "";
    const current = text.slice(col, col + 1);
    const previous = text.slice(Math.max(0, col - 1), col);
    if (
      col > 0
      && current.length > 0
      && previous.length > 0
      && characterClass(current) === characterClass(previous)
    ) {
      super.handleInput(INPUT_WORD_LEFT);
    }
    super.handleInput(INPUT_DELETE_WORD_RIGHT);
  }

  private openLineBelow(): void {
    super.handleInput(INPUT_LINE_END);
    this.insertTextAtCursor("\n");
    this.enterInsertMode();
  }

  private openLineAbove(): void {
    super.handleInput(INPUT_LINE_START);
    this.insertTextAtCursor("\n");
    super.handleInput(INPUT_UP);
    this.enterInsertMode();
  }

  private enterInsertMode(): void {
    this.vimState = { mode: "insert" };
  }

  private moveNormalCursorRight(): void {
    const { line, col } = this.getCursor();
    const text = this.getLines()[line] ?? "";
    if (col >= text.length) return;
    super.handleInput(INPUT_RIGHT);
    this.clampNormalCursor();
  }

  private moveWordForward(): void {
    const { line, col } = this.getCursor();
    const current = (this.getLines()[line] ?? "").slice(col, col + 1);
    if (characterClass(current) !== "whitespace") {
      super.handleInput(INPUT_WORD_RIGHT);
    }

    while (true) {
      const cursor = this.getCursor();
      const lines = this.getLines();
      const text = lines[cursor.line] ?? "";
      if (cursor.col < text.length && characterClass(text.slice(cursor.col, cursor.col + 1)) !== "whitespace") {
        break;
      }
      if (cursor.col >= text.length && cursor.line >= lines.length - 1) break;
      super.handleInput(INPUT_RIGHT);
    }
    this.clampNormalCursor();
  }

  private clampNormalCursor(): void {
    if (this.vimState.mode !== "normal") return;
    const { line, col } = this.getCursor();
    const text = this.getLines()[line] ?? "";
    if (text.length > 0 && col >= text.length) super.handleInput(INPUT_LEFT);
  }

  private handleUnhandledNormalInput(data: string, key: string): void {
    if (key.length === 1) return;
    super.handleInput(data);
  }

  private pendingLabel(pending: ActivePendingCommand): string {
    switch (pending) {
      case "delete": return "d";
      case "delete-inner": return "di";
      case "change": return "c";
      case "change-inner": return "ci";
    }
  }
}

/** Register Vim-style modal editing for Pi's interactive prompt composer. */
export default function vimMode(pi: ExtensionAPI): void {
  pi.on("session_start", (_event, ctx) => {
    if (ctx.mode !== "tui") return;
    ctx.ui.setEditorComponent(
      (tui, theme, keybindings) => new VimEditor(tui, theme, keybindings),
    );
  });
}
