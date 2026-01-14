# Safe Chip Features: Bootstrap Compiler Code Generation Plan

## Overview

This document outlines the **additional implementation work** required beyond AST generation to compile safe chip features (SIMD, NEON, SVE, MTE, PAC, Prefixed Pointers) from Silica source code to runnable AArch64 binaries. This work enables the bootstrap compiler to generate executable code for chip features, allowing the Phase 2 compiler (written in Silica) to use these features.

## Dependencies

**⚠️ CRITICAL DEPENDENCY**: This plan **depends on** the `bootstrap_compiler_ast_plan.md` being completed first.

Safe chip features require:
- Built-in types (`Vec128Int32`, `VecInt32`, `Pred`, etc.) to be parseable and representable in AST
- Built-in functions (`load_128_int32()`, `alloc_tagged_int()`, etc.) to be parseable in AST
- Compiler target specification to be parseable from command-line flags

**Implementation Order**:
1. ✅ Complete `bootstrap_compiler_ast_plan.md` first
2. ✅ Then proceed with this code generation plan

## Goal

Enable the bootstrap compiler to:
1. Type-check built-in chip feature types and functions
2. Generate AArch64 assembly for SIMD operations (NEON, SVE)
3. Generate AArch64 assembly for safe memory operations (MTE, PAC, Prefixed Pointers)
4. Generate hardware feature detection code (compile-time and startup-time)
5. Generate adaptive code selection (NEON vs SVE, feature availability)
6. Link chip feature code into runnable binaries

## Phase 1: Type Checking Extensions

### 1.1 Built-In Type Recognition

**Required**: Extend type checker to recognize built-in chip feature types

**Tasks**:
- Recognize SIMD vector types (`Vec128Int32`, `VecInt32`, `Pred`, etc.)
- Recognize safe memory types (`TaggedPtrInt`, `PacPtrInt`, `PrefixedPtrInt`, etc.)
- Validate built-in type usage (correct element types, correct operations)
- Check built-in type compatibility (which types can be used together)

**Deliverables**:
- Type checker can recognize all built-in SIMD types
- Type checker can recognize all safe memory types
- Type checker can validate built-in type usage

### 1.2 Built-In Function Type Checking

**Required**: Extend type checker to validate built-in function calls

**Tasks**:
- Type-check built-in SIMD function calls (`load_128_int32()`, `add_128_int32()`, etc.)
- Type-check built-in safe memory function calls (`alloc_tagged_int()`, `sign_ptr_int()`, etc.)
- Validate function parameter types match built-in signatures
- Validate function return types match built-in signatures
- Check feature availability (error if function used but feature unavailable)

**Deliverables**:
- Type checker can validate SIMD function calls
- Type checker can validate safe memory function calls
- Type checker can check feature availability

### 1.3 Trait Implementation Type Checking

**Required**: Extend type checker to validate marker trait implementations

**Tasks**:
- Type-check `SIMDProcessable` trait implementations
- Type-check `PartiallySIMDProcessable` trait implementations
- Type-check `Vec128Element` trait implementations
- Type-check `VecElement` trait implementations
- Validate trait bounds in function signatures

**Deliverables**:
- Type checker can validate marker trait implementations
- Type checker can check trait bounds
- Type checker can resolve trait relationships

### 1.4 Variant Type Checking

**Required**: Extend type checker to validate variant types (OptionPred)

**Tasks**:
- Type-check variant type definitions (`OptionPred = Some(Pred) | None`)
- Type-check variant constructor usage (`Some(pred)`, `None`)
- Type-check variant pattern matching
- Validate variant type in function parameters

**Deliverables**:
- Type checker can validate variant types
- Type checker can check variant constructors
- Type checker can validate pattern matching

## Phase 2: Code Generation - SIMD Vector Types

### 2.1 Vector Type Representation

**Required**: Generate AArch64 representation for SIMD vector types

