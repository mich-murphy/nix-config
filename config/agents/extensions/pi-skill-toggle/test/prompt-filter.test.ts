import { describe, expect, test } from "bun:test";
import {
  formatSkillsForPrompt,
  type BuildSystemPromptOptions,
  type Skill,
} from "@earendil-works/pi-coding-agent";
import { filterSystemPrompt } from "../prompt-filter";

const contextFiles = [
  { path: "/home/me/.pi/agent/AGENTS.md", content: "Always be careful." },
  { path: "/work/project/AGENTS.md", content: "Run project tests." },
];

const skills: Skill[] = [
  {
    name: "research",
    description: "Research primary sources",
    filePath: "/home/me/.agents/skills/research/SKILL.md",
    baseDir: "/home/me/.agents/skills/research",
    sourceInfo: {
      path: "/home/me/.agents/skills/research/SKILL.md",
      source: "test",
      scope: "user",
      origin: "top-level",
    },
    disableModelInvocation: false,
  },
  {
    name: "deploy",
    description: "Deploy software",
    filePath: "/home/me/.agents/skills/deploy/SKILL.md",
    baseDir: "/home/me/.agents/skills/deploy",
    sourceInfo: {
      path: "/home/me/.agents/skills/deploy/SKILL.md",
      source: "test",
      scope: "user",
      origin: "top-level",
    },
    disableModelInvocation: false,
  },
];

function renderContext(files: typeof contextFiles): string {
  if (files.length === 0) return "";
  return `\n\n<project_context>\n\nProject-specific instructions and guidelines:\n\n${files
    .map(
      ({ path, content }) =>
        `<project_instructions path="${path}">\n${content}\n</project_instructions>\n\n`,
    )
    .join("")}</project_context>\n`;
}

describe("filterSystemPrompt", () => {
  test("removes directory context and globally hidden skill names", () => {
    const options: BuildSystemPromptOptions = {
      cwd: "/work/project",
      selectedTools: ["read"],
      contextFiles,
      skills,
    };
    const prompt = `base instructions${renderContext(contextFiles)}${formatSkillsForPrompt(skills)}\nCurrent working directory: /work/project`;

    const result = filterSystemPrompt(prompt, options, {
      disabledContextPaths: new Set([contextFiles[0]!.path]),
      hiddenSkillNames: new Set(["deploy"]),
    });

    expect(result.failures).toEqual([]);
    expect(result.systemPrompt).not.toContain("Always be careful.");
    expect(result.systemPrompt).toContain("Run project tests.");
    expect(result.systemPrompt).toContain("research");
    expect(result.systemPrompt).not.toContain("Deploy software");
    expect(result.systemPrompt).toEndWith(
      "Current working directory: /work/project",
    );
  });

  test("reports prompt-format drift instead of silently claiming success", () => {
    const options: BuildSystemPromptOptions = {
      cwd: "/work/project",
      contextFiles,
      skills,
    };

    const result = filterSystemPrompt("an incompatible prompt", options, {
      disabledContextPaths: new Set([contextFiles[0]!.path]),
      hiddenSkillNames: new Set(["deploy"]),
    });

    expect(result.systemPrompt).toBe("an incompatible prompt");
    expect(result.failures).toEqual(["instructions", "skills"]);
  });

  test("reports section-specific drift and preserves unrelated earlier prompt edits", () => {
    const options: BuildSystemPromptOptions = {
      cwd: "/work/project",
      contextFiles,
      skills,
    };
    const skillSection = formatSkillsForPrompt(skills);
    const prompt = `earlier extension text\n${skillSection}`;
    const result = filterSystemPrompt(prompt, options, {
      disabledContextPaths: new Set([contextFiles[0]!.path]),
      hiddenSkillNames: new Set(["deploy"]),
    });
    expect(result.failures).toEqual(["instructions"]);
    expect(result.systemPrompt).toStartWith("earlier extension text\n");
    expect(result.systemPrompt).not.toContain("Deploy software");
  });

  test("reports only skill drift when the instruction section still matches", () => {
    const options: BuildSystemPromptOptions = { cwd: "/work/project", contextFiles, skills };
    const prompt = `prefix${renderContext(contextFiles)}incompatible skills`;
    const result = filterSystemPrompt(prompt, options, {
      disabledContextPaths: new Set([contextFiles[0]!.path]),
      hiddenSkillNames: new Set(["deploy"]),
    });
    expect(result.failures).toEqual(["skills"]);
    expect(result.systemPrompt).not.toContain("Always be careful.");
  });
});
