# Enforcement map

Every normative rule in SKILL.md and these references has exactly one
enforcement mechanism. This map is the audit that assigns it. When a rule is
added or changed, update the row; a rule with no mechanism is a suggestion
and gets deleted or downgraded to guidance on review.

The mechanisms, weakest to strongest:

- **Judgment** — a named owner (reviewer, judge, or user) is instructed to
  check it. Honest but soft; reserve for rules no machine can evaluate.
- **Structural** — no command surface exists for the violation, so compliance
  does not depend on the model remembering anything.
- **Controller** — a command rejects with a typed error; the ledger records
  the rejection, so violations are visible to the judge.

## Controller-enforced rules

| Rule | Mechanism |
| --- | --- |
| One run directory per work order | `init` rejects with `RunAlreadyExists` |
| No second active delivery | `open-delivery` rejects with `PriorOpen` |
| Follow-up deliveries bounded and receipt-bound | `standing_allows` / `ReceiptRequired`; grants consumed once via `GrantUsed` |
| Path-scoped grants bind every commit | `snapshot` rejects with `PathScopeRejected` |
| Merge needs PASS, dispositions, green checks, current human receipt | `publish` rejects with typed evidence errors |
| Merge receipt when policy requires it | `publish --step merge` rejects with `MergeRequiresReceipt`; consumed on confirmed merge |
| Tracker commands need a configured tracker | `set-status` / `observe-status` / `subtask-record` reject with `NoExternalTracker` |
| Follow-up `open-delivery` needs a fresh tracker read | rejects with `TrackerReadRequired` in tracked runs |
| No merge replay to repair tracker sync | a merged delivery is closed; `publish` rejects with "publish needs an open code delivery" |
| Completion needs verified delivery (and confirmed Done when tracked) | `complete` rejects with `CompletionRequiresJiraDone` |
| Cleanup only after completion, only the owned slot | `cleanup` rejects with `CleanupRequiresCompletion` / `NoOwnedSlot` |
| Slot safety: no dirty, foreign, redirected, or unfinished-owner binding | `bind-slot` rejects with `SlotUnsafe` / `SlotUnfinishedOwner` / `SlotReuseRequiresGrant` |
| Custody anchor before destructive slot operations | the git adapter refuses `reuse_slot` / `clean_slot` when the anchor write fails |
| Branch protection honored | the GitHub adapter uses exact-head protection and never passes admin bypass |
| Budgets monotonic, stalls bounded | `BudgetSpent` events; checkpoint stall budget |
| Escalation needs a completed reviewer launch | `run-agent` rejects with `EscalationRequiresReview` |
| Status transitions bounded and observed | `sync::intend` bounds retries at three; stale reads rejected |
| Ledger tampering | the projection is re-folded from events on load; edits fail the fold |

## Structural rules

| Rule | Structure |
| --- | --- |
| Agents launch only through `run-agent` | workers have no controller access; budgets and the active-launch rule bound what launches exist |
| Judge is never driven by the coordinator | `complete` and `usage-report` spawn it detached; `next` never names it |
| Receipts bind immutable artifacts | digests re-validated at grant use and at merge; a changed artifact rejects |
| Reviewer isolation | the harness profile restricts reviewer tools; Pi keeps read-only bash, Claude runs `dontAsk` with a read-only allowlist |
| Criteria frozen per task | baselines set once; criteria digests bind receipts and tracker reads |
| Slot destruction is recoverable | append-only `refs/no-mistakes/custody/<slot>/<head>` anchors |

## Judgment rules and their owners

| Rule | Owner |
| --- | --- |
| Tests prove behavior at a public boundary, not source text | reviewer (finding), judge (pattern) |
| No manufactured incidents, deployments, or third-party receipts | judge; human receipt for `human_only` criteria |
| No recursive, bookkeeping, polling, or summary agents | coordinator discipline; judge reads the ledger for violations |
| Simplest change meeting the criteria | reviewer |
| Requirements-reading decisions within the Authority list | coordinator, recorded in plan or checkpoint; user on hold |

A judgment rule that keeps being violated is telling you it wants a
mechanism: promote it to a controller rejection rather than restating the
prose more firmly.