**Tasks**:
- Map `Vec128Int32` to AArch64 128-bit vector registers (Q registers)
- Map `VecInt32` to AArch64 scalable vector registers (Z registers)
- Map `Pred` to AArch64 predicate registers (P registers)
- Generate register allocation for vector types
- Generate vector type size information

**Deliverables**:
- Code generator can represent NEON vector types
- Code generator can represent SVE vector types
- Code generator can represent predicate types
- Code generator can allocate vector registers

### 2.2 Vector Load/Store Code Generation

**Required**: Generate AArch64 code for vector load/store operations

**Tasks**:
- Generate NEON load instructions (`LD1` for `load_128_int32()`)
- Generate NEON store instructions (`ST1` for `store_128_int32()`)
- Generate SVE load instructions (`LD1W` for `load_vector_int32()`)
- Generate SVE store instructions (`ST1W` for `store_vector_int32()`)
- Generate predicate-based load/store (SVE with `OptionPred`)
- Generate alignment checking (16-byte for NEON)

**Deliverables**:
- Code generator can emit NEON load/store code
- Code generator can emit SVE load/store code
- Code generator can generate predicate-based operations
- Code generator can check alignment

### 2.3 Vector Arithmetic Code Generation

**Required**: Generate AArch64 code for vector arithmetic operations

**Tasks**:
- Generate NEON add instructions (`ADD` for `add_128_int32()`)
- Generate NEON multiply instructions (`MUL` for `mul_128_int32()`)
- Generate NEON subtract instructions (`SUB` for `sub_128_int32()`)
- Generate SVE add instructions (`ADD` for `add_vectors_int32()`)
- Generate SVE multiply instructions (`MUL` for `mul_vectors_int32()`)
- Generate type-specific arithmetic (int8, int16, int32, int64, float32, etc.)

**Deliverables**:
- Code generator can emit NEON arithmetic code
- Code generator can emit SVE arithmetic code
- Code generator can generate type-specific operations

### 2.4 Vector Comparison Code Generation

**Required**: Generate AArch64 code for vector comparison operations

**Tasks**:
- Generate NEON comparison instructions (`CMEQ`, `CMGT`, etc.)
- Generate SVE comparison instructions (`CMPEQ`, `CMPGT`, etc.)
- Generate predicate generation from comparisons
- Generate boolean vector generation

**Deliverables**:
- Code generator can emit comparison code
- Code generator can generate predicates
- Code generator can generate boolean vectors

## Phase 3: Code Generation - SVE Predicate Operations

### 3.1 Predicate Creation Code Generation

**Required**: Generate AArch64 code for predicate creation

**Tasks**:
- Generate `create_pred_true()` code (SVE `PTRUE` instruction)
- Generate `create_pred_from_mask()` code (convert boolean vector to predicate)
- Generate predicate initialization code
- Generate predicate size handling (variable SVE vector length)

**Deliverables**:
- Code generator can emit predicate creation code
- Code generator can generate predicate initialization
- Code generator can handle variable vector lengths

### 3.2 Predicate Test Code Generation

**Required**: Generate AArch64 code for predicate testing

**Tasks**:
- Generate `test_any_true()` code (SVE `PTEST` instruction)
- Generate `test_all_true()` code (SVE `PTEST` instruction)
- Generate predicate-to-boolean conversion
- Generate conditional code based on predicate tests

**Deliverables**:
- Code generator can emit predicate test code
- Code generator can generate conditional branches
- Code generator can convert predicates to booleans

### 3.3 Predicate-Based Operations Code Generation

**Required**: Generate AArch64 code for predicate-based vector operations

**Tasks**:
- Generate predicate-based load/store (SVE with predicate mask)
- Generate predicate-based arithmetic (masked operations)
- Generate predicate-based reductions
- Generate predicate-based compression

**Deliverables**:
- Code generator can emit predicate-based load/store
- Code generator can generate masked operations
- Code generator can generate predicate-based reductions

## Phase 4: Code Generation - SVE Reduction Operations

