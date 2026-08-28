/** Visibility of a skill in the model-facing system prompt. */
export type SkillVisibility = "visible" | "manual-only";

/** Visibility of an instruction file in the system prompt. */
export type InstructionVisibility = "included" | "excluded";

/** Scope at which a policy value applies. */
export type PolicyScope = "global" | "directory" | "session";

/** A scoped value or an instruction to inherit from the next lower-precedence scope. */
export type PolicyOverride<T> = T | "inherit";

/** Source that supplied an effective policy value. */
export type ResolutionSource = "source" | PolicyScope | "default";

/** Canonical provenance supplied by Pi for a configurable resource. */
export interface ResourceProvenance {
  readonly path: string;
  readonly scope: "user" | "project" | "temporary" | "inherited";
  readonly origin: string;
  readonly source: string;
}

/** An instruction file that can participate in policy resolution. */
export interface InstructionResource {
  readonly kind: "instruction";
  readonly path: string;
  readonly provenance: ResourceProvenance;
}

/** A skill that can participate in policy resolution. */
export interface SkillResource {
  readonly kind: "skill";
  readonly name: string;
  readonly description: string;
  readonly filePath: string;
  readonly provenance: ResourceProvenance;
  readonly sourceManualOnly: boolean;
}

/** Resources extracted from Pi's structured system-prompt options. */
export interface PolicyResources {
  readonly instructions: ReadonlyArray<InstructionResource>;
  readonly skills: ReadonlyArray<SkillResource>;
}

/** Immutable persisted policy values for one canonical working directory. */
export interface PersistedPolicySnapshot {
  readonly cwd: string;
  readonly generation: string;
  readonly globalSkills: Readonly<Record<string, SkillVisibility>>;
  readonly directorySkills: Readonly<Record<string, SkillVisibility>>;
  readonly directoryInstructions: Readonly<Record<string, InstructionVisibility>>;
}

/** Mutable, process-local overrides for the current Pi session. */
export interface SessionPolicy {
  readonly skills: Record<string, SkillVisibility>;
  readonly instructions: Record<string, InstructionVisibility>;
}

/** Effective visibility and provenance for an instruction file. */
export interface EffectiveInstructionPolicy extends InstructionResource {
  readonly visibility: InstructionVisibility;
  readonly resolvedFrom: Exclude<ResolutionSource, "source" | "global">;
}

/** Effective visibility and provenance for a skill. */
export interface EffectiveSkillPolicy extends SkillResource {
  readonly visibility: SkillVisibility;
  readonly sourceLocked: boolean;
  readonly resolvedFrom: ResolutionSource;
}

/** Fully resolved policy for the resources loaded in one directory. */
export interface EffectivePolicy {
  readonly cwd: string;
  readonly generation: string;
  readonly instructions: ReadonlyArray<EffectiveInstructionPolicy>;
  readonly skills: ReadonlyArray<EffectiveSkillPolicy>;
}

/** Mutable settings draft for one policy scope. */
export interface PolicyDraft {
  readonly scope: PolicyScope;
  readonly cwd: string;
  readonly generation: string;
  readonly skills: Record<string, PolicyOverride<SkillVisibility>>;
  readonly instructions: Record<string, PolicyOverride<InstructionVisibility>>;
  readonly readOnlySkillNames: ReadonlyArray<string>;
}

/** A global skill transition. Global policy has no inherit state. */
export interface GlobalSkillPolicyChange {
  readonly scope: "global";
  readonly kind: "skill";
  readonly id: string;
  readonly before: SkillVisibility;
  readonly after: SkillVisibility;
}

/** A directory or session skill transition. */
export interface ScopedSkillPolicyChange {
  readonly scope: "directory" | "session";
  readonly kind: "skill";
  readonly id: string;
  readonly before: PolicyOverride<SkillVisibility>;
  readonly after: PolicyOverride<SkillVisibility>;
}

/** A directory or session instruction transition. */
export interface ScopedInstructionPolicyChange {
  readonly scope: "directory" | "session";
  readonly kind: "instruction";
  readonly id: string;
  readonly before: PolicyOverride<InstructionVisibility>;
  readonly after: PolicyOverride<InstructionVisibility>;
}

