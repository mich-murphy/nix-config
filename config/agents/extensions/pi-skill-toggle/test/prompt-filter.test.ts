import { describe, expect, test } from "bun:test";
import {
  formatSkillsForPrompt,
  type BuildSystemPromptOptions,
  type Skill,
} from "@earendil-works/pi-coding-agent";
import { applyResourceToggles } from "../prompt-filter";
import { resourcePathId } from "../resource-path";

function resourcePaths(...paths: string[]) {
  return new Set(paths.map((path) => resourcePathId(path)));
}

function renderProjectContext(contextFiles: ReadonlyArray<{ path: string; content: string }>): string {
  if (contextFiles.length === 0) return "";
  const instructions = contextFiles.map(({ path, content }) =>
    `<project_instructions path="${path}">\n${content}\n</project_instructions>\n\n`,
  ).join("");
  return `\n\n<project_context>\n\nProject-specific instructions and guidelines:\n\n${instructions}</project_context>\n`;
}

function skill(name: string, filePath: string): Skill {
  return {
    name,
    description: `${name} at ${filePath}`,
    filePath,
    baseDir: filePath.replace(/\/SKILL\.md$/, ""),
    sourceInfo: {
      path: filePath,
      source: "local",
      scope: "project",
      origin: "top-level",
    },
    disableModelInvocation: false,
  };
}

describe("applyResourceToggles", () => {
  test("removes resources by path without affecting same-named resources", () => {
    const first = skill("deploy", "/work/client-a/.agents/skills/deploy/SKILL.md");
    const second = skill("deploy", "/work/client-b/.agents/skills/deploy/SKILL.md");
    const contextFiles = [
      { path: "/work/client-a/AGENTS.md", content: "client a" },
      { path: "/work/client-b/AGENTS.md", content: "client b" },
    ];
    const options: BuildSystemPromptOptions = {
      cwd: "/work",
      selectedTools: ["read"],
      contextFiles,
      skills: [first, second],
    };
    const prompt = `base${renderProjectContext(contextFiles)}${formatSkillsForPrompt([first, second])}`;

    const result = applyResourceToggles(
      prompt,
      options,
      resourcePaths(first.filePath, contextFiles[0]?.path ?? ""),
    );

    expect(result.failures).toEqual([]);
    expect(result.systemPrompt).not.toContain("client a");
    expect(result.systemPrompt).toContain("client b");
    expect(result.systemPrompt).not.toContain(first.description);
    expect(result.systemPrompt).toContain(second.description);
  });

  test("reports section-specific prompt drift only when a replacement is required", () => {
    const deploy = skill("deploy", "/work/project/.agents/skills/deploy/SKILL.md");
    const options: BuildSystemPromptOptions = {
      cwd: "/work/project",
      contextFiles: [{ path: "/work/project/AGENTS.md", content: "rules" }],
      skills: [deploy],
    };

    const result = applyResourceToggles(
      "incompatible prompt",
      options,
      resourcePaths("/work/project/AGENTS.md", deploy.filePath),
    );

    expect(result).toEqual({
      systemPrompt: "incompatible prompt",
      failures: ["instructions", "skills"],
    });
  });

  test("does not expect a skill section when read is inactive", () => {
    const deploy = skill("deploy", "/work/project/.agents/skills/deploy/SKILL.md");
    const options: BuildSystemPromptOptions = {
      cwd: "/work/project",
      selectedTools: ["bash"],
      skills: [deploy],
    };

    expect(applyResourceToggles("base", options, resourcePaths(deploy.filePath))).toEqual({
      systemPrompt: "base",
      failures: [],
    });
  });
});
