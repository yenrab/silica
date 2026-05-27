# Actor Monitor / Demonitor TODO

**Status**: Phase 2 TODO  
**Scope**: actor runtime monitoring primitives

## Summary

Silica exposes `monitor(target: actor_ref) -> monitor_ref` and `demonitor(ref: monitor_ref) -> :ok` for actors, but the current Apple Silicon runtime implementation is still placeholder-level. The type-checking and lowering surfaces exist, but the runtime must still implement real monitor state, monitor reference ownership, `DOWN` message delivery, and cancellation behavior.

This work belongs in Phase 2 because the core actor/supervisor implementation can run without full monitor semantics, while production-quality actor observability needs the runtime behavior to match the specification.

## Current Source State

- `monitor` is accepted as an actor concurrency operation and returns `monitor_ref`.
- `demonitor` is accepted as an actor concurrency operation and returns `:ok`.
- Runtime emission currently exposes the corresponding runtime symbols, but the implementation does not yet maintain monitor tables or deliver `DOWN` messages.

## Required Runtime Behavior

- `monitor(target)` must create an opaque `monitor_ref` owned by the calling actor.
- Each monitor must be independent, even when the same actor monitors the same target multiple times.
- If the target is already dead, the caller must receive a `DOWN` message with the appropriate no-process reason.
- If the target dies later, the caller must receive a `DOWN` message in its standard mailbox.
- `demonitor(ref)` must cancel the monitor so no later `DOWN` message is delivered for that reference.
- `demonitor(ref)` must be safe after the monitored actor has already exited.
- Invalid or foreign monitor references must produce the specified actor error behavior.

## Implementation Notes

- Monitor state should be stored in runtime-owned structures, not in user-visible actor state.
- `monitor_ref` values should not be forgeable or transferable in a way that lets one actor cancel another actor's monitors.
- `DOWN` delivery goes to the standard actor mailbox, not the supervisor ingress path.
- Tests should cover immediate target absence, later target exit, repeated monitors for one target, cancellation before exit, cancellation after exit, and invalid monitor references.

## Completion Criteria

- Runtime monitor tables exist and are cleaned up on actor exit.
- `DOWN` messages are delivered with the specified shape and reason.
- `demonitor` reliably prevents future delivery for the cancelled reference.
- Positive and negative trials cover the behavior above.
