# Hybrid Hardware Feature Detection Design

## Overview

Silica uses a **hybrid compile-time + startup-time detection approach** for AArch64 hardware features. This design provides maximum performance through compile-time optimization while correctly handling runtime-variable features.

## Design Philosophy

**Compile-Time Detection**: Features that affect code generation and optimization
- NEON presence
- SVE presence
- SVE2 presence
- Architecture level (armv8-a, armv8.1-a, armv8.2-a, armv9-a)

**Startup-Time Detection**: Features that vary at runtime or require runtime query
- SVE vector length (must be queried at runtime per SVE spec)
- MTE availability (may be disabled by kernel even if hardware supports)
- PAC availability (may be disabled by kernel even if hardware supports)
- Prefixed pointers (system-dependent configuration)

## Compiler Target Specification

### Compiler Flags

```bash
# Specify architecture level
silica-comp --arch armv8-a program.silica
silica-comp --arch armv8.1-a program.silica
silica-comp --arch armv8.2-a program.silica
silica-comp --arch armv9-a program.silica

# Specify extensions explicitly (comma-separated)
silica-comp --ext +neon program.silica
silica-comp --ext +neon,+sve program.silica
silica-comp --ext +neon,+sve,+sve2,+mte,+pac program.silica

# Combined: architecture + extensions
silica-comp --arch armv9-a --ext +sve,+sve2,+mte,+pac program.silica

# CPU-specific (implies features via lookup table)
silica-comp --cpu cortex-a78 program.silica
silica-comp --cpu neoverse-n2 program.silica
silica-comp --cpu apple-m1 program.silica

# Auto-detect on native AArch64 (optional convenience)
silica-comp --auto-detect program.silica  # Only works on AArch64 host
```

### Extension Names

- `+neon` - NEON SIMD (usually always present on AArch64)
- `+sve` - Scalable Vector Extension
- `+sve2` - SVE2 (newer extension)
- `+mte` - Memory Tagging Extensions
- `+pac` - Pointer Authentication Codes
- `+prefixed` - Prefixed Pointers
- `+crypto` - Cryptographic extensions
- `+fp16` - Half-precision floating point
- `+bf16` - Brain float 16

### Compiler Behavior

