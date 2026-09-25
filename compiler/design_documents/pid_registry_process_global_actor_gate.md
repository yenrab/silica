# PID registry: process-global reference, actor-gated table

## Goals

- **Process-global:** one logical registry per OS process. The runtime holds a single word (`_silica_pid_registry_ref` in BSS) naming the registry **actor**.
- **Actor-gated access:** the dense table, overflow list, and regions live **only** in that actor’s behavior state. Application code must use **`get_pid_registry()`** and send **`registry_beh`** messages (`:insert`, `:lookup`, `:delete`). There is no supported Silica module that exports direct `pid_reg_*` entry points alongside user trials.

## Implementation shape

1. **`pid_registry_actor.silica`** exports only **`registry_beh`**, **`init_pid_registry_get_ref`**, and **`get_pid_registry`**. Table helpers are file-private (`fn` without `export`).
2. **`_silica_pid_registry_init`** (emitter-injected `main` prologue) calls **`init_pid_registry_get_ref`**, spawns the actor with empty table state, and stores the returned **`actor_ref`** in **`_silica_pid_registry_ref`** (see `prims_actors_runtime_asm.silica`).
3. **`get_registry_actor()`** / **`get_pid_registry()`** reads that global slot. Mutations happen only inside **`registry_beh`**, which calls the private helpers.

## Security / discipline note

Silica does not enforce “no direct memory” at the language level. **Actor gating** is enforced by **not exporting** table primitives from any user-linkable unit and by documenting that **`pid_registry_actor`** is the only intended surface. A determined program could still reimplement a table; this design is about **supported** API and clear ownership of global state.

## Related

- **Effects:** only `sequence proc[...] ... produces ... end` carries `proc`; not the function’s `->` return type (see §9.3.1 / E3009).

- `atom_actor_registry_direct_index_design.md` — direct-index + overflow representation.
- Runtime: `_silica_rt_register_actor` also targets `_silica_pid_registry_ref` for the generic “name registry” path; **`_silica_pid_registry_init`** remains the PID-specific bootstrap used by programs that use the PID registry actor.
