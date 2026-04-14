# 08.4 Cast/Concurrency Test Suite

This directory contains comprehensive tests for the **cast()** function and cast-back messaging patterns in the Silica actor system. All tests use real actor behavior (not simulated) and demonstrate actual message passing between actors.

## Test Files

### `test_basic_cast.silica`
**Purpose**: Basic verification of `cast()` functionality
**Tests**:
- `cast()` with `ActorMessage` types
- `cast()` return value (bool) indicating success
- Multiple casts to the same actor
- Actor state management with struct types

### `test_cast_back_pattern.silica`
**Purpose**: Demonstrates the cast-back pattern using `reply_to` field
**Tests**:
- Actor A casts to Actor B
- Actor B processes message and casts back to Actor A using `reply_to` field
- Multiple cast-back messages
- Proper handling of `actor_ref` in message structs

### `test_actor_lambda_captures.silica`
**Purpose**: Tests actor lambdas that capture variables outside their scope
**Tests**:
- Capturing variables from outer scope (multiplier, offset)
- Capturing actor references (collector_ref)
- Using captured variables in actor behavior functions
- Casting to captured actor references

### `test_multi_actor_chain.silica`
**Purpose**: Complex multi-actor chain with cast and cast-back
**Tests**:
- Multi-hop actor chain: A → B → C → B2 → A
- Forward and backward message passing
- Multiple actors processing messages in sequence
- Full round-trip message flow

### `test_complex_capture_scenario.silica`
**Purpose**: Complex scenario with multiple captured variables
**Tests**:
- Multiple workers with different captured multipliers
- Shared captured variables (bonus, coordinator_ref)
- Worker-specific captured variables (worker_id, multiplier)
- Coordinator receiving results from multiple workers

### `test_cast_success_failure.silica`
**Purpose**: Verifies cast() return value handling
**Tests**:
- Checking `cast()` return value (bool)
- Conditional logic based on cast success
- State updates only on successful casts
- Multiple casts with success checking

### `test_nested_captures.silica`
**Purpose**: Deeply nested variable captures in actor lambdas
**Tests**:
- Multiple levels of scope (global, function, inner)
- Capturing variables from different scope levels
- Nested function creating actors with captures
- Complex capture chains

### `test_helper_functions.silica`
**Purpose**: Helper functions for actor creation and cast operations
**Tests**:
- Helper functions that create actors with `cast()` operations
- Helper functions that perform `cast()` calls
- Helper functions that coordinate multiple actors
- Reusable actor creation patterns
- Helper functions returning actor references

## Key Features Tested

### 1. Cast Function
- Syntax: `cast(actor_ref, message: ActorMessage) : proc[concurrency] bool`
- Returns `bool` indicating success/failure of message enqueueing
- Requires message type to implement `ActorMessage` trait

### 2. Cast-Back Pattern
- Messages include `reply_to: actor_ref` field
- Actors can cast responses back to originators
- Compile-time type checking for field access
- Optional `reply_to` field (compile-time verified)

### 3. Actor Lambda Captures
- Variables from outer scopes can be captured
- Actor references can be captured
- Multiple levels of nesting supported
- Captured variables used in behavior functions

### 4. Real Actor Behavior
All tests use real actors with:
- Actual message passing via `cast()`
- Real actor state management
- Actual mailbox operations
- Thread-safe message delivery

## Running the Tests

```bash
# From the silica-bootstrap-compiler directory
cd ../../experiments/08.4_concurrency

# Compile and run a test
make test                    # Compile all tests to LLVM IR
make executables-all         # Build full executables (requires LLVM tools)
make clean                   # Clean generated files
```

## Test Structure

Each test file follows this pattern:

1. **Type Definitions**: Define message and state types
2. **Trait Implementations**: `impl ActorMessage for ...` and `impl ActorState for ...`
3. **Actor Spawning**: Create actors with behavior functions
4. **Message Casting**: Use `cast()` to send messages
5. **Cast-Back**: Actors cast responses back using `reply_to` field

## Example: Cast-Back Pattern

```silica
type Request = { data: int, reply_to: actor_ref };
impl ActorMessage for Request { }

type Response = { result: int };
impl ActorMessage for Response { }

// Spawn echo actor
echo_ref: actor_ref <- spawn(EchoState { received: 0 }, ...);

// Spawn processor that casts back
processor_ref: actor_ref <- spawn(ProcessorState { processed: 0 },
    fn(msg: Request, state: ProcessorState) -> ProcessorState {
        result: int <- msg.data * 2;
        cast(msg.reply_to, Response { result: result });  // Cast-back
        ProcessorState { processed: state.processed + 1 }
    }
);

// Cast request with reply_to
cast(processor_ref, Request { data: 42, reply_to: echo_ref });
```

## Notes

- All tests use **real actor behavior** - actual message passing, not simulation
- All message types must implement `ActorMessage` trait
- All state types must implement `ActorState` trait
- `cast()` returns `bool` for success/failure indication
- `reply_to` field is optional but must be present in message type to use cast-back pattern
- Variable captures work across multiple scope levels
- All type checking and trait verification happens at compile time
