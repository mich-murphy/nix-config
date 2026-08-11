import { describe, expect, test } from "bun:test";
import type { BuildSystemPromptOptions, Skill } from "@earendil-works/pi-coding-agent";
import { buildSettingItems, formatContextStatus } from "../index";

function skill(name: string, path: string, sourceManual = false): Skill {
  return {
    name,
    description: `${name} description`,
    filePath: path,
    baseDir: path.replace(/\/SKILL\.md$/, ""),
    sourceInfo: { path, source: "test", scope: "user", origin: "top-level" },
    disableModelInvocation: sourceManual,
  };
}

describe("buildSettingItems", () => {
  test("shows branch selections and preserves source-manual skill policy", () => {
    const options: BuildSystemPromptOptions = {
      cwd: "/work/project",
      contextFiles: [{ path: "/work/project/AGENTS.md", content: "Project rules" }],
      skills: [
        skill("research", "/skills/research/SKILL.md"),
        skill("deploy", "/skills/deploy/SKILL.md", true),
      ],
    };

    const items = buildSettingItems(
      options,
      new Set(["/work/project/AGENTS.md"]),
      new Set(["/skills/research/SKILL.md"]),
      "/work/project",
    );

    expect(items[0]?.description).toBe("/work/project/AGENTS.md");
    expect(items.map(({ label, currentValue, values }) => ({ label, currentValue, values }))).toEqual([
      {
        label: "AGENTS.md · project",
        currentValue: "excluded",
        values: ["included", "excluded"],
      },
      {
        label: "research",
        currentValue: "manual-only",
        values: ["visible", "manual-only"],
      },
      {
        label: "deploy",
        currentValue: "manual-only (source)",
        values: undefined,
      },
    ]);
  });

  test("summarizes status with resource names instead of full paths", () => {
    const skills = [
      "bro",
      "code-review",
      "deploy",
      "hunk-review",
      "neo",
      "neo-architecture",
      "neo-delivery",
      "research",
    ].map((name) => skill(name, `/very/long/config/agents/skills/${name}/SKILL.md`));
    const options: BuildSystemPromptOptions = {
      cwd: "/work/project",
      contextFiles: [
        { path: "/global/AGENTS.md", content: "Global" },
        { path: "/work/project/AGENTS.md", content: "Project" },
      ],
      skills,
    };

    const status = formatContextStatus(
      options,
      {
        disabledContextPaths: new Set(["/global/AGENTS.md"]),
        hiddenSkillPaths: new Set(skills.map(({ filePath }) => filePath)),
      },
      "/work/project",
    );

    expect(status).toBe(
      "Instructions  1 included · 1 excluded\n" +
        "Skills        0 visible · 8 manual-only\n" +
        "Manual-only   bro, code-review, deploy, hunk-review, neo, neo-architecture, … +2",
    );
    expect(status).not.toContain("/very/long/");
  });
});
