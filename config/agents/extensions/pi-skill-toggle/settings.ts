import { basename } from "node:path";
import type { SettingItem } from "@earendil-works/pi-tui";
import type {
  ApplyResult,
  EffectivePolicy,
  PersistedPolicySnapshot,
  PolicyDraft,
  PolicyPlan,
  PolicyScope,
  SessionPolicy,
} from "./policy";

const DESCRIPTION_LIMIT = 180;

export function buildSettingItems(
  effective: EffectivePolicy,
  draft: PolicyDraft,
): SettingItem[] {
  const items: SettingItem[] = [];
  if (draft.scope !== "global") {
    for (const instruction of effective.instructions) {
      items.push({
        id: `instruction:${instruction.path}`,
        label: `${basename(instruction.path)} · ${instruction.provenance.scope}`,
        description: [
          `effective ${instruction.visibility} from ${instruction.resolvedFrom}`,
          provenanceText(instruction.provenance),
          instruction.path,
        ].join("\n"),
        currentValue: draft.instructions[instruction.path] ?? "inherit",
        values: ["inherit", "included", "excluded"],
      });
    }
  }

  const loadedNames = new Set<string>();
  for (const skill of effective.skills) {
    loadedNames.add(skill.name);
    if (skill.sourceLocked) {
      items.push({
        id: `source-manual:${skill.name}`,
        label: skill.name,
        description: [
          `${summarize(skill.description)} · effective manual-only from source`,
          provenanceText(skill.provenance),
          skill.filePath,
        ].join("\n"),
        currentValue: "manual-only (source)",
      });
      continue;
    }
    items.push({
      id: `skill:${skill.name}`,
      label: skill.name,
      description: [
        `${summarize(skill.description)} · effective ${skill.visibility} from ${skill.resolvedFrom}`,
        provenanceText(skill.provenance),
        skill.filePath,
      ].join("\n"),
      currentValue: draft.skills[skill.name] ?? (draft.scope === "global" ? "visible" : "inherit"),
      values: draft.scope === "global"
        ? ["visible", "manual-only"]
        : ["inherit", "visible", "manual-only"],
    });
  }

  for (const name of Object.keys(draft.skills).filter((name) => !loadedNames.has(name)).sort()) {
    items.push({
      id: `skill:${name}`,
      label: `${name} · not loaded`,
      description: "Policy exists for this skill name, but Pi did not load it in this directory",
      currentValue: draft.skills[name]!,
      values: draft.scope === "global"
        ? ["visible", "manual-only"]
        : ["inherit", "visible", "manual-only"],
    });
  }

  if (effective.skills.some((skill) => !skill.sourceLocked)) {
    items.push({
      id: "bulk:skills",
      label: "All editable skills",
      description: "Bulk operation for all editable skill rows in this scope",
      currentValue: "no change",
      values: draft.scope === "global"
        ? ["no change", "visible", "manual-only"]
        : ["no change", "inherit", "visible", "manual-only"],
    });
  }
  if (draft.scope !== "global" && effective.instructions.length > 0) {
    items.push({
      id: "bulk:instructions",
      label: "All instructions",
      description: "Bulk operation for all instruction rows in this scope",
      currentValue: "no change",
      values: ["no change", "inherit", "included", "excluded"],
    });
  }
  if (items.length > 0) {
    items.push({
      id: "bulk:reset",
      label: `Reset ${draft.scope} draft`,
      description: "Reset every editable resource in the selected scope",
      currentValue: "no change",
      values: ["no change", "reset"],
    });
  }
  return items;
}

export function updateDraft(
  draft: PolicyDraft,
  effective: EffectivePolicy,
  id: string,
  value: string,
): void {
  if (id.startsWith("instruction:")) {
    draft.instructions[id.slice("instruction:".length)] = value as PolicyDraft["instructions"][string];
  } else if (id.startsWith("skill:")) {
    draft.skills[id.slice("skill:".length)] = value as PolicyDraft["skills"][string];
  } else if (id === "bulk:skills" && value !== "no change") {
    for (const skill of effective.skills) if (!skill.sourceLocked) draft.skills[skill.name] = value as PolicyDraft["skills"][string];
  } else if (id === "bulk:instructions" && value !== "no change") {
    for (const instruction of effective.instructions) draft.instructions[instruction.path] = value as PolicyDraft["instructions"][string];
  } else if (id === "bulk:reset" && value === "reset") {
    const skillValue = draft.scope === "global" ? "visible" : "inherit";
    for (const name of Object.keys(draft.skills)) draft.skills[name] = skillValue;
    for (const path of Object.keys(draft.instructions)) draft.instructions[path] = "inherit";
  }
}

