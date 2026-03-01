# 08.2 Core Affinity Test Suite

This directory contains comprehensive tests for **Phase 3.1: Basic Core Affinity** features of the Silica actor system.

## Test Files

### `test_basic_core_affinity.silica`
**Purpose**: Comprehensive test of all basic core affinity features
**Tests**:
- Backward compatibility (spawn without affinity)
- Explicit `any_core` affinity
- Specific core targeting (`1`, `2`, etc.)
- `performance_cores` load balancing
- `efficiency_cores` load balancing
- Multiple actors with same affinity type

### `test_load_balancing.silica`
**Purpose**: Demonstrates round-robin load balancing behavior
**Tests**:
- Distribution across all cores (`any_core`)
- Distribution within performance cores only
- Distribution within efficiency cores only
- Mixed load balancing patterns

### `test_core_groups.silica`
**Purpose**: Tests core group targeting and real-world usage patterns
**Tests**:
- Performance-critical actors on performance cores
- Power-conscious actors on efficiency cores
- Specific core pinning for hardware coordination
- Mixed affinity strategies in same application

### `test_backward_compatibility.silica`
**Purpose**: Ensures old spawn syntax still works
**Tests**:
- Original 2-parameter spawn syntax
- Mixed old/new syntax in same program
- Complex state types with old syntax

### `test_edge_cases.silica`
**Purpose**: Tests boundary conditions and error handling
**Tests**:
- Core 0 targeting
- Invalid core numbers (too high)
- Negative core numbers (fallback behavior)
- Single-core system compatibility
- Load balancer under high load

### `test_performance_demo.silica`
**Purpose**: Real-world application patterns
**Tests**:
- UI/frontend actors (performance cores)
- Background processing (efficiency cores)
- Real-time data processing (performance cores)
- I/O coordinators (specific core pinning)
- Load balancers and worker pools (any core)

## Running the Tests

```bash
# From the silica-bootstrap-compiler directory
cd ../../experiments/08.2_core_affinity

# Compile and run a test
../../silica-bootstrap-compiler/target/release/silica-boot test_basic_core_affinity.silica output.ll
```

## Expected Behavior

### Load Balancing
- **Round-robin distribution**: Actors with same affinity type cycle through available cores
- **Core group isolation**: `performance_cores` only use performance cores, `efficiency_cores` only use efficiency cores
- **Fallback behavior**: Invalid core specifications fall back to `any_core` load balancing

### Core Detection
- **Automatic detection**: System detects available cores and classifies them
- **Heuristic classification**: Lower-numbered cores = performance, higher-numbered = efficiency
- **Graceful degradation**: Works on any number of cores (1-32+)

### Affinity Types
- **`any_core`**: Load balanced across ALL cores
- **`performance_cores`**: Load balanced within performance core subset
- **`efficiency_cores`**: Load balanced within efficiency core subset
- **Specific cores**: Direct pinning to exact core numbers
- **Backward compatibility**: 2-parameter spawn uses `any_core`

## Test Results Verification

### Successful Test Indicators
- **Compilation succeeds**: All syntax is valid
- **Runtime starts**: Actor spawning works
- **No crashes**: Core affinity APIs handle edge cases gracefully
- **Load distribution**: Actors are created and distributed across cores

### Performance Expectations
- **Responsiveness**: Performance-core actors should show better real-time behavior
- **Power efficiency**: Efficiency-core actors should use less power
- **Load balancing**: Even distribution across cores in same affinity group
- **Scalability**: Performance degrades gracefully under high actor counts

## Architecture Notes

### Current Implementation
- **Runtime API**: `silica_actor_spawn(initial_state, behavior, core_affinity)`
- **Core affinity values**:
  - `0` = any core (load balanced)
  - `-1` = performance cores (load balanced)
  - `-2` = efficiency cores (load balanced)
  - `1+` = specific core ID
- **Thread pinning**: Uses macOS pthreads `pthread_setaffinity_np`

### Core Classification Heuristics
- **4-core system**: All cores = performance
- **5-8 core system**: First half = performance, second half = efficiency
- **9+ core system**: First half = performance, second half = efficiency

### Future Extensions (Phase 3.2+)
- **Dynamic topology detection**: CPUID-based core classification
- **NUMA awareness**: Memory locality optimization
- **Custom core sets**: User-defined core groups
- **Runtime migration**: Change affinity dynamically

## Troubleshooting

### Common Issues
- **"Core not available"**: High core numbers may exceed system capabilities
- **"Permission denied"**: Thread affinity may require elevated privileges
- **"Load imbalance"**: Small core counts may show uneven distribution

### Debug Information
- Check system core count: `sysctl hw.ncpu`
- Monitor thread affinity: Use system monitoring tools
- Verify actor creation: Check for successful spawn calls

## Integration with Build System

These tests are designed to work with the silica-bootstrap-compiler build system and can be integrated into CI/CD pipelines for automated testing of core affinity functionality.
