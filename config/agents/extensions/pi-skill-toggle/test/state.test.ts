import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, test } from "bun:test";
import type {
  DirectoryPolicyPlan,
  GlobalPolicyPlan,
  PersistedPolicySnapshot,
  PolicyLoadInput,
  PolicyLoadResult,
} from "../policy";
import { SkillToggleStore } from "../state";

const temporaryDirectories: string[] = [];
afterEach(() => {
  for (const directory of temporaryDirectories.splice(0)) rmSync(directory, { recursive: true, force: true });
});

function context(options = {}) {
  const directory = mkdtempSync(join(tmpdir(), "pi-skill-toggle-"));
  temporaryDirectories.push(directory);
  const path = join(directory, "nested", "pi-skill-toggle.json");
  return { directory, path, store: new SkillToggleStore(path, undefined, options) };
}

function loaded(result: PolicyLoadResult): PersistedPolicySnapshot {
  expect(result._tag).toBe("ok");
  if (result._tag === "err") throw result.error;
  return result.value;
}

function load(store: SkillToggleStore, input: PolicyLoadInput): PersistedPolicySnapshot {
  return loaded(store.load(input));
}

function globalPlan(
  cwd: string,
  generation: string,
  changes: GlobalPolicyPlan["changes"],
): GlobalPolicyPlan {
  return { cwd, generation, scope: "global", changes };
}

function directoryPlan(
  cwd: string,
  generation: string,
  changes: DirectoryPolicyPlan["changes"],
): DirectoryPolicyPlan {
  return { cwd, generation, scope: "directory", changes };
}

