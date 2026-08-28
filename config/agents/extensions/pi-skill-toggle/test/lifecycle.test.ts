import { describe, expect, test } from "bun:test";
import {
  formatSkillsForPrompt,
  type BuildSystemPromptOptions,
  type ExtensionAPI,
  type Skill,
} from "@earendil-works/pi-coding-agent";
import { registerSkillToggle } from "../index";
import { PolicyStateError, type PersistedPolicySnapshot, type PolicyStateAdapter } from "../policy";

const deploy: Skill = {
  name: "deploy",
  description: "Deploy software",
  filePath: "/skills/deploy/SKILL.md",
  baseDir: "/skills/deploy",
  sourceInfo: { path: "/skills/deploy/SKILL.md", source: "test", scope: "user", origin: "top-level" },
  disableModelInvocation: false,
};
const options: BuildSystemPromptOptions = { cwd: "/work/one", skills: [deploy] };

type TestHandler = (event: unknown, context: unknown) => unknown | Promise<unknown>;

function promptFor(promptOptions: BuildSystemPromptOptions): string {
  const contextFiles = promptOptions.contextFiles ?? [];
  const context = contextFiles.length === 0
    ? ""
    : `\n\n<project_context>\n\nProject-specific instructions and guidelines:\n\n${contextFiles
      .map(({ path, content }) => `<project_instructions path="${path}">\n${content}\n</project_instructions>\n\n`)
      .join("")}</project_context>\n`;
  return `base${context}${formatSkillsForPrompt(promptOptions.skills ?? [])}`;
}

function snapshot(cwd: string, hidden = false): PersistedPolicySnapshot {
  return {
    cwd,
    generation: `generation-${cwd}`,
    globalSkills: hidden ? { deploy: "manual-only" } : {},
    directorySkills: {},
    directoryInstructions: {},
  };
}

function loaded(cwd: string, hidden = false) {
  return { _tag: "ok" as const, value: snapshot(cwd, hidden) };
}

function systemPromptResult(value: unknown): string | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== "object" || value === null || !("systemPrompt" in value)) {
    throw new Error("Expected a before_agent_start result");
  }
  const systemPrompt = value.systemPrompt;
  if (typeof systemPrompt !== "string") throw new Error("Expected a string system prompt");
  return systemPrompt;
}

function harness(store: PolicyStateAdapter) {
  const handlers = new Map<string, TestHandler[]>();
  const commands = new Map<string, unknown>();
  const statuses: Array<string | undefined> = [];
  const notifications: string[] = [];
  const widgetPlacements: Array<"aboveEditor" | "belowEditor" | undefined> = [];
  const piMock = {
    on(name: string, handler: TestHandler) {
      handlers.set(name, [...(handlers.get(name) ?? []), handler]);
    },
    registerCommand(name: string, command: unknown) {
      commands.set(name, command);
    },
  };
  const themeMock = { fg: (_color: string, text: string) => text };
  const ctx = {
    cwd: "/work/one",
    mode: "tui",
    hasUI: true,
    sessionManager: {
      getBranch: () => [],
      getSessionId: () => "session",
    },
    ui: {
      theme: themeMock,
      setWidget: (
        _key: string,
        content: string[] | ((tui: unknown, theme: typeof themeMock) => { render(width: number): string[] }) | undefined,
        widgetOptions?: { placement?: "aboveEditor" | "belowEditor" },
      ) => {
        if (content === undefined) {
          statuses.push(undefined);
          widgetPlacements.push(undefined);
          return;
        }
        const lines = typeof content === "function" ? content(undefined, themeMock).render(200) : content;
        statuses.push(lines[0]?.trim());
        widgetPlacements.push(widgetOptions?.placement);
      },
      notify: (message: string) => notifications.push(message),
    },
  };
  // SAFETY: The extension only calls on() and registerCommand() during registration. This test double implements those methods and captures their arguments; handlers receive the separately constructed runtime context below.
  registerSkillToggle(piMock as unknown as ExtensionAPI, store);
  const emit = async (name: string, event: unknown = {}): Promise<unknown> => {
    let result: unknown;
    for (const handler of handlers.get(name) ?? []) result = await handler(event, ctx);
    return result;
  };
  return { commands, ctx, emit, statuses, notifications, widgetPlacements };
}

