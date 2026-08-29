import { join } from "node:path";
import { describe, expect, test } from "bun:test";
import {
  formatSkillsForPrompt,
  getAgentDir,
  type BuildSystemPromptOptions,
  type ExtensionAPI,
  type Skill,
} from "@earendil-works/pi-coding-agent";
import { registerSkillToggle } from "../index";
import { resourcePathId } from "../resource-path";
import {
  SkillToggleStateError,
  type SkillToggleState,
  type SkillToggleStateStore,
} from "../state";

const skillPath = join(getAgentDir(), "skills/research/SKILL.md");
const research: Skill = {
  name: "research",
  description: "Research primary sources",
  filePath: skillPath,
  baseDir: join(skillPath, ".."),
  sourceInfo: {
    path: skillPath,
    source: "local",
    scope: "user",
    origin: "top-level",
  },
  disableModelInvocation: false,
};
const options: BuildSystemPromptOptions = { cwd: "/work/project", skills: [research] };

type TestHandler = (event: unknown, context: unknown) => unknown | Promise<unknown>;

function state(resources: SkillToggleState["resources"] = {}): SkillToggleState {
  return { version: 4, resources };
}

function harness(store: SkillToggleStateStore) {
  const handlers = new Map<string, TestHandler[]>();
  const commands: string[] = [];
  const notifications: string[] = [];
  const piMock = {
    on(name: string, handler: TestHandler) {
      handlers.set(name, [...(handlers.get(name) ?? []), handler]);
    },
    registerCommand(name: string) {
      commands.push(name);
    },
  };
  const ctx = {
    cwd: "/work/project",
    ui: {
      notify: (message: string) => notifications.push(message),
    },
  };
  // SAFETY: Registration uses only on() and registerCommand(). The test double captures both and supplies a separate event context to handlers.
  registerSkillToggle(piMock as unknown as ExtensionAPI, store);
  const emit = async (name: string, event: unknown): Promise<unknown> => {
    let result: unknown;
    for (const handler of handlers.get(name) ?? []) result = await handler(event, ctx);
    return result;
  };
  return { commands, emit, notifications };
}

describe("extension lifecycle", () => {
  test("registers only the skill-toggle command", () => {
    const testHarness = harness({
      load: () => ({ _tag: "ok", value: state() }),
      setValue: () => ({ _tag: "ok", value: state() }),
    });

    expect(testHarness.commands).toEqual(["skill-toggle"]);
  });

  test("applies persisted path toggles before the model starts", async () => {
    const disabled = {
      [skillPath]: {
        kind: "skill" as const,
        origin: "global" as const,
        owner: resourcePathId(join(getAgentDir(), "skills")),
        enabled: false as const,
      },
    };
    const testHarness = harness({
      load: () => ({ _tag: "ok", value: state(disabled) }),
      setValue: () => ({ _tag: "ok", value: state(disabled) }),
    });

    const result = await testHarness.emit("before_agent_start", {
      systemPrompt: `base${formatSkillsForPrompt([research])}`,
      systemPromptOptions: options,
    });

    expect(result).toEqual({ systemPrompt: "base" });
  });

  test("does not apply stale state to package or other excluded resources", async () => {
    const packagePath = "/packages/research/SKILL.md";
    const packageSkill: Skill = {
      ...research,
      filePath: packagePath,
      baseDir: "/packages/research",
      sourceInfo: {
        path: packagePath,
        source: "npm:example",
        scope: "user",
        origin: "package",
      },
    };
    const stale = {
      [packagePath]: {
        kind: "skill" as const,
        origin: "global" as const,
        owner: resourcePathId("/packages"),
        enabled: false as const,
      },
    };
    const testHarness = harness({
      load: () => ({ _tag: "ok", value: state(stale) }),
      setValue: () => ({ _tag: "ok", value: state(stale) }),
    });
    const prompt = `base${formatSkillsForPrompt([packageSkill])}`;

    const result = await testHarness.emit("before_agent_start", {
      systemPrompt: prompt,
      systemPromptOptions: { cwd: "/work/project", skills: [packageSkill] },
    });

    expect(result).toEqual({ systemPrompt: prompt });
  });

  test("leaves the prompt unchanged and deduplicates state failures", async () => {
    const error = new SkillToggleStateError("load", "broken state");
    const testHarness = harness({
      load: () => ({ _tag: "err", error }),
      setValue: () => ({ _tag: "err", error }),
    });
    const event = {
      systemPrompt: `base${formatSkillsForPrompt([research])}`,
      systemPromptOptions: options,
    };

    expect(await testHarness.emit("before_agent_start", event)).toBeUndefined();
    expect(await testHarness.emit("before_agent_start", event)).toBeUndefined();
    expect(testHarness.notifications).toHaveLength(1);
    expect(testHarness.notifications[0]).toContain("prompt was left unchanged");
  });
});
