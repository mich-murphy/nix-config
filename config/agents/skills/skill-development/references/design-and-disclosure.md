# Design and Progressive Disclosure

The discovery description owns when the skill should trigger. Include the job,
positive contexts, and meaningful boundaries there. Keep exact model names out
of portable frontmatter.

Keep `SKILL.md` as the short operational path: inputs, decisions, ordered work,
completion, and direct resource routes. Link every optional reference directly
from it and say when to load that branch. References must not require another
reference to reveal an operational requirement.

Use scripts for repeated parsing, generation, hashing, validation, and schema
checks. Use assets for output templates that an agent copies or fills. Remove
unused examples and placeholder files.

At roughly 500 lines, inspect the skill instead of splitting mechanically.
Delete material that duplicates the model's general knowledge, repeats another
authority, documents abandoned evolution, expands every possible branch, or
does not change observed behavior. Resource-navigation traces reveal content
that is missed, loaded unnecessarily, or placed in the wrong branch.
