#!/usr/bin/env bash
# Seeds a throwaway vault for the V1 click-through (docs/TASKS.md): 2 projects,
# 4 tasks linked via parent_project, an area, a resource, and a computed.yaml
# with a rollup count, a rollup percent and a deliberate divide-by-zero.
# Usage: scripts/seed-v1-vault.sh [dir]   (default /tmp/iris-v1-vault; wiped first)
set -euo pipefail
V="${1:-/tmp/iris-v1-vault}"
IRIS="$(cd "$(dirname "$0")/.." && pwd)/target/debug/iris"
[ -x "$IRIS" ] || { echo "build first: cargo build -p iris-cli" >&2; exit 1; }
rm -rf "$V"; mkdir -p "$V"
i() { "$IRIS" -C "$V" "$@" </dev/null >/dev/null; }
i init
i create projects/alpha.md -t project --body Alpha
i create projects/beta.md -t project --body Beta
i create areas/health.md -t area --body Health
i create resources/book.md -t resource --body Book
for t in a1 a2 a3 b1; do i create "tasks/$t.md" -t task --body "task $t"; done
id() { grep -m1 '^id:' "$V/$1" | cut -d' ' -f2; }
for t in a1 a2 a3; do i update "tasks/$t.md" --add-relation "parent_project:$(id projects/alpha.md)"; done
i update tasks/b1.md --add-relation "parent_project:$(id projects/beta.md)"
i update tasks/a1.md --status done
cat > "$V/.iris/computed.yaml" <<'YAML'
project:
  - name: tasks_total
    kind: rollup
    relation: parent_project
    direction: in
    fn: count
  - name: tasks_done_pct
    kind: rollup
    relation: parent_project
    direction: in
    where: 'status == "done"'
    fn: percent
  - name: broken
    kind: formula
    expr: '1 / 0'
YAML
echo "seeded $V — open it in Iris (swift run Iris)"
