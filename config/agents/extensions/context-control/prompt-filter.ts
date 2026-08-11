import { realpathSync } from "node:fs";
import { normalize, resolve } from "node:path";
import {
  formatSkillsForPrompt,
  type BuildSystemPromptOptions,
  type Skill,
} from "@earendil-works/pi-coding-agent";

export interface ContextControlSelection {
  disabledContextPaths: ReadonlySet<string>;
  hiddenSkillPaths: ReadonlySet<string>;
}

export function filterSystemPrompt(
  systemPrompt: string,
  options: BuildSystemPromptOptions,
  selection: ContextControlSelection,
): string {
  const contextFiles = options.contextFiles ?? [];
  const enabledContextFiles = contextFiles.filter(
    (file) => !selection.disabledContextPaths.has(resourcePathId(file.path, options.cwd)),
  );
  let filtered = replaceLastExact(
    systemPrompt,
    renderProjectContext(contextFiles),
    renderProjectContext(enabledContextFiles),
  );

  const skills = options.skills ?? [];
  const enabledSkills = skills.filter(
    (skill) => !selection.hiddenSkillPaths.has(resourcePathId(skill.filePath, options.cwd)),
  );
  filtered = replaceLastExact(
    filtered,
    formatSkillsForPrompt(skills),
    formatSkillsForPrompt(enabledSkills),
  );

  return filtered;
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

  const instructions = contextFiles
    .map(
      ({ path, content }) =>
        `<project_instructions path="${path}">\n${content}\n</project_instructions>\n\n`,
    )
    .join("");

  return `\n\n<project_context>\n\nProject-specific instructions and guidelines:\n\n${instructions}</project_context>\n`;
}

function replaceLastExact(input: string, original: string, replacement: string): string {
  if (original.length === 0 || original === replacement) return input;
  const index = input.lastIndexOf(original);
  if (index < 0) return input;
  return `${input.slice(0, index)}${replacement}${input.slice(index + original.length)}`;
}
export type { Skill };
