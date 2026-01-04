# CPU Topology Detection Experiments

This directory contains tests and experiments for the Silica CPU topology detection system, which automatically detects and classifies CPU cores for optimal actor placement.

## Overview

The topology detection system provides:

- **Real Hardware Detection**: Uses platform-specific APIs to detect actual CPU core types
- **Intelligent Classification**: Automatically categorizes cores as performance or efficiency
- **Load Balancing**: Distributes actors across appropriate cores
- **Cross-Platform Support**: Works on macOS, Linux/AArch64, Android/AArch64, and others

## Platform-Specific Detection

### macOS/AArch64
- Uses `sysctl` API for core enumeration
- Detects Apple Silicon big.LITTLE architecture
- Falls back to heuristics when detailed info unavailable

### Linux/AArch64
- Reads `/sys/devices/system/cpu/cpu*/cpu_capacity` for performance ratings
- Uses capacity values to classify core types (≥80% max = performance)
- Supports big.LITTLE and heterogeneous architectures

### Android/AArch64
- Uses Linux sysfs with Android-specific optimizations
- Ready for thermal management integration
- Handles Android's unique CPU topology patterns

### Fallback (Other Platforms)
- Uses CPU count-based heuristics
- Maintains compatibility across all platforms

## Test Files

### `test_topology_info.silica`
Basic test demonstrating topology-aware actor spawning. Shows how the system automatically places actors on appropriate core types using proper Silica syntax (no loops, no mutation).

### `test_core_classification.silica`
Tests core classification logic and demonstrates the difference between performance, efficiency, and any-core placement. Uses direct arithmetic expressions instead of loops.

### `test_actor_topology_placement.silica`
Advanced test showing intelligent actor placement based on workload characteristics:
- CPU-intensive tasks → performance cores (complex arithmetic)
- Background tasks → efficiency cores (simple operations)
- I/O tasks → specific cores (explicit core pinning)
- General tasks → load balanced (any_core)

### `test_topology_comparison.silica`
Demonstrates the difference between real hardware detection and fallback heuristics, showing consistent behavior across platforms. Uses case statements for conditional logic.

### `test_advanced_topology.silica`
Comprehensive test with multiple actors of different types, demonstrating full topology-aware scheduling. Replaces loop-based actor creation with individual spawn statements.

### `test_get_cpu_topology_info.silica`
Tests the `get_cpu_topology_info()` built-in function that returns detailed CPU topology information as a string. Demonstrates how Silica programs can query and use topology detection results.

### `test_helper_functions.silica`
Tests helper function support in actor behaviors. Demonstrates that function literals in `spawn()` calls can call external helper functions defined at module level, enabling code reuse and better organization of actor logic.

## Usage

The Makefile automatically detects available LLVM tools and builds full executables by default:

```bash
# Build full executable binaries for all tests (default - requires LLVM tools)
make all
make                    # Same as make all

# Build LLVM IR files only (works without LLVM tools)
make executables        # LLVM IR for executables
make modules           # LLVM IR for modules

# Build full executable binaries (same as make all)
make executables-all

# Individual test compilation
make test_topology_info.ll      # Compile single test to LLVM IR
make test_topology_info.bc      # Compile to bitcode (requires llvm-as)
make test_topology_info         # Build executable (requires full LLVM toolchain)

# Clean generated files
make clean

# Show available targets and files
make help
```

### Build Process

1. **LLVM IR Generation**: All tests compile successfully to `.ll` files
2. **Bitcode Conversion**: `.ll` → `.bc` using `llvm-as`
3. **Object Compilation**: `.bc` → `.o` using `llc`
4. **Executable Linking**: `.o` + runtime library → executable using `clang`

**Note**: LLVM tools (`llvm-as`, `llc`, `clang`) must be installed for executable building. The Makefile will detect their availability and build accordingly.

## Expected Output

Each test generates LLVM IR that demonstrates topology-aware actor spawning. The runtime topology detection provides information like:

```
CPU Topology: 8 total cores, 4 performance cores (cpu0, cpu1, cpu2, cpu3), 4 efficiency cores (cpu4, cpu5, cpu6, cpu7) [capacities: cpu0:1024, cpu1:1024, cpu2:1024, cpu3:1024, cpu4:512, cpu5:512, cpu6:512, cpu7:512]
```

## Key Features Demonstrated

1. **Automatic Core Classification**: Performance vs efficiency core detection
2. **Workload-Based Placement**: Different actor types placed optimally
3. **Load Balancing**: Even distribution across similar cores
4. **Explicit Pinning**: Manual core assignment when needed
5. **Cross-Platform Consistency**: Same API works across all supported platforms
6. **Graceful Degradation**: Falls back to heuristics when hardware detection unavailable

## Silica Syntax Notes

These tests demonstrate proper Silica language usage:

- **No loops**: Silica uses recursion, not iteration
- **No mutation**: All variables are immutable
- **Case statements**: Use pattern matching instead of if statements
- **Helper functions**: Function literals can call external helper functions

## Built-in Functions

### `get_cpu_topology_info()`

Returns a string containing detailed CPU topology information:

```silica
let topology_info = get_cpu_topology_info();
// Returns: "CPU Topology: 8 total cores, 4 performance cores (cpu0, cpu1, cpu2, cpu3), 4 efficiency cores (cpu4, cpu5, cpu6, cpu7) [capacities: cpu0:1024, cpu1:1024, ...]"
```

The function provides:
- Total number of CPU cores detected
- Number and IDs of performance cores
- Number and IDs of efficiency cores
- CPU capacity values (when available from `/sys` filesystem)
- Platform-specific detection method used

## Integration with Actor System

The topology detection integrates seamlessly with the actor spawning system:

```silica
// Automatic placement based on detected topology
spawn(initial_state, behavior_fn, performance_cores);
spawn(initial_state, behavior_fn, efficiency_cores);
spawn(initial_state, behavior_fn, any_core);

// Explicit core pinning
spawn(initial_state, behavior_fn, 0);  // Core 0
```

This enables optimal performance and power efficiency by placing actors on the most appropriate CPU cores for their workloads.
