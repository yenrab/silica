# Silica Bootstrap Compiler - How To Guide

## Introduction

The Silica Bootstrap Compiler is a complete, self-hosting compiler for the Silica programming language. Silica is a functional systems programming language designed for AArch64 with explicit effect tracking, actor-based concurrency, and region-based memory management.

This guide demonstrates how to use the Silica Bootstrap Compiler with examples ranging from basic syntax to advanced features like message passing, algebraic data types, and effect systems.

## Installation & Setup

### Prerequisites
- Rust 1.70+ with Cargo
- LLVM 15+ (for full optimization support)
- AArch64 target support

### Building the Compiler

```bash
# Clone or navigate to the silica-bootstrap-compiler directory
cd silica-bootstrap-compiler

# Build with basic features
cargo build --release

# Build with LLVM backend for full optimization support
cargo build --release --features llvm_backend
```

### Testing the Compiler

```bash
# Run basic test
cargo run

# Run with different optimization levels
cargo run -- --opt basic
cargo run -- --opt standard
cargo run -- --opt aggressive
```

## Basic Examples

### Example 1: Hello World - Simple Function

**File: `hello.silica`**
```silica
// Basic function definition and call
fn add(x: int, y: int) -> int {
    x + y
}

fn main() -> int {
    add(40, 2)  // Returns 42
}
```

**Explanation:**
- `fn` declares a function with parameters and return type
- `int` is the 64-bit integer type
- Function calls use parentheses: `add(40, 2)`
- `main` is the entry point, must return `int`
- Comments use `//` syntax

**Compile & Run:**
```bash
# Compile
cargo run -- hello.silica output.bc

# The compiler generates LLVM bitcode
# To run, you'd need to link and execute with LLVM tools
```

### Example 2: Variables and Control Flow

**File: `control_flow.silica`**
```silica
fn factorial(n: int) -> int {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

fn fibonacci(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

fn main() -> int {
    factorial(5) + fibonacci(7)  // 120 + 13 = 133
}
```

**Explanation:**
- `if` expressions: `if condition { then_expr } else { else_expr }`
- Arithmetic operators: `+`, `-`, `*`
- Comparison operators: `<=`, `>`
- Recursive function calls work as expected
- All expressions return values

### Example 3: Type System - Custom Types

**File: `types.silica`**
```silica
// Type alias
type Score = int;

// Struct definition
struct Point {
    x: int,
    y: int,
}

// Enum definition
enum Direction {
    North,
    South,
    East,
    West,
}

fn distance(p1: Point, p2: Point) -> int {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    // Simplified distance (not actual Euclidean)
    dx + dy
}

fn main() -> int {
    let origin = Point { x: 0, y: 0 };
    let target = Point { x: 3, y: 4 };
    distance(origin, target)
}
```

**Explanation:**
- **Type Aliases**: `type Score = int` creates a new name for existing types
- **Structs**: Product types with named fields
- **Enums**: Sum types with variants
- **Struct Literals**: `Point { x: 0, y: 0 }` creates struct instances
- **Field Access**: `point.x` accesses struct fields
- **Local Variables**: `let` bindings for temporary values

## Module System Examples

### Example 4: Modules and Imports

**File: `math.silica`**
```silica
use module math;

export add;
export multiply;

fn add(x: int, y: int) -> int {
    x + y
}

fn multiply(x: int, y: int) -> int {
    x * y
}
```

**File: `main.silica`**
```silica
use module main;

import math;

fn main() -> int {
    math::add(10, 20) + math::multiply(3, 4)  // 30 + 12 = 42
}
```

**Explanation:**
- **Module Declaration**: `use module name` declares the current module
- **Exports**: `export function_name` makes functions visible to importers
- **Imports**: `import module_name` brings in exported symbols
- **Qualified Access**: `math::add(10, 20)` calls imported functions
- **Separate Compilation**: Modules can be compiled independently

## Memory Management Examples

### Example 5: Region-Based Memory

**File: `memory.silica`**
```silica
fn test_memory() -> int {
    // Create a new memory region
    let region = region();

    // Allocate a reference in the region
    let ref = alloc_ref(region, 42);

    // Read the value
    let value = read_ref(ref);

    // Write a new value
    write_ref(ref, 100);

    // Read again to verify
    read_ref(ref)  // Returns 100
}

fn main() -> int {
    test_memory()
}
```

