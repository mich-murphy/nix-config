# Global Agent Instructions

## Skill Evaluations

Keep each skill's evaluation package under `skills/<skill>/evals/`. Package its
cases, model routes, runner, comparator, and recorded baseline results with the
skill; do not create a shared cross-skill case bundle.

Run a candidate and compare it with the recorded baseline from the repository
root:

```sh
python3 config/agents/skills/<skill>/evals/run-evals.py \
  --output /tmp/<skill>-candidate.json
python3 config/agents/skills/<skill>/evals/compare-evals.py \
  /tmp/<skill>-candidate.json
```
