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

## Not for cutover

Do not promote this tree over production `src/`. Keep DeviceIO (and follow-on class-A) fixes in `src_selfhost/`; refresh this staging tree only when rebuilding the seed.
