# Silica Traits Experiments

This directory contains experiments demonstrating Silica's trait system, including sub-trait relationships and trait composition.

## Experiment Files

### Basic Trait Experiments
- `test_traits_basic.silica` - Basic trait definition and implementation
- `test_comprehensive_traits.silica` - Multiple traits with different implementations

### Sub-trait Experiments
- `test_subtraits_basic.silica` - Basic sub-trait functionality using `includes` keyword
- `test_subtraits_multiple.silica` - Single trait including multiple sub-traits
- `test_subtraits_independent.silica` - Sub-traits implemented independently
- `test_subtraits_complex.silica` - Nested sub-trait relationships and complex inheritance

### Advanced Design Pattern Experiments
- `test_trait_strategy_pattern.silica` - Strategy pattern with interchangeable algorithms
- `test_trait_visitor_pattern.silica` - Visitor pattern with double dispatch
- `test_trait_builder_pattern.silica` - Builder pattern with fluent interfaces
- `test_trait_state_pattern.silica` - State pattern with behavioral polymorphism
- `test_trait_diamond_inheritance.silica` - Complex diamond inheritance hierarchies
- `test_trait_error_handling.silica` - Error handling patterns with trait composition

## Key Features Demonstrated

### 1. Basic Traits
- Trait definition with method signatures
- Trait implementation for specific types
- Method dispatch and calling

### 2. Sub-traits with `includes`
- Traits can include other traits as sub-traits
- `trait Derived includes Base { ... }` syntax
- Automatic inheritance of sub-trait methods

### 3. Multiple Sub-traits
- Single trait can include multiple sub-traits
- `trait Combined includes Trait1, Trait2, Trait3 { ... }`
- Accumulation of functionality from multiple sources

### 4. Independent Sub-trait Implementation
- Sub-traits can be implemented separately from including traits
- Types can choose which trait levels to implement
- Flexible trait adoption

### 5. Nested Inheritance
- Multi-level trait inheritance chains
- Complex trait relationships
- Method availability through inheritance hierarchy

### 6. Advanced Design Patterns
#### Strategy Pattern
- Encapsulates algorithms in traits
- Runtime algorithm selection
- Clean separation of concerns

#### Visitor Pattern
- Double dispatch mechanism
- Type-safe operations on heterogeneous collections
- Extensible operation definitions

#### Builder Pattern
- Fluent interface construction
- Complex object creation with validation
- Configurable object building

#### State Pattern
- Behavioral state encapsulation
- Clean state transitions
- Context-dependent behavior

#### Diamond Inheritance
- Complex multiple inheritance hierarchies
- Method resolution in diamond patterns
- Trait composition conflicts and resolution

#### Error Handling
- Strategy-based error management
- Recovery mechanisms through traits
- Hierarchical error processing

## Syntax Reference

```silica
// Basic trait definition
trait Printable {
    fn to_string(self) -> int;
}

// Sub-trait inheritance
trait Debug includes Printable {
    fn debug_info(self) -> int;
}

// Multiple sub-traits
trait FullFeatured includes Displayable, Serializable, Comparable {
    fn version(self) -> int;
}

// Implementation (must implement all methods from trait and sub-traits)
impl Debug for Point {
    fn to_string(self) -> int { ... }      // From Printable sub-trait
    fn debug_info(self) -> int { ... }     // From Debug trait
}
```

## Running the Experiments

```bash
# Run individual experiments
silica-boot experiments/06_traits/test_subtraits_basic.silica

# Run all trait experiments
make -C experiments/06_traits all
```

## Current Status

**All Sub-trait Experiments Working:**
- `test_subtraits_basic.silica`: ✅ Compiles and generates LLVM IR (31)
- `test_subtraits_multiple.silica`: ✅ Compiles and generates LLVM IR (25)
- `test_subtraits_independent.silica`: ✅ Compiles and generates LLVM IR (11)
- `test_subtraits_complex.silica`: ✅ Compiles and generates LLVM IR (18)

**Existing Trait Experiments:**
- `test_traits_basic.silica`: ✅ Basic trait functionality
- `test_comprehensive_traits.silica`: ✅ Multiple traits per type

## Expected Results

- `test_subtraits_basic.silica`: 31 (Point{x:3,y:7} → to_string=10, debug_info=21, sum=31)
- `test_subtraits_multiple.silica`: 25 (Rectangle{width:4,height:5} → display=4, serialize=5, compare=15, version=1, sum=25)
- `test_subtraits_independent.silica`: 11 (Simple{data:5} + Complex{data:3} → value=5, value=3, enhanced=3, sum=11)
- `test_subtraits_complex.silica`: 24 (Complex{value:2,scale:5} → base=2, middle=4, top=6, extra=5, combined=7, sum=24)

**Advanced Design Patterns:**
- `test_trait_strategy_pattern.silica`: 39 (sorting=5+30, search=-1+5, sum=39)
- `test_trait_visitor_pattern.silica`: 236 (areas=75+24+75+48+4+10, sum=236)
- `test_trait_builder_pattern.silica`: 366 (builders with configuration and validation)
- `test_trait_state_pattern.silica`: 1109 (connection state transitions and operations)
- `test_trait_diamond_inheritance.silica`: 1370 (complex trait hierarchies and method inheritance)
- `test_trait_error_handling.silica`: 912 (error handling strategies and recovery patterns)