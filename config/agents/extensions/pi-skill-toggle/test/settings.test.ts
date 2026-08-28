import { describe, expect, test } from "bun:test";
import { SkillPolicy, resolveEffectivePolicy, type PersistedPolicySnapshot, type PolicyResources } from "../policy";
import {
  buildSettingItems,
  formatSkillStatus,
  formatPolicyPlan,
  updateDraft,
} from "../settings";

const snapshot: PersistedPolicySnapshot = {
  cwd: "/work/project",
  generation: "g",
  globalSkills: { research: "manual-only", unloaded: "manual-only" },
  directorySkills: { research: "visible" },
  directoryInstructions: { "/work/project/AGENTS.md": "excluded" },
};
const resources: PolicyResources = {
  instructions: [{
    kind: "instruction",
    path: "/work/project/AGENTS.md",
    provenance: { path: "/work/project/AGENTS.md", scope: "project", origin: "top-level", source: "project-context" },
  }],
  skills: [
    {
      kind: "skill",
      name: "research",
      description: "Research primary sources",
      filePath: "/skills/research/SKILL.md",
      provenance: { path: "/skills/research/SKILL.md", scope: "user", origin: "package", source: "test-package" },
      sourceManualOnly: false,
    },
    {
      kind: "skill",
      name: "deploy",
      description: "Deploy",
      filePath: "/skills/deploy/SKILL.md",
      provenance: { path: "/skills/deploy/SKILL.md", scope: "user", origin: "top-level", source: "test" },
      sourceManualOnly: true,
    },
  ],
};
const adapter = {
  load: () => ({ _tag: "ok" as const, value: snapshot }),
  apply: () => ({ applied: [], skipped: [], errors: [] }),
  reset: () => ({ applied: [], skipped: [], errors: [] }),
};

describe("scope-aware settings", () => {
  test("shows effective value, selected-scope value, canonical provenance, path, and locks", () => {
    const effective = resolveEffectivePolicy(snapshot, { skills: {}, instructions: {} }, resources);
    const policy = new SkillPolicy(adapter);
    const draft = policy.draft("directory", effective, snapshot);
    const items = buildSettingItems(effective, draft);

    expect(items[0]).toMatchObject({ label: "AGENTS.md · project", currentValue: "excluded", values: ["inherit", "included", "excluded"] });
    expect(items[0]?.description).toContain("effective excluded from directory");
    expect(items[1]?.description).toContain("user · package · test-package");
    expect(items[1]?.description).toContain("/skills/research/SKILL.md");
    expect(items[2]).toMatchObject({ label: "deploy", currentValue: "manual-only (source)" });
    expect(items[2]?.values).toBeUndefined();
  });

  test("keeps unloaded policy names manageable", () => {
    const effective = resolveEffectivePolicy(snapshot, { skills: {}, instructions: {} }, resources);
    const policy = new SkillPolicy(adapter);
    const draft = policy.draft("global", effective, snapshot);
    const item = buildSettingItems(effective, draft).find(({ id }) => id === "skill:unloaded");
    expect(item).toMatchObject({ label: "unloaded · not loaded", currentValue: "manual-only" });
  });

  test("bulk operations stage through the same draft and plan path", () => {
    const effective = resolveEffectivePolicy(snapshot, { skills: {}, instructions: {} }, resources);
    const policy = new SkillPolicy(adapter);
    const draft = policy.draft("directory", effective, snapshot);
    updateDraft(draft, effective, "bulk:skills", "manual-only");
    updateDraft(draft, effective, "bulk:instructions", "included");
    const plan = policy.plan(draft, snapshot);
    expect(plan.changes).toEqual([
      { scope: "directory", kind: "skill", id: "research", before: "visible", after: "manual-only" },
      { scope: "directory", kind: "instruction", id: "/work/project/AGENTS.md", before: "excluded", after: "included" },
    ]);
    expect(formatPolicyPlan(plan)).toContain("research             directory: visible -> manual-only");
  });

  test("rejects invalid settings values without mutating the draft", () => {
    const effective = resolveEffectivePolicy(snapshot, { skills: {}, instructions: {} }, resources);
    const policy = new SkillPolicy(adapter);
    const draft = policy.draft("global", effective, snapshot);
    const before = { ...draft.skills };

    const result = updateDraft(draft, effective, "skill:research", "excluded");

    expect(result).toMatchObject({ _tag: "err" });
    expect(draft.skills).toEqual(before);
  });

  test("formats resolution and persistent versus temporary override counts", () => {
    const session = { skills: { research: "manual-only" as const }, instructions: {} };
    const effective = resolveEffectivePolicy(snapshot, session, resources);
    expect(formatSkillStatus(effective, snapshot, session)).toBe(
      "Directory     /work/project\n" +
      "Instructions  0 included · 1 excluded\n" +
      "Skills        0 visible · 2 manual-only\n" +
      "Resolved      1 source · 1 session · 1 directory\n" +
      "Overrides     directory 2 · session 1\n" +
      "Not loaded    unloaded",
    );
  });
});
