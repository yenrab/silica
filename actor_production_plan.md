# Silica Actor System Implementation Plan

## Overview
This document outlines the prioritized implementation plan for completing the Silica actor system. The actor system is fundamental to Silica's concurrency model, providing message-passing concurrency through actors that can be scheduled on specific CPU cores.

## Current Status
- Parser: ✅ Basic spawn/send/recv parsing implemented
- AST: ✅ SpawnExpr/SendExpr/RecvExpr defined
- Codegen: ✅ LLVM IR generation stubs exist
- Runtime: ✅ Basic actor structs and spawn/send/recv functions exist
- Types: ✅ Basic type inference for actors
- **Blocking Issue**: Function literals parsed but not code-generated

## Priority 1: Core Function Literal Support (BLOCKING)
1. **Implement function literal codegen**
   - Parser accepts `fn(...) -> ... { ... }` syntax
   - `generate_expression()` currently returns "Function literals not yet implemented"
   - Need LLVM IR generation for anonymous functions
   - **Status**: Not implemented

2. **Add function pointer handling**
   - Runtime expects `*mut u8` for behavior functions
   - Codegen needs to generate actual function pointers/closures
   - Must be passable to `silica_actor_spawn()`
   - **Status**: Not implemented

## Priority 2: Actor Behavior Function Semantics
3. **Implement behavior function calling convention**
   - Behavior functions: `(message, state) → state`
   - Runtime needs to invoke these functions correctly
   - Handle message processing and state transitions
   - **Status**: Parser/AST ready, runtime/codegen incomplete

4. **Add actor message loop**
   - Runtime's `silica_actor_spawn()` creates actor but lacks execution loop
   - Need infinite `recv() → process → loop` as per spec
   - Each actor runs in dedicated thread/green thread
   - **Status**: Basic spawn exists, loop missing

## Priority 3: Core Affinity System
5. **Add core affinity to spawn API**
   - Extend `spawn(initial_state, behavior_fn)` to `spawn(initial_state, behavior_fn, core_affinity)`
   - Maintain backward compatibility with default "any core"
   - **Status**: Not implemented

6. **Implement core affinity types**
   - Add `CoreId`, `CoreSet`, `AnyCore` types
   - Support single core, core ranges, or "any core" default
   - Integrate with type system
   - **Status**: Not implemented

7. **Runtime core scheduling**
   - Use OS threading APIs for CPU affinity (pthreads on macOS/AArch64)
   - Schedule actors on specified cores
   - Handle core availability and load balancing
   - **Status**: Not implemented

8. **Default "any core" policy**
   - Runtime automatically selects available cores when no affinity specified
   - Implement load balancing across cores
   - **Status**: Not implemented

## Priority 4: Message Passing System
9. **Complete send/recv runtime implementation**
   - `silica_actor_send()`/`silica_actor_recv()` need proper mailbox semantics
   - FIFO ordering per sender, blocking receive
   - Handle multiple concurrent senders
   - **Status**: Basic functions exist, semantics incomplete

10. **Implement actor references**
    - `actor_ref<Msg>` type needs proper typed reference handling
    - Currently just raw `*mut SilicaActor`
    - Add type safety and lifetime management
    - **Status**: Basic pointer type exists, proper typing missing

## Priority 5: Effect System Integration
11. **Fix mailbox effect parsing**
    - `proc[mailbox<int>]` causes parsing errors
    - Need parameterized effect support: `mailbox<Msg>`
    - **Status**: Parsing fails on parameterized effects

12. **Add concurrency effect checking**
    - Functions using `spawn`/`send`/`recv` need `proc[concurrency, mailbox<Msg>]`
    - Proper effect validation and inference
    - **Status**: Basic effects work, concurrency/mailbox effects incomplete

## Priority 6: Actor Lifecycle & Identity
13. **Implement actor termination**
    - Actors terminate when behavior functions can't handle messages
    - As described in spec section 13.2.4
    - Clean shutdown and resource cleanup
    - **Status**: Not implemented

14. **Add actor identity/self-reference**
    - `self()` function returning `proc[mailbox<Msg>, concurrency] actor_ref<Msg>`
    - Allow actors to reference themselves
    - **Status**: Not implemented

## Priority 7: Testing & Examples
15. **Create working actor test**
    - Use named behavior functions with `return` statements
    - Test basic spawn/send/recv cycle
    - Fix `test_actor_spawn.silica`
    - **Status**: Current test fails due to function literal issues

16. **Add core affinity tests**
    - Test spawning actors on specific cores vs "any core" default
    - Verify core assignment and migration
    - **Status**: Not implemented

17. **Add message passing tests**
    - `test_message_passing.silica` for different message types and patterns
    - Test complex message flows and error handling
    - **Status**: Not implemented

## Implementation Notes

### Dependencies
- Priority 1 blocks all other work (function literals required for behavior functions)
- Priorities 3-7 can be developed in parallel once Priority 1-2 are complete
- Core affinity (Priority 3) requires OS-specific threading APIs

### Testing Strategy
- Start with simple named behavior functions (avoid function literals initially)
- Progress to complex message passing and core affinity
- Each priority level should have working tests before moving to next

### API Evolution
- Maintain backward compatibility where possible
- Core affinity should be optional parameter with sensible default
- Effect system integration can be incremental

### Performance Considerations
- Core affinity enables real-time and performance-critical applications
- Message passing should minimize latency
- Actor scheduling should be efficient

## Success Criteria
- ✅ Basic actor spawning and message passing works
- ✅ Core affinity allows targeting specific CPU cores
- ✅ Effect system properly validates actor operations
- ✅ All spec-defined actor behaviors implemented
- ✅ Comprehensive test suite passes
- ✅ Performance suitable for concurrent applications
