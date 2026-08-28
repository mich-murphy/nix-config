import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterEach, describe, expect, test } from "bun:test";
import { getAgentDir, type BuildSystemPromptOptions, type Skill } from "@earendil-works/pi-coding-agent";
import { policyResourcesFromPrompt } from "../resources";

const temporaryDirectories: string[] = [];
afterEach(() => {
  for (const directory of temporaryDirectories.splice(0)) {
    rmSync(directory, { recursive: true, force: true });
  }
});

function temporaryDirectory(): string {
  const directory = mkdtempSync(join(tmpdir(), "pi-skill-toggle-resources-"));
  temporaryDirectories.push(directory);
  return directory;
}

describe("policyResourcesFromPrompt", () => {
  test("projects Pi provenance and classifies instruction ownership", () => {
    const root = temporaryDirectory();
    const project = join(root, "project");
    mkdirSync(project);
    const projectInstructions = join(project, "AGENTS.md");
    const inheritedInstructions = join(root, "AGENTS.md");
    writeFileSync(projectInstructions, "project rules");
    writeFileSync(inheritedInstructions, "inherited rules");

    const options: BuildSystemPromptOptions = {
      cwd: project,
      contextFiles: [
        { path: join(getAgentDir(), "AGENTS.md"), content: "user rules" },
        { path: projectInstructions, content: "project rules" },
        { path: inheritedInstructions, content: "inherited rules" },
      ],
    };

    const resources = policyResourcesFromPrompt(options);

    expect(resources.instructions.map(({ provenance }) => provenance.scope)).toEqual([
      "user",
      "project",
      "inherited",
    ]);
    expect(resources.instructions[1]?.path).toBe(realpathSync.native(projectInstructions));
  });

  test("retains canonical skill provenance and resolves symlinked paths", () => {
    const root = temporaryDirectory();
    const project = join(root, "project");
    const skillDirectory = join(root, "skills", "research");
    const skillFile = join(skillDirectory, "SKILL.md");
    const alias = join(root, "research-skill.md");
    mkdirSync(project);
    mkdirSync(skillDirectory, { recursive: true });
    writeFileSync(skillFile, "skill body");
    symlinkSync(skillFile, alias);

    const skill: Skill = {
      name: " research ",
      description: "Research primary sources",
      filePath: alias,
      baseDir: dirname(alias),
      sourceInfo: {
        path: alias,
        source: "test-package",
        scope: "temporary",
        origin: "package",
      },
      disableModelInvocation: true,
    };

    const resources = policyResourcesFromPrompt({ cwd: project, skills: [skill] });
    const projected = resources.skills[0];

    expect(projected).toBeDefined();
    expect(projected).toMatchObject({
      name: "research",
      description: "Research primary sources",
      filePath: realpathSync.native(skillFile),
      sourceManualOnly: true,
      provenance: {
        path: realpathSync.native(skillFile),
        source: "test-package",
        scope: "temporary",
        origin: "package",
      },
    });
  });
});
