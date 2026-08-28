import { describe, expect, test } from "bun:test";
import {
  PolicyStateError,
  SkillPolicy,
  resolveEffectivePolicy,
  type PersistedPolicySnapshot,
  type PolicyResources,
  type SessionPolicy,
} from "../policy";

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
      description: "Research",
      filePath: "/skills/research/SKILL.md",
      provenance: { path: "/skills/research/SKILL.md", scope: "user", origin: "top-level", source: "test" },
      sourceManualOnly: false,
    },
    {
      kind: "skill",
      name: "locked",
      description: "Locked",
      filePath: "/skills/locked/SKILL.md",
      provenance: { path: "/skills/locked/SKILL.md", scope: "user", origin: "top-level", source: "test" },
      sourceManualOnly: true,
    },
  ],
};

function snapshot(overrides: Partial<PersistedPolicySnapshot> = {}): PersistedPolicySnapshot {
  return {
    cwd: "/work/project",
    generation: "generation",
    globalSkills: {},
    directorySkills: {},
    directoryInstructions: {},
    ...overrides,
  };
}

const noSession: SessionPolicy = { skills: {}, instructions: {} };

describe("effective policy resolution", () => {
  test.each([
    ["default", snapshot(), noSession, "visible", "default"],
    ["global", snapshot({ globalSkills: { research: "manual-only" } }), noSession, "manual-only", "global"],
    ["directory", snapshot({ globalSkills: { research: "manual-only" }, directorySkills: { research: "visible" } }), noSession, "visible", "directory"],
    ["session", snapshot({ directorySkills: { research: "manual-only" } }), { skills: { research: "visible" }, instructions: {} }, "visible", "session"],
  ] as const)("resolves %s skill precedence", (_name, stored, session, visibility, source) => {
    const skill = resolveEffectivePolicy(stored, session, resources).skills[0];
    expect(skill).toBeDefined();
    if (!skill) throw new Error("Expected research skill fixture");
    expect([skill.visibility, skill.resolvedFrom]).toEqual([visibility, source]);
  });

  test.each([
    [snapshot(), noSession, "included", "default"],
    [snapshot({ directoryInstructions: { "/work/project/AGENTS.md": "excluded" } }), noSession, "excluded", "directory"],
    [snapshot({ directoryInstructions: { "/work/project/AGENTS.md": "excluded" } }), { skills: {}, instructions: { "/work/project/AGENTS.md": "included" } }, "included", "session"],
  ] as const)("resolves instruction precedence", (stored, session, visibility, source) => {
    const instruction = resolveEffectivePolicy(stored, session, resources).instructions[0];
    expect(instruction).toBeDefined();
    if (!instruction) throw new Error("Expected instruction fixture");
    expect([instruction.visibility, instruction.resolvedFrom]).toEqual([visibility, source]);
  });

  test("source-manual policy wins over every override and cannot be planned", () => {
    const stored = snapshot({
      globalSkills: { locked: "visible" },
      directorySkills: { locked: "visible" },
    });
    const session = { skills: { locked: "visible" as const }, instructions: {} };
    const effective = resolveEffectivePolicy(stored, session, resources);
    expect(effective.skills[1]).toMatchObject({ visibility: "manual-only", resolvedFrom: "source", sourceLocked: true });

    const adapter = {
      load: () => ({ _tag: "ok" as const, value: stored }),
      apply: () => ({ applied: [], skipped: [], errors: [] }),
      reset: () => ({ applied: [], skipped: [], errors: [] }),
    };
    const policy = new SkillPolicy(adapter);
    for (const scope of ["global", "directory", "session"] as const) {
      const draft = policy.draft(scope, effective, stored);
      draft.skills.locked = "visible";
      expect(policy.plan(draft, stored).changes.filter((change) => change.id === "locked")).toEqual([]);
    }
  });

  test("session overrides are in memory and clear on replacement", () => {
    const stored = snapshot();
    const policy = new SkillPolicy({
      load: () => ({ _tag: "ok" as const, value: stored }),
      apply: () => ({ applied: [], skipped: [], errors: [] }),
      reset: () => ({ applied: [], skipped: [], errors: [] }),
    });
    const effective = policy.resolve(stored, resources);
    const draft = policy.draft("session", effective, stored);
    draft.skills.research = "manual-only";
    policy.apply(policy.plan(draft, stored));
    expect(policy.resolve(stored, resources).skills[0]?.resolvedFrom).toBe("session");
    policy.clearSession();
    expect(policy.resolve(stored, resources).skills[0]?.resolvedFrom).toBe("default");
  });

  test("refresh failure has no policy payload that can cross directories", () => {
    const good = snapshot();
    let fail = false;
    const policy = new SkillPolicy({
      load: (input) => fail
        ? {
            _tag: "err" as const,
            error: new PolicyStateError("load", `bad state for ${input.cwd}`),
          }
        : { _tag: "ok" as const, value: good },
      apply: () => ({ applied: [], skipped: [], errors: [] }),
      reset: () => ({ applied: [], skipped: [], errors: [] }),
    });
    expect(policy.refresh({ cwd: "/work/project" })._tag).toBe("ok");
    fail = true;
    expect(policy.refresh({ cwd: "/other/project" })).toEqual({
      _tag: "err",
      error: new PolicyStateError("load", "bad state for /other/project"),
    });
  });
});