### 4.1 Reduction Code Generation

**Required**: Generate AArch64 code for SVE reduction operations

**Tasks**:
- Generate `reduce_add_vector_int32()` code (SVE `ADDV` instruction)
- Generate `reduce_max_vector_int32()` code (SVE `MAXV` instruction)
- Generate `reduce_min_vector_int32()` code (SVE `MINV` instruction)
- Generate reduction with predicates (masked reductions)
- Generate type-specific reductions (int8, int16, int32, int64, float32, etc.)

**Deliverables**:
- Code generator can emit SVE reduction code
- Code generator can generate masked reductions
- Code generator can generate type-specific reductions

### 4.2 Compression Code Generation

**Required**: Generate AArch64 code for SVE compression operations

**Tasks**:
- Generate `compress_vector_int32()` code (SVE `COMPACT` instruction)
- Generate `count_matches()` code (SVE `CNTP` instruction)
- Generate compression with predicates
- Generate result buffer allocation for compression

**Deliverables**:
- Code generator can emit compression code
- Code generator can generate match counting
- Code generator can generate result allocation

## Phase 5: Code Generation - Safe Memory Operations

### 5.1 MTE Code Generation

**Required**: Generate AArch64 code for Memory Tagging Extensions

**Tasks**:
- Generate `alloc_tagged_int()` code (allocation with MTE tags)
- Generate `read_tagged_buf_int()` code (tagged buffer access)
- Generate `set_tag()` code (tag manipulation)
- Generate `check_tag()` code (tag validation)
- Generate MTE tag checking instructions (`LDG`, `STG`)
- Generate tag mismatch handling (trap on violation)

**Deliverables**:
- Code generator can emit MTE allocation code
- Code generator can generate tagged access code
- Code generator can generate tag validation
- Code generator can handle tag violations

### 5.2 PAC Code Generation

**Required**: Generate AArch64 code for Pointer Authentication Codes

**Tasks**:
- Generate `sign_ptr_int()` code (PAC signing instructions: `PACIA`, `PACIB`)
- Generate `auth_ptr_int()` code (PAC authentication instructions: `AUTIA`, `AUTIB`)
- Generate pointer signing for function pointers
- Generate pointer authentication on dereference
- Generate PAC failure handling (trap on authentication failure)

**Deliverables**:
- Code generator can emit PAC signing code
- Code generator can generate PAC authentication code
- Code generator can generate pointer protection
- Code generator can handle authentication failures

### 5.3 Prefixed Pointer Code Generation

**Required**: Generate AArch64 code for Prefixed Pointers

**Tasks**:
- Generate `create_prefixed_int()` code (pointer prefix creation)
- Generate `deref_prefixed_int()` code (prefixed pointer dereference)
- Generate prefix validation code
- Generate prefix checking instructions
- Generate prefix mismatch handling

**Deliverables**:
- Code generator can emit prefixed pointer creation code
- Code generator can generate prefixed dereference code
- Code generator can generate prefix validation
- Code generator can handle prefix mismatches

## Phase 6: Code Generation - Hardware Feature Detection

### 6.1 Compile-Time Feature Detection

**Required**: Generate code based on compile-time feature flags

**Tasks**:
- Generate feature availability checks from compiler flags (`--arch`, `--ext`, `--cpu`)
- Generate conditional compilation based on features
- Generate error messages for unavailable features
- Generate informational messages for ignored extensions

**Deliverables**:
- Code generator can use compile-time feature information
- Code generator can generate conditional code
- Code generator can generate appropriate error messages

### 6.2 Startup-Time Feature Detection

**Required**: Generate code for runtime feature query

**Tasks**:
- Generate `query_runtime_features()` implementation
- Generate CPU feature register reads (ID_AA64PFR0_EL1, ID_AA64PFR1_EL1, etc.)
- Generate SVE vector length query (read ZCR_ELx)
- Generate MTE availability check (read system registers)
- Generate PAC availability check (read system registers)
- Generate feature caching (store in global `RuntimeFeatures`)

