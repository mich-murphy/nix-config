#!/bin/bash
input=$(cat)

MODEL=$(echo "$input" | jq -r '.model.display_name')
PCT=$(echo "$input" | jq -r '.context_window.used_percentage // 0' | cut -d. -f1)
COST=$(echo "$input" | jq -r '.cost.total_cost_usd // 0')

# Build progress bar
BAR_WIDTH=20
FILLED=$((PCT * BAR_WIDTH / 100))
EMPTY=$((BAR_WIDTH - FILLED))
BAR=""
for ((i=0; i<FILLED; i++)); do BAR="${BAR}▓"; done
for ((i=0; i<EMPTY; i++)); do BAR="${BAR}░"; done

# Color the bar based on usage
if [ "$PCT" -ge 80 ]; then
  COLOR="\033[31m"  # red
elif [ "$PCT" -ge 50 ]; then
  COLOR="\033[33m"  # yellow
else
  COLOR="\033[32m"  # green
fi
RESET="\033[0m"

printf "[%s] ${COLOR}%s${RESET} %s%% | \$%.2f USD" "$MODEL" "$BAR" "$PCT" "$COST"
