# Supervisors Implementation — Development Plan

**Date started**: April 18, 2026  
**Last design update**: May 14, 2026
**Primary specification**: [silica-specification.md](silica-specification.md) — §15.4 (Supervision and Fault Tolerance), §16.2.7

This document describes the target supervisor design. Earlier prototypes used public `spawn_linked`, user-authored supervisor behaviors, and `start_child(spec)`. Those are no longer the source-level design. The runtime may still use internal link-like metadata, but user code creates and talks to supervisors through the builtins described here.

---

## 1. Target Model

A supervisor is a runtime-managed process with a built-in behavior. User code cannot replace or modify that behavior.

User code defines a supervisor by implementing the canonical `stdlib/Supervisor.silica` trait:

```silica
use Supervisor;

impl MySup for Supervisor;

fn init(initial_state: ActorState) -> (
    supervisor_flags,
    List[child_spec, mem(normal)]
) {
    ...
}
```

`init/1` is the programmer-provided configuration callback. It is called exactly once when the supervisor starts. It returns the restart strategy and the initial child specs. The runtime then spawns those children, records them in the supervisor child table, and applies restart policy.

There is no user-written supervisor behavior function. A supervisor receives only the fixed supervisor-maintenance protocol through `call_supervisor`.

---

## 2. Public Surface

### Reference Types

```silica
actor_ref
supervisor_ref
```

`actor_ref` refers to ordinary actors. `supervisor_ref` refers to runtime-managed supervisors and is intentionally distinct.

### Creation Builtins

```silica
spawn(initial_state, behavior_fn) -> actor_ref

spawn_registered(
    initial_state,
    behavior_fn,
    name: atom
) -> actor_ref

spawn_registered_supervisor(
    supervisor_impl_type,
    initial_state,
    name: atom
) -> supervisor_ref
```

`spawn_registered` creates an ordinary registered actor. It is not linked and not supervisory.

`spawn_registered_supervisor(MySup, initial_state, :my_sup)` creates a runtime-managed supervisor, registers it, calls `MySup.init(initial_state)`, materializes the initial child specs, and returns a `supervisor_ref`.

There is no public `spawn_linked` in this design.

### Calls

```silica
call(actor_ref, ActorMessage) -> Reply

call_supervisor(
    supervisor_ref,
    SupervisorMessage
) -> {
    tag: :child_started | :ok | :children | :count | :child_info | :error,
    child: actor_ref,
    status: atom,
    children: List[
        {
            id: atom,
            agent_type: atom,
            child: actor_ref,
            restart: :permanent | :temporary | :transient
        },
        mem(normal)
    ],
    count: int64,
    child_info: {
        id: atom,
        agent_type: atom,
        child: actor_ref,
        restart: :permanent | :temporary | :transient
    },
    error: atom
}
```

Invalid source forms:

```silica
call(supervisor_ref, ...)
cast(supervisor_ref, ...)
call_supervisor(actor_ref, ...)
```

There is no supervisor cast path. Supervisor maintenance is synchronous through `call_supervisor`.

`call_supervisor` has one concrete return type. The reply type is not inferred from the message variant.

---

## 3. Supervisor Types

### supervisor_flags

```silica
supervisor_flags = {
    strategy: :one_for_one | :one_for_all | :rest_for_one,
    allowed_restart_count: int64,
    restarts_time_frame: int64
}
```

### child_spec

```silica
child_spec = {
    id: atom,
    agent_type: atom,
    initial_state: ActorState,
    behavior: fn(msg: ActorMessage, state: ActorState) -> ChildReturn,
    restart: :permanent | :temporary | :transient,
    shutdown: int64
}
```

The child spec uses explicit fields:

```silica
initial_state: 0,
behavior: worker_fn
```

It does not use a `start: (initial_state, behavior_fn)` tuple.

The supervisor owns child lifecycle. User code sends a `child_spec`; the supervisor runtime spawns, tracks, restarts, terminates, and removes the child.

### child_info

The exact `child_info` record layout can be finalized during implementation, but it must at least identify the child row and current child reference:

```silica
child_info = {
    id: atom,
    agent_type: atom,
    child: actor_ref,
    restart: :permanent | :temporary | :transient
}
```

---

## 4. Supervisor Protocol

```silica
SupervisorMessage =
    { op: :add_child, child: child_spec }
  | { op: :remove_child, id: atom }
  | { op: :restart_child, id: atom }
  | { op: :terminate_child, id: atom }
  | { op: :which_children }
  | { op: :count_children }
  | { op: :get_child, id: atom }
```

Suggested successful replies:

| Message | Successful reply |
|---------|------------------|
| `{ op: :add_child, child: child_spec }` | `{ tag: :child_started, child: actor_ref, ... }` |
| `{ op: :remove_child, id: atom }` | `{ tag: :ok, status: :removed, ... }` |
| `{ op: :restart_child, id: atom }` | `{ tag: :child_started, child: actor_ref, ... }` |
| `{ op: :terminate_child, id: atom }` | `{ tag: :ok, status: :terminated, ... }` |
| `{ op: :which_children }` | `{ tag: :children, children: List[child_info, mem(normal)], ... }` |
| `{ op: :count_children }` | `{ tag: :count, count: int64, ... }` |
| `{ op: :get_child, id: atom }` | `{ tag: :child_info, child_info: child_info, ... }` |

Any operation may return `{ tag: :error, error: reason_atom, ... }`.

No shutdown-clean messages are part of the public supervisor protocol.

---

## 5. Runtime Responsibilities

Each supervisor has a heap-allocated, growable child table. Rows are created from:

