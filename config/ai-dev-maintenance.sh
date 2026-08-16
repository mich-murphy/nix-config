#!/usr/bin/env bash
set -uo pipefail

readonly LOCAL_BIN="${HOME}/.local/bin"
readonly PI_PACKAGE='npm:@plannotator/pi-extension'
readonly MOSHI_TARGETS='claude,codex,opencode,gemini,antigravity,cursor,kimi,qwen,grok,omp,hermes'
export PATH="${LOCAL_BIN}:${HOME}/.nix-profile/bin:${PATH}"
USER=${USER:-$(id -un)}
export HOME USER

failures=()
declare -A tool_failed=()

tools=(claude codex pi herdr moshi)
declare -A tool_label=(
  [claude]='Claude Code'
  [codex]='Codex'
  [pi]='Pi'
  [herdr]='Herdr'
  [moshi]='Moshi'
)
declare -A tool_command=(
  [claude]='claude'
  [codex]='codex'
  [pi]='pi'
  [herdr]='herdr'
  [moshi]='moshi-hook'
)

usage() {
  cat <<'EOF'
Usage: ai-dev-maintenance <ensure-present|update|status>

  ensure-present  Install missing tools, then reconcile integrations.
  update          Update every tool independently, then reconcile integrations.
  status          Report versions, integrations, and the Moshi runtime without changes.
EOF
}

run_step() {
  local label=$1
  shift
  printf '\n==> %s\n' "$label"
  if "$@"; then
    printf 'OK: %s\n' "$label"
    return 0
  fi

  printf 'FAILED: %s\n' "$label" >&2
  failures+=("$label")
  return 1
}

install_claude() {
  curl -fsSL https://claude.ai/install.sh | bash -s stable
}

install_codex() {
  curl -fsSL https://chatgpt.com/codex/install.sh | CODEX_NON_INTERACTIVE=1 sh
}

install_pi() {
  setsid --wait sh -c 'curl -fsSL https://pi.dev/install.sh | sh'
}

install_herdr() {
  curl -fsSL https://herdr.dev/install.sh | sh
}

install_moshi() {
  curl -fsSL https://getmoshi.app/install.sh | MOSHI_HOOK_SKIP_FIRST_RUN=1 sh
}

version_of() {
  local command_name=$1
  if command -v "$command_name" >/dev/null 2>&1; then
    "$command_name" --version 2>&1 | head -n 1
  else
    printf 'not installed'
  fi
}

install_or_update_tools() {
  local mode=$1
  local tool command_name installer
  declare -A before=()

  for tool in "${tools[@]}"; do
    command_name=${tool_command[$tool]}
    installer="install_${tool}"
    before[$tool]=$(version_of "$command_name")

    if [[ $mode == ensure-present ]] && command -v "$command_name" >/dev/null 2>&1; then
      printf 'PRESENT: %s: %s\n' "${tool_label[$tool]}" "${before[$tool]}"
      continue
    fi

    if ! run_step "${tool_label[$tool]} ${mode}" "$installer"; then
      tool_failed[$tool]=1
    elif ! command -v "$command_name" >/dev/null 2>&1; then
      printf 'FAILED: %s installer did not provide %s\n' "${tool_label[$tool]}" "$command_name" >&2
      failures+=("${tool_label[$tool]} command verification")
      tool_failed[$tool]=1
    fi
  done

  printf '\nVersion summary:\n'
  for tool in "${tools[@]}"; do
    printf '  %s: %s -> %s\n' \
      "${tool_label[$tool]}" "${before[$tool]}" "$(version_of "${tool_command[$tool]}")"
  done
}

