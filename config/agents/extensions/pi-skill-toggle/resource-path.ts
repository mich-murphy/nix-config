import { realpathSync } from "node:fs";
import { normalize, resolve } from "node:path";

/** Resolve a resource path to a stable absolute identifier when possible. */
export function resourcePathId(path: string, cwd = process.cwd()): string {
  const absolute = normalize(resolve(cwd, path));
  try {
    return realpathSync.native(absolute);
  } catch {
    return absolute;
  }
}