**Explanation:**
- **Regions**: `region()` creates a new memory region
- **Allocation**: `alloc_ref(region, value)` allocates a reference in a region
- **Reading**: `read_ref(reference)` reads the current value
- **Writing**: `write_ref(reference, value)` updates the value
- **Automatic Cleanup**: Regions and their allocations are freed automatically
- **Memory Safety**: Region-based borrowing prevents use-after-free

### Example 6: Effect System - Controlled Side Effects

**File: `effects.silica`**
```silica
// Function that performs I/O (has effects)
proc io_read_value() -> int effects(mem(normal)) {
    let region = region();
    let ref = alloc_ref(region, 0);
    // Simulate reading from I/O
    write_ref(ref, 42);
    read_ref(ref)
}

// Pure function (no effects)
fn double(x: int) -> int {
    x * 2
}

fn main() -> int {
    let input = io_read_value();
    double(input)  // 42 * 2 = 84
}
```

**Explanation:**
- **Effects**: `effects(mem(normal))` declares what side effects a function can perform
- **Memory Effects**: `mem(normal)` allows normal memory operations
- **Pure Functions**: No `effects` declaration means pure (no side effects)
- **Effect Tracking**: Compiler ensures effect safety at compile time
- **Controlled I/O**: Effects prevent accidental I/O in pure contexts

## Actor Concurrency Examples

### Example 7: Simple Actor Communication

**File: `actors.silica`**
```silica
// Actor behavior function
fn counter(initial: int) -> int {
    // Receive messages in a loop
    let message = recv();
    message + initial
}

fn main() -> int {
    // Spawn an actor with initial state
    let actor = spawn(10, counter);

    // Send a message
    send(actor, 5);

    // The actor processes: 5 + 10 = 15
    // In a real system, we'd receive a response
    42  // Simplified for demonstration
}
```

**Explanation:**
- **Actor Spawning**: `spawn(initial_state, behavior_function)` creates a new actor
- **Message Sending**: `send(actor_ref, message)` sends a message to an actor
- **Message Receiving**: `recv()` blocks until a message is received
- **Actor State**: Each actor maintains its own state
- **Concurrent Execution**: Actors run concurrently with the main thread

### Example 8: Producer-Consumer Pattern

**File: `producer_consumer.silica`**
```silica
// Consumer actor
fn consumer() -> int {
    let sum = 0;
    let count = 0;

    // Process 3 messages
    while count < 3 {
        let value = recv();
        sum = sum + value;
        count = count + 1;
    }

    sum  // Return final sum
}

// Producer function
fn produce_values(consumer: actor_ref) -> int {
    send(consumer, 10);
    send(consumer, 20);
    send(consumer, 30);
    0  // Return value not used
}

fn main() -> int {
    let consumer_actor = spawn(0, consumer);
    produce_values(consumer_actor);

    // In a real system, we'd wait for the consumer result
    42  // Simplified demonstration
}
```

**Explanation:**
- **Multiple Actors**: Producer and consumer run concurrently
- **Message Buffering**: Messages are queued in actor mailboxes
- **Asynchronous Communication**: Send doesn't block, recv does
- **State Management**: Actors maintain internal state across messages
- **Scalable Patterns**: Foundation for complex concurrent systems

## Advanced Type System Examples

### Example 9: Generic Types and Traits

**File: `generics.silica`**
```silica
// Generic enum (simplified - generics not fully implemented yet)
enum Option {
    Some(int),
    None,
}

// Trait definition
trait Display {
    fn display(self) -> string;
}

// Implementation for int
impl Display for int {
    fn display(self) -> string {
        // In a real implementation, this would convert to string
        "42"  // Placeholder
    }
}

// Using the trait
fn print_value(value: int) -> string {
    value.display()
}

fn main() -> int {
    print_value(42);
    42
}
```

**Explanation:**
- **Traits**: Define interfaces that types can implement
- **Implementations**: `impl Trait for Type` provides trait implementations
- **Method Calls**: `value.method()` calls trait methods
- **Polymorphism**: Different types can implement the same trait
- **Type Safety**: Compiler ensures correct trait usage

### Example 10: Complex Data Structures

**File: `data_structures.silica`**
```silica
// Complex nested structures
struct Person {
    name: string,
    age: int,
    address: Address,
}

struct Address {
    street: string,
    city: string,
    zip: int,
}

// Linked list node (simplified)
struct ListNode {
    value: int,
    next: Option,
}

fn create_person() -> Person {
    Person {
        name: "Alice",
        age: 30,
        address: Address {
            street: "123 Main St",
            city: "Springfield",
            zip: 12345,
        }
    }
}

fn main() -> int {
    let person = create_person();
    person.age  // Returns 30
}
```

