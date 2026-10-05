# device_actor_addition

Board-only trials for device workers and the poke prims (`silica_device_actor_specification.md`;
plan `esp32s3_peek_poke_implementation_plan.md` work item 19c). Every register access happens inside
a device worker's `sequence proc[register_rwr] ... produces pure ... end`; `main` only spawns, casts
and waits. The descriptions and workers live in `lib/` (one description per device tag across the
suite: the checker rejects a second description for a tag anywhere in the units it compiles
together), the programs at the top level `use` them.

| Trial | Shows |
| --- | --- |
| `device_uart_hello` | a UART0 worker: `peek(:status)` polled, `poke(:fifo)` per byte; prints `hi` |
| `device_gpio_button` | a GPIO worker: `poke(:enable_w1ts / :out_w1ts / :out_w1tc)` for the LED, `peek(:in)` for the Left button, result by `cast` to a printer actor |
| `device_discarded_peek` | `_: uint32 <- peek(w, :status) impl Register32 {}` is still one load (check the `l32i` in `lib/device_uart_lib.sams`) |
| `device_cast_registered` | `spawn_device_registered` + `cast_device_registered` through the third (device) registry |
| `device_client_result` | the §4.6 handshake: an ordinary client casts the device work and the worker casts the result to the named receiver |
| `device_worker_no_window` | `spawn_device` / `cast_device` without `map_device` compile and run on every target (shared `.scout`) |
| `device_window_param_survives_let` | a call-free register body with lets before its peek/poke keeps the window parameter (the `spi2_enable` shape; emitter_core `type_name_is_region_handle` / `body_has_device_prim`) |
| `device_fault_lines` | a device worker that ends on a CPU exception (head of an empty list: LoadProhibited at 8) reports `exccause` / `epc` / `excvaddr` after `reason_tag`; plain `spawn_device` flags the ACB through `silica_rt_actor_mark_device` right after the spawn |
| `device_fault_lines_registered` | the same through `spawn_device_registered` (the registry entry flags the ACB itself) |

`refusals/` holds the two programs the ESP32-S3 emitter refuses at assembly time (see its README).

## Goldens and targets

- The eight device trials carry `<stem>.ESP32-S3_raw.scout` only: on a hosted target the shared
  compiler accepts them and the emitter rejects the prims with E2220 (`device_hosted_rejection_trial`
  under `error_enforcement_addition` pins that message), so they have no host golden and this
  directory is `INTEGRATE_PENDING` (skipped and not counted by the host run).
- `compare_scout_normalized.sh` (the board driver applies it to every trial here) folds `actor_id`,
  `supervisor_acb` and the `[silica] fault/abort` lines as `supervisors_addition` does, plus the `epc` line of
  the device fault report: a code address that moves with every build. `excvaddr` is compared as printed.
- `device_worker_no_window` has a shared `.scout`; it can be moved to `actors_addition` once its
  `.ascomp` goldens are recorded from a reviewed emission on each hosted target.
- The goldens are hand-derived from the port test board (`PORT_TEST_BOARD_byu_idaho_v4.0_feb26.md`):
  Left idle reads 0; UART0 is shared with the runtime console so a worker's bytes appear in the
  program output. Nobody presses Left during the run.

Board run: `make integrate TRIAL_TARGET=ESP32-S3_raw TRIAL_SUITES=device_actor_addition` from `trials/`.
