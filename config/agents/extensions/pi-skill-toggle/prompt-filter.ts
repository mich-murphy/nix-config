import {
  formatSkillsForPrompt,
  type BuildSystemPromptOptions,
} from "@earendil-works/pi-coding-agent";
import type { EffectivePolicy } from "./policy";
import { resourcePathId } from "./resource-path";

/** Result of applying effective policy to Pi's current system prompt. */
export interface PromptPolicyResult {
  readonly systemPrompt: string;
  readonly failures: ReadonlyArray<"instructions" | "skills">;
}

/** Apply effective resource visibility to a model-facing system prompt. */
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

/** Canonical resource identifiers to remove from a system prompt. */
export interface SkillToggleSelection {
  readonly disabledContextPaths: ReadonlySet<string>;
  readonly hiddenSkillNames: ReadonlySet<string>;
}

/** Filter Pi-rendered instruction and skill sections with exact replacement. */
export function filterSystemPrompt(
  systemPrompt: string,
  options: BuildSystemPromptOptions,
  selection: SkillToggleSelection,
): PromptPolicyResult {
  const failures: Array<"instructions" | "skills"> = [];
  const contextFiles = options.contextFiles ?? [];
  const enabledContextFiles = contextFiles.filter(
    (file) => !selection.disabledContextPaths.has(resourcePathId(file.path, options.cwd)),
  );
  const originalContext = renderProjectContext(contextFiles);
  const contextResult = replaceLastExact(
    systemPrompt,
    originalContext,
    enabledContextFiles.length === contextFiles.length ? originalContext : renderProjectContext(enabledContextFiles),
  );
  if (!contextResult.matched) failures.push("instructions");

  const skillsCanRender = options.selectedTools === undefined || options.selectedTools.includes("read");
  if (!skillsCanRender) return { systemPrompt: contextResult.value, failures };

  const skills = options.skills ?? [];
  const enabledSkills = skills.filter((skill) => !selection.hiddenSkillNames.has(skill.name));
  const originalSkills = formatSkillsForPrompt(skills);
  const skillResult = replaceLastExact(
    contextResult.value,
    originalSkills,
    enabledSkills.length === skills.length ? originalSkills : formatSkillsForPrompt(enabledSkills),
  );
  if (!skillResult.matched) failures.push("skills");
  return { systemPrompt: skillResult.value, failures };
}

/** One instruction file supplied in Pi's system-prompt options. */
export type ContextFile = NonNullable<BuildSystemPromptOptions["contextFiles"]>[number];

/** Render Pi's current project-context section for exact prompt replacement. */
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
