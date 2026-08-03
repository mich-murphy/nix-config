# Model, Effort, Delegation, and Latency

Route semantically; exact current model IDs live in `evals/routes.json` or
harness-specific agent configuration.

| Task shape | Lane | Initial effort |
| --- | --- | --- |
| Mechanical transformation with deterministic verification | Efficient | Low |
| Repository mapping or bounded evidence gathering | Balanced | Low or medium |
| Routine implementation with reliable acceptance tests | Balanced | Medium |
| Difficult debugging or edge-case-heavy implementation | Balanced | High |
| Ambiguous design, architecture, or consequential synthesis | Frontier | High |
| Security, data-loss, concurrency, or release-critical review | Frontier | High or xhigh |

For difficult work, establish the quality ceiling with the frontier lane at
balanced/high effort, reduce effort one level, then test a lower lane. For
deterministic high-volume work, begin efficient and escalate only when the
verifier exposes a gap. Change one variable per experiment.

Delegate only independent bounded work with an explicit output contract.
Prefer parallel read-only evidence gathering to concurrent mutation. Keep
ambiguous decisions, synthesis, shared edits, and acceptance with one owner.

Measure end-to-end and p50/p95 latency, time to first output when available,
tool and permission wait, retries, input/output/cached/reasoning tokens, cost,
and human rework per accepted task. A latency or token reduction is an
improvement only when acceptance, quality, and safety remain non-inferior.
Reduce latency through selected-branch loading, removing repeated instructions
and unused examples, exposing relevant tools only, parallel independent reads,
and eliminating unnecessary agent or evaluator calls.
