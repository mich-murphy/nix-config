---
name: markdown
description: Review all markdown files for correct syntax and formatting. Used when creating or editing markdown files.
---

# Validate Markdown Syntax & Formatting

Run `markdownlint-cli2` on markdown files using nix shell. Fix issues until linter passes.

## Commands

Lint all project markdown files (respects `.markdownlint-cli2.jsonc` config):

```sh
npx --yes markdownlint-cli2 "**/*.md" "#node_modules" "#.claude/skills"
```

Auto-fix what linter can handle (blank lines, whitespace):

```sh
npx --yes markdownlint-cli2 --fix "**/*.md" "#node_modules" "#.claude/skills"
```

## Workflow

1. Run lint command. Find all errors.
2. Run `--fix` for trivial stuff (MD022 blank
   lines around headings, MD032 blank lines around
   lists, etc.)
3. Hand-fix remaining errors `--fix` can't solve
   (MD060 table spacing, MD036
   emphasis-as-heading, etc.)
4. Re-run lint command. Confirm 0 errors.
