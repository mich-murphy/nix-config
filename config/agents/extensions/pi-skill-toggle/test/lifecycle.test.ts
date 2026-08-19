import { describe, expect, test } from "bun:test";
import { formatSkillsForPrompt, type BuildSystemPromptOptions, type Skill } from "@earendil-works/pi-coding-agent";
import { registerSkillToggle } from "../index";
import type { PersistedPolicySnapshot, PolicyStateAdapter } from "../policy";

const deploy: Skill = {
  name: "deploy",
  description: "Deploy software",
  filePath: "/skills/deploy/SKILL.md",
  baseDir: "/skills/deploy",
  sourceInfo: { path: "/skills/deploy/SKILL.md", source: "test", scope: "user", origin: "top-level" },
  disableModelInvocation: false,
};
const options: BuildSystemPromptOptions = { cwd: "/work/one", skills: [deploy] };

function promptFor(promptOptions: BuildSystemPromptOptions): string {
  return `base${formatSkillsForPrompt(promptOptions.skills ?? [])}`;
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

function harness(store: PolicyStateAdapter) {
  const handlers = new Map<string, Array<(event: any, ctx: any) => any>>();
  const commands = new Map<string, unknown>();
  const statuses: Array<string | undefined> = [];
  const notifications: string[] = [];
  const widgetPlacements: Array<"aboveEditor" | "belowEditor" | undefined> = [];
  const pi = {
    on(name: string, handler: (event: any, ctx: any) => any) {
      handlers.set(name, [...(handlers.get(name) ?? []), handler]);
    },
    registerCommand(name: string, command: unknown) {
      commands.set(name, command);
    },
  };
  const themeMock = { fg: (_color: string, text: string) => text };
  const ctx: any = {
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
        options?: { placement?: "aboveEditor" | "belowEditor" },
      ) => {
        if (content === undefined) {
          statuses.push(undefined);
          widgetPlacements.push(undefined);
          return;
        }
        const lines = typeof content === "function" ? content(undefined, themeMock).render(200) : content;
        statuses.push(lines[0]?.trim());
        widgetPlacements.push(options?.placement);
      },
      notify: (message: string) => notifications.push(message),
    },
  };
  registerSkillToggle(pi as never, store);
  const emit = async (name: string, event: any = {}) => {
    let result;
    for (const handler of handlers.get(name) ?? []) result = await handler(event, ctx);
    return result;
  };
  return { commands, ctx, emit, statuses, notifications, widgetPlacements };
}

describe("extension lifecycle", () => {
  test("registers the skill command family", () => {
    const test = harness({
      load: ({ cwd }) => snapshot(cwd),
      apply: () => ({ applied: [], skipped: [], errors: [] }),
      reset: () => ({ applied: [], skipped: [], errors: [] }),
    });
    expect([...test.commands.keys()]).toEqual(["skill-toggle", "skill-status", "skill-reset"]);
  });

  test("never applies another directory's snapshot after refresh failure and deduplicates errors", async () => {
    let unhealthy = false;
    const store: PolicyStateAdapter = {
      load: ({ cwd }) => {
        if (unhealthy) throw new Error(`malformed state for ${cwd}`);
        return snapshot(cwd, cwd === "/work/one");
      },
      apply: () => ({ applied: [], skipped: [], errors: [] }),
      reset: () => ({ applied: [], skipped: [], errors: [] }),
    };
    const test = harness(store);
    await test.emit("session_start", { reason: "startup" });
    let result = await test.emit("before_agent_start", { systemPrompt: promptFor(options), systemPromptOptions: options });
    expect(result.systemPrompt).not.toContain("Deploy software");

    unhealthy = true;
    test.ctx.cwd = "/work/two";
    const otherOptions = { ...options, cwd: "/work/two" };
    const unchanged = promptFor(otherOptions);
    result = await test.emit("before_agent_start", { systemPrompt: unchanged, systemPromptOptions: otherOptions });
    expect(result).toBeUndefined();
    expect(unchanged).toContain("Deploy software");
    expect(test.statuses.at(-1)).toBe("skills !");
    const notificationCount = test.notifications.length;
    await test.emit("before_agent_start", { systemPrompt: unchanged, systemPromptOptions: otherOptions });
    expect(test.notifications).toHaveLength(notificationCount);

    unhealthy = false;
    result = await test.emit("before_agent_start", { systemPrompt: unchanged, systemPromptOptions: otherOptions });
    expect(result.systemPrompt).toBe(unchanged);
    // Recovery in /work/two resolves "deploy" as visible (not hidden there), so
    // the widget now reports the loaded-skill count instead of being cleared.
    expect(test.statuses.at(-1)).toBe("skills 1");
  });

  test.each(["new", "resume", "fork", "reload"])("clears status across %s session replacement", async (reason) => {
    const test = harness({
      load: ({ cwd }) => snapshot(cwd),
      apply: () => ({ applied: [], skipped: [], errors: [] }),
      reset: () => ({ applied: [], skipped: [], errors: [] }),
    });
    await test.emit("session_start", { reason: "startup" });
    await test.emit("session_shutdown", { reason });
    expect(test.statuses.at(-1)).toBeUndefined();
    await test.emit("session_start", { reason });
  });

  test("refreshes on tree navigation and clears state on shutdown", async () => {
    let loads = 0;
    const test = harness({
      load: ({ cwd }) => { loads += 1; return snapshot(cwd); },
      apply: () => ({ applied: [], skipped: [], errors: [] }),
      reset: () => ({ applied: [], skipped: [], errors: [] }),
    });
    await test.emit("session_start", { reason: "startup" });
    await test.emit("session_tree", {});
    expect(loads).toBe(2);
    await test.emit("session_shutdown", { reason: "quit" });
    expect(test.statuses.at(-1)).toBeUndefined();
  });
});