export function formatPolicyPlan(plan: PolicyPlan): string {
  return plan.changes.map((change) =>
    `${displayId(change.id).padEnd(20)} ${change.scope}: ${change.before} -> ${change.after}`,
  ).join("\n");
}

export function formatApplyResult(result: ApplyResult): string {
  const parts = [`Applied ${result.applied.length}`];
  if (result.skipped.length > 0) parts.push(`skipped ${result.skipped.length}`);
  if (result.errors.length > 0) parts.push(`errors ${result.errors.length}`);
  return parts.join(" · ");
}

export function formatSkillStatus(
  effective: EffectivePolicy,
  snapshot: PersistedPolicySnapshot,
  session: SessionPolicy,
): string {
  const included = effective.instructions.filter((item) => item.visibility === "included").length;
  const visible = effective.skills.filter((item) => item.visibility === "visible").length;
  const resolutionCounts = new Map<string, number>();
  for (const item of [...effective.instructions, ...effective.skills]) {
    resolutionCounts.set(item.resolvedFrom, (resolutionCounts.get(item.resolvedFrom) ?? 0) + 1);
  }
  const directoryOverrides = Object.keys(snapshot.directoryInstructions).length + Object.keys(snapshot.directorySkills).length;
  const sessionOverrides = Object.keys(session.instructions).length + Object.keys(session.skills).length;
  const loadedNames = new Set(effective.skills.map((skill) => skill.name));
  const hiddenElsewhere = [...new Set([
    ...Object.entries(snapshot.globalSkills).filter(([, value]) => value === "manual-only").map(([name]) => name),
    ...Object.entries(snapshot.directorySkills).filter(([, value]) => value === "manual-only").map(([name]) => name),
    ...Object.entries(session.skills).filter(([, value]) => value === "manual-only").map(([name]) => name),
  ])].filter((name) => !loadedNames.has(name)).sort();

  const resolved = ["source", "session", "directory", "global", "default"]
    .filter((source) => resolutionCounts.has(source))
    .map((source) => `${resolutionCounts.get(source)} ${source}`)
    .join(" · ");
  const lines = [
    `Directory     ${effective.cwd}`,
    `Instructions  ${included} included · ${effective.instructions.length - included} excluded`,
    `Skills        ${visible} visible · ${effective.skills.length - visible} manual-only`,
    `Resolved      ${resolved || "none"}`,
    `Overrides     directory ${directoryOverrides} · session ${sessionOverrides}`,
  ];
  if (hiddenElsewhere.length > 0) lines.push(`Not loaded    ${summarizeNames(hiddenElsewhere)}`);
  return lines.join("\n");
}

export function scopeDescription(scope: PolicyScope): string {
  if (scope === "global") return "Skills only · applies in every directory";
  if (scope === "directory") return "Persistent instruction and skill overrides for this directory";
  return "Temporary overrides · cleared on session replacement, reload, and restart";
}

function provenanceText(provenance: { scope: string; origin: string; source: string }): string {
  return `${provenance.scope} · ${provenance.origin} · ${provenance.source}`;
}

function displayId(id: string): string {
  return id.includes("/") ? basename(id) : id;
}

function summarize(content: string): string {
  const compact = content.replace(/\s+/g, " ").trim();
  return compact.length <= DESCRIPTION_LIMIT ? compact || "(no description)" : `${compact.slice(0, DESCRIPTION_LIMIT - 1)}…`;
}

function summarizeNames(names: readonly string[]): string {
  const visible = [...names].sort().slice(0, 6);
  const remaining = names.length - visible.length;
  return remaining > 0 ? `${visible.join(", ")}, … +${remaining}` : visible.join(", ");
}
