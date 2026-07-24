# Local Agent-Code Review Tools: Hunk and Alternatives

Research snapshot: 2026-07-22 (Australia/Melbourne).

## Conclusion

Hunk is the most-starred open-source terminal review tool in the relevant
comparison, but it is not currently the best complete implementation of the
desired workflow.

- **Hunk** has the strongest popularity and maintenance signals: 7,583 GitHub
  stars, 213 forks, a release three days before this review, and commits in all
  of the preceding 13 weeks. Its missing feature is the decisive one here: the
  reviewer cannot submit an explicit approval or send a completed set of
  comments back to the waiting agent.
- **Revdiff** is the strongest source-level reference for adding that missing
  feedback handoff to a terminal tool. Its launcher blocks the calling agent,
  captures line annotations through an atomic output file, and distinguishes
  “annotations produced” with exit code 10. It does **not** implement a hard
  approval gate: normal quit with no comments, discard-and-quit, and some
  interrupted exits all look like exit 0 with no output to the caller.
- **GitHuman** most closely implements the requested semantics today.
  `githuman ask` opens a review of staged changes, waits, and returns structured
  comments, todos, and review status when the reviewer clicks **Continue
  assistant**. It supports `approved` and `changes_requested`. Its risks are
  much lower adoption and a repository that has been inactive since April.
- **Lumen** is the best terminal-native alternative to trial. Inline annotations
  can be submitted from the TUI with `s`; Lumen exits and writes them to stdout
  so Claude Code, Codex, or another calling agent receives them directly. It
  lacks a distinct approval decision tied to the reviewed diff.
- **Plannotator's code-review mode**, considered separately from its plan-review
  mode, is a strong browser-based option and already returns review feedback to
  the agent. It does not currently provide the desired pre-commit approval gate.
- **Difit** provides an excellent browser review surface and can print comments
  to the waiting agent when it exits, but closing without comments is treated as
  “no feedback,” not as an explicit approval.

The practical recommendation is to test Revdiff and GitHuman against the real
workflow before investing in a Hunk review-gate feature. Revdiff is the closest
open terminal implementation to copy for blocking launch and feedback return;
GitHuman remains the closest semantic match for explicit approval. Keep Hunk if
its TUI, live session API, and maintenance quality outweigh the missing decision
protocol.

## Scope and Method

The target workflow is narrowly defined:

1. An agent produces local code changes.
2. A human reviews the actual diff before commit.
3. The human leaves comments anchored to files and lines.
4. The interface has an explicit completion action.
5. Feedback returns to the waiting coding agent without manual polling or
   copy/paste.
6. Ideally, approval authorizes only the exact staged patch that was reviewed.

