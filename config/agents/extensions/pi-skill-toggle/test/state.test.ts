import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, test } from "bun:test";
import type { PolicyChange, PolicyPlan } from "../policy";
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

function plan(cwd: string, generation: string, scope: "global" | "directory", changes: Omit<PolicyChange, "scope">[]): PolicyPlan {
  return { cwd, generation, scope, changes: changes.map((change) => ({ ...change, scope })) };
}

describe("SkillToggleStore", () => {
  test("starts with empty layered policy", () => {
    const { store } = context();
    const loaded = store.load({ cwd: "/work/project" });
    expect(loaded.cwd).toBe("/work/project");
    expect(loaded.globalSkills).toEqual({});
    expect(loaded.directorySkills).toEqual({});
    expect(loaded.directoryInstructions).toEqual({});
    expect(loaded.generation).toHaveLength(16);
  });

  test("keeps global skills shared and directory overrides sparse", () => {
    const { store } = context();
    const first = store.load({ cwd: "/work/one" });
    store.apply(plan("/work/one", first.generation, "global", [
      { kind: "skill", id: "research", before: "visible", after: "manual-only" },
    ]));
    const directory = store.load({ cwd: "/work/one" });
    store.apply(plan("/work/one", directory.generation, "directory", [
      { kind: "skill", id: "research", before: "inherit", after: "visible" },
      { kind: "instruction", id: "/work/one/AGENTS.md", before: "inherit", after: "excluded" },
    ]));

    expect(store.load({ cwd: "/work/one" })).toMatchObject({
      globalSkills: { research: "manual-only" },
      directorySkills: { research: "visible" },
      directoryInstructions: { "/work/one/AGENTS.md": "excluded" },
    });
    expect(store.load({ cwd: "/work/two" })).toMatchObject({
      globalSkills: { research: "manual-only" },
      directorySkills: {},
      directoryInstructions: {},
    });
  });

  test("returning an override to inherit removes it from persisted output", () => {
    const { store, path } = context();
    let loaded = store.load({ cwd: "/work/project" });
    store.apply(plan(loaded.cwd, loaded.generation, "directory", [
      { kind: "skill", id: "deploy", before: "inherit", after: "manual-only" },
    ]));
    loaded = store.load({ cwd: "/work/project" });
    store.apply(plan(loaded.cwd, loaded.generation, "directory", [
      { kind: "skill", id: "deploy", before: "manual-only", after: "inherit" },
    ]));

    expect(store.load({ cwd: "/work/project" }).directorySkills).toEqual({});
    expect(JSON.parse(readFileSync(path, "utf8")).skillPolicyByDirectory).toEqual({});
  });

  test("independent store instances merge stale plans while conflicting transitions are skipped", () => {
    const { store, path } = context();
    const otherStore = new SkillToggleStore(path);
    const loaded = store.load({ cwd: "/work/project" });
    const a = plan(loaded.cwd, loaded.generation, "directory", [
      { kind: "skill", id: "a", before: "inherit", after: "manual-only" },
    ]);
    const b = plan(loaded.cwd, loaded.generation, "directory", [
      { kind: "skill", id: "b", before: "inherit", after: "visible" },
    ]);
    expect(store.apply(a).applied).toHaveLength(1);
    expect(otherStore.apply(b).applied).toHaveLength(1);

    const conflict = plan(loaded.cwd, loaded.generation, "directory", [
      { kind: "skill", id: "a", before: "inherit", after: "visible" },
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
    const loaded = state.store.load({ cwd: alias });
    state.store.apply(plan(alias, loaded.generation, "directory", [
      { kind: "instruction", id: join(alias, "AGENTS.md"), before: "inherit", after: "excluded" },
    ]));
    expect(state.store.load({ cwd: project }).directoryInstructions).toEqual({
      [join(project, "AGENTS.md")]: "excluded",
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

    expect(store.load({ cwd: "/work/project" })).toMatchObject({
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
    expect(store.load({ cwd: "/work/project" }).globalSkills).toEqual({ research: "manual-only" });
    expect(existsSync(path)).toBe(true);
  });

  test("recovers a stale lock and times out on a live lock", () => {
    const stale = context({ staleLockMs: 1, lockTimeoutMs: 30 });
    mkdirSync(join(stale.path, ".."), { recursive: true });
    writeFileSync(`${stale.path}.lock`, "");
    utimesSync(`${stale.path}.lock`, new Date(0), new Date(0));
    expect(stale.store.reset("all", "/work/project").errors).toEqual([]);

    const live = context({ staleLockMs: 60_000, lockTimeoutMs: 10 });
    mkdirSync(join(live.path, ".."), { recursive: true });
    writeFileSync(`${live.path}.lock`, "");
    expect(() => live.store.reset("all", "/work/project")).toThrow("Timed out waiting");
  });

  test("cleans up atomic temporary files after a failed write", () => {
    const failed = context({ beforeRename: () => { throw new Error("injected rename failure"); } });
    expect(() => failed.store.reset("all", "/work/project")).toThrow("injected rename failure");
    expect(readdirSync(join(failed.path, "..")).filter((name) => name.endsWith(".tmp"))).toEqual([]);
  });

  test("rejects malformed and unsupported state with recovery guidance", () => {
    const malformed = context();
    mkdirSync(join(malformed.path, ".."), { recursive: true });
    writeFileSync(malformed.path, "{broken");
    expect(() => malformed.store.load({ cwd: "/work/project" })).toThrow("Fix or move the file");

    const unsupported = context();
    mkdirSync(join(unsupported.path, ".."), { recursive: true });
    writeFileSync(unsupported.path, '{"version":99}');
    expect(() => unsupported.store.load({ cwd: "/work/project" })).toThrow("version 99");
  });
});
