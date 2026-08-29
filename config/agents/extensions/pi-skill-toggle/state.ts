import { randomUUID } from "node:crypto";
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
import type {
  ToggleResource,
  ToggleResourceKind,
  ToggleResourceOrigin,
} from "./resources";
import { resourcePathId, type ResourcePath } from "./resource-path";

const STATE_VERSION = 4;

const DEFAULT_STATE_PATH = join(getAgentDir(), "pi-skill-toggle.json");

/** One disabled resource retained in persistent state. */
export interface StoredToggleResource {
  readonly kind: ToggleResourceKind;
  readonly origin: ToggleResourceOrigin;
  readonly owner: ResourcePath;
  readonly enabled: false;
}

/** Parsed version 4 skill-toggle state. */
export interface SkillToggleState {
  readonly version: typeof STATE_VERSION;
  readonly resources: Readonly<Record<string, StoredToggleResource>>;
}

/** Requested persisted value for an editable resource. */
export type ResourceToggleValue = "enabled" | "disabled";

/** Operation that can fail at the persistent state boundary. */
export type SkillToggleStateOperation = "load" | "update";

/** Expected persistent state or lock failure. */
export class SkillToggleStateError extends Error {
  /** Stable discriminator for state failures. */
  readonly _tag = "SkillToggleStateError" as const;

  /** State operation that failed. */
  readonly operation: SkillToggleStateOperation;

  /** Lower-level failure retained for local diagnosis. */
  override readonly cause: unknown;

  /** Create a classified persistent state failure. */
  constructor(operation: SkillToggleStateOperation, message: string, cause?: unknown) {
    super(message);
    this.name = "SkillToggleStateError";
    this.operation = operation;
    this.cause = cause;
  }
}

/** Result returned by state load and update operations. */
export type SkillToggleStateResult =
  | { readonly _tag: "ok"; readonly value: SkillToggleState }
  | { readonly _tag: "err"; readonly error: SkillToggleStateError };

/** File-store timing and test seam options. */
export interface SkillToggleStoreOptions {
  /** Maximum time spent waiting for the state lock. */
  readonly lockTimeoutMs?: number;

  /** Age after which an abandoned lock can be removed. */
  readonly staleLockMs?: number;

  /** Test seam invoked before atomic replacement. */
  readonly beforeRename?: (temporaryPath: string, destinationPath: string) => void;
}

/** State operations required by the extension command and prompt handler. */
export interface SkillToggleStateStore {
  /** Load and synchronize persistent state. */
  load(resources?: ReadonlyArray<ToggleResource>): SkillToggleStateResult;

  /** Enable or disable one resource. */
  setValue(
    resource: ToggleResource,
    value: ResourceToggleValue,
    resources?: ReadonlyArray<ToggleResource>,
  ): SkillToggleStateResult;
}

/** Persistent resource-toggle store using locked atomic replacement. */
export class SkillToggleStore implements SkillToggleStateStore {
  private readonly lockTimeoutMs: number;
  private readonly staleLockMs: number;

  /** Create a store at the supplied path. */
  constructor(
    private readonly path = DEFAULT_STATE_PATH,
    private readonly options: SkillToggleStoreOptions = {},
  ) {
    this.lockTimeoutMs = options.lockTimeoutMs ?? 2_000;
    this.staleLockMs = options.staleLockMs ?? 30_000;
  }

  /** Load state, discard pre-v4 state, and remove entries whose source path no longer exists. */
  load(resources: ReadonlyArray<ToggleResource> = []): SkillToggleStateResult {
    return this.run("load", () => this.withLock(() => {
      const loaded = this.readState();
      const synchronized = synchronizeState(loaded.state, resources);
      if (loaded.needsWrite || synchronized.changed) this.writeState(synchronized.state);
      return synchronized.state;
    }));
  }

