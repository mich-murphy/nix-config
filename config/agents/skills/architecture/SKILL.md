---
name: architecture
description: >
  Explore a codebase to find opportunities for architectural improvement, focusing on making
  the codebase more testable by deepening shallow modules. Use when user wants to improve
  architecture, find refactoring opportunities, consolidate tightly-coupled modules, or make
  a codebase more AI-navigable.
---

# Improve Codebase Architecture

Explore codebase like AI would. Surface architectural
friction, find testability wins, propose module-deepening
refactors as GitHub issue RFCs.

**Deep module** (Ousterhout, "A Philosophy of Software
Design"): small interface hiding big implementation.
Deep modules are more testable, more AI-navigable.
Test at boundary instead of inside.

## Process

### 1. Explore codebase

Use an available background exploration agent to navigate
the codebase organically. Do NOT follow rigid heuristics —
note where you hit friction:

- Where does understanding one concept force bouncing
  between many small files?
- Where are modules so shallow that interface is nearly
  as complex as implementation?
- Where were pure functions extracted for testability,
  but real bugs hide in how they're called?
- Where do tightly-coupled modules create integration
  risk in seams between them?
- Which parts are untested or hard to test?

Friction you encounter IS signal.

### 2. Present candidates

Show numbered list of deepening opportunities. For each:

- **Cluster**: Which modules/concepts involved
- **Why coupled**: Shared types, call patterns,
  co-ownership of concept
- **Dependency category**: See
  [REFERENCE.md](REFERENCE.md) for four categories
- **Test impact**: What existing tests get replaced
  by boundary tests

Do NOT propose interfaces yet. Ask user:
"Which of these would you like to explore?"

### 3. User picks candidate

### 4. Frame problem space

Before spawning sub-agents, write user-facing
explanation of problem space for chosen candidate:

- Constraints any new interface must satisfy
- Dependencies it must rely on
- Rough illustrative code sketch to ground
  constraints — not a proposal, orientation only

Show to user, then immediately proceed to Step 5.
User reads and thinks while sub-agents work in parallel.

### 5. Design multiple interfaces

Spawn 3+ background agents in parallel using the host agent's
delegation capability.
Each must produce **radically different** interface for
deepened module.

Prompt each sub-agent with separate technical brief
(file paths, coupling details, dependency category,
what's hidden). Brief is independent of user-facing
explanation in Step 4. Give each agent different
design constraint:

- Agent 1: "Minimize interface — aim for 1-3 entry
  points max"
- Agent 2: "Maximize flexibility — support many use
  cases and extension"
- Agent 3: "Optimize for most common caller — make
  default case trivial"
- Agent 4 (if applicable): "Design around ports &
  adapters pattern for cross-boundary dependencies"

Each sub-agent outputs:

1. Interface signature (types, methods, params)
2. Usage example showing how callers use it
3. What complexity it hides internally
4. Dependency strategy (how deps handled — see
   [REFERENCE.md](REFERENCE.md))
5. Trade-offs

Present designs sequentially, then compare in prose.

After comparing, give your recommendation: which
design is strongest and why. If elements from
different designs combine well, propose hybrid.
Be opinionated — user wants strong read, not menu.

### 6. User picks interface (or accepts recommendation)
