import { basename, dirname } from "node:path";
import type {
  BuildSystemPromptOptions,
  ExtensionAPI,
  ExtensionContext,
  Skill,
} from "@earendil-works/pi-coding-agent";
import {
  DynamicBorder,
  getAgentDir,
  getSettingsListTheme,
} from "@earendil-works/pi-coding-agent";
import {
  Container,
  type SettingItem,
  SettingsList,
  Text,
} from "@earendil-works/pi-tui";
import {
  filterSystemPrompt,
  resourcePathId,
  type ContextFile,
} from "./prompt-filter";
import {
  readStateFromBranch,
  STATE_TYPE,
  type ContextControlState,
} from "./state";

const STATUS_KEY = "context-control";
const DESCRIPTION_LIMIT = 180;

export default function contextControl(pi: ExtensionAPI) {
  let disabledContextPaths = new Set<string>();
  let hiddenSkillPaths = new Set<string>();

  function currentState(): ContextControlState {
    return {
      disabledContextPaths: [...disabledContextPaths].sort(),
      hiddenSkillPaths: [...hiddenSkillPaths].sort(),
    };
  }

  function persist(): void {
    pi.appendEntry(STATE_TYPE, currentState());
  }

  function restore(ctx: ExtensionContext): void {
    const state = readStateFromBranch(ctx.sessionManager.getBranch());
    disabledContextPaths = new Set(state.disabledContextPaths);
    hiddenSkillPaths = new Set(state.hiddenSkillPaths);
    renderStatus(ctx);
  }

  function renderStatus(ctx: ExtensionContext): void {
    const disabledCount = disabledContextPaths.size + hiddenSkillPaths.size;
    ctx.ui.setStatus(
      STATUS_KEY,
      disabledCount > 0
        ? ctx.ui.theme.fg("warning", `context −${disabledCount}`)
        : undefined,
    );
  }

  function updateResource(kind: "context" | "skill", path: string, disabled: boolean): void {
    const target = kind === "context" ? disabledContextPaths : hiddenSkillPaths;
    if (disabled) target.add(path);
    else target.delete(path);
  }

  pi.registerCommand("context", {
    description: "Toggle instruction files and skills for this session branch",
    handler: async (args, ctx) => {
      if (args.trim()) {
        ctx.ui.notify("Usage: /context", "error");
        return;
      }
      if (ctx.mode !== "tui") {
        ctx.ui.notify("/context requires TUI mode", "error");
        return;
      }

      const options = ctx.getSystemPromptOptions();
      const items = buildSettingItems(
        options,
        disabledContextPaths,
        hiddenSkillPaths,
        ctx.cwd,
      );

      if (items.length === 0) {
        ctx.ui.notify("No instruction files or skills are loaded", "info");
        return;
      }

      await ctx.ui.custom((tui, theme, _keybindings, done) => {
        const container = new Container();
        container.addChild(new DynamicBorder((text: string) => theme.fg("accent", text)));
        container.addChild(new Text(theme.fg("accent", theme.bold("Context")), 1, 0));
        container.addChild(
          new Text(
            theme.fg(
              "muted",
              `${options.contextFiles?.length ?? 0} instruction files · ${options.skills?.length ?? 0} skills · session branch only`,
            ),
            1,
            0,
          ),
        );

        const list = new SettingsList(
          items,
          Math.min(items.length + 2, 20),
          getSettingsListTheme(),
          (id, value) => {
            const separator = id.indexOf(":");
            if (separator < 0) return;
            const kind = id.slice(0, separator) as "context" | "skill";
            const path = id.slice(separator + 1);
            updateResource(kind, path, value === "excluded" || value === "manual-only");
            persist();
            renderStatus(ctx);
          },
          () => done(undefined),
          { enableSearch: true },
        );
        container.addChild(list);
        container.addChild(new DynamicBorder((text: string) => theme.fg("accent", text)));

        return {
          render: (width: number) => container.render(width),
          invalidate: () => container.invalidate(),
          handleInput: (data: string) => {
            list.handleInput?.(data);
            tui.requestRender();
          },
        };
      });
    },
  });

  pi.registerCommand("context-status", {
    description: "Show session-local context exclusions",
    handler: async (_args, ctx) => {
      ctx.ui.notify(
        formatContextStatus(
          ctx.getSystemPromptOptions(),
          { disabledContextPaths, hiddenSkillPaths },
          ctx.cwd,
        ),
        "info",
      );
    },
  });

  pi.on("session_start", async (_event, ctx) => restore(ctx));
  pi.on("session_tree", async (_event, ctx) => restore(ctx));
  pi.on("session_shutdown", async (_event, ctx) => {
    ctx.ui.setStatus(STATUS_KEY, undefined);
  });

  pi.on("before_agent_start", async (event) => {
    if (disabledContextPaths.size === 0 && hiddenSkillPaths.size === 0) return;
    return {
      systemPrompt: filterSystemPrompt(event.systemPrompt, event.systemPromptOptions, {
        disabledContextPaths,
        hiddenSkillPaths,
      }),
    };
  });
}

