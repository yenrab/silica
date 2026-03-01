# Silica Testing Framework Design Specification

## Overview
This document specifies the complete design for Silica's built-in testing framework. The framework provides unit testing, integration testing, and property-based testing capabilities integrated into the Silica language itself.

## Core Design Principles

### 1. Attribute-Based Test Discovery
Tests are identified using attributes, similar to Rust's `#[test]`:

```silica
#[test]
fn test_addition() {
    assert_equals(add(2, 3), 5)
}

#[integration_test]
fn test_cross_module() {
    use math_utils
    assert_true(math_utils::add(1, 2) == 3)
}

#[property_test]
fn commutative_property(a: int, b: int) {
    assert_equals(add(a, b), add(b, a))
}
```

### 2. Built-in Assertion Functions
Core assertions that work with Silica's effect system:

```silica
-- Basic assertions
assert_true(condition: bool) -> unit
assert_false(condition: bool) -> unit
assert_equals<T>(expected: T, actual: T) -> unit
assert_not_equals<T>(unexpected: T, actual: T) -> unit

-- Exception testing
assert_panics(action: () -> unit) -> unit
assert_panics_with<E>(expected_error: E, action: () -> E) -> unit

-- Actor testing
assert_actor_state(actor: actor_ref, expected_state: any) -> unit
```

### 3. Test Runner Architecture

The test runner is a special compilation mode:

```
silica --test main.silica
├── Parse all modules
├── Discover #[test] functions
├── Generate test runner main()
├── Compile and execute
└── Report results
```

### 4. Test Result Types

```silica
type test_result =
    TestPassed(name: string)
  | TestFailed(name: string, message: string, location: source_location)
  | TestPanicked(name: string, panic_info: string)

type test_summary = {
    passed: int,
    failed: int,
    panicked: int,
    total_time: duration
}
```

## Integration with Existing Architecture

### 1. AST Extensions Required
Add test declarations to the AST in `ast.rs`:

```rust
pub enum Declaration {
    Function(FunctionDecl),
    Test(TestDecl),  // NEW
    IntegrationTest(TestDecl),  // NEW
    PropertyTest(PropertyTestDecl),  // NEW
    // ... existing declarations
}

pub struct TestDecl {
    pub name: String,
    pub body: Expression,
    pub location: SourceLocation,
}

pub struct PropertyTestDecl {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub body: Expression,
    pub location: SourceLocation,
}
```

### 2. Parser Extensions Required
Extend `parser.rs` to handle test attributes:

- Add `#[test]`, `#[integration_test]`, `#[property_test]` to lexer token recognition
- Modify `declaration()` to handle test declarations
- Add parsing for test function parameters (property tests)

### 3. Runtime Integration
Tests run in the same runtime as regular Silica programs, but with:
- Test result collection effects
- Special test execution context
- Panic recovery mechanisms

### 4. Module System Integration
- Tests can import any modules
- Integration tests can span multiple modules
- Test discovery works across module boundaries

## Implementation Phases & Priorities

### Phase 1: Core Infrastructure

#### 1. Basic Test Attributes & Discovery
**Files to modify:**
- `src/lexer.rs`: Add `#[test]`, `#[integration_test]`, `#[property_test]` tokens
- `src/ast.rs`: Add `TestDecl`, `PropertyTestDecl` structs and `Declaration` variants
- `src/parser.rs`: Add parsing for test declarations
- `src/lib.rs`: Add test discovery logic to compilation process

**Implementation steps:**
1. Extend lexer to recognize `#[test]`, `#[integration_test]`, `#[property_test]`
2. Add AST nodes for test declarations
3. Modify parser's `declaration()` to handle test attributes
4. Add test discovery pass to compiler pipeline

#### 2. Core Assertion Functions
**Files to modify:**
- `src/codegen.rs`: Add LLVM code generation for assertion functions
- `src/runtime.rs`: Add runtime assertion implementations
- `src/lib.rs`: Add assertion functions to built-in environment

**Implementation steps:**
1. Implement `assert_true`, `assert_false`, `assert_equals` in runtime
2. Add LLVM generation for assertion calls
3. Add assertion functions to compiler's built-in function table
4. Implement panic recovery for failed assertions

#### 3. Test Runner Infrastructure
**Files to modify:**
- `src/main.rs`: Add `--test` command line flag
- `src/lib.rs`: Add test runner compilation mode
- `src/codegen.rs`: Add test runner main function generation

**Implementation steps:**
1. Add `--test` flag to command line parsing
2. Create test runner compilation mode in `Compiler`
3. Generate test runner main function that calls all discovered tests
4. Execute compiled test binary and collect results

### Phase 2: Enhanced Testing

