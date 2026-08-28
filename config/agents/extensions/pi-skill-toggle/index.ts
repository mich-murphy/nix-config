import { basename } from "node:path";
import type {
  BuildSystemPromptOptions,
  ExtensionAPI,
  ExtensionContext,
} from "@earendil-works/pi-coding-agent";
import { DynamicBorder, getSettingsListTheme } from "@earendil-works/pi-coding-agent";
import { Container, SettingsList, Text, visibleWidth } from "@earendil-works/pi-tui";
import {
  SkillPolicy,
  type EffectivePolicy,
  type PersistedPolicySnapshot,
  type PolicyScope,
  type PolicyStateAdapter,
} from "./policy";
import { applyPolicyToSystemPrompt } from "./prompt-filter";
import { policyResourcesFromPrompt } from "./resources";
import {
  buildSettingItems,
  formatApplyResult,
  formatSkillStatus,
  formatPolicyPlan,
  scopeDescription,
  updateDraft,
} from "./settings";
import { SkillToggleStore } from "./state";

const STATUS_KEY = "pi-skill-toggle";
const CONTEXT_FILE_NAMES = new Set(["CLAUDE.md", "AGENTS.md"]);

// Right-aligns a single line of pre-themed text above the editor, matching
// the footer's own right-aligned model/stats line.
function rightAlignedWidget(text: string) {
  return {
    render(width: number): string[] {
      const padding = Math.max(0, width - visibleWidth(text) - 1);
      return [" ".repeat(padding) + text];
    },
    invalidate(): void {},
  };
}

export default function skillToggle(pi: ExtensionAPI) {
  registerSkillToggle(pi, new SkillToggleStore());
}