describe("SkillToggleStore", () => {
  test("starts with empty layered policy", () => {
    const { store } = context();
    const current = load(store, { cwd: "/work/project" });
    expect(current.cwd).toBe("/work/project");
    expect(current.globalSkills).toEqual({});
    expect(current.directorySkills).toEqual({});
    expect(current.directoryInstructions).toEqual({});
    expect(current.generation).toHaveLength(16);
  });

  test("keeps global skills shared and directory overrides sparse", () => {
    const { store } = context();
    const first = load(store, { cwd: "/work/one" });
    store.apply(globalPlan("/work/one", first.generation, [
      { scope: "global", kind: "skill", id: "research", before: "visible", after: "manual-only" },
    ]));
    const directory = load(store, { cwd: "/work/one" });
    store.apply(directoryPlan("/work/one", directory.generation, [
      { scope: "directory", kind: "skill", id: "research", before: "inherit", after: "visible" },
      { scope: "directory", kind: "instruction", id: "/work/one/AGENTS.md", before: "inherit", after: "excluded" },
    ]));

    expect(load(store, { cwd: "/work/one" })).toMatchObject({
      globalSkills: { research: "manual-only" },
      directorySkills: { research: "visible" },
      directoryInstructions: { "/work/one/AGENTS.md": "excluded" },
    });
    expect(load(store, { cwd: "/work/two" })).toMatchObject({
      globalSkills: { research: "manual-only" },
      directorySkills: {},
      directoryInstructions: {},
    });
  });

  test("returning an override to inherit removes it from persisted output", () => {
    const { store, path } = context();
    let current = load(store, { cwd: "/work/project" });
    store.apply(directoryPlan(current.cwd, current.generation, [
      { scope: "directory", kind: "skill", id: "deploy", before: "inherit", after: "manual-only" },
    ]));
    current = load(store, { cwd: "/work/project" });
    store.apply(directoryPlan(current.cwd, current.generation, [
      { scope: "directory", kind: "skill", id: "deploy", before: "manual-only", after: "inherit" },
    ]));

    expect(load(store, { cwd: "/work/project" }).directorySkills).toEqual({});
    expect(JSON.parse(readFileSync(path, "utf8")).skillPolicyByDirectory).toEqual({});
  });

  test("independent store instances merge stale plans while conflicting transitions are skipped", () => {
    const { store, path } = context();
    const otherStore = new SkillToggleStore(path);
    const current = load(store, { cwd: "/work/project" });
    const a = directoryPlan(current.cwd, current.generation, [
      { scope: "directory", kind: "skill", id: "a", before: "inherit", after: "manual-only" },
    ]);
    const b = directoryPlan(current.cwd, current.generation, [
      { scope: "directory", kind: "skill", id: "b", before: "inherit", after: "visible" },
    ]);
    expect(store.apply(a).applied).toHaveLength(1);
    expect(otherStore.apply(b).applied).toHaveLength(1);

    const conflict = directoryPlan(current.cwd, current.generation, [
      { scope: "directory", kind: "skill", id: "a", before: "inherit", after: "visible" },
    ]);
    expect(store.apply(conflict)).toMatchObject({ applied: [], skipped: conflict.changes });
  });

  test("canonicalizes symlinked working directories and resource paths", () => {
    const state = context();
    const project = join(state.directory, "project");
    const alias = join(state.directory, "alias");
    mkdirSync(project);
    writeFileSync(join(project, "AGENTS.md"), "rules");
    symlinkSync(project, alias);
    const current = load(state.store, { cwd: alias });
    state.store.apply(directoryPlan(alias, current.generation, [
      { scope: "directory", kind: "instruction", id: join(alias, "AGENTS.md"), before: "inherit", after: "excluded" },
    ]));
    expect(load(state.store, { cwd: project }).directoryInstructions).toEqual({
      [realpathSync.native(join(project, "AGENTS.md"))]: "excluded",
    });
  });

  test("migrates version 2 global skills and directory instructions", () => {
    const { store, path } = context();
    mkdirSync(join(path, ".."), { recursive: true });
    writeFileSync(path, JSON.stringify({
      version: 2,
      hiddenSkillNames: ["research"],
      legacyHiddenSkillPaths: [],
      contextByDirectory: { "/work/project": ["/work/project/AGENTS.md"] },
      migratedLegacySessionIds: [],
    }));

    expect(load(store, { cwd: "/work/project" })).toMatchObject({
      globalSkills: { research: "manual-only" },
      directoryInstructions: { "/work/project/AGENTS.md": "excluded" },
    });
    expect(JSON.parse(readFileSync(path, "utf8")).version).toBe(3);
  });

  test("migrates the previous extension state path", () => {
    const directory = mkdtempSync(join(tmpdir(), "pi-skill-toggle-"));
    temporaryDirectories.push(directory);
    const path = join(directory, "pi-skill-toggle.json");
    const legacyPath = join(directory, "context-control.json");
    writeFileSync(legacyPath, JSON.stringify({
      version: 2,
      hiddenSkillNames: ["research"],
      legacyHiddenSkillPaths: [],
      contextByDirectory: {},
      migratedLegacySessionIds: [],
    }));
    const store = new SkillToggleStore(path, legacyPath);
    expect(load(store, { cwd: "/work/project" }).globalSkills).toEqual({ research: "manual-only" });
    expect(existsSync(path)).toBe(true);
  });

  test("recovers a stale lock and returns a typed error for a live lock", () => {
    const stale = context({ staleLockMs: 1, lockTimeoutMs: 30 });
    mkdirSync(join(stale.path, ".."), { recursive: true });
    writeFileSync(`${stale.path}.lock`, "");
    utimesSync(`${stale.path}.lock`, new Date(0), new Date(0));
    expect(stale.store.reset("all", "/work/project").errors).toEqual([]);

    const live = context({ staleLockMs: 60_000, lockTimeoutMs: 10 });
    mkdirSync(join(live.path, ".."), { recursive: true });
    writeFileSync(`${live.path}.lock`, "");
    const result = live.store.reset("all", "/work/project");
    expect(result.errors[0]).toMatchObject({ _tag: "PolicyStateError", operation: "reset" });
    expect(result.errors[0]?.message).toContain("Timed out waiting");
  });

  test("returns write failures and cleans up atomic temporary files", () => {
    const failed = context({ beforeRename: () => { throw new Error("injected rename failure"); } });
    const result = failed.store.reset("all", "/work/project");
    expect(result.errors[0]).toMatchObject({ _tag: "PolicyStateError", operation: "reset" });
    expect(result.errors[0]?.message).toContain("injected rename failure");
    expect(readdirSync(join(failed.path, "..")).filter((name) => name.endsWith(".tmp"))).toEqual([]);
  });

  test("returns malformed and unsupported state errors with recovery guidance", () => {
    const malformed = context();
    mkdirSync(join(malformed.path, ".."), { recursive: true });
    writeFileSync(malformed.path, "{broken");
    const malformedResult = malformed.store.load({ cwd: "/work/project" });
    expect(malformedResult._tag).toBe("err");
    if (malformedResult._tag === "err") expect(malformedResult.error.message).toContain("Fix or move the file");

    const unsupported = context();
    mkdirSync(join(unsupported.path, ".."), { recursive: true });
    writeFileSync(unsupported.path, "{\"version\":99}");
    const unsupportedResult = unsupported.store.load({ cwd: "/work/project" });
    expect(unsupportedResult._tag).toBe("err");
    if (unsupportedResult._tag === "err") expect(unsupportedResult.error.message).toContain("version 99");
  });
});
