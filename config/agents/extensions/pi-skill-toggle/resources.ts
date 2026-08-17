import { dirname } from "node:path";
import { getAgentDir, type BuildSystemPromptOptions } from "@earendil-works/pi-coding-agent";
import type { PolicyResources, ResourceProvenance } from "./policy";
import { resourcePathId } from "./prompt-filter";

export function policyResourcesFromPrompt(options: BuildSystemPromptOptions): PolicyResources {
  const cwd = resourcePathId(options.cwd, options.cwd);
  const agentDirectory = resourcePathId(getAgentDir(), cwd);
  return {
    instructions: (options.contextFiles ?? []).map((file) => {
      const path = resourcePathId(file.path, cwd);
      return {
        kind: "instruction" as const,
        path,
        provenance: instructionProvenance(path, cwd, agentDirectory),
      };
    }),
    skills: (options.skills ?? []).map((skill) => ({
      kind: "skill" as const,
      name: skill.name.trim(),
      description: skill.description,
      filePath: resourcePathId(skill.filePath, cwd),
      provenance: {
        path: resourcePathId(skill.sourceInfo.path, cwd),
        scope: skill.sourceInfo.scope,
        origin: skill.sourceInfo.origin,
        source: skill.sourceInfo.source,
      },
      sourceManualOnly: skill.disableModelInvocation,
    })),
  };
}

function instructionProvenance(path: string, cwd: string, agentDirectory: string): ResourceProvenance {
  const parent = dirname(path);
  const scope = parent === agentDirectory
    ? "user"
    : parent === cwd
      ? "project"
      : "inherited";
  return {
    path,
    scope,
    origin: "top-level",
    source: scope === "user" ? "agent" : "project-context",
  };
}