  /** Enable or disable one resource while preserving every unrelated setting. */
  setValue(
    resource: ToggleResource,
    value: ResourceToggleValue,
    resources: ReadonlyArray<ToggleResource> = [],
  ): SkillToggleStateResult {
    if (!isResourceToggleValue(value)) {
      return {
        _tag: "err",
        error: new SkillToggleStateError("update", `Unsupported resource toggle value: ${String(value)}`),
      };
    }
    return this.run("update", () => this.withLock(() => {
      const loaded = this.readState();
      const synchronized = synchronizeState(loaded.state, resources);
      const current = { ...synchronized.state.resources };
      if (value === "enabled" || resource.editability === "manual-only") {
        delete current[resource.id];
      } else {
        current[resource.id] = storedResource(resource);
      }
      const state = normalizeState({ version: STATE_VERSION, resources: current });
      this.writeState(state);
      return state;
    }));
  }

  private run(
    operation: SkillToggleStateOperation,
    effect: () => SkillToggleState,
  ): SkillToggleStateResult {
    try {
      return { _tag: "ok", value: effect() };
    } catch (cause) {
      const detail = cause instanceof Error ? cause.message : String(cause);
      return {
        _tag: "err",
        error: new SkillToggleStateError(
          operation,
          `Could not ${operation} Pi skill-toggle state at ${this.path}: ${detail}`,
          cause,
        ),
      };
    }
  }

  private readState(): { state: SkillToggleState; needsWrite: boolean } {
    let content: string;
    try {
      content = readFileSync(this.path, "utf8");
    } catch (cause) {
      if (isNodeError(cause) && cause.code === "ENOENT") {
        return { state: emptyState(), needsWrite: false };
      }
      throw cause;
    }

    let value: unknown;
    try {
      value = JSON.parse(content);
    } catch (cause) {
      const detail = cause instanceof Error ? cause.message : String(cause);
      throw new Error(`Malformed state. Fix or remove the file. ${detail}`);
    }
    const parsed = parseSkillToggleState(value);
    if (parsed) {
      return {
        state: parsed,
        needsWrite: JSON.stringify(value) !== JSON.stringify(parsed),
      };
    }
    if (isRecord(value) && typeof value.version === "number" && value.version < STATE_VERSION) {
      return { state: emptyState(), needsWrite: true };
    }
    const version = isRecord(value) && "version" in value ? String(value.version) : "missing";
    throw new Error(`Unsupported state version ${version}. Fix or remove the file.`);
  }

  private writeState(state: SkillToggleState): void {
    const normalized = normalizeState(state);
    mkdirSync(dirname(this.path), { recursive: true });
    const temporaryPath = `${this.path}.${process.pid}.${randomUUID()}.tmp`;
    try {
      writeFileSync(temporaryPath, `${JSON.stringify(normalized, null, 2)}\n`, {
        encoding: "utf8",
        mode: 0o600,
      });
      this.options.beforeRename?.(temporaryPath, this.path);
      renameSync(temporaryPath, this.path);
    } finally {
      rmSync(temporaryPath, { force: true });
    }
  }

  private withLock<T>(effect: () => T): T {
    const lockPath = `${this.path}.lock`;
    mkdirSync(dirname(this.path), { recursive: true });
    const deadline = Date.now() + this.lockTimeoutMs;
    let descriptor: number | undefined;
    while (descriptor === undefined) {
      try {
        const openedDescriptor = openSync(lockPath, "wx", 0o600);
        try {
          writeFileSync(openedDescriptor, `${process.pid}\n`, "utf8");
          descriptor = openedDescriptor;
        } catch (cause) {
          closeSync(openedDescriptor);
          rmSync(lockPath, { force: true });
          throw cause;
        }
      } catch (cause) {
        if (!isNodeError(cause) || cause.code !== "EEXIST") throw cause;
        removeAbandonedLock(lockPath, this.staleLockMs);
        if (Date.now() >= deadline) throw new Error(`Timed out waiting for state lock: ${lockPath}`);
        sleepSync(Math.min(20, this.lockTimeoutMs));
      }
    }
    try {
      return effect();
    } finally {
      closeSync(descriptor);
      rmSync(lockPath, { force: true });
    }
  }
}