pi_package_is_present() {
  local settings=${HOME}/.pi/agent/settings.json
  [[ -f $settings ]] || return 1
  jq -e --arg source "$PI_PACKAGE" '
    (.packages // []) | any(
      if type == "string" then
        . == $source or startswith($source + "@")
      else
        (.source // "") == $source or ((.source // "") | startswith($source + "@"))
      end
    )
  ' "$settings" >/dev/null
}

reconcile_pi_package() {
  local mode=$1
  [[ ${tool_failed[pi]:-0} == 0 ]] || return 0
  command -v pi >/dev/null 2>&1 || return 0

  if [[ $mode == ensure-present ]] && pi_package_is_present; then
    printf 'PRESENT: Pi package %s\n' "$PI_PACKAGE"
  elif [[ $mode == update ]] && pi_package_is_present; then
    run_step 'Pi package update' pi update "$PI_PACKAGE" || true
  else
    run_step 'Pi package install' pi install "$PI_PACKAGE" || true
  fi
}

reconcile_integrations() {
  local mode=$1
  mkdir -p "${HOME}/.claude" "${HOME}/.codex" "${HOME}/.config/opencode"

  if [[ ${tool_failed[herdr]:-0} == 0 ]] && command -v herdr >/dev/null 2>&1; then
    local target
    # Pi's Herdr extension is already supplied by Home Manager's managed link farm.
    for target in claude codex opencode; do
      if [[ $target != opencode && ${tool_failed[$target]:-0} == 1 ]]; then
        printf 'SKIPPED: Herdr integration: %s (tool update failed)\n' "$target"
        continue
      fi
      run_step "Herdr integration: ${target}" herdr integration install "$target" || true
    done
  fi

  if [[ ${tool_failed[moshi]:-0} == 0 ]] && command -v moshi-hook >/dev/null 2>&1; then
    if [[ ${tool_failed[claude]:-0} == 1 || ${tool_failed[codex]:-0} == 1 ]]; then
      printf 'SKIPPED: Moshi integrations (managed agent update failed)\n'
    else
      run_step 'Moshi integrations' moshi-hook install --target "$MOSHI_TARGETS" || true
    fi
    if [[ $mode == update ]]; then
      run_step 'Moshi user unit restart' systemctl --user restart moshi-hook.service || true
    elif ! systemctl --user is-active --quiet moshi-hook.service; then
      run_step 'Moshi user unit start' systemctl --user start moshi-hook.service || true
    fi
  fi
}

report_tool() {
  local label=$1 command_name=$2
  local version
  if ! version=$(version_of "$command_name"); then
    printf '%-12s %s\n' "${label}:" "version check failed"
    return 1
  fi
  printf '%-12s %s\n' "${label}:" "$version"
  [[ $version != 'not installed' ]]
}

status() {
  local failed=0
  printf 'ai-dev maintenance status\n\n'
  report_tool 'Node' node || failed=1
  report_tool 'Claude' claude || failed=1
  report_tool 'Codex' codex || failed=1
  report_tool 'Pi' pi || failed=1
  report_tool 'Herdr' herdr || failed=1
  report_tool 'Moshi' moshi-hook || failed=1
  report_tool 'OpenCode' opencode || failed=1

  printf '\nHerdr integrations:\n'
  if command -v herdr >/dev/null 2>&1; then
    herdr integration status || failed=1
  else
    printf 'not available\n'
    failed=1
  fi

  printf '\nMoshi runtime:\n'
  if command -v moshi-hook >/dev/null 2>&1; then
    moshi-hook status --json || failed=1
  else
    printf 'not available\n'
    failed=1
  fi
  systemctl --user is-active --quiet moshi-hook.service || failed=1
  if ! ss -H -ltn 'sport = :24543' | awk '{print $4}' | grep -Eq '^127\.0\.0\.1:24543$'; then
    printf 'Moshi listener is not contained on 127.0.0.1:24543\n' >&2
    failed=1
  fi

  return "$failed"
}

finish() {
  if ((${#failures[@]} > 0)); then
    printf '\nMaintenance completed with %d failure(s):\n' "${#failures[@]}" >&2
    printf '  - %s\n' "${failures[@]}" >&2
    return 1
  fi
  printf '\nMaintenance completed successfully.\n'
}

main() {
  local mode=${1:-}
  case "$mode" in
    ensure-present | update)
      install_or_update_tools "$mode"
      reconcile_pi_package "$mode"
      reconcile_integrations "$mode"
      finish
      ;;
    status)
      status
      ;;
    -h | --help | help)
      usage
      ;;
    *)
      usage >&2
      return 2
      ;;
  esac
}

main "$@"
