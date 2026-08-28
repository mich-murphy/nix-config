import { createHash, randomUUID } from "node:crypto";
import {
  closeSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";
import { PolicyConflictError, PolicyStateError } from "./policy";
import type {
  ApplyResult,
  InstructionVisibility,
  PersistedPolicySnapshot,
  PolicyChange,
  PolicyLoadInput,
  PolicyLoadResult,
  PolicyPlan,
  PolicyScope,
  PolicyStateAdapter,
  SkillVisibility,
} from "./policy";
import { resourcePathId } from "./resource-path";

const STATE_VERSION = 3;
const LEGACY_STATE_TYPE = "context-control-state";

/** Default path for persisted skill-toggle policy. */
export const DEFAULT_STATE_PATH = join(getAgentDir(), "pi-skill-toggle.json");

/** Previous extension state path used as a migration source. */
export const LEGACY_STATE_PATH = join(getAgentDir(), "context-control.json");

interface StoredStateV3 {
  version: typeof STATE_VERSION;
  globalSkillPolicy: Record<string, SkillVisibility>;
  skillPolicyByDirectory: Record<string, Record<string, SkillVisibility>>;
  instructionPolicyByDirectory: Record<string, Record<string, InstructionVisibility>>;
  legacyHiddenSkillPaths: string[];
  migratedLegacySessionIds: string[];
}

interface StoredStateV2 {
  version: 2;
  hiddenSkillNames: string[];
  legacyHiddenSkillPaths: string[];
  contextByDirectory: Record<string, string[]>;
  migratedLegacySessionIds: string[];
}

interface LegacyState {
  disabledContextPaths: string[];
  hiddenSkillPaths: string[];
  migratedLegacySessionIds?: string[];
}

/** File-store timing and test seam options. */
export interface SkillToggleStoreOptions {
  /** Maximum time spent waiting to acquire the state lock. */
  readonly lockTimeoutMs?: number;

  /** Age after which an abandoned lock file can be removed. */
  readonly staleLockMs?: number;

  /** Test seam invoked after writing a temporary file and before atomic rename. */
  readonly beforeRename?: (temporaryPath: string, destinationPath: string) => void;
}

/** Locked, atomically replaced JSON policy store. */
export class SkillToggleStore implements PolicyStateAdapter {
  private readonly lockTimeoutMs: number;
  private readonly staleLockMs: number;

  /** Create a store for the supplied current and optional legacy state paths. */
  constructor(
    private readonly path = DEFAULT_STATE_PATH,
    private readonly legacyPath = path === DEFAULT_STATE_PATH ? LEGACY_STATE_PATH : undefined,
    private readonly options: SkillToggleStoreOptions = {},
  ) {
    this.lockTimeoutMs = options.lockTimeoutMs ?? 2_000;
    this.staleLockMs = options.staleLockMs ?? 30_000;
  }

  /** Load and, when needed, migrate policy state without throwing expected I/O failures. */
  load(input: PolicyLoadInput): PolicyLoadResult {
    try {
      return { _tag: "ok", value: this.loadUnsafe(input) };
    } catch (cause) {
      return { _tag: "err", error: stateError("load", this.path, cause) };
    }
  }

  /** Apply a persistent plan without throwing expected I/O or lock failures. */
  apply(plan: PolicyPlan): ApplyResult {
    if (plan.scope === "session") {
      const error = new PolicyStateError("apply", "Session policy is not persisted");
      return { applied: [], skipped: plan.changes, errors: [error] };
    }
    try {
      return this.applyUnsafe(plan);
    } catch (cause) {
      return {
        applied: [],
        skipped: plan.changes,
        errors: [stateError("apply", this.path, cause)],
      };
    }
  }

  /** Reset persistent policy without throwing expected I/O or lock failures. */
  reset(scope: PolicyScope | "all", cwd: string): ApplyResult {
    if (scope === "session") return { applied: [], skipped: [], errors: [] };
    try {
      return this.resetUnsafe(scope, cwd);
    } catch (cause) {
      return { applied: [], skipped: [], errors: [stateError("reset", this.path, cause)] };
    }
  }

  private loadUnsafe(input: PolicyLoadInput): PersistedPolicySnapshot {
    const initial = this.readState(input.cwd);
    const prepared = prepareState(initial.state, input);
    if (!initial.needsWrite && !prepared.changed) return snapshot(prepared.state, input.cwd);

    return this.withLock(() => {
      const current = this.readState(input.cwd);
      const next = prepareState(current.state, input);
      this.writeState(next.state);
      return snapshot(next.state, input.cwd);
    });
  }

  private applyUnsafe(plan: PolicyPlan): ApplyResult {
    return this.withLock(() => {
      const state = this.readState(plan.cwd).state;
      const directory = directoryId(plan.cwd);
      const applied: PolicyChange[] = [];
      const skipped: PolicyChange[] = [];

      for (const change of plan.changes) {
        const current = scopedValue(state, directory, change);
        if (current !== change.before) {
          skipped.push(change);
          continue;
        }
        applyChange(state, directory, change);
        applied.push(change);
      }

      if (applied.length > 0) this.writeState(state);
      return {
        applied,
        skipped,
        errors: skipped.map((change) => new PolicyConflictError(change)),
        snapshot: snapshot(state, plan.cwd),
      };
    });
  }

  private resetUnsafe(scope: Exclude<PolicyScope, "session"> | "all", cwd: string): ApplyResult {
    return this.withLock(() => {
      const state = this.readState(cwd).state;
      const directory = directoryId(cwd);
      const changes: PolicyChange[] = [];
      if (scope === "global" || scope === "all") {
        for (const [id, before] of Object.entries(state.globalSkillPolicy)) {
          changes.push({ scope: "global", kind: "skill", id, before, after: "visible" });
        }
        state.globalSkillPolicy = {};
      }
      if (scope === "directory") {
        collectResetChanges(changes, directory, state);
        delete state.skillPolicyByDirectory[directory];
        delete state.instructionPolicyByDirectory[directory];
      } else if (scope === "all") {
        for (const key of new Set([
          ...Object.keys(state.skillPolicyByDirectory),
          ...Object.keys(state.instructionPolicyByDirectory),
        ])) collectResetChanges(changes, key, state);
        state.skillPolicyByDirectory = {};
        state.instructionPolicyByDirectory = {};
        state.legacyHiddenSkillPaths = [];
      }
      if (changes.length > 0 || scope === "all") this.writeState(state);
      return { applied: changes, skipped: [], errors: [], snapshot: snapshot(state, cwd) };
    });
  }

  private readState(cwd: string): ReadResult {
    const current = readStoredState(this.path, cwd);
    if (current.exists || !this.legacyPath) return current;
    const legacy = readStoredState(this.legacyPath, cwd);
    return legacy.exists ? { ...legacy, needsWrite: true } : current;
  }

  private writeState(state: StoredStateV3): void {
    writeStoredState(state, this.path, this.options.beforeRename);
  }

  private withLock<T>(operation: () => T): T {
    const lockPath = `${this.path}.lock`;
    mkdirSync(dirname(this.path), { recursive: true });
    const deadline = Date.now() + this.lockTimeoutMs;
    let descriptor: number | undefined;
    while (descriptor === undefined) {
      try {
        descriptor = openSync(lockPath, "wx", 0o600);
      } catch (error) {
        if (!isNodeError(error) || error.code !== "EEXIST") throw error;
        removeStaleLock(lockPath, this.staleLockMs);
        if (Date.now() >= deadline) throw new Error(`Timed out waiting for pi-skill-toggle state lock: ${lockPath}`);
        sleepSync(Math.min(20, this.lockTimeoutMs));
      }
    }
    try {
      return operation();
    } finally {
      closeSync(descriptor);
      rmSync(lockPath, { force: true });
    }
  }
}

interface ReadResult {
  state: StoredStateV3;
  needsWrite: boolean;
  exists: boolean;
}

function prepareState(input: StoredStateV3, options: PolicyLoadInput): { state: StoredStateV3; changed: boolean } {
  const state = copyState(input);
  let changed = false;
  if (options.sessionId && options.legacyEntries && !state.migratedLegacySessionIds.includes(options.sessionId)) {
    const legacy = readLegacyStateFromBranch(options.legacyEntries);
    if (legacy.disabledContextPaths.length > 0 || legacy.hiddenSkillPaths.length > 0) {
      const directory = directoryId(options.cwd);
      const instructions = state.instructionPolicyByDirectory[directory] ?? {};
      for (const path of legacy.disabledContextPaths) instructions[resourcePathId(path, options.cwd)] = "excluded";
      state.instructionPolicyByDirectory[directory] = instructions;
      state.legacyHiddenSkillPaths.push(...legacy.hiddenSkillPaths.map((path) => resourcePathId(path, options.cwd)));
      state.migratedLegacySessionIds.push(options.sessionId);
      changed = true;
    }
  }

  const skillNameByPath = new Map((options.skills ?? []).map((skill) => [resourcePathId(skill.filePath, options.cwd), normalizeName(skill.name)]));
  const unresolved: string[] = [];
  for (const path of state.legacyHiddenSkillPaths) {
    const name = skillNameByPath.get(path);
    if (name) {
      state.globalSkillPolicy[name] = "manual-only";
      changed = true;
    } else unresolved.push(path);
  }
  if (unresolved.length !== state.legacyHiddenSkillPaths.length) state.legacyHiddenSkillPaths = unresolved;
  return { state: normalizeState(state), changed };
}

function readStoredState(path: string, cwd: string): ReadResult {
  let content: string;
  try {
    content = readFileSync(path, "utf8");
  } catch (error) {
    if (isNodeError(error) && error.code === "ENOENT") return { state: emptyState(), needsWrite: false, exists: false };
    throw error;
  }

  let value: unknown;
  try {
    value = JSON.parse(content);
  } catch (error) {
    throw new Error(`Malformed Pi skill-toggle state at ${path}. Fix or move the file, then run /skill-status. ${errorMessage(error)}`);
  }
  if (isStoredStateV3(value)) return { state: normalizeState(value), needsWrite: false, exists: true };
  if (isStoredStateV2(value)) return { state: migrateV2(value), needsWrite: true, exists: true };
  if (isLegacyState(value)) return { state: migrateLegacy(value, cwd), needsWrite: true, exists: true };
  const version = isRecord(value) && "version" in value ? String(value.version) : "missing";
  throw new Error(`Invalid Pi skill-toggle state at ${path} (version ${version}). Fix or move the file, then run /skill-status.`);
}

function migrateV2(value: StoredStateV2): StoredStateV3 {
  return normalizeState({
    ...emptyState(),
    globalSkillPolicy: Object.fromEntries(value.hiddenSkillNames.map((name) => [name, "manual-only"])),
    instructionPolicyByDirectory: Object.fromEntries(Object.entries(value.contextByDirectory).map(([directory, paths]) => [
      directory,
      Object.fromEntries(paths.map((path) => [path, "excluded"])),
    ])),
    legacyHiddenSkillPaths: [...value.legacyHiddenSkillPaths],
    migratedLegacySessionIds: [...value.migratedLegacySessionIds],
  });
}

function migrateLegacy(value: LegacyState, cwd: string): StoredStateV3 {
  return normalizeState({
    ...emptyState(),
    instructionPolicyByDirectory: {
      [directoryId(cwd)]: Object.fromEntries(value.disabledContextPaths.map((path) => [resourcePathId(path, cwd), "excluded"])),
    },
    legacyHiddenSkillPaths: value.hiddenSkillPaths.map((path) => resourcePathId(path, cwd)),
    migratedLegacySessionIds: [...(value.migratedLegacySessionIds ?? [])],
  });
}

function snapshot(state: StoredStateV3, cwd: string): PersistedPolicySnapshot {
  const normalized = normalizeState(state);
  const directory = directoryId(cwd);
  return {
    cwd: directory,
    generation: generation(normalized),
    globalSkills: { ...normalized.globalSkillPolicy },
    directorySkills: { ...(normalized.skillPolicyByDirectory[directory] ?? {}) },
    directoryInstructions: { ...(normalized.instructionPolicyByDirectory[directory] ?? {}) },
  };
}

function scopedValue(state: StoredStateV3, directory: string, change: PolicyChange): string {
  if (change.scope === "global") return state.globalSkillPolicy[change.id] ?? "visible";
  if (change.kind === "skill") return state.skillPolicyByDirectory[directory]?.[change.id] ?? "inherit";
  return state.instructionPolicyByDirectory[directory]?.[change.id] ?? "inherit";
}

function applyChange(state: StoredStateV3, directory: string, change: PolicyChange): void {
  if (change.scope === "global") {
    if (change.after === "visible") delete state.globalSkillPolicy[change.id];
    else state.globalSkillPolicy[normalizeName(change.id)] = change.after;
    return;
  }
  if (change.kind === "skill") {
    const current = state.skillPolicyByDirectory[directory] ?? {};
    if (change.after === "inherit") delete current[change.id];
    else current[normalizeName(change.id)] = change.after;
    setSparseDirectoryPolicy(state.skillPolicyByDirectory, directory, current);
    return;
  }
  const current = state.instructionPolicyByDirectory[directory] ?? {};
  if (change.after === "inherit") delete current[change.id];
  else current[resourcePathId(change.id, directory)] = change.after;
  setSparseDirectoryPolicy(state.instructionPolicyByDirectory, directory, current);
}

function setSparseDirectoryPolicy<T extends string>(
  policies: Record<string, Record<string, T>>,
  directory: string,
  current: Record<string, T>,
): void {
  if (Object.keys(current).length > 0) policies[directory] = current;
  else delete policies[directory];
}

function collectResetChanges(output: PolicyChange[], directory: string, state: StoredStateV3): void {
  for (const [id, before] of Object.entries(state.skillPolicyByDirectory[directory] ?? {})) {
    output.push({ scope: "directory", kind: "skill", id, before, after: "inherit" });
  }
  for (const [id, before] of Object.entries(state.instructionPolicyByDirectory[directory] ?? {})) {
    output.push({ scope: "directory", kind: "instruction", id, before, after: "inherit" });
  }
}

function writeStoredState(state: StoredStateV3, path: string, beforeRename?: SkillToggleStoreOptions["beforeRename"]): void {
  const normalized = normalizeState(state);
  mkdirSync(dirname(path), { recursive: true });
  const temporaryPath = `${path}.${process.pid}.${randomUUID()}.tmp`;
  try {
    writeFileSync(temporaryPath, `${JSON.stringify(normalized, null, 2)}\n`, { encoding: "utf8", mode: 0o600 });
    beforeRename?.(temporaryPath, path);
    renameSync(temporaryPath, path);
  } finally {
    rmSync(temporaryPath, { force: true });
  }
}

function normalizeState(state: StoredStateV3): StoredStateV3 {
  return {
    version: STATE_VERSION,
    globalSkillPolicy: normalizeSkillPolicy(state.globalSkillPolicy, true),
    skillPolicyByDirectory: normalizeDirectoryPolicies(state.skillPolicyByDirectory, normalizeSkillPolicy),
    instructionPolicyByDirectory: normalizeDirectoryPolicies(state.instructionPolicyByDirectory, normalizeInstructionPolicy),
    legacyHiddenSkillPaths: uniqueSorted(state.legacyHiddenSkillPaths.map((path) => resourcePathId(path))),
    migratedLegacySessionIds: uniqueSorted(state.migratedLegacySessionIds),
  };
}

function normalizeDirectoryPolicies<T extends string>(
  policies: Record<string, Record<string, T>>,
  normalizePolicy: (policy: Record<string, T>) => Record<string, T>,
): Record<string, Record<string, T>> {
  const entries = Object.entries(policies).map(([directory, policy]) => [directoryId(directory), normalizePolicy(policy)] as const);
  return Object.fromEntries(entries.filter(([, policy]) => Object.keys(policy).length > 0).sort(([a], [b]) => a.localeCompare(b)));
}

function normalizeSkillPolicy(policy: Record<string, SkillVisibility>, global = false): Record<string, SkillVisibility> {
  return Object.fromEntries(Object.entries(policy)
    .map(([name, value]) => [normalizeName(name), value] as const)
    .filter(([name, value]) => name.length > 0 && (!global || value !== "visible"))
    .sort(([a], [b]) => a.localeCompare(b)));
}

function normalizeInstructionPolicy(policy: Record<string, InstructionVisibility>): Record<string, InstructionVisibility> {
  return Object.fromEntries(Object.entries(policy).map(([path, value]) => [resourcePathId(path), value] as const).sort(([a], [b]) => a.localeCompare(b)));
}

function emptyState(): StoredStateV3 {
  return { version: STATE_VERSION, globalSkillPolicy: {}, skillPolicyByDirectory: {}, instructionPolicyByDirectory: {}, legacyHiddenSkillPaths: [], migratedLegacySessionIds: [] };
}

function copyState(state: StoredStateV3): StoredStateV3 {
  return structuredClone(state);
}

/** `state` must already be normalized; the only caller (`snapshot`) guarantees this. */
function generation(state: StoredStateV3): string {
  return createHash("sha256").update(JSON.stringify(state)).digest("hex").slice(0, 16);
}

function readLegacyStateFromBranch(entries: Iterable<unknown>): LegacyState {
  let current: LegacyState = { disabledContextPaths: [], hiddenSkillPaths: [] };
  for (const entry of entries) {
    if (isRecord(entry) && entry.type === "custom" && entry.customType === LEGACY_STATE_TYPE && isLegacyState(entry.data)) {
      current = { disabledContextPaths: [...entry.data.disabledContextPaths], hiddenSkillPaths: [...entry.data.hiddenSkillPaths] };
    }
  }
  return current;
}

function isStoredStateV3(value: unknown): value is StoredStateV3 {
  return isRecord(value) && value.version === STATE_VERSION && isVisibilityRecord(value.globalSkillPolicy, ["visible", "manual-only"])
    && isNestedVisibilityRecord(value.skillPolicyByDirectory, ["visible", "manual-only"])
    && isNestedVisibilityRecord(value.instructionPolicyByDirectory, ["included", "excluded"])
    && isStringArray(value.legacyHiddenSkillPaths) && isStringArray(value.migratedLegacySessionIds);
}

function isStoredStateV2(value: unknown): value is StoredStateV2 {
  return isRecord(value) && value.version === 2 && isStringArray(value.hiddenSkillNames) && isStringArray(value.legacyHiddenSkillPaths)
    && isStringArrayRecord(value.contextByDirectory) && isStringArray(value.migratedLegacySessionIds);
}

function isLegacyState(value: unknown): value is LegacyState {
  return isRecord(value) && isStringArray(value.disabledContextPaths) && isStringArray(value.hiddenSkillPaths)
    && (value.migratedLegacySessionIds === undefined || isStringArray(value.migratedLegacySessionIds));
}

function isVisibilityRecord(value: unknown, values: readonly string[]): value is Record<string, never> {
  return isRecord(value) && Object.values(value).every((entry) => typeof entry === "string" && values.includes(entry));
}

function isNestedVisibilityRecord(value: unknown, values: readonly string[]): value is Record<string, Record<string, never>> {
  return isRecord(value) && Object.values(value).every((entry) => isVisibilityRecord(entry, values));
}

function isStringArrayRecord(value: unknown): value is Record<string, string[]> {
  return isRecord(value) && Object.values(value).every(isStringArray);
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}
function uniqueSorted(values: readonly string[]): string[] {
  return [...new Set(values)].sort();
}
function normalizeName(name: string): string {
  return name.trim();
}
function directoryId(cwd: string): string {
  return resourcePathId(cwd, cwd);
}
function removeStaleLock(path: string, staleLockMs: number): void {
  try {
    if (Date.now() - statSync(path).mtimeMs > staleLockMs) rmSync(path, { force: true });
  } catch {
    // Another process released the lock between open and stat.
  }
}
function sleepSync(milliseconds: number): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
}
function isNodeError(error: unknown): error is NodeJS.ErrnoException {
  return error instanceof Error && "code" in error;
}
function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function stateError(
  operation: "load" | "apply" | "reset",
  path: string,
  cause: unknown,
): PolicyStateError {
  const message = cause instanceof PolicyStateError
    ? cause.message
    : `Could not ${operation} Pi skill-toggle state at ${path}: ${errorMessage(cause)}`;
  return new PolicyStateError(operation, message, cause);
}
