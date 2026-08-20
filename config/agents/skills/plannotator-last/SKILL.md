---
name: plannotator-last
description: Open Plannotator on the latest rendered assistant message and use the returned annotations to revise that message or continue.
disable-model-invocation: true
---

# Plannotator Last

Use this skill when the user wants to annotate the latest assistant response in Plannotator.

Do not send a commentary/status message before running the command. The command
targets the latest rendered assistant response, so a preamble can mistakenly become the
thing being annotated.

When running under Pi, use `PI_SESSION_FILE` to select the invoking session
explicitly. Extract the most recent assistant message that contains rendered text and
pass it on stdin, bypassing Plannotator's ambiguous project-wide mtime lookup:

```bash
if [[ -n "${PI_SESSION_FILE:-}" && -f "$PI_SESSION_FILE" ]]; then
  message=$(jq -rs '
    [ .[]
      | select(.type == "message" and .message.role == "assistant")
      | [ .message.content[]? | select(.type == "text") | .text ]
      | join("\n\n")
      | select(length > 0)
    ]
    | last // empty
  ' "$PI_SESSION_FILE")
  if [[ -z "$message" ]]; then
    echo "No rendered assistant message found in $PI_SESSION_FILE" >&2
    exit 1
  fi
  printf '%s\n' "$message" | plannotator last --stdin
else
  plannotator last
fi
```

Do not simplify the Pi branch to `plannotator last`: when several Pi processes are
open in related directories, Plannotator may otherwise choose another process's session
by modification time. The fallback is for harnesses that do not expose
`PI_SESSION_FILE`.

Behavior:

1. Launch the command with Bash.
2. Wait for the annotation session to finish.
3. If feedback is returned, incorporate it into the follow-up response.
4. If the session closes without feedback, mention that briefly and continue.

Run the command yourself rather than telling the user to invoke shell syntax manually.
