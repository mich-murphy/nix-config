#!/bin/sh
# Claude Code PreToolUse hook, registered by SKILL.md for the rest of the
# session once no-mistakes is invoked. Before every shell command it notes
# the coordinator session's transcript path against the working directory,
# so the controller (`init`, `judge`) can find the conversation that drove
# the run and hand it to the LLM-as-judge. Never blocks: exit 0 always.
set -u
input=$(cat) || exit 0
command -v jq >/dev/null 2>&1 || exit 0
transcript=$(printf '%s' "$input" | jq -r '.transcript_path // empty' 2>/dev/null) || exit 0
cwd=$(printf '%s' "$input" | jq -r '.cwd // empty' 2>/dev/null) || exit 0
session=$(printf '%s' "$input" | jq -r '.session_id // empty' 2>/dev/null) || exit 0
[ -n "$transcript" ] && [ -n "$cwd" ] || exit 0
dir=${XDG_CACHE_HOME:-"${HOME:-}/.cache"}/no-mistakes/coordinator
mkdir -p "$dir" 2>/dev/null || exit 0
name=$(printf '%s' "$cwd" | tr '/' '-')
jq -cn --arg session "$session" --arg transcript "$transcript" --arg cwd "$cwd" \
  --arg at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  '{session_id: $session, transcript_path: $transcript, cwd: $cwd, at: $at}' \
  > "$dir/$name.json.tmp" 2>/dev/null && mv -f "$dir/$name.json.tmp" "$dir/$name.json" 2>/dev/null
exit 0