describe("extension lifecycle", () => {
  test("registers the skill command family", () => {
    const testHarness = harness({
      load: ({ cwd }) => loaded(cwd),
      apply: () => ({ applied: [], skipped: [], errors: [] }),
      reset: () => ({ applied: [], skipped: [], errors: [] }),
    });
    expect([...testHarness.commands.keys()]).toEqual(["skill-toggle", "skill-status", "skill-reset"]);
  });

  test("never applies another directory's snapshot after refresh failure and deduplicates errors", async () => {
    let unhealthy = false;
    const store: PolicyStateAdapter = {
      load: ({ cwd }) => unhealthy
        ? { _tag: "err", error: new PolicyStateError("load", `malformed state for ${cwd}`) }
        : loaded(cwd, cwd === "/work/one"),
      apply: () => ({ applied: [], skipped: [], errors: [] }),
      reset: () => ({ applied: [], skipped: [], errors: [] }),
    };
    const testHarness = harness(store);
    await testHarness.emit("session_start", { reason: "startup" });
    let result = await testHarness.emit("before_agent_start", {
      systemPrompt: promptFor(options),
      systemPromptOptions: options,
    });
    expect(systemPromptResult(result)).not.toContain("Deploy software");

    unhealthy = true;
    testHarness.ctx.cwd = "/work/two";
    const otherOptions = { ...options, cwd: "/work/two" };
    const unchanged = promptFor(otherOptions);
    result = await testHarness.emit("before_agent_start", {
      systemPrompt: unchanged,
      systemPromptOptions: otherOptions,
    });
    expect(result).toBeUndefined();
    expect(unchanged).toContain("Deploy software");
    expect(testHarness.statuses.at(-1)).toBe("skills !");
    const notificationCount = testHarness.notifications.length;
    await testHarness.emit("before_agent_start", { systemPrompt: unchanged, systemPromptOptions: otherOptions });
    expect(testHarness.notifications).toHaveLength(notificationCount);

    unhealthy = false;
    result = await testHarness.emit("before_agent_start", {
      systemPrompt: unchanged,
      systemPromptOptions: otherOptions,
    });
    expect(systemPromptResult(result)).toBe(unchanged);
    expect(testHarness.statuses.at(-1)).toBe("skills 1");
  });

  test("separates the loaded context file from the skill count with a bullet", async () => {
    const testHarness = harness({
      load: ({ cwd }) => loaded(cwd),
      apply: () => ({ applied: [], skipped: [], errors: [] }),
      reset: () => ({ applied: [], skipped: [], errors: [] }),
    });
    const contextOptions: BuildSystemPromptOptions = {
      ...options,
      contextFiles: [{ path: "/work/one/AGENTS.md", content: "Run tests." }],
    };

    await testHarness.emit("before_agent_start", {
      systemPrompt: promptFor(contextOptions),
      systemPromptOptions: contextOptions,
    });

    expect(testHarness.statuses.at(-1)).toBe("AGENTS.md • skills 1");
  });

  test.each(["new", "resume", "fork", "reload"])("clears status across %s session replacement", async (reason) => {
    const testHarness = harness({
      load: ({ cwd }) => loaded(cwd),
      apply: () => ({ applied: [], skipped: [], errors: [] }),
      reset: () => ({ applied: [], skipped: [], errors: [] }),
    });
    await testHarness.emit("session_start", { reason: "startup" });
    await testHarness.emit("session_shutdown", { reason });
    expect(testHarness.statuses.at(-1)).toBeUndefined();
    await testHarness.emit("session_start", { reason });
  });

  test("refreshes on tree navigation and clears state on shutdown", async () => {
    let loads = 0;
    const testHarness = harness({
      load: ({ cwd }) => {
        loads += 1;
        return loaded(cwd);
      },
      apply: () => ({ applied: [], skipped: [], errors: [] }),
      reset: () => ({ applied: [], skipped: [], errors: [] }),
    });
    await testHarness.emit("session_start", { reason: "startup" });
    await testHarness.emit("session_tree", {});
    expect(loads).toBe(2);
    await testHarness.emit("session_shutdown", { reason: "quit" });
    expect(testHarness.statuses.at(-1)).toBeUndefined();
  });
});
