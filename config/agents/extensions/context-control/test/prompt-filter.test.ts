import { describe, expect, test } from "bun:test";
import { formatSkillsForPrompt, type BuildSystemPromptOptions, type Skill } from "@earendil-works/pi-coding-agent";
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
    sourceInfo: { path: "/home/me/.agents/skills/research/SKILL.md", source: "test", scope: "user", origin: "top-level" },
    disableModelInvocation: false,
  },
  {
    name: "deploy",
    description: "Deploy software",
    filePath: "/home/me/.agents/skills/deploy/SKILL.md",
    baseDir: "/home/me/.agents/skills/deploy",
    sourceInfo: { path: "/home/me/.agents/skills/deploy/SKILL.md", source: "test", scope: "user", origin: "top-level" },
    disableModelInvocation: false,
  },
];

function renderContext(files: typeof contextFiles): string {
  if (files.length === 0) return "";
  return `\n\n<project_context>\n\nProject-specific instructions and guidelines:\n\n${files
    .map(({ path, content }) => `<project_instructions path="${path}">\n${content}\n</project_instructions>\n\n`)
    .join("")}</project_context>\n`;
}

describe("filterSystemPrompt", () => {
  test("removes disabled instruction files and hidden skills while preserving the rest of the prompt", () => {
    const options: BuildSystemPromptOptions = {
      cwd: "/work/project",
      selectedTools: ["read"],
      contextFiles,
      skills,
    };
    const prompt = `base instructions${renderContext(contextFiles)}${formatSkillsForPrompt(skills)}\nCurrent working directory: /work/project`;

    const filtered = filterSystemPrompt(prompt, options, {
      disabledContextPaths: new Set([contextFiles[0]!.path]),
      hiddenSkillPaths: new Set([skills[1]!.filePath]),
    });

    expect(filtered).not.toContain("Always be careful.");
    expect(filtered).toContain("Run project tests.");
    expect(filtered).toContain("research");
    expect(filtered).not.toContain("Deploy software");
    expect(filtered).toEndWith("Current working directory: /work/project");
  });
});