export function formatContextStatus(
  options: BuildSystemPromptOptions,
  selection: {
    disabledContextPaths: ReadonlySet<string>;
    hiddenSkillPaths: ReadonlySet<string>;
  },
  cwd: string,
): string {
  const contextFiles = options.contextFiles ?? [];
  const excludedContextCount = contextFiles.filter((file) =>
    selection.disabledContextPaths.has(resourcePathId(file.path, cwd)),
  ).length;
  const skills = options.skills ?? [];
  const manualSkills = skills
    .filter(
      (skill) =>
        skill.disableModelInvocation ||
        selection.hiddenSkillPaths.has(resourcePathId(skill.filePath, cwd)),
    )
    .map((skill) => skill.name)
    .sort();

  const lines = [
    `Instructions  ${contextFiles.length - excludedContextCount} included · ${excludedContextCount} excluded`,
    `Skills        ${skills.length - manualSkills.length} visible · ${manualSkills.length} manual-only`,
  ];
  if (manualSkills.length > 0) {
    lines.push(`Manual-only   ${summarizeNames(manualSkills)}`);
  }
  return lines.join("\n");
}

export function buildSettingItems(
  options: BuildSystemPromptOptions,
  disabledContextPaths: ReadonlySet<string>,
  hiddenSkillPaths: ReadonlySet<string>,
  cwd: string,
): SettingItem[] {
  const contextItems = (options.contextFiles ?? []).map((file) =>
    contextSetting(file, disabledContextPaths, cwd),
  );
  const skillItems = (options.skills ?? []).map((skill) =>
    skillSetting(skill, hiddenSkillPaths, cwd),
  );
  return [...contextItems, ...skillItems];
}

function contextSetting(
  file: ContextFile,
  disabledPaths: ReadonlySet<string>,
  cwd: string,
): SettingItem {
  const path = resourcePathId(file.path, cwd);
  return {
    id: `context:${path}`,
    label: `${basename(path)} · ${contextScope(path, cwd)}`,
    description: path,
    currentValue: disabledPaths.has(path) ? "excluded" : "included",
    values: ["included", "excluded"],
  };
}

function skillSetting(
  skill: Skill,
  hiddenPaths: ReadonlySet<string>,
  cwd: string,
): SettingItem {
  const path = resourcePathId(skill.filePath, cwd);
  if (skill.disableModelInvocation) {
    return {
      id: `source-manual:${path}`,
      label: skill.name,
      description: `${summarize(skill.description)}\nManual-only in SKILL.md · ${path}`,
      currentValue: "manual-only (source)",
    };
  }
  return {
    id: `skill:${path}`,
    label: skill.name,
    description: `${summarize(skill.description)}\n${path}`,
    currentValue: hiddenPaths.has(path) ? "manual-only" : "visible",
    values: ["visible", "manual-only"],
  };
}

function summarize(content: string): string {
  const compact = content.replace(/\s+/g, " ").trim();
  return compact.length <= DESCRIPTION_LIMIT
    ? compact || "(no description)"
    : `${compact.slice(0, DESCRIPTION_LIMIT - 1)}…`;
}

function contextScope(path: string, cwd: string): "user" | "project" | "inherited" {
  const parent = dirname(path);
  if (parent === resourcePathId(cwd, cwd)) return "project";
  if (parent === resourcePathId(getAgentDir(), cwd)) return "user";
  return "inherited";
}

function summarizeNames(names: readonly string[]): string {
  const visible = names.slice(0, 6);
  const remaining = names.length - visible.length;
  return remaining > 0
    ? `${visible.join(", ")}, … +${remaining}`
    : visible.join(", ");
}
