# Global Pi instructions

## Subagent model selection

When calling `Agent` or writing a `SubagentWorkflow`, select child models from
the active parent model:

- When the parent uses `openai-codex`:
  - Explore agents use `openai-codex/gpt-5.6-luna`.
  - Other agents omit `model` so they inherit the parent's OpenAI model.
  - Never select a Claude or Anthropic model.
- When the parent uses `claude-sdk/fable`:
  - Explore agents use `claude-sdk/haiku`.
  - Other agents use `claude-sdk/sonnet`.
  - Never select an `anthropic/*` model.
- When the parent uses another `claude-sdk` model:
  - Agents omit `model` and inherit the parent unless the user requests
    otherwise.
  - Never select an OpenAI or `anthropic/*` model.

Use exact provider and model identifiers. Do not substitute a model from another
provider when the requested model is unavailable.