/** A valid transition that can be applied by the policy service. */
export type PolicyChange =
  | GlobalSkillPolicyChange
  | ScopedSkillPolicyChange
  | ScopedInstructionPolicyChange;

/** A staged set of global skill transitions. */
export interface GlobalPolicyPlan {
  readonly scope: "global";
  readonly cwd: string;
  readonly generation: string;
  readonly changes: ReadonlyArray<GlobalSkillPolicyChange>;
}

/** A staged set of directory transitions. */
export interface DirectoryPolicyPlan {
  readonly scope: "directory";
  readonly cwd: string;
  readonly generation: string;
  readonly changes: ReadonlyArray<
    | ScopedSkillPolicyChange & { readonly scope: "directory" }
    | ScopedInstructionPolicyChange & { readonly scope: "directory" }
  >;
}

/** A staged set of process-local session transitions. */
export interface SessionPolicyPlan {
  readonly scope: "session";
  readonly cwd: string;
  readonly generation: string;
  readonly changes: ReadonlyArray<
    | ScopedSkillPolicyChange & { readonly scope: "session" }
    | ScopedInstructionPolicyChange & { readonly scope: "session" }
  >;
}

/** A valid staged plan whose transitions match its declared scope. */
export type PolicyPlan = GlobalPolicyPlan | DirectoryPolicyPlan | SessionPolicyPlan;

/** Operation performed by the persistent policy adapter. */
export type PolicyStateOperation = "load" | "apply" | "reset";

/** Expected state-file or lock failure returned by the persistent adapter. */
export class PolicyStateError extends Error {
  /** Stable discriminator for policy state failures. */
  readonly _tag = "PolicyStateError" as const;

  /** State operation that failed. */
  readonly operation: PolicyStateOperation;

  /** Unclassified lower-level failure retained for debugging. */
  override readonly cause: unknown;

  /** Create a classified state adapter failure. */
  constructor(operation: PolicyStateOperation, message: string, cause?: unknown) {
    super(message);
    this.name = "PolicyStateError";
    this.operation = operation;
    this.cause = cause;
  }
}

/** A transition skipped because persisted state changed after planning. */
export class PolicyConflictError extends Error {
  /** Stable discriminator for concurrent policy conflicts. */
  readonly _tag = "PolicyConflictError" as const;

  /** Transition skipped because its previous value no longer matched. */
  readonly change: PolicyChange;

  /** Create a conflict for one skipped transition. */
  constructor(change: PolicyChange) {
    super(`Concurrent change detected for ${change.kind} ${change.id}`);
    this.name = "PolicyConflictError";
    this.change = change;
  }
}

/** Known failure returned while applying policy. */
export type PolicyError = PolicyStateError | PolicyConflictError;

/** Result of applying or resetting policy state. */
export interface ApplyResult {
  readonly applied: ReadonlyArray<PolicyChange>;
  readonly skipped: ReadonlyArray<PolicyChange>;
  readonly errors: ReadonlyArray<PolicyError>;
  readonly snapshot?: PersistedPolicySnapshot;
}

/** Result returned by a persistent policy load. */
export type PolicyLoadResult =
  | { readonly _tag: "ok"; readonly value: PersistedPolicySnapshot }
  | { readonly _tag: "err"; readonly error: PolicyStateError };

/** Minimal loaded-skill identity needed by state migration. */
export interface SkillMigrationResource {
  readonly name: string;
  readonly filePath: string;
}

/** Inputs needed to load and migrate policy state. */
export interface PolicyLoadInput {
  readonly cwd: string;
  readonly skills?: ReadonlyArray<SkillMigrationResource>;
  readonly legacyEntries?: Iterable<unknown>;
  readonly sessionId?: string;
}

/** Application-owned contract implemented by persistent policy storage. */
export interface PolicyStateAdapter {
  /** Load policy for a directory and perform any required migration. */
  load(input: PolicyLoadInput): PolicyLoadResult;

  /** Apply a staged persistent policy plan. */
  apply(plan: PolicyPlan): ApplyResult;

  /** Reset persistent policy at the requested scope. */
  reset(scope: PolicyScope | "all", cwd: string): ApplyResult;
}

