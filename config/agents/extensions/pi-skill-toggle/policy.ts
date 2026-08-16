import type { Skill } from "@earendil-works/pi-coding-agent";

export type SkillVisibility = "visible" | "manual-only";
export type InstructionVisibility = "included" | "excluded";
export type PolicyScope = "global" | "directory" | "session";
export type PolicyOverride<T> = T | "inherit";
export type ResolutionSource = "source" | PolicyScope | "default";

export interface ResourceProvenance {
  path: string;
  scope: "user" | "project" | "temporary" | "inherited";
  origin: string;
  source: string;
}

export interface InstructionResource {
  kind: "instruction";
  path: string;
  provenance: ResourceProvenance;
}

export interface SkillResource {
  kind: "skill";
  name: string;
  description: string;
  filePath: string;
  provenance: ResourceProvenance;
  sourceManualOnly: boolean;
}

export interface PolicyResources {
  instructions: InstructionResource[];
  skills: SkillResource[];
}

export interface PersistedPolicySnapshot {
  cwd: string;
  generation: string;
  globalSkills: Record<string, SkillVisibility>;
  directorySkills: Record<string, SkillVisibility>;
  directoryInstructions: Record<string, InstructionVisibility>;
}

export interface SessionPolicy {
  skills: Record<string, SkillVisibility>;
  instructions: Record<string, InstructionVisibility>;
}

export interface EffectiveInstructionPolicy extends InstructionResource {
  visibility: InstructionVisibility;
  resolvedFrom: Exclude<ResolutionSource, "source" | "global">;
}

export interface EffectiveSkillPolicy extends SkillResource {
  visibility: SkillVisibility;
  sourceLocked: boolean;
  resolvedFrom: ResolutionSource;
}

export interface EffectivePolicy {
  cwd: string;
  generation: string;
  instructions: EffectiveInstructionPolicy[];
  skills: EffectiveSkillPolicy[];
}

export interface PolicyDraft {
  scope: PolicyScope;
  cwd: string;
  generation: string;
  skills: Record<string, PolicyOverride<SkillVisibility>>;
  instructions: Record<string, PolicyOverride<InstructionVisibility>>;
  readOnlySkillNames: string[];
}

export interface PolicyChange {
  scope: PolicyScope;
  kind: "skill" | "instruction";
  id: string;
  before: string;
  after: string;
}

export interface PolicyError {
  change?: PolicyChange;
  message: string;
}

export interface PolicyPlan {
  scope: PolicyScope;
  cwd: string;
  generation: string;
  changes: PolicyChange[];
}

export interface ApplyResult {
  applied: PolicyChange[];
  skipped: PolicyChange[];
  errors: PolicyError[];
  snapshot?: PersistedPolicySnapshot;
}

export type PolicyRefreshResult =
  | { ok: true; policy: PersistedPolicySnapshot; generation: string }
  | { ok: false; error: Error };

export interface PolicyLoadInput {
  cwd: string;
  skills?: readonly Skill[];
  legacyEntries?: Iterable<unknown>;
  sessionId?: string;
}

export interface PolicyStateAdapter {
  load(input: PolicyLoadInput): PersistedPolicySnapshot;
  apply(plan: PolicyPlan): ApplyResult;
  reset(scope: PolicyScope | "all", cwd: string): ApplyResult;
}

export class SkillPolicy {
  private session: SessionPolicy = { skills: {}, instructions: {} };

  constructor(private readonly state: PolicyStateAdapter) {}

  refresh(input: PolicyLoadInput): PolicyRefreshResult {
    try {
      const policy = this.state.load(input);
      return { ok: true, policy, generation: policy.generation };
    } catch (error) {
      return { ok: false, error: asError(error) };
    }
  }

  resolve(snapshot: PersistedPolicySnapshot, resources: PolicyResources): EffectivePolicy {
    return resolveEffectivePolicy(snapshot, this.session, resources);
  }

  draft(scope: PolicyScope, effective: EffectivePolicy, snapshot: PersistedPolicySnapshot): PolicyDraft {
    return createPolicyDraft(scope, effective, snapshot, this.session);
  }

  plan(draft: PolicyDraft, snapshot: PersistedPolicySnapshot): PolicyPlan {
    return planPolicyChanges(draft, snapshot, this.session);
  }

  apply(plan: PolicyPlan): ApplyResult {
    if (plan.scope !== "session") return this.state.apply(plan);
    for (const change of plan.changes) {
      const target = change.kind === "skill" ? this.session.skills : this.session.instructions;
      if (change.after === "inherit") delete target[change.id];
      else target[change.id] = change.after as never;
    }
    return { applied: [...plan.changes], skipped: [], errors: [] };
  }

  reset(scope: PolicyScope | "all", cwd: string): ApplyResult {
    if (scope === "session" || scope === "all") {
      this.session = { skills: {}, instructions: {} };
      if (scope === "session") return { applied: [], skipped: [], errors: [] };
    }
    return this.state.reset(scope, cwd);
  }

  clearSession(): void {
    this.session = { skills: {}, instructions: {} };
  }

