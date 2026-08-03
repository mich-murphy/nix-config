---
name: neo
description: Investigate and route consequential software-engineering changes through explicit product, architecture, program-design, prototype, and delivery decisions before implementation. Use when the user explicitly invokes $neo for a complex, ambiguous, cross-boundary, persistent-data, security-sensitive, or expensive-to-reverse change. Do not use for implementation, ordinary lightweight planning, code review, or questions about The Matrix.
---

# Route a Neo Planning Task

Act as the public entry point to the Neo suite. Bootstrap routing locally, then
continue required planning stages in the same context until user input is
required. Do not duplicate discovery during routing or implement code.

Resolve `<neo-cli>` as [scripts/neo.py](../../scripts/neo.py) relative to this
skill's source location, independent of the current working directory. Use
that resolved path for every `neo.py` command.

## Start

1. Read repository instructions and [risk-routing.md](references/risk-routing.md).
2. Use risk signals the user already confirmed without asking again. Otherwise,
   inspect only enough evidence to propose uncertain or consequential signals
   for confirmation; leave system discovery to the discovery stage.
3. Initialize, assess, and route in one local operation:

   ```sh
   python3 <neo-cli> start <slug> --title "<title>" \
     --signals "<comma-separated-confirmed-signals>"
   ```

4. If the route is `direct`, exit Neo immediately and continue with ordinary
   Codex planning in this context. Create no Neo planning artifacts.
5. Otherwise begin `$neo-discover` directly from the generated handoff.

## Route

After each valid gate, run `status` and generate a handoff for `current_stage`:

```sh
python3 <neo-cli> handoff <slug> \
  --expect <stage>
```

Follow the matching stage skill:

- `discover` -> `$neo-discover`
- `product` -> `$neo-product`
- `architecture` -> `$neo-architecture`
- `program` -> `$neo-program`
- `delivery` -> `$neo-delivery`
- `finalize` -> `$neo-finalize`

Continue consecutive stages in this context while their decisions are already
approved or non-consequential. Stop only for required user input or final
approval. Start a fresh context only when context pressure would make the work
unsafe; a fresh session is not a stage requirement. Never replace the
deterministic route with model judgment.

## Boundaries

- Use the CLI for state changes; never hand-edit `state.json`.
- Ask before recording a consequential user decision.
- Keep exploration in `.neo/tasks/<slug>/`.
- Do not prototype or implement within the router context.
- Validate planning artifacts only with `neo.py gate`, `neo.py finalize`, or
  `neo.py validate`. Do not run general Markdown linters, package downloads, or
  any network-dependent validation on Neo's critical path.
