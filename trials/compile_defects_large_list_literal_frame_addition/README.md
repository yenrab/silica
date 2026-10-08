# compile_defects_large_list_literal_frame_addition

One valid Silica program that fails to build on the ESP32-S3 target (see
`defect_large_list_literal_frame.silica` and the `compile_defects_addition` README for why a defect of this
kind gets a directory of its own).

- `defect_large_list_literal_frame`: a function whose body is a ~400-element list literal of records
  (`{ name, offset, width, access }`) builds the whole list in its own stack frame. At 397 elements the frame
  is 76,288 bytes, over the Xtensa `entry` limit of 32,760 bytes, so the assembler rejects the function; board
  actor stacks are far smaller than that anyway. The Mac/Linux emitters tolerate the frame.
  Found 2026-10-05 while generating a device description.

Reproduce: compile with `binaries/silica-compiler-ESP32-S3_raw` and assemble (or build an image with
`board_app.sh`, no run); the assembler reports the `entry` immediate out of range. On the host,
`make integrate` here passes.

Expected output (`.scout`): `400`, `1596`, `4`, then the exit status `0`.

Move to `list_addition` once fixed (large list literals built on the heap), then delete this directory.
