# Model and Effort Routing

## Contents

- [Route by Task Semantics](#route-by-task-semantics)
- [Map Current Model Families](#map-current-model-families)
- [Escalate From Evidence](#escalate-from-evidence)
- [Evaluate the Route](#evaluate-the-route)

## Route by Task Semantics

| Task | Minimum lane | Starting effort | Escalate when |
| --- | --- | --- | --- |
| Formatting, deterministic rename, extraction | Efficient | Low | Dynamic uses, unclear scope, or verifier gaps appear |
| Repository map or large read-only scan | Balanced | Low/medium | Synthesis spans several systems or conventions conflict |
| Local refactor with clear tests | Balanced | Medium | Failures do not localize or design alternatives matter |
| Difficult debugging or weakly tested legacy seam | Balanced/frontier | High | Public behavior, unknown consumers, or nonlocal invariants appear |
| Module/API design or cross-boundary refactor | Frontier | High | Use still higher effort only after a representative miss |
| Security, data loss, concurrency, release-critical review | Frontier | High/xhigh | Always require executable or specialist corroboration |
| Long-horizon migration | Frontier | Xhigh | Compare maximum effort only when accepted outcomes improve |

The model tier sets a capability ceiling; effort controls inference spent within that
ceiling. Change one routing variable at a time while holding task, prompt, tools, and
acceptance checks fixed.

## Map Current Model Families

This mapping reflects repository research assessed **2026-08-02** and is
time-sensitive. Recheck vendor documentation before hard-coding a model ID.

| Semantic lane | OpenAI/Codex starting point | Anthropic/Claude starting point | Pi or other harness |
| --- | --- | --- | --- |
| Efficient | GPT-5.6 Luna; Terra at low effort if Luna is unavailable | Claude Haiku 4.5 | Choose the provider's fast, low-cost model only for tightly specified work |
| Balanced | GPT-5.6 Terra | Claude Sonnet 5 | Choose a strong coding/tool-use model with a cheap verifier |
| Frontier | GPT-5.6 Sol | Claude Opus 5; Fable 5 for the hardest long-running work | Choose the strongest available reasoning/coding model and validate locally |

Codex's documented balanced default is Sol at medium reasoning for demanding agent
work. Anthropic documents capability-first and efficiency-first routes: start strong
when the quality ceiling or failure cost is unknown; start efficient for bounded,
high-volume work with cheap validation.

Do not invent model fields in portable `SKILL.md` frontmatter. Put exact pinned models
and effort in harness-specific agent configuration; keep semantic routing in the
portable skill.

## Escalate From Evidence

Start medium for balanced work. Increase effort or tier only when a run shows:

- missed nonlocal callers or invariants;
- inability to distinguish behavior, refactoring, and migration modes;
- weak alternative analysis at a consequential boundary;
- repeated test failures that do not localize;
- unresolved edge cases whose failure cost matters; or
- incomplete verification or handoff despite adequate context and tools.

Before escalating, check task framing, repository context, tool failures, permission
limits, and acceptance criteria. More inference does not correct a wrong goal, hidden
contract, unavailable tool, or falsely green test.

Lower effort or tier after representative tasks continue to meet the same acceptance
bar. Mechanical edits can move down more readily than judgment-heavy review. Another
model's review yields hypotheses, not proof; reproduce findings in code or tests.

## Evaluate the Route

Measure cost per accepted task:

- correct behavior and preserved contracts;
- unnecessary edits and regressions;
- design-review findings and later rework;
- human review and comprehension time;
- tool failures and retries;
- latency, tokens, and monetary cost; and
- no-change accuracy when cleanup is not justified.

Use realistic refactors across languages and repository shapes. Compare with-skill and
without-skill runs in fresh sessions, preserve complete outputs, and retain confirmed
misses as regression cases. Do not claim one vendor's model is equivalent or superior
to another from vendor routing guidance alone.
