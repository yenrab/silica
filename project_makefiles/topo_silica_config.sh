#!/usr/bin/env bash
# Membership + topological order for silica.config.
# Order: units with no in-tree `use` deps first, then dependents (Kahn).
# main.silica is always emitted last when present.
#
# Usage:
#   topo_silica_config.sh [--list-units]
#   topo_silica_config.sh                 # write ordered paths to stdout
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

MODE=generate

usage() {
  echo "Usage: $0 [--list-units]" >&2
  echo "  --list-units   print sorted membership paths only (no topo)" >&2
  exit 2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --list-units) MODE=list; shift ;;
    -h|--help) usage ;;
    *) usage ;;
  esac
done

list_units() {
  find . \( -path './_wd_probe' -o -path './.git' \) -prune -o -type f -name '*.silica' -print \
    | sed 's|^\./||' \
    | LC_ALL=C sort -u
}

if [[ "$MODE" == list ]]; then
  list_units
  exit 0
fi

units_file=$(mktemp)
trap 'rm -f "$units_file"' EXIT
list_units > "$units_file"

if [[ ! -s "$units_file" ]]; then
  echo "FAIL: no .silica units under $ROOT" >&2
  exit 1
fi

# Edges: dependency module -> dependent path (dep must compile first).
# Module name = basename without .silica. Duplicate basenames are an error.
awk -v units_file="$units_file" '
function modname(path,    base) {
  base = path
  sub(/^.*\//, "", base)
  sub(/\.silica$/, "", base)
  return base
}
function trim(s) {
  gsub(/^[[:space:]]+|[[:space:]]+$/, "", s)
  return s
}
BEGIN {
  n = 0
  while ((getline path < units_file) > 0) {
    if (path == "") continue
    m = modname(path)
    if (m in mod_path && mod_path[m] != path) {
      printf "FAIL: duplicate module basename \"%s\":\n  %s\n  %s\n", \
        m, mod_path[m], path > "/dev/stderr"
      exit 1
    }
    mod_path[m] = path
    paths[++n] = path
    path_mod[path] = m
    indeg[path] = 0
  }
  close(units_file)

  for (i = 1; i <= n; i++) {
    path = paths[i]
    while ((getline line < path) > 0) {
      if (line ~ /^[[:space:]]*use[[:space:]]+/) {
        rest = line
        sub(/^[[:space:]]*use[[:space:]]+/, "", rest)
        sub(/;.*$/, "", rest)
        rest = trim(rest)
        ndeps = split(rest, deps, /,/)
        for (d = 1; d <= ndeps; d++) {
          dep = trim(deps[d])
          if (dep == "" || !(dep in mod_path)) continue
          dep_path = mod_path[dep]
          if (dep_path == path) continue
          key = dep_path SUBSEP path
          if (key in seen_edge) continue
          seen_edge[key] = 1
          adj[dep_path] = adj[dep_path] path "\n"
          indeg[path]++
        }
      }
    }
    close(path)
  }

  # Kahn: ready set sorted for stable order. Hold main.silica until the end.
  ready_n = 0
  for (i = 1; i <= n; i++) {
    path = paths[i]
    if (indeg[path] == 0 && path != "main.silica") {
      ready[++ready_n] = path
    }
  }
  for (i = 2; i <= ready_n; i++) {
    v = ready[i]; j = i - 1
    while (j >= 1 && ready[j] > v) { ready[j + 1] = ready[j]; j-- }
    ready[j + 1] = v
  }

  out_n = 0
  while (ready_n > 0) {
    path = ready[1]
    for (i = 1; i < ready_n; i++) ready[i] = ready[i + 1]
    ready_n--
    out[++out_n] = path
    done[path] = 1
    nk = split(adj[path], kids, "\n")
    for (k = 1; k <= nk; k++) {
      kid = kids[k]
      if (kid == "") continue
      indeg[kid]--
      if (indeg[kid] == 0 && kid != "main.silica") {
        i = ++ready_n
        ready[i] = kid
        while (i > 1 && ready[i - 1] > kid) { ready[i] = ready[i - 1]; i-- }
        ready[i] = kid
      }
    }
  }

  if ("main.silica" in path_mod) {
    if (indeg["main.silica"] != 0) {
      printf "FAIL: main.silica still has %d unmet use deps (cycle?)\n", indeg["main.silica"] > "/dev/stderr"
      exit 1
    }
    out[++out_n] = "main.silica"
    done["main.silica"] = 1
  }

  if (out_n != n) {
    printf "FAIL: use-dependency cycle among .silica units (emitted %d of %d):\n", out_n, n > "/dev/stderr"
    for (i = 1; i <= n; i++) {
      path = paths[i]
      if (!(path in done)) printf "  %s (indegree %d)\n", path, indeg[path] > "/dev/stderr"
    }
    exit 1
  }

  for (i = 1; i <= out_n; i++) print out[i]
}
'
