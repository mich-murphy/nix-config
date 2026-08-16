---
name: herdr-tickets
description: "Run a set of blocking-edge tickets as one Herdr tab per ticket, each driving the implement skill, advancing the frontier as tickets complete."
disable-model-invocation: true
---

# Herdr Tickets

Take a set of tickets that declare blocking edges — the output of `to-tickets` —
and run the ready ones in parallel, one Herdr tab each, every tab driving the
`implement` skill on its own git worktree.

You are the herder. You do not implement anything yourself, and you do not judge
whether a ticket's work is any good. `implement` decides when its ticket is
ready; the human does a final pass before anything becomes a pull request.

Nothing here is specific to one agent kind. You may be Claude, Codex, or
anything else Herdr recognizes, and the agents you start may be a different kind
again.

## Preconditions

```bash
test "${HERDR_ENV:-}" = 1
```

Not inside Herdr → say so and stop. Never control a Herdr session from outside
it.

The installed binary is the authority on the control surface. Print its own
agent instructions and the current command groups before driving it:

```bash
herdr --skill
herdr agent
herdr pane
herdr tab
```

This skill fixes only the topology and the loop; everything about how to talk to
Herdr comes from the binary, so it cannot go stale against an upgrade.

## 1. Know what kind you are

Some steps below depend on the agent kind. Find your own rather than assuming:

```bash
herdr agent list
```

Match the entry whose `pane_id` equals `$HERDR_PANE_ID` and read its `agent`
field. Use that kind for the agents you start, unless the user names a different
one. `herdr agent` lists the kinds this build supports.

## 2. Load the tickets

Read every ticket in the set. Build the dependency graph from each ticket's
"Blocked by" line. Tickets may be local markdown files or issues on a tracker;
`docs/agents/issue-tracker.md` says which, when it exists.

Record for each ticket: identifier, title, blockers, state — pending, running,
done, failed — and, once it runs, **its branch**. Later tickets are cut from
those branches, so a lost branch name strands everything downstream.

Report the graph before touching anything: what is on the frontier, what is
blocked and by what.

## 3. Pick the wave

The **frontier** is every ticket whose blockers are all `done`. Those are the
only tickets eligible to start.

Cap concurrency. Default **3 tabs**, hard ceiling **5** — a bigger herd costs
more in review and merge conflicts than it saves in wall-clock. Honour a lower
cap the user asks for. If the repo's agent instructions (`AGENTS.md`,
`CLAUDE.md`, or equivalent) set a stricter limit, that wins.

If the frontier is larger than the cap, start the tickets that unblock the most
downstream work first.

## 4. Give each ticket a worktree

Never run two tabs against one working tree.

**Cut each ticket from its blockers, not from the base.** Nothing the herd
produces is merged anywhere — each finished ticket is an unmerged branch waiting
on the human. A dependent ticket cut from `main` therefore starts without the
work it declared it needed, and rediscovers that only after its agent has read
the code and found the change missing.

- No blockers → branch from the base (`main`, or whatever the user names).
- One blocker → branch from **that blocker's branch**.
- Several blockers → merge them into an integration branch first, resolve any
  conflicts there while no agent is running, and branch from that. Tell the user
  you did it and what conflicted.

```bash
git worktree add .worktrees/<slot> -b <branch> <blocker-branch-or-base>
```

Follow the repo's own worktree convention when it has one — path layout, slot
names, branch prefixes. Where the repo pins fixed slots to fixed dev-server
ports, take a real slot rather than inventing a name, or the tab has no lane to
run a dev server on.

State the base in the ticket's handover so the agent can confirm it landed
where it expected.

A new worktree contains only **tracked** files. Tickets living in an ignored or
untracked directory — `.scratch/`, a notes folder — do not exist inside it.
Refer to every ticket by absolute path in the prompt, or the agent opens its tab
and finds nothing to implement.

`herdr worktree create` is the alternative when the user wants one workspace per
ticket. It creates a workspace, not a tab, so it does not fit the topology
below.

## 5. Open a tab per ticket and start the agent

One tab per ticket, labelled so the user can find it, rooted in that ticket's
worktree, without stealing focus:

```bash
herdr tab create --workspace "$HERDR_WORKSPACE_ID" --cwd <worktree-path> --label <ticket-id> --no-focus
```

Pass `--workspace` explicitly. Omitted, the tab is created in whatever workspace
is **focused**, which may be the user's or another client's — the herd then
scatters into a workspace you are not in.

Read the new pane from `.result.root_pane.pane_id`. Start the agent in it, named
for the ticket so later commands can address it by name:

```bash
herdr agent start <ticket-slug> --kind <kind> --pane <root-pane-id>
```

Names must match `[a-z][a-z0-9_-]{0,31}` and be unique among live agents.

**Do not pass permission or approval flags.** `agent start` launches the agent
through the pane's interactive shell, so the user's own shell alias for that
command applies — and each agent kind spells its bypass differently. Adding
flags here either duplicates what the alias already set or silently contradicts
it. Let the environment decide; it already has.

Confirm it took rather than assuming — the started process should carry the
alias's own flags:

```bash
ps -eo args | grep <kind>
```

