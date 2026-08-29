import type {
  ExtensionAPI,
  ExtensionCommandContext,
} from "@earendil-works/pi-coding-agent";
import { DynamicBorder, getSettingsListTheme } from "@earendil-works/pi-coding-agent";
import { Container, type SettingItem, SettingsList, Text } from "@earendil-works/pi-tui";
import { applyResourceToggles } from "./prompt-filter";
import { resourcePathId, type ResourcePath } from "./resource-path";
import {
  toggleResourcesFromPrompt,
  type ToggleResource,
} from "./resources";
import {
  SkillToggleStore,
  type SkillToggleState,
  type SkillToggleStateResult,
  type SkillToggleStateStore,
} from "./state";

/** Register the skill-toggle extension with its default persistent store. */
export default function skillToggle(pi: ExtensionAPI): void {
  registerSkillToggle(pi, new SkillToggleStore());
}

/** Register the skill-toggle command and prompt handler with an injected store. */
export function registerSkillToggle(pi: ExtensionAPI, store: SkillToggleStateStore): void {
  let lastStateFailure = "";
  let lastPromptFailure = "";

  function loadState(
    resources: ReadonlyArray<ToggleResource>,
    ctx: Pick<ExtensionCommandContext, "ui">,
  ): SkillToggleState | undefined {
    const result = store.load(resources);
    if (result._tag === "ok") {
      lastStateFailure = "";
      return result.value;
    }
    reportStateFailure(result, ctx);
    return undefined;
  }

  function reportStateFailure(
    result: Extract<SkillToggleStateResult, { readonly _tag: "err" }>,
    ctx: Pick<ExtensionCommandContext, "ui">,
  ): void {
    if (result.error.message !== lastStateFailure) {
      ctx.ui.notify(`${result.error.message}\nThe prompt was left unchanged.`, "error");
    }
    lastStateFailure = result.error.message;
  }

  pi.registerCommand("skill-toggle", {
    description: "Enable or disable user-managed instructions and skills",
    handler: async (args, ctx) => {
      if (args.trim()) {
        ctx.ui.notify("Usage: /skill-toggle", "error");
        return;
      }
      if (ctx.mode !== "tui") {
        ctx.ui.notify("/skill-toggle requires TUI mode", "error");
        return;
      }

      const options = ctx.getSystemPromptOptions();
      const resources = toggleResourcesFromPrompt(options);
      if (resources.length === 0) {
        ctx.ui.notify("No user-managed instructions or skills are loaded", "info");
        return;
      }
      const loadedState = loadState(resources, ctx);
      if (!loadedState) return;
      let state: SkillToggleState = loadedState;

      const resourcesById = new Map<string, ToggleResource>(
        resources.map((resource) => [resource.id, resource]),
      );
      const items = buildSettingItems(resources, state);
      const itemsById = new Map(items.map((item) => [item.id, item]));

      await ctx.ui.custom((tui, theme, _keybindings, done) => {
        const container = new Container();
        const topBorder = new DynamicBorder((text: string) => theme.fg("accent", text));
        const bottomBorder = new DynamicBorder((text: string) => theme.fg("accent", text));
        const title = new Text("", 1, 0);
        const help = new Text("", 1, 0);
        const updateThemedText = (): void => {
          title.setText(theme.fg("accent", theme.bold("Skill Toggle")));
          help.setText(theme.fg("dim", "enter/space toggle · type to search · esc close"));
        };
        updateThemedText();
        container.addChild(topBorder);
        container.addChild(title);

        const list = new SettingsList(
          items,
          Math.min(items.length + 2, 20),
          getSettingsListTheme(),
          (id, value) => {
            const resource = resourcesById.get(id);
            const item = itemsById.get(id);
            if (!resource || !item || resource.editability === "manual-only") return;
            const previouslyEnabled = !Object.hasOwn(state.resources, id);
            if (value !== "enabled" && value !== "disabled") {
              item.currentValue = previouslyEnabled ? "enabled" : "disabled";
              ctx.ui.notify(`Unsupported toggle value: ${value}`, "error");
              return;
            }
            const result = store.setValue(resource, value, resources);
            if (result._tag === "err") {
              item.currentValue = previouslyEnabled ? "enabled" : "disabled";
              reportStateFailure(result, ctx);
              return;
            }
            state = result.value;
            lastStateFailure = "";
          },
          () => done(undefined),
          { enableSearch: true },
        );
        container.addChild(list);
        container.addChild(help);
        container.addChild(bottomBorder);

        return {
          render: (width: number) => container.render(width),
          invalidate: () => {
            updateThemedText();
            container.invalidate();
          },
          handleInput: (data: string) => {
            list.handleInput?.(data);
            tui.requestRender();
          },
        };
      });
    },
  });

  pi.on("before_agent_start", async (event, ctx) => {
    const resources = toggleResourcesFromPrompt(event.systemPromptOptions);
    const state = loadState(resources, ctx);
    if (!state) return;
    const eligiblePaths = new Set<string>(resources.map((resource) => resource.id));
    const disabledPaths = new Set<ResourcePath>(
      Object.keys(state.resources)
        .filter((path) => eligiblePaths.has(path))
        .map((path) => resourcePathId(path)),
    );
    const result = applyResourceToggles(
      event.systemPrompt,
      event.systemPromptOptions,
      disabledPaths,
    );
    const failure = result.failures.join(",");
    if (failure && failure !== lastPromptFailure) {
      ctx.ui.notify(
        `Skill toggle could not update the ${result.failures.join(" and ")} prompt section. Pi's prompt format may have changed.`,
        "error",
      );
    }
    lastPromptFailure = failure;
    return { systemPrompt: result.systemPrompt };
  });
}

function buildSettingItems(
  resources: ReadonlyArray<ToggleResource>,
  state: SkillToggleState,
): SettingItem[] {
  return resources.map((resource) => ({
    id: resource.id,
    label: `[${resource.origin}] ${resource.label}`,
    description: resource.description,
    currentValue: resource.editability === "manual-only"
      ? "manual only"
      : Object.hasOwn(state.resources, resource.id)
        ? "disabled"
        : "enabled",
    ...(resource.editability === "manual-only" ? {} : { values: ["enabled", "disabled"] }),
  }));
}