Tools were investigated from their official repositories, documentation,
release pages, package registries, and first-party websites. Repository metrics
were read from the [GitHub REST API](https://docs.github.com/en/rest/repos/repos)
on 2026-07-22. Commit regularity uses GitHub's default-branch participation and
commit history. npm downloads cover 2026-06-22 through 2026-07-21.

GitHub does not publish a project “rating.” Stars measure attention, downloads
measure package retrieval, and neither proves quality or trust. “Contributor”
also does not mean “maintainer”: write access is normally private. Maintainer
concentration below is inferred from visible commit authorship and releases,
and is labelled accordingly.

## Direct Open-Source Comparison

| Tool | Human review and agent return | Explicit local approval | Popularity and recency | Maintenance concentration |
| --- | --- | --- | --- | --- |
| [Hunk](https://github.com/modem-dev/hunk) | Inline session comments are agent-readable, but the agent must pull them; no review submission event | No | 7,583 stars; 213 forks; v0.17.3 on 2026-07-19; pushed 2026-07-21 | 53 visible contributors, but Ben Vinegar authored 438 of 522 Jan–Jul commits |
| [Revdiff](https://github.com/umputun/revdiff) | The launcher waits for TUI exit and returns line/file annotations directly to the calling agent | No; an empty normal quit is treated as review completion, but is not distinguishable from discard | 696 stars; 70 forks; v1.11.1 on 2026-07-13; pushed 2026-07-19 | One code owner; Umputun authored 321 of 396 non-bot commits (81%); 46 non-bot author identities in Git history |
| [Plannotator](https://github.com/backnotprop/plannotator) code review | Inline annotations and **Send Feedback** return directly to the calling agent | No local commit approval; close and feedback are distinct | 7,188 stars; 517 forks; v0.24.2 and push on 2026-07-21 | 111 visible contributors; one dominant human maintainer, with regular outside contributions |
| [Difit](https://github.com/yoshiko-pg/difit) | Comments can be copied as prompts; its agent skill also waits and prints comments to stdout on exit | No; shutdown without comments means “no comments” | 3,003 stars; 151 forks; v5.0.8 and push on 2026-07-11 | 56 visible contributors; Yoshiko is the core maintainer and sole npm collaborator |
| [Lumen](https://github.com/jnsahaj/lumen) | `s`, then `Enter`, submits formatted annotations to stdout and exits; `q` dismisses | No distinct approval state | 2,602 stars; 130 forks; v2.32.0 and push on 2026-07-16; 437 commits | 26 author identities in 2026; Sahaj Jain authored 116 of 154 commits (75%) |
| [GitHuman](https://github.com/mcollina/githuman) | `githuman ask` waits for **Continue assistant**, then returns structured status, todos, and comments | Yes: `approved` or `changes_requested` | 255 stars; 19 forks; v0.9.0 and last push on 2026-04-12 | Matteo Collina authored 134 of 137 Jan–Apr commits |

Current metric endpoints:

- [Hunk repository API](https://api.github.com/repos/modem-dev/hunk)
- [Revdiff repository](https://github.com/umputun/revdiff)
- [Plannotator repository API](https://api.github.com/repos/backnotprop/plannotator)
- [Difit repository API](https://api.github.com/repos/yoshiko-pg/difit)
- [Lumen repository](https://github.com/jnsahaj/lumen)
- [GitHuman repository](https://github.com/mcollina/githuman)

### Adoption signal beyond stars

Over the latest complete 30-day window, npm reported 28,508 downloads for
Difit, 17,335 for Hunk's `hunkdiff` package, and 111 for GitHuman. This makes
Difit the npm usage leader even though Hunk has more GitHub stars. Lumen is
primarily distributed through Cargo and Homebrew, so these npm figures are not
a fair cross-ecosystem ranking.

- [Difit npm downloads](https://api.npmjs.org/downloads/point/2026-06-22:2026-07-21/difit)
- [Hunk npm downloads](https://api.npmjs.org/downloads/point/2026-06-22:2026-07-21/hunkdiff)
- [GitHuman npm downloads](https://api.npmjs.org/downloads/point/2026-06-22:2026-07-21/githuman)

## Workflow Findings

### Hunk

Hunk is explicitly a “review-first terminal diff viewer for agent-authored
changesets.” It has watch mode, a multi-file stream, inline comments, and a
session broker through which agents can read comments. Its documented agent
workflow still requires opening Hunk in another terminal and asking the agent
to use the Hunk skill against the live session. There is no **Submit review**,
**Approve**, or **Request changes** event in the current interface.

This means Hunk is the strongest review surface and project-health candidate,
but not an end-to-end human gate. Popularity does not remove that blocker.

Sources: [Hunk README and agent workflow](https://github.com/modem-dev/hunk#working-with-agents),
[Hunk releases](https://github.com/modem-dev/hunk/releases).

### Revdiff

Revdiff materially changes this comparison. At audited commit
[`1b563f8`](https://github.com/umputun/revdiff/tree/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5),
it has already implemented most of the terminal-to-agent feedback transport
that Hunk lacks:

1. The Codex and Claude skills invoke a launcher synchronously and tell the
   agent to wait for the reviewer. The launcher opens a terminal overlay or
   split using agterm, tmux, Zellij, herdr, kitty, WezTerm/Kaku, cmux, Ghostty,
   iTerm2, or Emacs vterm. Terminal-specific branches either block directly or
   poll an atomically renamed sentinel containing the child exit status.
2. Revdiff receives `--output=<temporary-file>` and
   `REVDIFF_EXIT_CODE_ON_ANNOTATIONS=true`. The TUI holds annotations in memory
   as file, line/range, diff side, and comment fields. On deliberate quit it
   atomically writes a stable Markdown representation to the temporary file.
3. Exit 10 means annotations were produced; exit 0 means no annotation output;
   other nonzero statuses are launcher failures. The wrapper prints the output
   file to its own stdout, so the suspended tool call completes with comments
   already in the agent's result. No daemon polling or later user instruction
   is required in the normal path.
4. The supplied Codex/Claude skill classifies the returned comments, asks for
   confirmation before editing, applies changes, then reopens the same target.
   A subsequent no-output result ends the loop. Pi implements a tighter native
   tool: it suspends Pi's TUI, hands the terminal to Revdiff, parses annotations
   into structured objects, and resumes the agent after the child exits.

Sources: [Codex skill launch and capture
contract](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/plugins/codex/skills/revdiff/SKILL.md#L145-L217),
[launcher output and exit-code protocol](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/plugins/codex/skills/revdiff/scripts/launch-revdiff.sh#L19-L60),
[annotation model and serialization](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/app/annotation/store.go#L12-L40),
[final handoff behavior](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/app/main.go#L303-L343),
[Pi direct-terminal adapter](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/plugins/pi/extensions/revdiff.ts#L221-L305).

#### What it does not solve

Revdiff's code-review integration is a manually invoked skill/tool, not a
pre-commit hook. The repository's automatic Claude/Codex hooks belong to the
separate plan-review plugin. Nothing in the code-review launcher intercepts
`git commit`, and nothing releases a previously blocked commit.

The TUI also has no distinct **Approve** or **Send feedback** command. `q`
quits and exports whatever annotations exist. `Q` sets a discard flag and quits
without output. The finalizer returns 0 for both discarded reviews and empty
reviews, so the agent-side skill's “review complete” conclusion is a convention:
it cannot distinguish affirmative approval from abandonment or discard. A
SIGHUP or SIGTERM saves annotations only to history and deliberately suppresses
the live output handoff; the normal launcher can consequently receive an empty
successful result unless its timeout/history recovery path is used. Real TUI
or launcher errors do fail nonzero.

Sources: [quit and discard dispatch](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/app/ui/model.go#L1060-L1077),
[discard confirmation](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/app/ui/handlers.go#L140-L148),
[empty/discard/signal finalization](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/app/main.go#L303-L315),
[skill completion convention](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/plugins/codex/skills/revdiff/SKILL.md#L261-L269),
[signal handling](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/app/signal.go#L17-L57).

Nor is approval bound to the staged patch. `--staged` selects `git diff
--cached`, but the caller receives no diff hash or reviewed-state envelope.
Revdiff's `Space` key does maintain useful per-file review progress: it hashes
file path, rename origin, status, and changed-line types/content with SHA-256,
and clears reviewed marks when a reload changes that semantic fingerprint.
Those marks exist only inside the running TUI, ignore line numbers and context
by design, and are not included in the exit result. They are navigation state,
not authorization for a commit. Review history records the current HEAD hash
and a copy of annotated-file diffs, not an approval of the complete staged
patch.

Sources: [per-file fingerprint algorithm](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/app/diff/fingerprint.go#L10-L65),
[reviewed-state reconciliation](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/app/ui/sidepane/filetree.go#L367-L404),
[history contents](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/app/history/history.go#L60-L79).

#### What Hunk should reuse

The most transferable pieces are architectural rather than UI code:

- The terminal-adapter launcher demonstrates how a blocked agent process can
  safely open a TUI elsewhere, preserve the working directory, wait, and carry
  back both an exit status and payload.
- The three-way process protocol—no comments, comments captured, real
  failure—is substantially safer than treating all nonzero exits as errors.
  Hunk should extend it to four explicit outcomes: `approved`, `feedback`,
  `cancelled`, and `failed`.
- Atomic output and sentinel writes prevent a waiting hook from consuming
  partial feedback. Revdiff also persists annotations plus their diff as a
  recovery path.
- The `O` flush path can send a complete annotation snapshot without closing
  the TUI. Current `--post-flush-command` support passes that snapshot on stdin
  to a synchronous user command. Hunk's daemon could provide a cleaner native
  event, but the pattern permits an agent to start fixing while review remains
  open.
- Per-file semantic fingerprints are good live-review progress state. A commit
  gate still needs a separate fingerprint over the entire exact staged patch,
  checked again after the review returns and immediately before permitting the
  commit.

Sources: [atomic annotation write](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/app/annotation/store.go#L174-L182),
[in-session flush](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/app/ui/output.go#L23-L71),
[post-flush command input](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/app/handoff/handoff.go#L11-L31).

#### Maintenance and trust assessment

Revdiff is unusually active but also unusually young. It began on 2026-03-31,
had 404 commits by the audited 2026-07-19 head, and had commits in every one of
the preceding 13 weeks. Monthly activity was 34 commits in March, 277 in April,
41 in May, 32 in June, and 20 through July 19. It made 69 tagged releases from
v0.1.0 on April 1 through v1.11.1 on July 13. The latest commit was an accepted
outside contribution, and 46 non-bot author identities appear in the history.

Governance is nevertheless concentrated. `CODEOWNERS` assigns every path only
to `@umputun`; Umputun authored 321 of 396 non-bot commits (81%). This is a
one-core-maintainer project with a meaningful external contributor tail, not a
multi-maintainer project. The rapid release rate demonstrates responsiveness
but the short operating history cannot yet establish long-term continuity.

The author uses a persistent public pseudonymous identity with 2,800 followers,
52 public repositories, sponsorship history, a first-party project portfolio,
and several older, widely adopted Go projects including Remark42, Reproxy, and
go-pkgz/auth. The project is MIT-licensed, tests with Go's race detector, runs
lint and shellcheck in CI, and publishes through GoReleaser. Counterweights are
the single code owner and the absence of an explicit signing, SBOM, or
release-attestation step in the checked-in release workflow. These are solid
author and engineering signals, but not a substitute for reviewing a mandatory
gate dependency.

Sources: [repository and current metrics](https://github.com/umputun/revdiff),
[release history](https://github.com/umputun/revdiff/releases),
[single-owner CODEOWNERS](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/.github/CODEOWNERS),
[CI workflow](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/.github/workflows/ci.yml),
[release workflow](https://github.com/umputun/revdiff/blob/1b563f81a33cb1d6e091f0a0a15f33f7e9df57a5/.github/workflows/release.yml),
[author profile](https://github.com/umputun),
[author project portfolio](https://umputun.dev/).

### GitHuman

GitHuman's stated purpose matches the target directly: it reviews the staging
area before commit. The official workflow is:

1. The agent stages its changes and invokes `githuman ask`.
2. GitHuman opens or reconnects to its local web UI and waits.
3. The human comments on exact diff lines and sets review status.
4. **Continue assistant** completes the human turn.
5. The command exits with new comments, todos, and status, optionally as JSON.

This is currently the clearest open implementation of the missing Hunk
decision channel. Before adopting it as a hard commit gate, verify that a
subsequent staged-diff change invalidates an earlier approval; the public docs
do not establish a cryptographic or patch-hash binding.

The author signal is unusually strong for a small project: GitHuman is created
by Matteo Collina, whom Fastify lists as a lead maintainer and whose broader
Node.js security work is publicly documented. That reduces identity risk, but
not the bus-factor or inactivity risk.

Sources: [GitHuman README](https://github.com/mcollina/githuman#ask-a-human-to-review),
[GitHuman product workflow](https://githuman.dev/),
[Fastify team](https://fastify.dev/),
[Node.js security credit](https://nodejs.org/en/blog/vulnerability/june-2026-security-releases).

### Lumen

Lumen is a Rust TUI and the closest interaction-level alternative to Hunk.
Review comments may target a selection, hunk, or whole file. Pressing `s` and
confirming exits the TUI and emits only formatted annotations to stdout; it
routes the UI through `/dev/tty` when stdout is captured. The official docs
show the intended `!lumen diff` flow for both Claude Code and Codex.

This already solves feedback delivery without a daemon or polling protocol.
However, `q` merely dismisses and there is no separate approval result. Lumen
would therefore need a small decision envelope or wrapper to become a strict
pre-commit gate, but considerably less workflow plumbing than Hunk needs.

Its 2026 activity was uneven but recently sustained: 95 commits in January,
only 9 across February–April, then 16 in May, 20 in June, and 14 through July
16. Twenty-six author identities appeared in 2026, though Sahaj Jain authored
116 of 154 commits (75%). This is a healthy recent rebound with useful outside
contributions, but still a one-core-maintainer project.

Sources: [Lumen annotations and agent integration](https://github.com/jnsahaj/lumen#coding-agent-integrations),
[Lumen releases](https://github.com/jnsahaj/lumen/releases).

### Plannotator code-review mode

Plannotator's plan review and code review must be treated as separate product
paths. Only its code-review path is relevant here. `/plannotator-review` can
review local Git diffs, add line/file/general annotations, and return feedback
to the invoking agent. Current releases add guided review, live Git-status
views, commit views, file comments, and in-app agent engines.

The code-review flow is a credible alternative if a browser is acceptable and
has the best maintenance cadence in the sample: 357 default-branch commits in
the preceding 13 weeks, with activity in all 13. It still lacks a documented
local “approve this exact staged patch and release the commit” result. Its
existing plan approval should not be reused as if plan acceptance and code
authorization were the same event.

Trust signals include open source, frequent immutable releases, SHA-256
sidecars, release attestations, and visible external contributions. The main
risk is core-maintainer concentration behind a pseudonymous individual account,
despite a broad contributor list.

Sources: [Plannotator README](https://github.com/backnotprop/plannotator),
[v0.24.2 release and attestations](https://github.com/backnotprop/plannotator/releases/tag/v0.24.2).

### Difit

Difit provides a polished GitHub-style browser diff, range comments, persisted
review state, working/staged/commit/branch/PR targets, and official agent
skills. Its current skill says the agent launches Difit and waits; review
comments are printed to stdout when the command exits. Its general UI also
offers **Copy All Prompt**.

The ambiguity is completion semantics: shutdown without comments is explicitly
treated as “no review comments were provided.” That is convenient but not a
safe approval gate because approval, abandonment, and a crashed/closed window
are not distinguished. Project health is otherwise strong: activity in 12 of
the last 13 weeks, 91 GitHub releases, npm provenance, a security policy, and
the largest npm download count in this group.

Sources: [Difit README](https://github.com/yoshiko-pg/difit),
[Difit agent skill](https://github.com/yoshiko-pg/difit/blob/main/SKILL.md),
[Difit releases](https://github.com/yoshiko-pg/difit/releases),
[npm package and provenance](https://www.npmjs.com/package/difit).

## Promising but Higher-Risk Alternatives

### Reviu

[Reviu](https://github.com/reviu-dev/reviu) is a native Rust/GPUI desktop Git
client with a built-in Claude or Codex panel. The human can review the local
diff, leave inline comments, and send them directly back to that agent without
copy/paste. This is a very close UX match if moving the agent into Reviu is
acceptable.

The local Git and agent panel are free and run without an account. The client
became public under FSL-1.1 (converting to Apache-2.0 after two years), while its
Pro GitHub backend remains closed. It had only 5 stars and no forks at this
snapshot, despite 1,290 repository commits and 30 releases, most recently
v0.17.0 on 2026-07-08. This suggests energetic development but almost no
independent adoption signal yet, and the visible project is effectively
single-author.

### Pyor

[Pyor](https://pyor.review/) has a polished local pre-PR review flow. Its public
agent tooling provides `prepare`, `open`, and `wait`; the desktop interface's
**Send to Claude** action returns comments through a local feedback channel.
The official skill supports Claude Code, Codex, Cursor, and other agents.

The blocker is verifiability. The desktop application is closed source. The
public [Pyor CLI repository](https://github.com/Pyor-review/pyor-cli) had 0
stars, 0 forks, 19 commits, and no releases. Pyor publishes a security and
privacy position, but there is not yet enough public code, adoption, maintainer
history, or independent audit evidence to rank its maintainability alongside
Hunk, Plannotator, Difit, or Lumen.

### CorgReview

[CorgReview](https://www.corgreview.com/) advertises precisely the desired
review statuses—approve, request changes, or comment—and structured Markdown
for agents. It is a paid closed binary, and no public source repository,
maintainer roster, reproducible build, or comparable release history was found
in the primary-source review. Its workflow is relevant, but its maintenance and
supply-chain trust cannot currently be vetted to the requested standard.

## Tools That Are Not Direct Competitors

- Difftastic, Delta, diff-so-fancy, Lazygit, GitHub Desktop, and similar tools
  can improve diff reading or staging, but do not provide a completed inline
  human-review handoff to a waiting coding agent.
- CodeRabbit, Kodus, Copilot code review, Reviewdog, and similar products are
  automated reviewers. They can augment a human review but do not replace the
  human approval-and-feedback surface being evaluated.
- GitHub/GitLab pull-request review has mature approval semantics, but it occurs
  after a commit and usually after a push. It is a downstream enforcement
  option, not the local pre-commit workflow requested here.

## Recommendation

Use a short evaluation in this order:

1. **Revdiff** for the implementation and terminal fit: it already blocks the
   invoking agent and hands line-specific feedback back synchronously. Test its
   overlay launcher in the terminals you actually use, but do not treat empty
   output as commit approval.
2. **GitHuman** for the semantic fit: staged diff, explicit review status,
   blocking wait, and structured return. Test stale-approval invalidation and
   whether April's last update is acceptable.
3. **Lumen** as the smaller terminal alternative: its `s`-to-stdout feedback
   path does most of what Hunk lacks, though it has neither Revdiff's broad
   launcher/recovery machinery nor a separate approval result.
4. **Plannotator code review** for the lowest-friction browser trial because it
   is already installed. Keep this invocation distinct from the automatic plan
   hook and assess only its local diff-review path.
5. **Hunk** if live shared sessions, terminal rendering, watch mode, and project
   health remain more valuable than immediate workflow completion. In that
   case, port Revdiff's blocking terminal-launch and atomic handoff patterns,
   but add an explicit review decision API rather than treating TUI exit or an
   empty comment list as approval.
6. Treat **Reviu** and **Pyor** as promising desktop experiments, not yet as
   maintainability-proven dependencies. Avoid adopting CorgReview as a hard
   gate until its provenance and maintenance model are more transparent.

Hunk can accurately be called the most popular and one of the best-maintained
open-source terminal viewers in this niche. It cannot yet be called the best
tool for the requested workflow, because Revdiff already implements the
terminal-to-agent feedback loop and GitHuman implements a more explicit review
decision. Neither fully proves approval of the exact staged patch, so a strict
pre-commit gate still requires additional semantics.
