import { realpathSync } from "node:fs";
import { normalize, resolve } from "node:path";
import {
  formatSkillsForPrompt,
  type BuildSystemPromptOptions,
} from "@earendil-works/pi-coding-agent";
import type { EffectivePolicy } from "./policy";

export interface PromptPolicyResult {
  systemPrompt: string;
  failures: Array<"instructions" | "skills">;
}

export function applyPolicyToSystemPrompt(
  systemPrompt: string,
  options: BuildSystemPromptOptions,
  policy: EffectivePolicy,
): PromptPolicyResult {
  const excludedInstructions = new Set(policy.instructions
    .filter((item) => item.visibility === "excluded")
    .map((item) => item.path));
  const hiddenSkills = new Set(policy.skills
    .filter((item) => item.visibility === "manual-only")
    .map((item) => item.name));
  return filterSystemPrompt(systemPrompt, options, {
    disabledContextPaths: excludedInstructions,
    hiddenSkillNames: hiddenSkills,
  });
}

export interface SkillToggleSelection {
  disabledContextPaths: ReadonlySet<string>;
  hiddenSkillNames: ReadonlySet<string>;
}

export function filterSystemPrompt(
  systemPrompt: string,
  options: BuildSystemPromptOptions,
  selection: SkillToggleSelection,
): PromptPolicyResult {
  const failures: PromptPolicyResult["failures"] = [];
  const contextFiles = options.contextFiles ?? [];
  const enabledContextFiles = contextFiles.filter(
    (file) => !selection.disabledContextPaths.has(resourcePathId(file.path, options.cwd)),
  );
  const contextResult = replaceLastExact(
    systemPrompt,
    renderProjectContext(contextFiles),
    renderProjectContext(enabledContextFiles),
  );
  if (!contextResult.matched) failures.push("instructions");

  const skills = options.skills ?? [];
  const enabledSkills = skills.filter((skill) => !selection.hiddenSkillNames.has(skill.name));
  const skillResult = replaceLastExact(
    contextResult.value,
    formatSkillsForPrompt(skills),
    formatSkillsForPrompt(enabledSkills),
  );
  if (!skillResult.matched) failures.push("skills");
  return { systemPrompt: skillResult.value, failures };
}

export type ContextFile = NonNullable<BuildSystemPromptOptions["contextFiles"]>[number];

export function resourcePathId(path: string, cwd = process.cwd()): string {
  const absolute = normalize(resolve(cwd, path));
  try {
    return realpathSync.native(absolute);
  } catch {
    return absolute;
  }
}

export function renderProjectContext(contextFiles: readonly ContextFile[]): string {
  if (contextFiles.length === 0) return "";
  const instructions = contextFiles.map(({ path, content }) =>
    `<project_instructions path="${path}">\n${content}\n</project_instructions>\n\n`,
  ).join("");
  return `\n\n<project_context>\n\nProject-specific instructions and guidelines:\n\n${instructions}</project_context>\n`;
}

function replaceLastExact(
  input: string,
  original: string,
  replacement: string,
): { value: string; matched: boolean } {
  if (original === replacement) return { value: input, matched: true };
  if (original.length === 0) return { value: input, matched: false };
  const index = input.lastIndexOf(original);
  if (index < 0) return { value: input, matched: false };
  return {
    value: `${input.slice(0, index)}${replacement}${input.slice(index + original.length)}`,
    matched: true,
  };
}