export function registerSkillToggle(pi: ExtensionAPI, store: PolicyStateAdapter): void {
  const policy = new SkillPolicy(store);
  let current: PersistedPolicySnapshot | undefined;
  let lastRefreshFailure = "";
  let lastPromptFailure = "";
  // Callers consistently resolve() with the same options reference right after
  // refresh() already resolved it once (e.g. before_agent_start); reuse that
  // result instead of recomputing policyResourcesFromPrompt/policy.resolve.
  // Safety depends on two invariants, since policy.resolve() also reads
  // SkillPolicy.session state that policy.apply() mutates in place for
  // scope=session: refresh() always produces a brand-new snapshot object, and
  // every resolve(options) is synchronously preceded by a refresh(ctx, options)
  // call with that same options reference. Do not resolve() twice around a
  // session-scoped apply() without an intervening refresh().
  let resolvedCache: { snapshot: PersistedPolicySnapshot; options: BuildSystemPromptOptions; effective: EffectivePolicy } | undefined;

  function refresh(
    ctx: ExtensionContext,
    options?: BuildSystemPromptOptions,
  ): PersistedPolicySnapshot | undefined {
    const result = policy.refresh({
      cwd: ctx.cwd,
      skills: options?.skills,
      legacyEntries: ctx.sessionManager.getBranch(),
      sessionId: ctx.sessionManager.getSessionId(),
    });
    if (!result.ok) {
      current = undefined;
      renderStatus(ctx, undefined, true);
      const failure = `${ctx.cwd}\n${result.error.message}`;
      if (failure !== lastRefreshFailure) {
        ctx.ui.notify(`Could not load skill policy: ${result.error.message}\nThe prompt is unchanged; no cached policy was applied.`, "error");
      }
      lastRefreshFailure = failure;
      return undefined;
    }
    current = result.policy;
    lastRefreshFailure = "";
    if (options) {
      renderStatus(ctx, resolve(options));
    }
    return current;
  }

  function resolve(options: BuildSystemPromptOptions): EffectivePolicy | undefined {
    if (!current) return undefined;
    if (resolvedCache && resolvedCache.snapshot === current && resolvedCache.options === options) {
      return resolvedCache.effective;
    }
    const effective = policy.resolve(current, policyResourcesFromPrompt(options));
    resolvedCache = { snapshot: current, options, effective };
    return effective;
  }

  function renderStatus(
    ctx: ExtensionContext,
    effective?: EffectivePolicy,
    failed = false,
  ): void {
    if (failed) {
      ctx.ui.setWidget(
        STATUS_KEY,
        (_tui, theme) => rightAlignedWidget(theme.fg("error", "skills !")),
        { placement: "aboveEditor" },
      );
      return;
    }
    if (!effective) {
      ctx.ui.setWidget(STATUS_KEY, undefined);
      return;
    }
    const contextFile = effective.instructions.find(
      (item) => item.visibility === "included" && CONTEXT_FILE_NAMES.has(basename(item.path)),
    );
    const loadedSkills = effective.skills.filter((item) => item.visibility === "visible").length;
    ctx.ui.setWidget(
      STATUS_KEY,
      (_tui, theme) => {
        const skillSummary = `skills ${loadedSkills}`;
        const status = contextFile
          ? `${basename(contextFile.path)} • ${skillSummary}`
          : skillSummary;
        return rightAlignedWidget(theme.fg("dim", status));
      },
      { placement: "aboveEditor" },
    );
  }

  pi.registerCommand("skill-toggle", {
    description: "Configure global, directory, or session skill policy",
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
      const snapshot = refresh(ctx, options);
      if (!snapshot) return;
      const effective = resolve(options);
      if (!effective) return;

      const selected = await ctx.ui.select("Skill policy scope", ["Global", "Directory", "Session"]);
      if (!selected) return;
      const scope = selected.toLowerCase() as PolicyScope;
      const draft = policy.draft(scope, effective, snapshot);
      const items = buildSettingItems(effective, draft);
      if (items.length === 0) {
        ctx.ui.notify(`No resources can be configured in ${scope} scope`, "info");
        return;
      }

      await ctx.ui.custom((tui, theme, _keybindings, done) => {
        const container = new Container();
        container.addChild(new DynamicBorder((text: string) => theme.fg("accent", text)));
        container.addChild(new Text(theme.fg("accent", theme.bold(`Skill Toggle · ${selected}`)), 1, 0));
        container.addChild(new Text(theme.fg("muted", scopeDescription(scope)), 1, 0));
        const list = new SettingsList(
          items,
          Math.min(items.length + 2, 20),
          getSettingsListTheme(),
          (id, value) => updateDraft(draft, effective, id, value),
          () => done(undefined),
          { enableSearch: true },
        );
        container.addChild(list);
        container.addChild(new Text(theme.fg("dim", "Close to review staged transitions · bulk rows affect all loaded matches"), 1, 0));
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

      const plan = policy.plan(draft, snapshot);
      if (plan.changes.length === 0) return;
      if (!(await ctx.ui.confirm("Apply skill policy plan?", formatPolicyPlan(plan)))) {
        ctx.ui.notify("Skill policy changes discarded", "info");
        return;
      }

      const result = policy.apply(plan);
      if (result.snapshot) current = result.snapshot;
      const nextEffective = current ? policy.resolve(current, policyResourcesFromPrompt(options)) : undefined;
      renderStatus(ctx, nextEffective, result.errors.length > 0);
      ctx.ui.notify(formatApplyResult(result), result.errors.length > 0 ? "error" : "info");
    },
  });

  pi.registerCommand("skill-status", {
    description: "Show effective skill policy and its resolution sources",
    handler: async (_args, ctx) => {
      const options = ctx.getSystemPromptOptions();
      const snapshot = refresh(ctx, options);
      if (!snapshot) return;
      const effective = resolve(options)!;
      renderStatus(ctx, effective);
      ctx.ui.notify(formatSkillStatus(effective, snapshot, policy.sessionOverrides()), "info");
    },
  });

  pi.registerCommand("skill-reset", {
    description: "Reset the skill policy for this directory",
    handler: async (args, ctx) => {
      if (args.trim()) {
        ctx.ui.notify("Usage: /skill-reset", "error");
        return;
      }
      if (ctx.hasUI && !(await ctx.ui.confirm("Reset skill policy?", "Reset directory policy?"))) return;
      const result = policy.reset("directory", ctx.cwd);
      current = result.snapshot;
      const options = ctx.getSystemPromptOptions();
      if (!current) current = refresh(ctx, options);
      const effective = resolve(options);
      renderStatus(ctx, effective, result.errors.length > 0);
      ctx.ui.notify(formatApplyResult(result), result.errors.length > 0 ? "error" : "info");
    },
  });

  pi.on("session_start", async (_event, ctx) => {
    policy.clearSession();
    current = undefined;
    lastRefreshFailure = "";
    lastPromptFailure = "";
    refresh(ctx);
  });
  pi.on("session_tree", async (_event, ctx) => {
    current = undefined;
    refresh(ctx);
  });
  pi.on("session_shutdown", async (_event, ctx) => {
    policy.clearSession();
    current = undefined;
    lastRefreshFailure = "";
    lastPromptFailure = "";
    ctx.ui.setWidget(STATUS_KEY, undefined);
  });

  pi.on("before_agent_start", async (event, ctx) => {
    const snapshot = refresh(ctx, event.systemPromptOptions);
    if (!snapshot) return;
    const effective = resolve(event.systemPromptOptions)!;
    const result = applyPolicyToSystemPrompt(event.systemPrompt, event.systemPromptOptions, effective);
    const failure = result.failures.join(",");
    renderStatus(ctx, effective, failure.length > 0);
    if (failure && failure !== lastPromptFailure) {
      ctx.ui.notify(
        `Skill policy could not be applied to: ${result.failures.join(", ")}. Pi's prompt format may have changed.`,
        "error",
      );
    }
    lastPromptFailure = failure;
    return { systemPrompt: result.systemPrompt };
  });
}