1. **Parse target specification**: Extract architecture, extensions, CPU from flags
2. **Validate extensions**: 
   - Check each extension against known extensions
   - Print informational message for unknown/invalid extensions
   - Ignore invalid extensions (don't error, continue compilation)
   - Example: `info: unknown extension '+invalid', ignoring`
3. **Build feature set**: Combine architecture baseline + valid extensions + CPU features
4. **Type checking**: Error if code uses unavailable features
5. **Code generation**: Generate code only for available features
6. **LLVM backend**: Pass target features to LLVM for instruction generation

### Extension Validation

When explicit extensions are specified via `--ext`, the compiler validates each extension:

- **Valid extensions**: Added to feature set, compilation proceeds normally
- **Invalid/unknown extensions**: 
  - Print informational message: `info: unknown extension '+<name>', ignoring`
  - Ignore the extension (don't add to feature set)
  - Continue compilation with remaining valid extensions
  - Do not treat as an error

**Example**:
```bash
silica-comp --ext +neon,+invalid,+sve program.silica
# Output: info: unknown extension '+invalid', ignoring
# Compilation continues with +neon and +sve
```

**Rationale**: 
- Allows forward compatibility (new extensions can be added without breaking old code)
- Provides helpful feedback without stopping compilation
- Graceful degradation when typos or experimental extensions are used

## Runtime Feature Query

### RuntimeFeatures Type

```silica
// Built-in runtime feature structure (immutable after startup)
type RuntimeFeatures = {
    // SVE vector length (required - must be queried at runtime)
    sve_vector_length: int,
    
    // Optional features (may be disabled by kernel even if hardware supports)
    mte_available: bool,
    pac_available: bool,
    prefixed_available: bool,
    
    // Additional runtime info
    cache_line_size: int,
    numa_nodes: int
}
```

### Query Function

```silica
// One-time query function (called at program startup)
fn query_runtime_features() -> RuntimeFeatures {
    // Reads system registers:
    // - ZCR_EL1 for SVE vector length
    // - ID_AA64PFR1_EL1 for MTE availability
    // - ID_AA64ISAR1_EL1 for PAC availability
    // - System calls for kernel availability
    // - Cache line size, NUMA topology, etc.
}
```

### Usage Pattern

```silica
// Program startup (one-time)
let runtime_features <- query_runtime_features();

// Use cached values throughout program (immutable after startup)
case runtime_features.sve_vector_length of {
    128 -> { /* Optimize for 128-bit vectors */ };
    256 -> { /* Optimize for 256-bit vectors */ };
    512 -> { /* Optimize for 512-bit vectors */ };
    _ -> { /* Generic SVE code */ }
}

// Verify kernel support
case runtime_features.mte_available of {
    true -> { /* Use MTE-accelerated operations */ };
    false -> { /* Fall back to software checks */ }
}
```

## Feature Matrix

| Feature | Compile-Time | Startup-Time | Rationale |
|---------|--------------|--------------|-----------|
| **NEON** | ✅ Presence | ❌ | Always present on AArch64, compile-time optimization |
| **SVE** | ✅ Presence | ✅ Vector length | Presence known at compile-time, length is runtime |
| **SVE2** | ✅ Presence | ❌ | Presence known at compile-time |
| **MTE** | ⚠️ Optional | ✅ Availability | Compiler can assume, but verify at startup (kernel may disable) |
| **PAC** | ⚠️ Optional | ✅ Availability | Compiler can assume, but verify at startup (kernel may disable) |
| **Prefixed** | ⚠️ Optional | ✅ Availability | System-dependent, verify at startup |

## Integration with Code Generation

### Compile-Time Decisions

The compiler uses feature flags to make code generation decisions:

```silica
// Compiler knows at compile-time:
// - NEON is available (if +neon specified)
// - SVE is available (if +sve specified)
// - Can generate NEON/SVE code directly
// - Can optimize assuming features exist

fn bulk_map(graph: Graph) -> proc[mem(normal)] Graph {
    // Compiler generates NEON code if +neon specified
    // Compiler generates SVE code if +sve specified
    // No runtime checks needed for presence
}
```

### Startup-Time Decisions

Runtime queries handle variable features:

```silica
// Runtime queries (one-time, cached):
let features <- query_runtime_features();

// Use cached values:
case features.sve_vector_length of {
    128 -> { /* Use 128-bit SVE operations */ };
    256 -> { /* Use 256-bit SVE operations */ };
    512 -> { /* Use 512-bit SVE operations */ };
    _ -> { /* Adapt to vector length */ }
}

// MTE/PAC availability (verify kernel support):
case features.mte_available of {
    true -> { /* Use MTE-accelerated operations */ };
    false -> { /* Fall back to software checks */ }
}
```

## Benefits

### Performance
- ✅ **Compile-time optimization**: Compiler can optimize assuming features exist
- ✅ **Dead code elimination**: Remove unused feature code paths
- ✅ **One-time startup query**: Minimal overhead, cached after first query
- ✅ **No per-operation checks**: Features verified once at startup

### Flexibility
- ✅ **Explicit control**: Compiler flags provide explicit feature specification
- ✅ **Runtime adaptation**: Handles kernel-dependent features correctly
- ✅ **SVE vector length**: Correctly handles runtime-determined vector length

### Correctness
- ✅ **Type system enforcement**: Compiler errors for unavailable features
- ✅ **Runtime verification**: Validates compile-time assumptions match reality
- ✅ **Fallback paths**: Graceful degradation when features unavailable

## Implementation Details

### Compiler Target Specification Parsing

1. Parse `--arch`, `--ext`, `--cpu` flags
2. Build feature availability set:
   - Baseline: armv8-a (always)
   - Extensions: from `--ext` flags
   - CPU-specific: lookup table for known CPUs
3. Store in compiler context for type checking and code generation

### CPU Feature Lookup Table

```rust
// Example CPU feature mappings (in compiler)
CPU_FEATURES: {
    "cortex-a78": ["+neon", "+sve", "+mte", "+pac"],
    "neoverse-n2": ["+neon", "+sve", "+sve2", "+mte", "+pac"],
    "apple-m1": ["+neon", "+sve", "+fp16"],
    // ... more CPUs
}
```

### System Register Reading

```silica
// Low-level register reading (implemented in runtime)
fn read_zcr_el1() -> int {
    // Read ZCR_EL1 register for SVE vector length
    // Returns vector length in bits (128, 256, 512, etc.)
}

fn read_id_aa64pfr1_el1() -> int {
    // Read ID_AA64PFR1_EL1 register for MTE availability
    // Returns MTE feature bits
}

fn read_id_aa64isar1_el1() -> int {
    // Read ID_AA64ISAR1_EL1 register for PAC availability
    // Returns PAC feature bits
}
```

### Global Caching

```silica
// Global cached value (set once at startup, immutable after)
let global_runtime_features: option<RuntimeFeatures> = None;

fn get_runtime_features() -> RuntimeFeatures {
    case global_runtime_features of {
        Some(features) -> features;  // Return cached value
        None -> {
            // Query and cache (one-time)
            let features <- query_runtime_features();
            global_runtime_features <- Some(features);
            features
        }
    }
}
```

## Example Usage

### Compilation

```bash
# Compile for specific target with extensions
silica-comp --arch armv9-a --ext +neon,+sve,+sve2,+mte,+pac program.silica

# Or use CPU-specific
silica-comp --cpu cortex-a78 program.silica
```

### Runtime

```silica
// Program startup (one-time)
let features <- query_runtime_features();

// Use throughout program (cached, no repeated queries)
case features.sve_vector_length of {
    128 -> { /* Optimize for 128-bit vectors */ };
    256 -> { /* Optimize for 256-bit vectors */ };
    _ -> { /* Generic SVE code */ }
}

// Verify kernel support
case features.mte_available of {
    true -> { /* Use MTE */ };
    false -> { /* Fallback */ }
}
```

## Conclusion

The hybrid approach provides:
- **Maximum performance** through compile-time optimization
- **Correctness** through runtime verification of variable features
- **Flexibility** through explicit compiler flags and runtime adaptation
- **Simplicity** through one-time startup queries and global caching

This design aligns perfectly with Silica's AArch64-native philosophy while handling the reality of runtime-variable features like SVE vector length and kernel-dependent features like MTE/PAC.