/** Policy service that owns resolution, planning, and session overrides. */
export class SkillPolicy {
  private session: SessionPolicy = { skills: {}, instructions: {} };

  /** Create a policy service backed by the supplied state adapter. */
  constructor(private readonly state: PolicyStateAdapter) {}

  /** Refresh the persistent snapshot for a directory. */
  refresh(input: PolicyLoadInput): PolicyLoadResult {
    return this.state.load(input);
  }

  /** Resolve effective values for the currently loaded resources. */
  resolve(snapshot: PersistedPolicySnapshot, resources: PolicyResources): EffectivePolicy {
    return resolveEffectivePolicy(snapshot, this.session, resources);
  }

  /** Create an editable draft for one scope. */
  draft(scope: PolicyScope, effective: EffectivePolicy, snapshot: PersistedPolicySnapshot): PolicyDraft {
    return createPolicyDraft(scope, effective, snapshot, this.session);
  }

  /** Compare a draft with current scope values and produce exact transitions. */
  plan(draft: PolicyDraft, snapshot: PersistedPolicySnapshot): PolicyPlan {
    return planPolicyChanges(draft, snapshot, this.session);
  }

  /** Apply a persistent plan or update process-local session overrides. */
  apply(plan: PolicyPlan): ApplyResult {
    if (plan.scope !== "session") return this.state.apply(plan);
    for (const change of plan.changes) applySessionChange(this.session, change);
    return { applied: [...plan.changes], skipped: [], errors: [] };
  }

  /** Reset one scope or all policy state. */
  reset(scope: PolicyScope | "all", cwd: string): ApplyResult {
    if (scope === "session" || scope === "all") {
      this.session = { skills: {}, instructions: {} };
      if (scope === "session") return { applied: [], skipped: [], errors: [] };
    }
    return this.state.reset(scope, cwd);
  }

  /** Clear all process-local session overrides. */
  clearSession(): void {
    this.session = { skills: {}, instructions: {} };
  }

  /** Return a defensive copy of process-local session overrides. */
  sessionOverrides(): SessionPolicy {
    return {
      skills: { ...this.session.skills },
      instructions: { ...this.session.instructions },
    };
  }
}

/** Resolve policy precedence for all currently loaded resources. */
export function resolveEffectivePolicy(
  snapshot: PersistedPolicySnapshot,
  session: SessionPolicy,
  resources: PolicyResources,
): EffectivePolicy {
  const instructions = resources.instructions.map((resource): EffectiveInstructionPolicy => {
    const sessionVisibility = session.instructions[resource.path];
    if (sessionVisibility !== undefined) {
      return { ...resource, visibility: sessionVisibility, resolvedFrom: "session" };
    }
    const directoryVisibility = snapshot.directoryInstructions[resource.path];
    if (directoryVisibility !== undefined) {
      return { ...resource, visibility: directoryVisibility, resolvedFrom: "directory" };
    }
    return { ...resource, visibility: "included", resolvedFrom: "default" };
  });

  const skills = resources.skills.map((resource): EffectiveSkillPolicy => {
    if (resource.sourceManualOnly) {
      return { ...resource, visibility: "manual-only", sourceLocked: true, resolvedFrom: "source" };
    }
    const sessionVisibility = session.skills[resource.name];
    if (sessionVisibility !== undefined) {
      return { ...resource, visibility: sessionVisibility, sourceLocked: false, resolvedFrom: "session" };
    }
    const directoryVisibility = snapshot.directorySkills[resource.name];
    if (directoryVisibility !== undefined) {
      return { ...resource, visibility: directoryVisibility, sourceLocked: false, resolvedFrom: "directory" };
    }
    const globalVisibility = snapshot.globalSkills[resource.name];
    if (globalVisibility !== undefined) {
      return { ...resource, visibility: globalVisibility, sourceLocked: false, resolvedFrom: "global" };
    }
    return { ...resource, visibility: "visible", sourceLocked: false, resolvedFrom: "default" };
  });

  return { cwd: snapshot.cwd, generation: snapshot.generation, instructions, skills };
}