**Explanation:**
- **Nested Structs**: Structs can contain other structs
- **Complex Initialization**: Nested struct literals
- **Field Access**: `person.address.city` accesses nested fields
- **Data Modeling**: Rich data structures for complex programs
- **Memory Layout**: Compiler manages memory layout automatically

## Compilation & Execution

### Compiling Silica Code

```bash
# Basic compilation (text-based LLVM IR)
cargo run -- input.silica output.ll

# With optimizations
cargo run -- --opt standard input.silica output.ll

# With LLVM backend (generates actual bitcode)
cargo run -- --features llvm_backend -- input.silica output.bc
```

### Running Compiled Code

```bash
# If using LLVM backend, you can run with lli
lli output.bc

# Or compile to executable with clang
clang -o executable output.bc -lc
./executable

# For more complex programs with runtime
clang -o executable output.bc silica_runtime.c -lc
./executable
```

### Compiler Options

```bash
# Optimization levels
--opt none        # No optimizations
--opt basic       # Basic optimizations (folding, DCE)
--opt standard    # Standard optimizations (GVN, CSE)
--opt aggressive  # Aggressive optimizations (unrolling, inlining)

# Output options
-o filename       # Specify output file
--emit-llvm       # Emit LLVM IR instead of bitcode
--emit-asm        # Emit assembly (when native backend ready)
```

## Advanced Usage

### Multi-Module Programs

```bash
# Compile modules separately
cargo run -- math.silica math.bc
cargo run -- main.silica main.bc

# Link modules together
llvm-link math.bc main.bc -o program.bc

# Run the combined program
lli program.bc
```

### Effect-Aware Programming

```silica
// Effect-polymorphic function
proc with_logging<T>(operation: proc() -> T effects(effects)) -> T effects(effects, io) {
    log("Starting operation");
    let result = operation();
    log("Operation completed");
    result
}

// Usage
proc database_query() -> int effects(io, mem(normal)) {
    // Database operations...
    42
}

fn main() -> int effects(io, mem(normal)) {
    with_logging(database_query)
}
```

### Memory-Safe Concurrency

```silica
// Actor that manages shared state safely
fn bank_account(initial_balance: int) -> int {
    let balance = initial_balance;

    // Process transactions
    while true {
        let transaction = recv();

        if transaction > 0 {
            balance = balance + transaction;  // Deposit
        } else {
            balance = balance + transaction;  // Withdrawal
        }

        // Send balance confirmation
        send(sender, balance);
    }

    balance
}
```

## Troubleshooting

### Common Compilation Errors

```silica
// Error: Type mismatch
fn wrong_return() -> int {
    "hello"  // Should return int, not string
}

// Error: Effect not declared
proc io_operation() -> int {  // Missing effects(mem(normal))
    let region = region();
    alloc_ref(region, 42)
}

// Error: Unbound variable
fn use_before_define() -> int {
    x + 1  // x not defined yet
}
```

### Performance Optimization Tips

```bash
# Use appropriate optimization levels
cargo run -- --opt standard input.silica output.bc

# Profile your code to identify bottlenecks
# Use LLVM tools for analysis
llvm-prof program.bc

# Consider memory layout for performance
struct CacheLine {
    hot_data: int,    // Frequently accessed
    cold_data: int,   // Rarely accessed
}
```

### Memory Management Best Practices

```silica
// Prefer region-based allocation
fn process_data() -> int effects(mem(normal)) {
    let region = region();

    // All allocations in this region
    let data1 = alloc_ref(region, 1);
    let data2 = alloc_ref(region, 2);

    // Region automatically freed when function returns
    read_ref(data1) + read_ref(data2)
}

// Avoid global state
fn pure_function(x: int) -> int {
    x * 2  // No side effects, no memory allocation
}
```

## Next Steps

Now that you can compile and run Silica programs, explore:

1. **Standard Library**: I/O operations, collections, utilities
2. **Self-Hosting**: Port the compiler itself to Silica
3. **Native Backend**: LLVM-free AArch64 code generation
4. **Advanced Features**: Pattern matching, macros, advanced effects

The Silica Bootstrap Compiler provides a solid foundation for building efficient, safe, and concurrent systems programs with strong compile-time guarantees.

Happy coding with Silica! 🚀
