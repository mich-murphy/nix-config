# Global Agent Instructions

## Skill Evaluations

Keep each skill's immutable evaluation definitions under
`skills/<skill>/evals/`: cases, routing cases, routes, specialized evaluator
assets where required, and one compact `release-manifest.json`. Do not create a
shared cross-skill case bundle. Generated runs, traces, assessments, metrics,
and minimized result artifacts belong in MLflow, not Git.

Use the central evaluator from the repository root. Generic skills share its
engine and comparator; Neo and skill-development use explicit specialized
adapters behind the same command:

```sh
python3 config/agents/eval_cli.py run <skill> \
  --output /tmp/<skill>-candidate.json
python3 config/agents/eval_cli.py compare <skill> \
  /tmp/<skill>-candidate.json --output /tmp/<skill>-comparison.json
```

Use `--offline` for development without publication. Behavioral validity is
reported independently from `evidence_state=pending`. A strict release must
publish and join complete MLflow evidence before updating the compact manifest;
raw prompt, response, source, tool payload, environment, credential, and
unbounded event content must never enter default publication surfaces.
Retry the ignored spool with `eval_cli.py retry <skill> <spool>`. A successful
retry atomically replaces that spool with the metadata-only, published candidate
that joins to the unchanged comparison content hash; save `--output` separately
as the publication receipt used by `eval_cli.py release`.

Old commands mapped as follows and have no compatibility wrappers:

```text
skills/<skill>/evals/run-evals.py       -> eval_cli.py run <skill>
skills/<skill>/evals/compare-evals.py   -> eval_cli.py compare <skill>
```
