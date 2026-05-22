# FFI trial harness

Phase 0 provides linkable C wrapper fixtures for outbound FFI compiler work. The Silica compiler is not involved in this phase.

## Layout

```text
ffi_addition/
  fixtures/
    dangerous_exposure_source/
      legacy/
        silica_legacy_math_wrapper.h    # C header (authoring reference)
        silica_legacy_math_wrapper.meta # sidecar metadata
      text/
        silica_text_wrapper.h
        silica_text_wrapper.meta
      lib/
        libsilica_legacy_math.a         # built by Makefile
        libsilica_text.a
    src/                                # C sources (not compiled by Silica)
      silica_legacy_math.c
      silica_text.c
  Makefile
  README.md
```

## Wrappers

| Symbol | Library | Purpose |
| ------ | ------- | ------- |
| `silica_legacy_math_add_int64` | `libsilica_legacy_math.a` | Scalar int64 add |
| `silica_text_echo` | `libsilica_text.a` | String in/out via ptr+len C ABI; prefixes input with `Echo: ` |

Sidecar `.meta` files follow the FFI wrapper specification: `link_library`, `wrapper { symbol, result, error_domain }`. No `blocking` field (cast-mediated FFI worker model).

## Phase 0

Build fixtures and verify symbol exports:

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-0
```

`phase-a` is an alias for `phase-0`.

## Phase 3

Module checker enforces `dangerous_*` naming, export rules for raw foreign bindings, and `wrapper_meta` / `meta` path constraints (no sidecar loading yet).

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-3
```

Trials under `module_addition/`:

| Trial | Expect |
| ----- | ------ |
| `dangerous_naming_valid.silica` | compile OK through type check |

## Phase 4

Sidecar `.meta` loader: reads referenced sidecar files, validates `link_library` and `wrapper { symbol, result, error_domain }`, associates foreign bindings, and records link libraries on `SIRModule`.

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-4
```

Trials under `metadata_addition/` (fixture tree symlinked into each temp compile dir):

| Trial | Expect |
| ----- | ------ |
| `dangerous_sidecar_match.silica` | compile OK through metadata pass and type check |

The valid trial uses `fixtures/dangerous_exposure_source/legacy/silica_legacy_math_wrapper.meta`.

## Phase 5

Cast-mediated FFI worker placement: `external_danger` sequences only in spawn-passed worker behaviors, `dangerous_*` calls only inside those sequences, and cast-only client behaviors that initiate foreign work.

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-5
```

Trials under `placement_addition/` (stub + app modules in each temp compile dir):

| Trial | Expect |
| ----- | ------ |
| `dangerous_ffi_worker_valid.silica` | compile OK through placement pass and emit |

## Phase 6

Structural taint for `dangerous_*` returns: scalar and region tracking, `produces pure` enforcement, message boundaries (including one FFI result cast per worker `external_danger` sequence), and restricted-effect use.

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-6
```

Trials under `taint_addition/` (stub + app modules in each temp compile dir):

| Trial | Expect |
| ----- | ------ |
| `dangerous_taint_worker_valid.silica` | compile OK through taint pass and emit |

## Phase 7

Silica-side ABI validation for foreign `c_wrapper` declarations and adapter/raw string layering (no C header parsing).

```bash
make -C compiler/silica-compiler/trials/ffi_addition phase-7
```

Trials under `abi_addition/`:

| Trial | Expect |
| ----- | ------ |
| `dangerous_scalar_abi_valid.silica` | compile OK through ABI pass and emit |
| `dangerous_string_two_layer_valid.silica` | compile OK (adapter `string` over raw ptr+len) |

Expected-failure FFI cases live under `trials/error_enforcement_addition/ffi_addition/`, where they are compared as `.cur_fail` vs `.golden_fail` with the rest of the compiler error-enforcement suite.

## Rebuilding fixtures

After editing C sources or sidecar files:

```bash
make -C compiler/silica-compiler/trials/ffi_addition clean
make -C compiler/silica-compiler/trials/ffi_addition fixtures
```

Archives are built for **macOS AArch64** (`-arch arm64`) using `clang` and `ar`.