**Deliverables**:
- Code generator can emit runtime feature query code
- Code generator can generate system register reads
- Code generator can generate feature caching

### 6.3 Adaptive Code Selection

**Required**: Generate adaptive code that selects optimal implementation

**Tasks**:
- Generate NEON vs SVE selection code (check feature availability)
- Generate SIMD vs scalar fallback code
- Generate MTE vs software bounds checking selection
- Generate PAC vs no-PAC code paths
- Generate feature-based function dispatch

**Deliverables**:
- Code generator can emit adaptive selection code
- Code generator can generate feature-based dispatch
- Code generator can generate fallback code paths

## Phase 7: Code Generation - Variant Types

### 7.1 OptionPred Code Generation

**Required**: Generate AArch64 code for OptionPred variant type

**Tasks**:
- Generate variant type representation (tag + payload)
- Generate `Some(pred)` constructor code
- Generate `None` constructor code
- Generate variant pattern matching code
- Generate variant size and alignment

**Deliverables**:
- Code generator can represent OptionPred variant
- Code generator can generate variant constructors
- Code generator can generate pattern matching

### 7.2 Variant Function Parameter Handling

**Required**: Generate code for variant types in function parameters

**Tasks**:
- Generate code to pass OptionPred to functions
- Generate code to extract predicate from OptionPred
- Generate code to handle None case in functions
- Generate code to handle Some case in functions

**Deliverables**:
- Code generator can handle variant parameters
- Code generator can generate variant extraction
- Code generator can generate variant handling

## Phase 8: Runtime Support

### 8.1 SIMD Runtime Functions

**Required**: Implement runtime support for SIMD operations

**Tasks**:
- Implement SIMD capability detection helpers
- Implement vector length query helpers
- Implement SIMD register management
- Implement SIMD operation helpers (if needed)

**Deliverables**:
- Runtime functions for SIMD detection
- Runtime functions for vector length queries
- Runtime functions for SIMD management

### 8.2 Safe Memory Runtime Functions

**Required**: Implement runtime support for safe memory operations

**Tasks**:
- Implement MTE allocation helpers
- Implement PAC signing/authentication helpers
- Implement prefixed pointer helpers
- Implement safe memory error handling

**Deliverables**:
- Runtime functions for MTE operations
- Runtime functions for PAC operations
- Runtime functions for prefixed pointers
- Runtime functions for error handling

### 8.3 Feature Detection Runtime Functions

**Required**: Implement runtime support for feature detection

**Tasks**:
- Implement `query_runtime_features()` function
- Implement system register reading functions
- Implement feature caching functions
- Implement feature availability checking functions

**Deliverables**:
- Runtime function for feature query
- Runtime functions for register reading
- Runtime functions for feature caching

## Phase 9: Standard Library Integration

### 9.1 Built-In Type Standard Library

**Required**: Create standard library definitions for built-in types

**Tasks**:
- Define standard implementations for all SIMD vector types
- Define standard implementations for all safe memory types
- Define standard implementations for variant types
- Define standard type conversions

**Deliverables**:
- Standard library definitions for built-in types
- Standard type implementations
- Standard conversion functions

### 9.2 Built-In Function Standard Library

**Required**: Create standard library definitions for built-in functions

**Tasks**:
- Define standard implementations for all SIMD functions
- Define standard implementations for all safe memory functions
- Define standard implementations for feature detection functions
- Define standard helper functions

**Deliverables**:
- Standard library definitions for built-in functions
- Standard function implementations
- Standard helper functions

## Phase 10: Testing and Validation

### 10.1 SIMD Operation Tests

**Required**: Create tests for SIMD code generation

**Tasks**:
- Test NEON load/store code generation
- Test NEON arithmetic code generation
- Test SVE load/store code generation
- Test SVE arithmetic code generation
- Test SVE predicate operations
- Test SVE reduction operations

