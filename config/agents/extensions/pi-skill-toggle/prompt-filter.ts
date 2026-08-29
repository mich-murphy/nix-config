import {
  formatSkillsForPrompt,
  type BuildSystemPromptOptions,
} from "@earendil-works/pi-coding-agent";
import { resourcePathId, type ResourcePath } from "./resource-path";

interface PromptToggleResult {
  readonly systemPrompt: string;
  readonly failures: ReadonlyArray<"instructions" | "skills">;
}

/** Remove disabled instruction files and skills from Pi's model-facing prompt. */
export function applyResourceToggles(
  systemPrompt: string,
  options: BuildSystemPromptOptions,
  disabledResourcePaths: ReadonlySet<ResourcePath>,
): PromptToggleResult {
  const failures: Array<"instructions" | "skills"> = [];
  const contextFiles = options.contextFiles ?? [];
  const enabledContextFiles = contextFiles.filter(
    (file) => !disabledResourcePaths.has(resourcePathId(file.path, options.cwd)),
  );
  const originalContext = renderProjectContext(contextFiles);
  const contextResult = enabledContextFiles.length === contextFiles.length
    ? { value: systemPrompt, matched: true }
    : replaceLastExact(systemPrompt, originalContext, renderProjectContext(enabledContextFiles));
  if (!contextResult.matched) failures.push("instructions");

  const skillsCanRender = options.selectedTools === undefined || options.selectedTools.includes("read");
  if (!skillsCanRender) return { systemPrompt: contextResult.value, failures };

  const skills = options.skills ?? [];
  const enabledSkills = skills.filter(
    (skill) => !disabledResourcePaths.has(resourcePathId(skill.filePath, options.cwd)),
  );
  const originalSkills = formatSkillsForPrompt(skills);
  const skillResult = enabledSkills.length === skills.length
    ? { value: contextResult.value, matched: true }
    : replaceLastExact(contextResult.value, originalSkills, formatSkillsForPrompt(enabledSkills));
  if (!skillResult.matched) failures.push("skills");
  return { systemPrompt: skillResult.value, failures };
}

type ContextFile = NonNullable<BuildSystemPromptOptions["contextFiles"]>[number];

function renderProjectContext(contextFiles: readonly ContextFile[]): string {
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
  if (original.length === 0) return { value: input, matched: false };
  const index = input.lastIndexOf(original);
  if (index < 0) return { value: input, matched: false };
  return {
    value: `${input.slice(0, index)}${replacement}${input.slice(index + original.length)}`,
    matched: true,
  };
}
