#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

repo=".opencode/vendor/ui-ux-pro-max"
skill=".opencode/skills/ui-ux-pro-max"
source="$repo/.claude/skills/ui-ux-pro-max"

if [ ! -d "$repo/.git" ]; then
  mkdir -p .opencode/vendor
  git clone --depth 1 https://github.com/nextlevelbuilder/ui-ux-pro-max-skill "$repo"
fi

if [ -e "$skill" ] && [ ! -L "$skill" ]; then
  echo "error: $skill exists and is not a symlink; remove it first" >&2
  exit 1
fi

mkdir -p .opencode/skills
ln -sfn "../vendor/ui-ux-pro-max/.claude/skills/ui-ux-pro-max" "$skill"
printf 'ui-ux-pro-max: %s\n' "$(readlink -f "$skill")"