## 6. Hand the ticket over

Submit one prompt per tab that names the ticket and delegates to `implement`.
Do not restate the ticket's content — the ticket is the contract, and a
paraphrase in the prompt is a second source of truth that will drift.

Invoke the skill the way that agent kind invokes skills. Where slash commands
exist, that is:

```bash
herdr agent prompt <ticket-slug> "/implement <absolute-path-to-ticket>"
```

For a kind without slash commands, name the skill and the ticket in prose
instead. The requirement is that the agent runs `implement` against that ticket
and nothing else.

**Dispatch the whole wave before waiting on anything.** Do not pass `--wait`
here. `--wait` blocks until the agent next settles, and an agent that starts
work immediately does not settle for a long time — so a waved dispatch with
`--wait` runs the herd serially, which is the one thing this skill exists to
avoid. Submit every prompt, then confirm the wave moved:

```bash
herdr agent list
```

Every dispatched agent should read `working` within a few seconds. One still
`idle` did not take the prompt.

## 7. Watch the herd

```bash
herdr agent list
herdr agent get <ticket-slug>
```

Act on state:

- `working` — leave it alone.
- `blocked` — Herdr saw a prompt or question. Tell the user what it is asking.
  Answer only what the ticket already decides; anything else is the user's call.
- `idle` or `done` — the turn settled. Get the agent's verdict (below) and take
  it at face value: did `implement` report the ticket finished, or did it stop
  short? A settled agent has stopped talking, which is not the same as having
  finished. You are reading its conclusion, not forming your own about the code.
- `unknown` — inspect. It does not prove completion.

### You probably cannot read the transcript

Interactive agents render on the terminal's **alternate screen**. Rows that
leave it never enter Herdr's scrollback, so for a full-screen TUI agent every
read source fails, and each fails differently:

- `recent` and `recent-unwrapped` return **zero bytes**.
- `visible` and `detection` return only the bottom chrome — status line, context
  meter, mode indicator — with the conversation blank above it.

None of that means the agent is idle or broken. Raising `--lines` does not help;
the rows are gone. Do not diagnose a healthy agent from an empty read.

Two consequences. Lifecycle state, not output, is your progress signal while an
agent works. And to get a verdict when it settles, ask for it as a file:

```bash
herdr agent prompt <ticket-slug> "Write your final status for this ticket as Markdown to <absolute-path>. State whether the ticket is complete, which acceptance criteria you met, which you did not, and anything you deliberately deferred. Reply with the path only."
```

Write it somewhere outside the worktree, so a verdict never lands in the diff
the human reviews. Then read that file directly. Do not ask for file output in
the first prompt — it competes with the ticket for the agent's attention.

**A verdict request restarts the agent.** It flips from `done` back to
`working`, so a watcher waiting for "all settled" re-arms and a naive reader
concludes implementation resumed. Track which prompt you sent to which agent: a
verdict request is not ticket work, and its `working` means nothing.

`pane read` returns plain text on stdout, **not JSON**; piping it to `jq` yields
nothing and looks exactly like an empty pane. Most other commands — `tab
create`, `pane get`, `agent list`, `wait-output` — do return JSON. Errors arrive
as JSON on stderr with exit status 1, and a pipe replaces that status with the
last command's, so check the payload rather than `$?`.

For an ordinary command pane rather than a TUI agent, `wait-output` works, but
defaults to `--source recent` and so inherits its emptiness on a young pane —
pass `--source visible`. It also searches the echoed command line, so matching a
string from the command you just ran matches instantly and proves nothing.

## 8. Advance the frontier

When a ticket's agent reports it finished, record its branch, mark it done,
recompute the frontier, and dispatch whatever it unblocked — cutting each new
worktree from its blockers per section 4, up to the cap. Repeat until no ticket
is runnable. Do not wait for the user between waves.

Report each turnover: what finished and on which branch, what that unblocked,
what you started and from where, what is still blocked and on what.

A ticket whose agent stopped short blocks everything downstream of it. Do not
start its dependents. Say plainly that the branch of the graph is stalled and
why, and let the user decide whether to retry, reassign, or cut it.

Stop and hand back when every ticket is done, or when nothing is runnable and
something is still pending.

## 9. Leave the herd inspectable

Do not close tabs, panes or worktrees you created unless the user asks. They are
the evidence — transcript, diff and branch all live there.

Report at the end: each ticket, its state, its branch, the branch it was cut
from, and its tab. The human needs the chain to merge them in the right order.

Surface anything the agents flagged that outlives their own ticket — coverage a
ticket deleted and nothing replaced, a defect found in the spec, a criterion an
agent judged vacuous. That is the part a diff does not show.

## Rules

- Never implement a ticket yourself. If one looks too small to be worth a tab,
  say so and let the user decide — do not quietly absorb it.
- Never start a ticket whose blockers are not all done, however idle the herd is.
- Never cut a dependent ticket from the base when its blocker has an unmerged
  branch.
- Never run two tabs against one working tree.
- Never merge to the base, push a branch, or open a pull request. The human does
  the final pass.
- Never add permission or approval flags to a dispatched agent.
- Keep focus in the calling pane. Use `--no-focus`.
