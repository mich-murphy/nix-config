import { normalize, resolve } from "node:path";

declare const resourcePathBrand: unique symbol;

/** Absolute normalized identity for a Pi-loaded resource or its owner directory. */
export type ResourcePath = string & { readonly [resourcePathBrand]: true };

/** Parse a resource path into an absolute identity without dereferencing symlinks. */
export function resourcePathId(path: string, cwd = process.cwd()): ResourcePath {
  // SAFETY: resolve() makes the path absolute and normalize() removes lexical ambiguity. The brand is private to this parser.
  return normalize(resolve(cwd, path)) as ResourcePath;
}
