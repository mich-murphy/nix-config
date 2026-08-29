import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterEach, describe, expect, test } from "bun:test";
import { resourcePathId } from "../resource-path";
import type { ToggleResource } from "../resources";
import { SkillToggleStore, type SkillToggleStateResult } from "../state";

const temporaryDirectories: string[] = [];
afterEach(() => {
  for (const directory of temporaryDirectories.splice(0)) {
    rmSync(directory, { recursive: true, force: true });
  }
});

function testContext(options = {}) {
  const directory = mkdtempSync(join(tmpdir(), "pi-skill-toggle-"));
  temporaryDirectories.push(directory);
  const statePath = join(directory, "state", "pi-skill-toggle.json");
  return { directory, statePath, store: new SkillToggleStore(statePath, options) };
}

function resource(
  path: string,
  label: string,
  origin: "global" | "project" = "project",
  editability: ToggleResource["editability"] = "editable",
): ToggleResource {
  return {
    id: resourcePathId(path),
    kind: "skill",
    origin,
    owner: resourcePathId(dirname(dirname(path))),
    label,
    description: label,
    editability,
    order: 0,
  };
}

function loaded(result: SkillToggleStateResult) {
  expect(result._tag).toBe("ok");
  if (result._tag === "err") throw result.error;
  return result.value;
}

describe("SkillToggleStore", () => {
  test("stores disabled resources by path and preserves same-named resources independently", () => {
    const context = testContext();
    const firstPath = join(context.directory, "client-a", "deploy", "SKILL.md");
    const secondPath = join(context.directory, "client-b", "deploy", "SKILL.md");
    mkdirSync(dirname(firstPath), { recursive: true });
    mkdirSync(dirname(secondPath), { recursive: true });
    writeFileSync(firstPath, "first");
    writeFileSync(secondPath, "second");
    const first = resource(firstPath, "deploy");
    const second = resource(secondPath, "deploy");

    loaded(context.store.setValue(first, "disabled", [first, second]));
    const state = loaded(context.store.setValue(second, "disabled", [first, second]));

    expect(Object.keys(state.resources)).toEqual([firstPath, secondPath]);
  });

  test("rejects invalid runtime toggle values", () => {
    const context = testContext();
    const path = join(context.directory, "invalid", "SKILL.md");
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, "skill");

    const result: unknown = Reflect.apply(
      context.store.setValue,
      context.store,
      [resource(path, "invalid"), "unexpected"],
    );

    expect(result).toMatchObject({
      _tag: "err",
      error: { _tag: "SkillToggleStateError", operation: "update" },
    });
    expect(context.store.load()).toMatchObject({ _tag: "ok", value: { resources: {} } });
  });

  test("cleanup removes missing paths while retaining every existing setting", () => {
    const context = testContext();
    const retainedPath = join(context.directory, "retained", "SKILL.md");
    const removedPath = join(context.directory, "removed", "SKILL.md");
    mkdirSync(dirname(retainedPath), { recursive: true });
    mkdirSync(dirname(removedPath), { recursive: true });
    writeFileSync(retainedPath, "retained");
    writeFileSync(removedPath, "removed");
    const retained = resource(retainedPath, "same-name");
    const removed = resource(removedPath, "same-name");
    loaded(context.store.setValue(retained, "disabled", [retained, removed]));
    loaded(context.store.setValue(removed, "disabled", [retained, removed]));

    rmSync(removedPath);
    const state = loaded(context.store.load([retained]));

    expect(Object.keys(state.resources)).toEqual([retainedPath]);
    expect(state.resources[retainedPath]).toMatchObject({ enabled: false });
  });

  test("source-authored manual-only skills cannot retain an extension override", () => {
    const context = testContext();
    const path = join(context.directory, "manual", "SKILL.md");
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, "manual");
    const editable = resource(path, "manual");
    loaded(context.store.setValue(editable, "disabled", [editable]));

    const locked = resource(path, "manual", "project", "manual-only");
    const state = loaded(context.store.load([locked]));

    expect(state.resources).toEqual({});
  });

  test("discards pre-v4 state instead of carrying layered policy forward", () => {
    const context = testContext();
    mkdirSync(dirname(context.statePath), { recursive: true });
    writeFileSync(context.statePath, JSON.stringify({
      version: 3,
      globalSkillPolicy: { research: "manual-only" },
    }));

    expect(loaded(context.store.load()).resources).toEqual({});
    expect(JSON.parse(readFileSync(context.statePath, "utf8"))).toEqual({
      version: 4,
      resources: {},
    });
  });

  test("returns malformed state as a typed failure without replacing it", () => {
    const context = testContext();
    mkdirSync(dirname(context.statePath), { recursive: true });
    writeFileSync(context.statePath, "{broken");

    const result = context.store.load();

    expect(result).toMatchObject({
      _tag: "err",
      error: { _tag: "SkillToggleStateError", operation: "load" },
    });
    expect(readFileSync(context.statePath, "utf8")).toBe("{broken");
  });

  test("reports atomic replacement failures and removes temporary files", () => {
    const context = testContext({
      beforeRename: () => {
        throw new Error("injected rename failure");
      },
    });
    const path = join(context.directory, "skill", "SKILL.md");
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, "skill");

    const result = context.store.setValue(resource(path, "skill"), "disabled");

    expect(result).toMatchObject({
      _tag: "err",
      error: { _tag: "SkillToggleStateError", operation: "update" },
    });
    expect(readdirSync(dirname(context.statePath)).filter((name) => name.endsWith(".tmp"))).toEqual([]);
  });
});
