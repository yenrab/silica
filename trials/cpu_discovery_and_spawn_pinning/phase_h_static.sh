#!/usr/bin/env bash
# Phase H (cpu_topology_implementation_plan): static verification that Apple Silicon
# runtime assembly defines topology/capability symbols and failure sentinel.
# Full end-to-end trials need sequence lowering in the emitter (main is currently a stub
# for proc[concurrency] sequence blocks — see cpu_topology_runtime_queries.sams).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ASM="$ROOT/src/emitter/apple_silicon/terms/prims/prims_actors_runtime_asm.silica"
if [[ ! -f "$ASM" ]]; then
  echo "Phase H: missing $ASM" >&2
  exit 1
fi
checks=(
  "_silica_rt_get_cpu_topology"
  "_silica_rt_get_core_capabilities"
  "_silica_rt_apple_hw_optional_caps_list"
  "_silica_rt_apple_build_cache_levels"
  "L_cap_emit_sentinel"
  "hw.optional.neon"
  "hw.memsize"
)
for s in "${checks[@]}"; do
  if ! grep -qF "$s" "$ASM"; then
    echo "Phase H: expected pattern not found in prims_actors_runtime_asm.silica: $s" >&2
    exit 1
  fi
done
echo "Phase H static checks: OK (${#checks[@]} patterns in prims_actors_runtime_asm.silica)"