  sessionOverrides(): SessionPolicy {
    return {
      skills: { ...this.session.skills },
      instructions: { ...this.session.instructions },
    };
  }
}

export function resolveEffectivePolicy(
  snapshot: PersistedPolicySnapshot,
  session: SessionPolicy,
  resources: PolicyResources,
): EffectivePolicy {
  const instructions = resources.instructions.map((resource): EffectiveInstructionPolicy => {
    if (session.instructions[resource.path]) {
      return { ...resource, visibility: session.instructions[resource.path]!, resolvedFrom: "session" };
    }
    if (snapshot.directoryInstructions[resource.path]) {
      return { ...resource, visibility: snapshot.directoryInstructions[resource.path]!, resolvedFrom: "directory" };
    }
    return { ...resource, visibility: "included", resolvedFrom: "default" };
  });

  const skills = resources.skills.map((resource): EffectiveSkillPolicy => {
    if (resource.sourceManualOnly) {
      return { ...resource, visibility: "manual-only", sourceLocked: true, resolvedFrom: "source" };
    }
    if (session.skills[resource.name]) {
      return { ...resource, visibility: session.skills[resource.name]!, sourceLocked: false, resolvedFrom: "session" };
    }
    if (snapshot.directorySkills[resource.name]) {
      return { ...resource, visibility: snapshot.directorySkills[resource.name]!, sourceLocked: false, resolvedFrom: "directory" };
    }
    if (snapshot.globalSkills[resource.name]) {
      return { ...resource, visibility: snapshot.globalSkills[resource.name]!, sourceLocked: false, resolvedFrom: "global" };
    }
    return { ...resource, visibility: "visible", sourceLocked: false, resolvedFrom: "default" };
  });

  return { cwd: snapshot.cwd, generation: snapshot.generation, instructions, skills };
}

function createPolicyDraft(
  scope: PolicyScope,
  effective: EffectivePolicy,
  snapshot: PersistedPolicySnapshot,
  session: SessionPolicy,
): PolicyDraft {
  const skills: PolicyDraft["skills"] = {};
  const instructions: PolicyDraft["instructions"] = {};
  for (const skill of effective.skills) {
    skills[skill.name] = scope === "global"
      ? snapshot.globalSkills[skill.name] ?? "visible"
      : scope === "directory"
        ? snapshot.directorySkills[skill.name] ?? "inherit"
        : session.skills[skill.name] ?? "inherit";
  }
  const scopedSkills = scope === "global"
    ? snapshot.globalSkills
    : scope === "directory"
      ? snapshot.directorySkills
      : session.skills;
  for (const [name, value] of Object.entries(scopedSkills)) skills[name] = value;

  if (scope !== "global") {
    for (const instruction of effective.instructions) {
      instructions[instruction.path] = scope === "directory"
        ? snapshot.directoryInstructions[instruction.path] ?? "inherit"
        : session.instructions[instruction.path] ?? "inherit";
    }
    const scopedInstructions = scope === "directory"
      ? snapshot.directoryInstructions
      : session.instructions;
    for (const [path, value] of Object.entries(scopedInstructions)) instructions[path] = value;
  }
  return {
    scope,
    cwd: snapshot.cwd,
    generation: snapshot.generation,
    skills,
    instructions,
    readOnlySkillNames: effective.skills.filter((skill) => skill.sourceLocked).map((skill) => skill.name),
  };
}

function planPolicyChanges(
  draft: PolicyDraft,
  snapshot: PersistedPolicySnapshot,
  session: SessionPolicy,
): PolicyPlan {
  const changes: PolicyChange[] = [];
  const previousSkills: Record<string, string> = draft.scope === "global"
    ? snapshot.globalSkills
    : draft.scope === "directory"
      ? snapshot.directorySkills
      : session.skills;
  const previousInstructions: Record<string, string> = draft.scope === "directory"
    ? snapshot.directoryInstructions
    : draft.scope === "session"
      ? session.instructions
      : {};

  collectChanges(
    changes,
    draft.scope,
    "skill",
    Object.fromEntries(Object.entries(previousSkills).filter(([name]) => !draft.readOnlySkillNames.includes(name))),
    Object.fromEntries(Object.entries(draft.skills).filter(([name]) => !draft.readOnlySkillNames.includes(name))),
    draft.scope === "global" ? "visible" : "inherit",
  );
  collectChanges(changes, draft.scope, "instruction", previousInstructions, draft.instructions, "inherit");
  return { scope: draft.scope, cwd: draft.cwd, generation: draft.generation, changes };
}

function collectChanges(
  output: PolicyChange[],
  scope: PolicyScope,
  kind: PolicyChange["kind"],
  previous: Record<string, string>,
  desired: Record<string, string>,
  fallback: string,
): void {
  for (const id of [...new Set([...Object.keys(previous), ...Object.keys(desired)])].sort()) {
    const before = previous[id] ?? fallback;
    const after = desired[id] ?? fallback;
    if (before !== after) output.push({ scope, kind, id, before, after });
  }
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}
