# 11_patterns - Functional Programming Patterns in Silica

This directory contains examples of functional programming patterns implemented using Silica's trait system. All implementations use concrete types instead of generics, making them compatible with the Phase 1 bootstrap compiler.

## Files

### Core Pattern Examples

- **`maybe_monad_example.silica`** - Comprehensive implementation of the Maybe monad (Option monad)
  - Safe handling of optional values and potential failures
  - Monad operations: `pure`, `bind`, `map`
  - Practical examples: safe division, safe square root, chained operations
  - Law verification for monad properties

- **`functor_usage_example.silica`** - Practical functor usage with concrete types
  - TrackedValue type that counts operations
  - Functor operations and law verification
  - Composition and chaining examples

### Trait-Based Implementations

- **`test_functor_trait.silica`** - Functor trait with concrete implementations
  - OptionInt and ListInt functor implementations
  - Runtime law verification
  - Structure preservation examples

- **`test_monad_trait.silica`** - Monad trait with multiple implementations
  - OptionInt, ListInt, and StateInt monad implementations
  - Type-changing bind operations
  - Law verification for monad properties

- **`test_monoid_trait.silica`** - Monoid trait with concrete implementations
  - IntMonoid, StringMonoid, BoolMonoid, ListMonoid implementations
  - Associativity and identity law verification
  - Closure property examples

- **`foldable_computation_trait.silica`** - Combines Functor and Monoid for distributed computation
  - Split data into parts for parallel processing
  - Compute on parts using monoid operations to combine results
  - Reassemble final answer from computed parts
  - Examples: numeric aggregation, statistical computation, matrix operations, tree processing

### Future Concepts

- **`gpu_trait.silica`** - GPU computation effects (future Phase 2 feature)
  - Effect system design for GPU operations
  - Memory management and synchronization
  - Kernel generation concepts

## Key Features Demonstrated

1. **Trait-based Polymorphism**: All patterns use Silica's trait system for abstraction
2. **Runtime Law Verification**: Mathematical laws are checked at runtime rather than compile-time
3. **Concrete Type Safety**: No generics - all implementations use specific types
4. **Composition**: Patterns can be combined and chained safely
5. **Error Handling**: Maybe monad provides safe optional value handling
6. **Distributed Computation**: Foldable computation enables splitting, parallel processing, and reassembly using functor and monoid operations

## Running the Examples

Use the Silica bootstrap compiler to compile and run these examples:

```bash
cd AArch64/experiments/11_patterns
../../../silica-bootstrap-compiler/target/debug/silica-boot maybe_monad_example.silica output.ll -I ../../stdlib
```

## Phase 1 vs Phase 2

These implementations work with the current Phase 1 compiler. The commented sections in some files show how Phase 2 features (generics, higher-kinded types, refinement types) would enable compile-time guarantees for these patterns.
