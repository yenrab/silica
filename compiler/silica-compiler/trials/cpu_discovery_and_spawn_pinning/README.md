# cpu_discovery trials

Silica sources covering **CPU topology / affinity** surface from `cpu_topology_implementation_plan` (parser Phase C onward): optional third `spawn` argument, nullary topology list helpers, `get_cpu_topology`, unary `get_core_capabilities`.

| File | Intent |
|------|--------|
| `cpu_spawn_third_arg_affinity.silica` | `spawn(..., ..., int64)` and `spawn(..., ..., List[int64,normal])` third arguments. |
| `cpu_spawn_third_nonliteral_affinity.silica` | Third argument is a non-literal `int64` expression (`2 + 3`). |
| `cpu_topology_runtime_queries.silica` | `get_efficiency_cores`, `get_performance_cores`, `get_cpu_topology`, `get_core_capabilities`; printed value is always `0` so `.scout` output is machine-independent. |
| `cpu_topology_phase_h_verify.silica` | Phase H **fixture**: five `print_int64` lines (flags) + `println`; **`cpu_topology_phase_h_verify.scout`** is the **target** golden: five `1`s (sentinel / topology / core0 / numa checks on Apple Silicon) plus one line for `echo $?` (`2` = `:ok` atom index today). Stub `main` does not print yet — **`make integrate` will mismatch** this `.scout` until `sequence` lowers. |

## Phase H (verification)

**Static checks** (no binary): from `compiler/silica-compiler`, run:

```bash
make -C trials/cpu_discovery phase-h
```

This runs `phase_h_static.sh`, which asserts that `src/emitter/apple_silicon/terms/prims/prims_actors_runtime_asm.silica` still defines the sysctl-backed topology/capability symbols and the **`get_core_capabilities`** failure sentinel.

**Runtime / `.scout` trials** for topology builtins are **not** executed end-to-end yet: the emitter currently emits a **stub `main`** for `sequence proc[concurrency, …]` (see generated `.sams`), so `print_int64` and runtime helpers in these sources do not run—including **`cpu_topology_phase_h_verify.silica`**. Remove **`INTEGRATE_PENDING`** only after lowering is implemented and `.ascomp` / `.scout` goldens are regenerated.

## Build

From this directory (with `../../src/silica-compiler` built):

```bash
make
# or
make integrate   # diffs .sams vs .ascomp when present; runs and checks .scout
```

## Top-level `trials/Makefile integrate`

While `INTEGRATE_PENDING` is present in this directory, the parent integrate target **skips** `cpu_discovery` so missing assembly goldens do not fail the tree. After you add `.ascomp` files for each trial (copy from generated `.sams` once they match the intended codegen) and confirm `.scout` files, **delete `INTEGRATE_PENDING`** so `cpu_discovery` participates in the full integrate run.