#### 4. Integration Tests
**Files to modify:**
- `src/ast.rs`: Ensure `IntegrationTest` variant exists
- `src/parser.rs`: Add `#[integration_test]` parsing
- `src/lib.rs`: Add integration test discovery

**Implementation steps:**
1. Parse `#[integration_test]` attributes
2. Add integration test discovery
3. Support setup/teardown mechanisms
4. Allow integration tests to span multiple modules

#### 5. Property-Based Testing
**Files to modify:**
- `src/ast.rs`: Add `PropertyTestDecl` with parameters
- `src/parser.rs`: Parse property test parameters
- `src/codegen.rs`: Generate test case generation code

**Implementation steps:**
1. Parse property test parameters
2. Implement automatic test case generation
3. Add configurable test parameters
4. Generate multiple test cases per property test

#### 6. Exception Testing
**Files to modify:**
- `src/runtime.rs`: Add panic testing functions
- `src/codegen.rs`: Add exception assertion generation

**Implementation steps:**
1. Implement `assert_panics` and `assert_panics_with`
2. Add exception type matching
3. Support panic message validation
4. Integrate with Silica's exception handling

### Phase 3: Developer Experience

#### 7. Test Result Reporting
**Files to modify:**
- `src/main.rs`: Add colored output for test results
- `src/lib.rs`: Add test result formatting

**Color scheme (accessible for colorblind developers):**
- Red: Failed tests
- Blue: Passed tests
- Yellow: Warnings/pending tests

**Implementation steps:**
1. Implement colored console output
2. Add detailed failure messages
3. Include test execution timing
4. Add progress indicators during test execution

#### 8. Test Filtering & Selection
**Files to modify:**
- `src/main.rs`: Add test selection command line options
- `src/lib.rs`: Add test filtering logic

**Command line options:**
- `--test test_name`: Run specific test
- `--test module::test_name`: Run test in specific module
- `--test unit`: Run only unit tests
- `--test integration`: Run only integration tests

#### 9. Test Coverage (Future Enhancement)
**Files to modify:**
- `src/codegen.rs`: Add coverage instrumentation
- `src/lib.rs`: Add coverage reporting

**Implementation steps:**
1. Add line coverage tracking
2. Generate coverage reports
3. Support coverage-guided test generation
4. Integrate coverage data collection

## Command Line Interface

### Test Execution
```bash
# Run all tests in current module
silica --test main.silica

# Run specific test
silica --test main.silica --filter test_addition

# Run tests in specific module
silica --test main.silica --filter math_utils

# Run only integration tests
silica --test main.silica --type integration

# Run with verbose output
silica --test main.silica --verbose
```

### Test Output Format
```
Running tests in module main...

test_addition ... PASS (0.001s)
test_subtraction ... PASS (0.002s)
test_multiplication ... FAIL (0.001s)
  Expected: 15, Got: 16
  at main.silica:42:5

test_division_by_zero ... PANIC (0.001s)
  Division by zero
  at main.silica:48:12

Results: 2 passed, 1 failed, 1 panicked (0.005s total)
```

## Error Handling

### Test Failures
- Failed assertions should not crash the entire test runner
- Each test runs in isolation with panic recovery
- Failed tests should provide clear error messages with source locations

### Compilation Errors in Tests
- Test discovery should handle malformed test functions gracefully
- Invalid test attributes should produce clear error messages
- Tests with compilation errors should be skipped with warnings

## Performance Considerations

### Test Execution
- Tests should run in parallel where possible (respecting actor dependencies)
- Property tests should have reasonable default iteration limits
- Integration tests may need sequential execution guarantees

### Compilation Overhead
- Test discovery should be fast
- Test runner generation should not significantly impact compile times
- Debug symbols should be included for better error reporting

## Future Extensions

### Advanced Property Testing
- Custom generators for complex types
- Shrinking of failing test cases
- Stateful property testing

### Benchmarking Integration
- `#[bench]` attributes for performance testing
- Statistical analysis of benchmark results
- Comparison with baseline performance

### IDE Integration
- Test discovery in language server
- Run/debug individual tests from IDE
- Test result visualization

## Implementation Order Recommendation

1. Start with **Basic Test Attributes & Discovery** - foundation for everything
2. Add **Core Assertion Functions** - tests need assertions
3. Implement **Test Runner Infrastructure** - makes tests runnable
4. Add **Test Result Reporting** - provides usable feedback
5. Implement **Integration Tests** - for multi-module testing
6. Add **Exception Testing** - for error handling validation
7. Implement **Property-Based Testing** - for advanced testing
8. Add **Test Filtering** - for development workflow

This design provides a complete, accessible testing framework that integrates seamlessly with Silica's process-oriented architecture and effect system.