**Deliverables**:
- Test suite for NEON operations
- Test suite for SVE operations
- Test cases for all SIMD operations

### 10.2 Safe Memory Operation Tests

**Required**: Create tests for safe memory code generation

**Tasks**:
- Test MTE allocation and access code
- Test PAC signing and authentication code
- Test prefixed pointer code
- Test safe memory error handling

**Deliverables**:
- Test suite for MTE operations
- Test suite for PAC operations
- Test suite for prefixed pointers

### 10.3 Feature Detection Tests

**Required**: Create tests for feature detection code generation

**Tasks**:
- Test compile-time feature detection
- Test startup-time feature query
- Test adaptive code selection
- Test fallback code paths

**Deliverables**:
- Test suite for feature detection
- Test cases for adaptive selection
- Test cases for fallback paths

### 10.4 Integration Tests

**Required**: Create integration tests for complete chip feature usage

**Tasks**:
- Test complete SIMD operations end-to-end
- Test complete safe memory operations end-to-end
- Test feature detection in real scenarios
- Test performance of generated code

**Deliverables**:
- Integration test suite
- Performance benchmarks
- End-to-end validation tests

## Summary of Required Work

### Type Checking (Phase 1)
- [ ] Built-in type recognition
- [ ] Built-in function type checking
- [ ] Trait implementation type checking
- [ ] Variant type checking

### Code Generation - SIMD (Phases 2-4)
- [ ] Vector type representation
- [ ] Vector load/store code generation
- [ ] Vector arithmetic code generation
- [ ] Vector comparison code generation
- [ ] Predicate operations code generation
- [ ] Reduction operations code generation

### Code Generation - Safe Memory (Phase 5)
- [ ] MTE code generation
- [ ] PAC code generation
- [ ] Prefixed pointer code generation

### Code Generation - Feature Detection (Phase 6)
- [ ] Compile-time feature detection
- [ ] Startup-time feature detection
- [ ] Adaptive code selection

### Code Generation - Variants (Phase 7)
- [ ] OptionPred code generation
- [ ] Variant function parameter handling

### Runtime Support (Phase 8)
- [ ] SIMD runtime functions
- [ ] Safe memory runtime functions
- [ ] Feature detection runtime functions

### Standard Library (Phase 9)
- [ ] Built-in type standard library
- [ ] Built-in function standard library

### Testing (Phase 10)
- [ ] SIMD operation tests
- [ ] Safe memory operation tests
- [ ] Feature detection tests
- [ ] Integration tests

## Estimated Time

- **Phase 1**: 2-3 days (type checking extensions)
- **Phase 2**: 3-4 days (vector types and load/store)
- **Phase 3**: 2-3 days (predicate operations)
- **Phase 4**: 2-3 days (reduction operations)
- **Phase 5**: 4-5 days (safe memory operations - complex)
- **Phase 6**: 3-4 days (feature detection)
- **Phase 7**: 1-2 days (variant types)
- **Phase 8**: 2-3 days (runtime support)
- **Phase 9**: 2-3 days (standard library)
- **Phase 10**: 3-5 days (testing)

**Total**: 22-33 days

## Success Criteria

✅ **Type Checking**:
- All built-in types can be type-checked
- All built-in functions can be validated
- All trait implementations can be checked
- All variant types can be validated

✅ **Code Generation**:
- SIMD operations generate correct AArch64 assembly
- Safe memory operations generate correct AArch64 assembly
- Feature detection generates correct detection code
- Adaptive code selection works correctly

✅ **Runtime**:
- Runtime functions work correctly
- Feature detection executes properly
- Safe memory operations execute correctly

✅ **Testing**:
- All chip feature operations have test coverage
- Generated code is validated
- Performance meets expectations

This provides the foundation for the bootstrap compiler to generate runnable binaries for safe chip features, enabling the Phase 2 compiler (written in Silica) to use these features in its implementation.
