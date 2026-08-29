import { homedir } from "node:os";
import { join } from "node:path";
import { describe, expect, test } from "bun:test";
import {
  getAgentDir,
  type BuildSystemPromptOptions,
  type Skill,
} from "@earendil-works/pi-coding-agent";
import { resourcePathId } from "../resource-path";
import { toggleResourcesFromPrompt } from "../resources";

function skill(
  name: string,
  filePath: string,
  sourceInfo: Skill["sourceInfo"],
  disableModelInvocation = false,
): Skill {
  return {
    name,
    description: `${name} description`,
    filePath,
    baseDir: join(filePath, ".."),
    sourceInfo,
    disableModelInvocation,
  };
}

describe("toggleResourcesFromPrompt", () => {
  test("groups global resources before project resources and sorts skills by name", () => {
    const options: BuildSystemPromptOptions = {
      cwd: "/work/project/src",
      contextFiles: [
        { path: join(getAgentDir(), "AGENTS.md"), content: "global" },
        { path: "/work/AGENTS.md", content: "parent" },
        { path: "/work/project/CLAUDE.md", content: "project" },
      ],
      skills: [
        skill("zeta", join(homedir(), ".agents/skills/zeta/SKILL.md"), {
          path: join(homedir(), ".agents/skills/zeta/SKILL.md"),
          source: "local",
          scope: "user",
          origin: "top-level",
        }),
        skill("alpha", join(getAgentDir(), "skills/alpha/SKILL.md"), {
          path: join(getAgentDir(), "skills/alpha/SKILL.md"),
          source: "local",
          scope: "user",
          origin: "top-level",
        }),
        skill("deploy", "/work/project/.agents/skills/deploy/SKILL.md", {
          path: "/work/project/.agents/skills/deploy/SKILL.md",
          source: "local",
          scope: "project",
          origin: "top-level",
        }),
      ],
    };

    expect(toggleResourcesFromPrompt(options).map(({ origin, kind, label }) =>
      `${origin}:${kind}:${label}`,
    )).toEqual([
      "global:instruction:AGENTS.md",
      "global:skill:alpha",
      "global:skill:zeta",
      "project:instruction:AGENTS.md",
      "project:instruction:CLAUDE.md",
      "project:skill:deploy",
    ]);
  });

  test("deduplicates repeated discovery paths", () => {
    const path = join(getAgentDir(), "AGENTS.md");
    const resources = toggleResourcesFromPrompt({
      cwd: "/work/project",
      contextFiles: [
        { path, content: "first" },
        { path, content: "duplicate" },
      ],
    });

    expect(resources).toHaveLength(1);
    expect(resources[0]?.id).toBe(resourcePathId(path));
  });

  test("preserves the discovery path when a global instruction is symlinked elsewhere", () => {
    const path = join(getAgentDir(), "AGENTS.md");
    const resources = toggleResourcesFromPrompt({
      cwd: "/work/project",
      contextFiles: [{ path, content: "rules" }],
    });

    expect(resources[0]).toMatchObject({ id: path, origin: "global", label: "AGENTS.md" });
  });

  test("excludes packages, temporary skills, and skills outside standard roots", () => {
    const options: BuildSystemPromptOptions = {
      cwd: "/work/project",
      skills: [
        skill("package-skill", "/packages/skill/SKILL.md", {
          path: "/packages/skill/SKILL.md",
          source: "npm:test",
          scope: "user",
          origin: "package",
        }),
        skill("temporary", "/tmp/skill/SKILL.md", {
          path: "/tmp/skill/SKILL.md",
          source: "cli",
          scope: "temporary",
          origin: "top-level",
        }),
        skill("extension-skill", join(getAgentDir(), "extensions/example/skill/SKILL.md"), {
          path: join(getAgentDir(), "extensions/example/index.ts"),
          source: "local",
          scope: "user",
          origin: "top-level",
        }),
      ],
    };

    expect(toggleResourcesFromPrompt(options)).toEqual([]);
  });

  test("marks source-authored manual-only skills as read-only", () => {
    const path = join(getAgentDir(), "skills/manual/SKILL.md");
    const resources = toggleResourcesFromPrompt({
      cwd: "/work/project",
      skills: [skill("manual", path, {
        path,
        source: "local",
        scope: "user",
        origin: "top-level",
      }, true)],
    });

    expect(resources[0]?.editability).toBe("manual-only");
  });
});
