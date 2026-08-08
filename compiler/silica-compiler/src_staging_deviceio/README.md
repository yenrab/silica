# Staging overlay seed (DeviceIO)

Temporary tree used to rebuild a **seed** `silica-compiler` that understands DeviceIO file intrinsics (`read_lines`, `file_exists`, `append_file`, `delete_file`) without editing frozen `src/`.

## How it was built

1. Copy frozen `src/` → this directory (exclude build artifacts).
2. Overlay / surgically patch TC, SIR, emitter, effect checker, and related helpers for DeviceIO + `len` alias + E1047 disable (named structs used throughout the compiler).
3. `make build` with `silica-boot` → `./silica-compiler`.

## Use as HOST for self-host

```bash
make -C ../src_selfhost build-selfhost \
  HOST_COMPILER="$(pwd)/silica-compiler"
```

Or copy the binary over the frozen `src/silica-compiler` artifact (sources under `src/` stay frozen).

## Emitter call-ABI fixes (required for unsmoking emit_sir_function)

In `emitter/apple_silicon_mac/terms/term_emitter.silica` (mirrored in `src/` and `src_selfhost/`):

1. **Discarded DeviceIO lets** — `compute_let_rhs_spill_reg` returns no spill for `_` / `%_` so `_: int8 <- println(...)` does not park `0` in X19–X28 (which later got reloaded as the string arg to `control@function_prologue`).
2. **Tail + live stack args** — `emit_generic_user_call_asm_inner` forces non-tail when call-arg tuple/record slabs are still on SP (p5 `LDP`+`BR` was popping the arg region and corrupting LR / hanging in `term_to_asm_debug_with_outer`).

Rebuild this staging tree, install as the new seed, then recompile `src_selfhost` before removing the smoked `emit_sir_function` body.

**Seed rebuild / silica-boot (in progress):** boot was extended for modern surface syntax needed by this tree: `?` / `ref?(L,space,T)`, `region(L,space)`, `buf(L,space,T,N)`, atom types/literals `:name`, `List[…]` type args, `mem(space)` as a type arg, `sequence`/`produces`/`pure`/`end`, anonymous records vs `{` blocks, keyword field names (`.region`), list patterns `[]`/`[h,t]`, `export trait`, symbolic buf capacity, bare `_` wildcards. Staging `type_interner.silica` now parses; remaining boot gaps are deeper stdlib (`OrderedMap` `required {…}` trait blocks, etc.). Use `CARGO_TARGET_DIR=…/silica-bootstrap-compiler/target` when rebuilding boot so the binary is not written only to the sandbox cache.

**Also:** `var@emit_ldr_mem_ref` is mirrored from `src/`. Fixed unbalanced parens on WeightedGraph printer in staging/selfhost `type_interner`.

## Not for cutover

Do not promote this tree over production `src/`. Keep DeviceIO (and follow-on class-A) fixes in `src_selfhost/`; refresh this staging tree only when rebuilding the seed.