function synchronizeState(
  state: SkillToggleState,
  resources: ReadonlyArray<ToggleResource>,
): { state: SkillToggleState; changed: boolean } {
  const discovered = new Map<string, ToggleResource>(
    resources.map((resource) => [resource.id, resource]),
  );
  const next: Record<string, StoredToggleResource> = {};
  let changed = false;
  for (const [path, stored] of Object.entries(state.resources)) {
    if (!resourceExists(path)) {
      changed = true;
      continue;
    }
    const resource = discovered.get(path);
    if (resource?.editability === "manual-only") {
      changed = true;
      continue;
    }
    const updated = resource ? storedResource(resource) : stored;
    next[path] = updated;
    if (!storedResourcesEqual(stored, updated)) changed = true;
  }
  return {
    state: normalizeState({ version: STATE_VERSION, resources: next }),
    changed,
  };
}

function isResourceToggleValue(value: unknown): value is ResourceToggleValue {
  return value === "enabled" || value === "disabled";
}

function storedResource(resource: ToggleResource): StoredToggleResource {
  return {
    kind: resource.kind,
    origin: resource.origin,
    owner: resource.owner,
    enabled: false,
  };
}

function storedResourcesEqual(left: StoredToggleResource, right: StoredToggleResource): boolean {
  return left.kind === right.kind && left.origin === right.origin && left.owner === right.owner;
}

function normalizeState(state: SkillToggleState): SkillToggleState {
  const entries = Object.entries(state.resources)
    .map(([path, resource]) => [resourcePathId(path), {
      kind: resource.kind,
      origin: resource.origin,
      owner: resourcePathId(resource.owner),
      enabled: false as const,
    }] as const)
    .sort(([left], [right]) => left.localeCompare(right));
  return { version: STATE_VERSION, resources: Object.fromEntries(entries) };
}

function emptyState(): SkillToggleState {
  return { version: STATE_VERSION, resources: {} };
}

function parseSkillToggleState(value: unknown): SkillToggleState | undefined {
  if (!isRecord(value) || value.version !== STATE_VERSION || !isRecord(value.resources)) {
    return undefined;
  }
  const resources: Record<string, StoredToggleResource> = {};
  for (const [path, resource] of Object.entries(value.resources)) {
    if (!isRawStoredToggleResource(resource)) return undefined;
    resources[resourcePathId(path)] = {
      kind: resource.kind,
      origin: resource.origin,
      owner: resourcePathId(resource.owner),
      enabled: false,
    };
  }
  return normalizeState({ version: STATE_VERSION, resources });
}

function isRawStoredToggleResource(value: unknown): value is {
  readonly kind: ToggleResourceKind;
  readonly origin: ToggleResourceOrigin;
  readonly owner: string;
  readonly enabled: false;
} {
  return isRecord(value)
    && (value.kind === "instruction" || value.kind === "skill")
    && (value.origin === "global" || value.origin === "project")
    && typeof value.owner === "string"
    && value.enabled === false;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function resourceExists(path: string): boolean {
  try {
    statSync(path);
    return true;
  } catch (cause) {
    if (isNodeError(cause) && (cause.code === "ENOENT" || cause.code === "ENOTDIR")) return false;
    throw cause;
  }
}

function removeAbandonedLock(path: string, staleLockMs: number): void {
  try {
    const owner = Number.parseInt(readFileSync(path, "utf8").trim(), 10);
    if (Number.isSafeInteger(owner) && owner > 0) {
      try {
        process.kill(owner, 0);
        return;
      } catch (cause) {
        if (!isNodeError(cause) || cause.code !== "ESRCH") return;
        rmSync(path, { force: true });
        return;
      }
    }
    if (Date.now() - statSync(path).mtimeMs > staleLockMs) rmSync(path, { force: true });
  } catch {
    // Another process released the lock between open and inspection.
  }
}

function sleepSync(milliseconds: number): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
}

function isNodeError(error: unknown): error is NodeJS.ErrnoException {
  return error instanceof Error && "code" in error;
}
