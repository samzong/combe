#!/bin/bash
input=$(cat)
claude_dir="${CLAUDE_CONFIG_DIR:-$HOME/.claude}"

rl=$(echo "$input" | jq -c '.rate_limits // empty' 2>/dev/null)
if [ -n "$rl" ]; then
  tmp=$(mktemp "$claude_dir/rate-limits.json.XXXXXX" 2>/dev/null) &&
    echo "$input" | jq -c '.rate_limits + {updated_at: (now | floor)}' > "$tmp" 2>/dev/null &&
    mv -f "$tmp" "$claude_dir/rate-limits.json" 2>/dev/null ||
    rm -f "$tmp" 2>/dev/null
fi

model=$(echo "$input" | jq -r '.model.display_name')
cwd=$(echo "$input" | jq -r '.workspace.current_dir')
dir="${cwd/#$HOME/\~}"

GREEN=$'\033[32m'
BLUE_BOLD=$'\033[1;34m'
RED=$'\033[31m'
YELLOW=$'\033[33m'
DIM=$'\033[2m'
RESET=$'\033[0m'

git_segment=""
if git -C "$cwd" --no-optional-locks rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  branch=$(git -C "$cwd" --no-optional-locks symbolic-ref --short HEAD 2>/dev/null || git -C "$cwd" --no-optional-locks rev-parse --short HEAD 2>/dev/null)
  if [ -n "$branch" ]; then
    if [ -n "$(git -C "$cwd" --no-optional-locks status --porcelain 2>/dev/null)" ]; then
      git_segment="${BLUE_BOLD} (${branch} ${RED}✗${BLUE_BOLD})${RESET}"
    else
      git_segment="${BLUE_BOLD} (${branch} ${GREEN}✔${BLUE_BOLD})${RESET}"
    fi
  fi
fi

five=$(echo "$input" | jq -r '.rate_limits.five_hour.used_percentage // empty')
week=$(echo "$input" | jq -r '.rate_limits.seven_day.used_percentage // empty')

quota_segment=""
if [ -n "$five" ]; then
  five_r=$(printf '%.0f' "$five")
  if [ "$five_r" -ge 90 ]; then five_color=$RED
  elif [ "$five_r" -ge 70 ]; then five_color=$YELLOW
  else five_color=$DIM
  fi
  quota_segment="${quota_segment} ${DIM}5h:${five_color}${five_r}%${RESET}"
fi
if [ -n "$week" ]; then
  week_r=$(printf '%.0f' "$week")
  if [ "$week_r" -ge 90 ]; then week_color=$RED
  elif [ "$week_r" -ge 70 ]; then week_color=$YELLOW
  else week_color=$DIM
  fi
  quota_segment="${quota_segment} ${DIM}7d:${week_color}${week_r}%${RESET}"
fi

printf '%s\n' "${DIM}${model}${RESET} ${GREEN}${dir}${git_segment}${RESET}${quota_segment} »"
