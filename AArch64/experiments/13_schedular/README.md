# Work-Stealing Scheduler Demonstration

THIS IS CURRENTLY NON-FUNCTIONAL AND IS ONLY FOR CONCEPTUALIZATION PURPOSES


This demonstration application showcases a work-stealing scheduler for Silica, designed to show Erlang engineers how Silica's actor-based concurrency model handles process migration between CPU cores.

## Features

- **Work-Stealing**: Cores that run out of work can steal processes from other cores
- **Process Affinity**: Supports three affinity types:
  - **Pinned**: Processes pinned to a specific core (cannot be moved)
  - **Group-Assigned**: Processes assigned to core groups (performance or efficiency cores)
  - **Any-Core**: Processes that can be stolen by any core
- **Core Groups**: Distinguishes between performance cores and efficiency cores
- **Stealing Rules**: 
  - Steals up to half of processes with messages (capped at 5 total)
  - Respects affinity constraints
  - Prevents concurrent stealing (scheduler processes requests sequentially)

## Architecture

### Actors

1. **Scheduler Actor**: Central coordinator that:
   - Tracks process assignments per core
   - Manages work-stealing requests
   - Enforces affinity rules
   - Logs metrics

2. **Core Actors**: One per physical core that:
   - Track processes assigned to the core
   - Monitor process idle/busy status
   - Respond to scheduler queries
   - Execute process move commands

3. **Worker Process Actors**: Long-running computational actors that:
   - Process messages with complex computation
   - Notify core actors of status changes
   - Maintain state across messages

## Message Types

### Scheduler Messages
- `CoreIdle(int)`: Core requesting work steal
- `ProcessCountResponse(int, int, int)`: Response with core_id, total processes, processes with messages
- `MoveComplete(actor_ref, int, int)`: Confirmation of process move
- `GetMetrics`: Request for scheduler metrics

### Core Messages
- `QueryProcessCount`: Query from scheduler
- `MoveProcess(actor_ref)`: Add process to core
- `RemoveProcess(actor_ref)`: Remove process from core
- `WorkerStatus(actor_ref, bool)`: Worker status update (active/idle)

## Current Status

This is an initial implementation with:
- ✅ Type definitions and message structures
- ✅ Basic actor behaviors
- ✅ Placeholder move function
- ⚠️ Work-stealing algorithm (partially implemented)
- ⚠️ Core initialization (TODO)
- ⚠️ Worker process spawning (TODO)
- ⚠️ Full stealing logic (TODO)

## Building

```bash
make              # Build full executable
make test         # Test compilation only
make clean        # Clean generated files
```

## Future Enhancements

- Complete work-stealing algorithm implementation
- Core topology detection and core actor initialization
- Worker process spawning with various affinities
- Message queue checking for accurate stealing decisions
- Metrics collection and reporting
- Runtime actor migration (currently placeholder)

## Notes for Erlang Engineers

This demonstration shows how Silica's actor model differs from Erlang's:

1. **Core Affinity**: Silica allows explicit control over which cores actors run on
2. **Type Safety**: All messages are type-checked at compile time
3. **Effect Tracking**: Process monad tracks concurrency effects
4. **Memory Regions**: Region-based memory management (not shown in this demo)

The work-stealing pattern is similar to Erlang's scheduler, but with explicit core affinity control.