1. `child_spec` values returned by `init/1`.
2. `child_spec` values accepted through `call_supervisor(..., { op: :add_child, child: spec })`.

Each row records at least:

- current `actor_ref`
- `id`
- `agent_type`
- `initial_state`
- `behavior`
- `restart`
- `shutdown`
- row order for `:rest_for_one`

When a supervised child exits, the runtime uses the owning `supervisor_ref` and child table row to deliver a structured failure notification to the supervisor ingress. The runtime-owned supervisor behavior drains the ingress before ordinary supervisor-maintenance calls and applies the configured restart strategy.

Restart uses the stored `initial_state` and `behavior` fields from the row. Public user code does not call `spawn_linked`; the runtime uses an internal child-spawn operation that records the owning supervisor and child metadata atomically before the child can process messages.

---

## 6. Implementation Phases

### Phase A — Type and Parser Surface

- Add `supervisor_ref`.
- Add `spawn_registered_supervisor`.
- Add `call_supervisor`.
- Add concrete `SupervisorMessage` handling and the direct record return type for `call_supervisor`.
- Remove public `spawn_linked` from the language surface.
- Reject `call(supervisor_ref, ...)`, `cast(supervisor_ref, ...)`, and `call_supervisor(actor_ref, ...)`.

### Phase B — Canonical Supervisor Trait

- Update `stdlib/Supervisor.silica` to the canonical `init/1` shape.
- Replace `child_spec.start` with `child_spec.initial_state` and `child_spec.behavior`.
- Require every `impl T for Supervisor` to provide a matching `init/1`.
- Ensure `init/1` is invoked only by supervisor startup, not by ordinary actor spawn.

### Phase C — Runtime Supervisor Startup

- Implement `spawn_registered_supervisor`.
- Allocate a supervisor runtime control block and return `supervisor_ref`.
- Register the supervisor name.
- Call `T.init(initial_state)` exactly once.
- Materialize returned child specs into the heap child table.
- Spawn each child with internal supervised-child metadata.

### Phase D — Supervisor Protocol

- Implement `call_supervisor`.
- Implement the fixed maintenance messages:
  - `:add_child`
  - `:remove_child`
  - `:restart_child`
  - `:terminate_child`
  - `:which_children`
  - `:count_children`
  - `:get_child`
- Return only the concrete record type shown in the `call_supervisor` signature.

### Phase E — Restart Semantics

- Implement `:permanent`, `:temporary`, and `:transient`.
- Implement `:one_for_one`, `:one_for_all`, and `:rest_for_one`.
- Enforce `allowed_restart_count` within `restarts_time_frame`.
- On escalation, terminate the supervisor and propagate failure to its parent supervisor if it has one.

### Phase F — Failure Reporting

- Keep supervisor ingress separate from `FailureReporter`.
- Deliver structured child-exit metadata to the supervisor.
- Deliver human-readable unwind reports to the root `FailureReporter`.
- Preserve the existing root stderr fallback when no supervisor/failure reporter is available.

### Phase G — Integration and Compatibility Cleanup

- Rename or retire trials that imply public `spawn_linked` or user-authored supervisor behavior.
- Update examples to use `spawn_registered_supervisor` and `call_supervisor`.
- Update goldens and diagnostics.
- Add negative tests for invalid supervisor calls and invalid ordinary actor/supervisor ref mixing.

---

## 7. Example

```silica
use Supervisor;

impl MySup for Supervisor;

fn worker_fn(msg: int64, state: int64) -> (:reply, int64, int64) {
    (:reply, msg + state, state)
}

fn init(initial_state: int64) -> (
    {
        strategy: :one_for_one | :one_for_all | :rest_for_one,
        allowed_restart_count: int64,
        restarts_time_frame: int64
    },
    List[
        {
            id: atom,
            agent_type: atom,
            initial_state: int64,
            behavior: fn(msg: int64, state: int64) -> (:reply, int64, int64),
            restart: :permanent | :temporary | :transient,
            shutdown: int64
        },
        mem(normal)
    ]
) {
    ...
}

fn main() -> int64 {
    sequence proc[concurrency]
        sup: supervisor_ref <- spawn_registered_supervisor(MySup, 0, :my_sup);

        reply: {
            tag: :child_started | :ok | :children | :count | :child_info | :error,
            child: actor_ref,
            status: atom,
            children: List[
                {
                    id: atom,
                    agent_type: atom,
                    child: actor_ref,
                    restart: :permanent | :temporary | :transient
                },
                mem(normal)
            ],
            count: int64,
            child_info: {
                id: atom,
                agent_type: atom,
                child: actor_ref,
                restart: :permanent | :temporary | :transient
            },
            error: atom
        } <- call_supervisor(sup, {
            op: :add_child,
            child: {
                id: :worker,
                agent_type: :worker,
                initial_state: 0,
                behavior: worker_fn,
                restart: :permanent,
                shutdown: 0
            }
        } impl SupervisorMessage {});

        sum: int64 <- case reply.tag of {
            :child_started -> {
                sequence proc[concurrency]
                    value: int64 <- call(reply.child, 40 impl ActorMessage {})
                produces
                    pure value
                end
            }
            :error -> -1;
            _: atom -> -1
        }
    produces
        pure 0
    end
}
```

The source-level distinction is explicit:

```silica
worker: actor_ref <- spawn_registered(0, worker_fn, :worker);
sup: supervisor_ref <- spawn_registered_supervisor(MySup, 0, :my_sup);
```

There is no hidden inference from `impl MySup for Supervisor`, no dummy supervisor behavior, no public `spawn_linked`, and no polymorphic `call_supervisor` return.
