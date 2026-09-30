#!/usr/bin/env bash
# Point all three Iris repos at scripts/hooks (absolute path: a relative
# core.hooksPath would resolve against design/ for .git-design).
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
chmod +x "$ROOT/scripts/hooks/"*
git --git-dir="$ROOT/.git" config core.hooksPath "$ROOT/scripts/hooks"
for g in .git-private .git-design; do
  [ -d "$ROOT/$g" ] && git --git-dir="$ROOT/$g" config core.hooksPath "$ROOT/scripts/hooks"
done
echo "pre-push docs check installed for: .git $( [ -d "$ROOT/.git-private" ] && echo .git-private ) $( [ -d "$ROOT/.git-design" ] && echo .git-design )"