function applySessionChange(session: SessionPolicy, change: PolicyChange): void {
  if (change.scope !== "session") return;
  if (change.kind === "skill") {
    if (change.after === "inherit") delete session.skills[change.id];
    else session.skills[change.id] = change.after;
    return;
  }
  if (change.after === "inherit") delete session.instructions[change.id];
  else session.instructions[change.id] = change.after;
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
  const editableSkills = Object.fromEntries(
    Object.entries(draft.skills).filter(([name]) => !draft.readOnlySkillNames.includes(name)),
  );

  if (draft.scope === "global") {
    const changes: GlobalSkillPolicyChange[] = [];
    collectGlobalSkillChanges(
      changes,
      withoutReadOnlySkills(snapshot.globalSkills, draft.readOnlySkillNames),
      editableSkills,
    );
    return { scope: "global", cwd: draft.cwd, generation: draft.generation, changes };
  }

  if (draft.scope === "directory") {
    const skillChanges: Array<ScopedSkillPolicyChange & { readonly scope: "directory" }> = [];
    const instructionChanges: Array<ScopedInstructionPolicyChange & { readonly scope: "directory" }> = [];
    collectScopedSkillChanges(
      skillChanges,
      "directory",
      withoutReadOnlySkills(snapshot.directorySkills, draft.readOnlySkillNames),
      editableSkills,
    );
    collectInstructionChanges(instructionChanges, "directory", snapshot.directoryInstructions, draft.instructions);
    return {
      scope: "directory",
      cwd: draft.cwd,
      generation: draft.generation,
      changes: [...skillChanges, ...instructionChanges],
    };
  }

  const skillChanges: Array<ScopedSkillPolicyChange & { readonly scope: "session" }> = [];
  const instructionChanges: Array<ScopedInstructionPolicyChange & { readonly scope: "session" }> = [];
  collectScopedSkillChanges(
    skillChanges,
    "session",
    withoutReadOnlySkills(session.skills, draft.readOnlySkillNames),
    editableSkills,
  );
  collectInstructionChanges(instructionChanges, "session", session.instructions, draft.instructions);
  return {
    scope: "session",
    cwd: draft.cwd,
    generation: draft.generation,
    changes: [...skillChanges, ...instructionChanges],
  };
}

function withoutReadOnlySkills<T extends SkillVisibility>(
  skills: Readonly<Record<string, T>>,
  readOnlyNames: ReadonlyArray<string>,
): Record<string, T> {
  return Object.fromEntries(
    Object.entries(skills).filter(([name]) => !readOnlyNames.includes(name)),
  );
}

function collectGlobalSkillChanges(
  output: GlobalSkillPolicyChange[],
  previous: Readonly<Record<string, SkillVisibility>>,
  desired: Readonly<Record<string, PolicyOverride<SkillVisibility>>>,
): void {
  for (const id of allIds(previous, desired)) {
    const before = previous[id] ?? "visible";
    const requested = desired[id] ?? "visible";
    const after = requested === "inherit" ? "visible" : requested;
    if (before !== after) output.push({ scope: "global", kind: "skill", id, before, after });
  }
}

function collectScopedSkillChanges<S extends "directory" | "session">(
  output: Array<ScopedSkillPolicyChange & { readonly scope: S }>,
  scope: S,
  previous: Readonly<Record<string, SkillVisibility>>,
  desired: Readonly<Record<string, PolicyOverride<SkillVisibility>>>,
): void {
  for (const id of allIds(previous, desired)) {
    const before = previous[id] ?? "inherit";
    const after = desired[id] ?? "inherit";
    if (before !== after) output.push({ scope, kind: "skill", id, before, after });
  }
}

function collectInstructionChanges<S extends "directory" | "session">(
  output: Array<ScopedInstructionPolicyChange & { readonly scope: S }>,
  scope: S,
  previous: Readonly<Record<string, InstructionVisibility>>,
  desired: Readonly<Record<string, PolicyOverride<InstructionVisibility>>>,
): void {
  for (const id of allIds(previous, desired)) {
    const before = previous[id] ?? "inherit";
    const after = desired[id] ?? "inherit";
    if (before !== after) output.push({ scope, kind: "instruction", id, before, after });
  }
}

function allIds(
  previous: Readonly<Record<string, string>>,
  desired: Readonly<Record<string, string>>,
): string[] {
  return [...new Set([...Object.keys(previous), ...Object.keys(desired)])].sort();
}
