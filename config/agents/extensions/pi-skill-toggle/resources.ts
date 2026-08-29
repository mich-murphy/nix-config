import { homedir } from "node:os";
import { basename, dirname, isAbsolute, join, relative, sep } from "node:path";
import {
  CONFIG_DIR_NAME,
  getAgentDir,
  type BuildSystemPromptOptions,
} from "@earendil-works/pi-coding-agent";
import { resourcePathId, type ResourcePath } from "./resource-path";

/** Kind of model-facing resource controlled by the extension. */
export type ToggleResourceKind = "instruction" | "skill";

/** Human-facing origin group used to order toggle rows. */
export type ToggleResourceOrigin = "global" | "project";

/** Whether the extension may change a resource's model visibility. */
export type ToggleResourceEditability = "editable" | "manual-only";

/** A user-managed instruction file or skill that can appear in the toggle menu. */
export interface ToggleResource {
  readonly id: ResourcePath;
  readonly kind: ToggleResourceKind;
  readonly origin: ToggleResourceOrigin;
  readonly owner: ResourcePath;
  readonly label: string;
  readonly description: string;
  readonly editability: ToggleResourceEditability;
  readonly order: number;
}

/** Extract eligible user-managed resources from Pi's structured prompt options. */
export function toggleResourcesFromPrompt(
  options: BuildSystemPromptOptions,
): ReadonlyArray<ToggleResource> {
  const cwd = resourcePathId(options.cwd, options.cwd);
  const agentDirectory = resourcePathId(getAgentDir(), cwd);
  const globalSkillRoots = [
    resourcePathId(join(getAgentDir(), "skills"), cwd),
    resourcePathId(join(homedir(), ".agents", "skills"), cwd),
  ];

  const instructions = (options.contextFiles ?? []).flatMap<ToggleResource>((file, index) => {
    const id = resourcePathId(file.path, cwd);
    const parent = dirname(id);
    const origin: ToggleResourceOrigin | undefined = parent === agentDirectory
      ? "global"
      : isPathInsideOrEqual(cwd, parent)
        ? "project"
        : undefined;
    if (!origin) return [];
    return [{
      id,
      kind: "instruction" as const,
      origin,
      owner: resourcePathId(parent),
      label: basename(id),
      description: `${origin} instruction\n${id}`,
      editability: "editable" as const,
      order: index,
    }];
  });

  const skills = (options.skills ?? []).flatMap<ToggleResource>((skill) => {
    if (skill.sourceInfo.origin !== "top-level") return [];
    if (skill.sourceInfo.scope !== "user" && skill.sourceInfo.scope !== "project") return [];

    const id = resourcePathId(skill.filePath, cwd);
    const globalRoot = globalSkillRoots.find((root) => isPathInsideOrEqual(id, root));
    const projectOwner = skill.sourceInfo.scope === "project"
      ? projectSkillOwner(id, cwd)
      : undefined;
    const origin: ToggleResourceOrigin | undefined = globalRoot
      ? "global"
      : projectOwner
        ? "project"
        : undefined;
    if (!origin) return [];

    return [{
      id,
      kind: "skill" as const,
      origin,
      owner: globalRoot ?? projectOwner ?? cwd,
      label: skill.name.trim(),
      description: `${skill.description.trim() || "(no description)"}\n${id}`,
      editability: skill.disableModelInvocation ? "manual-only" as const : "editable" as const,
      order: 0,
    }];
  });

  return uniqueResources([...instructions, ...skills]).sort(compareResources);
}

function uniqueResources(resources: ReadonlyArray<ToggleResource>): ToggleResource[] {
  const unique = new Map<string, ToggleResource>();
  for (const resource of resources) {
    if (!unique.has(resource.id)) unique.set(resource.id, resource);
  }
  return [...unique.values()];
}

function compareResources(left: ToggleResource, right: ToggleResource): number {
  const originDifference = originRank(left.origin) - originRank(right.origin);
  if (originDifference !== 0) return originDifference;
  const kindDifference = kindRank(left.kind) - kindRank(right.kind);
  if (kindDifference !== 0) return kindDifference;
  if (left.kind === "instruction" && right.kind === "instruction") {
    const orderDifference = left.order - right.order;
    if (orderDifference !== 0) return orderDifference;
  }
  return left.label.localeCompare(right.label) || left.id.localeCompare(right.id);
}

function originRank(origin: ToggleResourceOrigin): number {
  return origin === "global" ? 0 : 1;
}

function kindRank(kind: ToggleResourceKind): number {
  return kind === "instruction" ? 0 : 1;
}

function projectSkillOwner(path: string, cwd: string): ResourcePath | undefined {
  const markers = [
    `${sep}${CONFIG_DIR_NAME}${sep}skills${sep}`,
    `${sep}.agents${sep}skills${sep}`,
  ];
  for (const marker of markers) {
    const markerIndex = path.indexOf(marker);
    if (markerIndex < 0) continue;
    const owner = path.slice(0, markerIndex) || sep;
    if (isPathInsideOrEqual(cwd, owner)) return resourcePathId(owner);
  }
  return undefined;
}

function isPathInsideOrEqual(path: string, parent: string): boolean {
  const difference = relative(parent, path);
  return difference === "" || (!difference.startsWith(`..${sep}`) && difference !== ".." && !isAbsolute(difference));
}
