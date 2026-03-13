# The Silica Programming Language Specification

## Related Documents

| Document | Purpose |
|----------|---------|
| [silica-specification-additional.md](silica-specification-additional.md) | Compiler failure rules: anti-patterns that must fail at compile time |

---

## 1. Introduction

### 1.1 Overview
Silica is a functional systems programming language designed for AArch64 architectures. It emphasizes explicit effect tracking, message-passing concurrency through actors, and memory safety through region-based ownership without garbage collection.

### 1.2 Design Principles
- **Explicit Effects**: All side effects are tracked in type signatures
- **Process Monad**: Sequential computations represented as monadic processes
- **Actor-Based Concurrency**: Message passing as primary concurrency mechanism
- **Region-Based Memory**: Safe memory management without garbage collection
- **No Loops**: Recursion only; runtime handles internal looping
- **AArch64-Native**: First-class support for ARM hardware features
- **No Generics**: Polymorphism achieved through traits, not generic type parameters
- **LLM-Friendly and Human-Readable**: Syntax and semantics designed to be easily parseable by Large Language Models while remaining intuitive for human developers. This includes explicit type annotations, unambiguous syntax patterns, consistent naming conventions, and clear structural patterns that reduce ambiguity in both automated analysis and human comprehension.

### 1.3 LLM-Friendly Design Principles

Silica is designed to be both LLM-friendly and human-readable. The following design choices support this goal:

#### 1.3.1 Explicit Type Annotations
- Function parameters and return types are always explicit: `fn add(x: int64, y: int64) -> int64`
- Variable bindings use explicit type annotations: `value: int64 <- 42`
- Pattern matching uses typed patterns: `case x of { n: int64 -> n * 2 }`
- Catch-all patterns in case expressions must use declared types: `_: int64 -> 0` (not just `_ -> 0`)
- **Benefit**: Eliminates ambiguity for LLMs parsing code, while making types clear to human readers

#### 1.3.2 Unambiguous Syntax Patterns
- Distinct operators for different operations:
  - `<-` for binding/assignment (clear left-to-right flow)
  - `=` for type aliases and equality comparison
  - `->` for function types and case branches
- Explicit effect declarations on sequences: `sequence proc[device_io, concurrency]`
- Clear block delimiters: `sequence...produces pure...end` for sequence blocks, `{...}` for case expressions
- **Benefit**: Reduces parsing ambiguity and makes code structure immediately apparent

#### 1.3.3 Consistent Naming Conventions
- Type names use PascalCase: `Point`, `OptionInt`, `ResultIntString`
- Function and variable names use snake_case: `add_numbers`, `my_value`
- Keywords are lowercase: `fn`, `struct`, `trait`, `impl`
- **Benefit**: Predictable patterns that LLMs can learn and apply consistently

#### 1.3.4 Concrete Types Over Generics
- Instead of generic `Option<T>`, use concrete types: `OptionInt`, `OptionString`
- Instead of generic `Result<T, E>`, use concrete types: `ResultIntString`
- Trait-based polymorphism provides flexibility without generic complexity
- **Benefit**: Eliminates type inference ambiguity, making code more explicit and easier to parse

#### 1.3.5 Structured Pattern Matching
- Pattern matching uses explicit type annotations: `n: int64 if n > 0 -> ...`
- Catch-all patterns require type declarations: `_: int64 -> 0`
- Guards are clearly separated: `pattern if condition -> expression`
- Exhaustiveness checking ensures all cases are covered
- **Benefit**: Clear control flow that LLMs can analyze and humans can verify

#### 1.3.6 Explicit Effect Tracking
- All side effects are declared on sequence blocks: `sequence proc[device_io, mem(normal)]`
- Function declarations do not contain effect declarations; effects are declared only on sequences
- Effect requirements propagate through sequence blocks
- **Benefit**: Makes side effects visible to both LLMs and humans, enabling better code analysis

#### 1.3.7 Module and Import Clarity
- Explicit module declarations and imports: `use calculator;`
- Clear export declarations: `export add/2;`
- Module structure matches file structure
- **Benefit**: Makes dependencies explicit and traceable

### 1.4 Target Platform
Silica targets AArch64 (64-bit ARM) architectures with optional support for:
- Scalable Vector Extensions (SVE/SVE2)
- NEON vector instructions
- Memory Tagging Extensions (MTE)
- Pointer Authentication (PAC)

### 1.5 Compiler Interface

#### 1.5.1 Command Line Usage
```
silica-comp [options] <input.silica> [output.bc]
```

#### 1.5.2 Module Search Path Options
- `--search-path <path>`, `-I <path>`: Add directory to module search paths
  - Can be specified multiple times
  - Default: current directory (`.`)

#### 1.5.3 Examples
```bash
# Basic compilation
silica-comp main.silica

# With custom module paths
silica-comp -I ./modules -I ./stdlib main.silica

# Full command
silica-comp -I modules -I stdlib main.silica app.bc
```

### 1.6 Compiler Error Messages

Silica compiler error messages are designed to be both LLM-parseable and human-readable. Error messages follow a structured format that embeds machine-readable metadata within natural language text.

#### 1.6.1 Error Message Format

All compilation errors follow this pattern:

```
❌ Compilation error: [ErrorType] error at [file]:[line]:[column] [[ErrorCode]]

[Human-readable error description]
See specification: spec:[section]

<!-- SILICA-ERROR-METADATA
{
  "@context": "https://aalang.dev/silica-dev/error/",
  "errorCode": "[ErrorCode]",
  "errorType": "[error|warning|info]",
  "severity": "[error|warning|info]",
  "location": {
    "file": "[file]",
    "line": [line],
    "column": [column],
    "offset": [offset]
  },
  "specification": {
    "section": "[section]"
  }
}
-->
```

#### 1.6.2 Error Message Components

**Human-Readable Component**:
- Clear, natural language description of the error
- Specific location information (file, line, column)
- Reference to specification section using `spec:` prefix
- Emoji indicator (❌ for errors)

**LLM-Parseable Component**:
- Structured JSON metadata embedded in HTML comment
- Error code classification (e.g., `E2000`)
- Error type and severity level
- Location information with file, line, column, and offset
- Reference to specification section

#### 1.6.3 Example Error Message

```
❌ Compilation error: TypeError error at test_undeclared_function.silica:4:24 [E2000]

Undefined function: undeclared_function
See specification: spec:§6.1

<!-- SILICA-ERROR-METADATA
{
  "@context": "https://aalang.dev/silica-dev/error/",
  "errorCode": "E2000",
  "errorType": "error",
  "severity": "error",
  "location": {
    "file": "test_undeclared_function.silica",
    "line": 4,
    "column": 24,
    "offset": 94
  },
    "specification": {
    "section": "§6.1"
  }
}
-->
```

#### 1.6.4 Error Categories

- **Lexical Errors**: Invalid tokens, unterminated strings, etc.
- **Syntax Errors**: Invalid grammar, missing tokens, etc.
- **Type Errors**: Type mismatches, missing implementations, etc.
- **Effect Errors**: Missing effect declarations, invalid effect usage
- **Pattern Matching Errors**: Non-exhaustive patterns, type mismatches
- **Module Errors**: Missing imports, circular dependencies, etc.
- **Lifetime Errors (E21xx)**: Reference outlives region, region not in scope. See spec:§12.1.4.
- **Region Isolation Errors (E21xx)**: Region mismatch (ref(L1, ...) used with region(L2, ...) when L1 ≠ L2). See spec:§12.4.1.
- **Bounds Errors (E21xx)**: Buffer index out of bounds. See spec:§12.3.3.

Each error category has a specific error code range and includes relevant metadata for LLM parsing. Memory region errors follow the format in §1.6.1 with `spec:§12.1.4`, `spec:§12.4.1`, or `spec:§12.3.3` as appropriate.

## 2. Lexical Structure

### 2.1 Character Set
Silica source code is UTF-8 encoded. The language uses ASCII characters for keywords, operators, and punctuation. Unicode is allowed in string literals, character literals, and comments.

### 2.2 Tokens

#### 2.2.1 Keywords
The following identifiers are reserved keywords:

```
actor      actor_ref  atom      atomic    boolean      buf       case
cast       char       concurrency core_id  core_set device_io effect
efficiency_cores else end        enum      export    false     float16   float32
float64    fn         for        from      hot_swap   if        impl      import    int8
int16      int32      int64      mailbox  mem       module    network_io normal    not
of         performance_cores proc      produces  pub       pure      recv      ref       region    register_rwr return
self       send       sequence   spawn     string    struct    trait     true      type
underscore unit       use        where
```

#### 2.2.2 Identifiers
Identifiers start with a letter (a-z, A-Z) or underscore (_), followed by any number of letters, digits (0-9), or underscores.

```
identifier ::= letter (letter | digit | "_")*
letter     ::= "a" | "b" | ... | "z" | "A" | "B" | ... | "Z"
digit      ::= "0" | "1" | ... | "9"
```

Examples of valid identifiers:
- `x`, `value`, `my_function`
- `_private`, `actor_ref`
- `TypeName`, `CONSTANT_VALUE`

#### 2.2.3 Literals

##### Integer Literals
```
integer_literal ::= decimal_literal | hex_literal | binary_literal

decimal_literal ::= digit+
hex_literal     ::= "0x" hex_digit+
binary_literal  ::= "0b" ("0" | "1")+

hex_digit      ::= digit | "a" | "b" | "c" | "d" | "e" | "f"
                  | "A" | "B" | "C" | "D" | "E" | "F"
```

Examples:
- `42`, `0x2A`, `0b101010`

##### Floating-Point Literals
```
float_literal ::= decimal_float | scientific_float

decimal_float     ::= digit+ "." digit+
scientific_float  ::= decimal_float ("e" | "E") ["+" | "-"] digit+
```

Examples:
- `3.14`, `0.5`, `1.0e10`, `2.5E-3`

##### Boolean Literals
```
boolean_literal ::= "true" | "false"
```

##### Character Literals
Character literals are enclosed in single quotes:
```
character_literal ::= "'" character "'"
```

Escape sequences:
```
escape_sequence ::= "\\" | "\'" | "\"" | "\n" | "\t" | "\r"
```

##### String Literals
String literals are enclosed in double quotes:
```
string_literal ::= "\"" {character | escape_sequence} "\""
```

##### Unit Literal
The unit value is represented by empty parentheses:
```
unit_literal ::= "(" ")"
```

##### Atom Literals
Atom literals are symbolic constants prefixed with a colon (`:`). They evaluate to themselves and are compared by identity. The atom name consists of all characters from the first character after the colon until a delimiter is encountered. **All characters are acceptable in atom names except the start character `:`**; any character not in the delimiter set below is valid, including Unicode (emojis, accented letters, etc.).

```
atom_literal ::= ":" atom_name
```

**Atom delimiters** (characters that end an atom name):
- Whitespace: space, tab, newline, carriage return
- Grouping: `( ) { } [ ] ,`
- Operators: `+ - * / % < > = ! : ; . |`
- Other: `& @ # ? ~ ^` plus backtick and backslash

Examples:
- `:ok`, `:error`, `:not_found`
- `:ready`, `:pending`, `:done`
- `:red`, `:green`, `:blue`
- `:🔥`, `:café`, `:_private` (Unicode and underscore allowed)

Atoms are compile-time constants stored in an atom table. They require no runtime allocation and are compared by identity rather than by value, making equality checks constant-time. The colon prefix distinguishes atoms from identifiers and keywords in all contexts.

##### List Literals
List literals are enclosed in square brackets and require an explicit type annotation:
```
list_literal ::= "[" [element_list] "]" ":" "List" "[" element_type "]"

element_list ::= expression {"," expression}
```

Examples:
- `[]: List[string]` - empty list of strings
- `["hello", "world"]: List[string]` - list with two string elements
- `[1, 2, 3]: List[int64]` - list with three integer elements
- `[fn(x: int64) -> int64 { x * 2 }]: List[(int64 -> int64)]` - list with function elements

**Type Checking**: All elements in a list literal must have exactly the same type. The explicit type annotation `List[ElementType]` must match the types of all elements. Mixing different types in a list literal is a compile-time error, even if both types implement `Collectable`.

#### 2.2.4 Operators and Punctuation

##### Arithmetic Operators
```
"+"   "-"   "*"   "/"   "%"
```

##### Comparison Operators
```
"=="  "!="  "<"   "<="  ">"   ">="
```

##### Logical Operators
```
"and"  "or"  "not"
```

##### Assignment and Binding
```
"<-"  "="   "|"
```

##### Function and Type Operators
```
"->"  ":"   "::"
```

##### Grouping and Separation
```
"("   ")"   "["   "]"   "{"   "}"
","   ";"   "."   "|"
```

### 2.3 Comments

#### 2.3.1 Line Comments
Line comments start with `//` and continue to the end of the line:
```
line_comment ::= "//" {any_character_except_newline}
```

#### 2.3.2 Block Comments
Block comments are enclosed in `{-` and `-}` and can span multiple lines:
```
block_comment ::= "{-" {any_character} "-}"
```

Comments are ignored by the lexer and have no effect on program semantics.

### 2.4 Whitespace
Whitespace characters (spaces, tabs, newlines) are used to separate tokens but are otherwise ignored. At least one whitespace character or comment must separate adjacent tokens.

### 2.5 Lexical Errors
- Invalid escape sequences in literals
- Unterminated string or character literals
- Invalid numeric literals (e.g., `0xGG`)
- Reserved keywords used as identifiers

## 3. Syntax

### 3.1 Grammar Notation
This specification uses Extended Backus-Naur Form (EBNF):
- `{item}` - zero or more repetitions
- `[item]` - optional item
- `item | item` - alternatives
- `"terminal"` - literal terminals
- `nonterminal` - grammar rules

### 3.2 Program Structure
```
program ::= {declaration}

declaration ::= function_declaration
              | type_declaration
              | type_alias_declaration
              | effect_declaration
              | import_declaration
              | export_declaration
              | struct_declaration
              | enum_declaration
              | trait_declaration
              | impl_declaration
              | module_declaration
```

### 3.3 Expressions
```
expression ::= literal
             | identifier
             | atom_literal
             | "(" expression ")"
             | expression binary_operator expression
             | unary_operator expression
             | function_call
             | function_literal
             | case_expression
             | if_expression
             | sequence_expression
             | struct_literal
             | field_access
             | tuple_literal
             | constructor_call
             | cast_expression
```

#### 3.3.1 Literals
```
literal ::= integer_literal
          | float_literal
          | boolean_literal
          | character_literal
          | string_literal
          | unit_literal
          | atom_literal
```

#### 3.3.2 Function Calls
```
function_call ::= expression "(" [argument_list] ")"
                 | qualified_method_call

qualified_method_call ::= identifier "." identifier "(" [argument_list] ")"

argument_list ::= expression {"," expression}
```

Qualified method calls allow explicit disambiguation of method calls when multiple traits define methods with the same name. The syntax `TraitName.method_name(args)` explicitly calls the method from the specified trait.

**Qualified Method Call Examples:**

```silica
// Multiple traits define log() method
trait Loggable {
    fn log(self: Self, level: int64) -> int64;
}

trait Debuggable {
    fn log(self: Self, level: int64) -> int64;
}

// Explicit disambiguation using qualified call
fn example(obj: Loggable) -> int64 {
    Loggable.log(obj, 1)  // Qualified call: explicitly use Loggable.log()
}
```

#### 3.3.3 Case Expressions
```
case_expression ::= "case" expression "of" "{" {case_branch} "}"
case_branch    ::= pattern ["if" expression] "->" expression ";"
```

Case expressions support optional guard expressions. A guard is evaluated after pattern matching succeeds, and the branch is only taken if the guard evaluates to `true`.

**Important**: Catch-all patterns (`_`) must use declared types. The wildcard pattern must be typed: `_: type -> expression`.

Example:
```silica
case x of {
    n: int64 if n > 0 -> n * 2;
    n: int64 if n < 0 -> -n * 2;
    _: int64 -> 0
}
```

**Example - Pattern matching with guards using helper functions:**
```silica
fn is_positive(x: int64) -> boolean {
    x > 0
}

fn test_guards(x: int64) -> int64 {
    case x of {
        y: int64 if is_positive(y) -> y * 2;
        _: int64 -> 0             // Fallback
    }
}
```

**Example - Pattern matching with multiple guards:**
```silica
fn categorize(x: int64) -> string {
    case x of {
        n: int64 if n > 100 -> "large";
        n: int64 if n > 10 -> "medium";
        n: int64 if n > 0 -> "small";
        _: int64 -> "zero or negative"
    }
}
```

**Exhaustiveness Checking with Guards:**

When guards are used in case expressions, exhaustiveness checking considers both pattern coverage and guard conditions:

1. **Pattern Exhaustiveness**: All possible values of the matched type must be covered by patterns
2. **Guard Exhaustiveness**: For each pattern, guards must cover all possible cases, or a catch-all pattern must be provided

**Exhaustiveness Rules:**

- **Patterns without guards**: Patterns without guards cover all values matching the pattern
- **Patterns with guards**: Patterns with guards only cover values that match both the pattern AND the guard condition
- **Catch-all patterns**: Catch-all patterns (`_: type`) cover all values not matched by previous patterns, regardless of guards

**Formal Guard Exhaustiveness Rules:**

The exhaustiveness checking algorithm for guards follows these formal rules:

**Rule 1: Pattern Coverage Set**

For a pattern `P` with guard `G`, the coverage set is:

```
Coverage(P, G) = { v | v matches P and G(v) = true }
```

If no guard is present, `G(v) = true` for all values.

**Rule 2: Guard Exhaustiveness**

A case expression `case e of { P1 if G1 -> E1; P2 if G2 -> E2; ...; Pn -> En }` is exhaustive if and only if:

```
Union(Coverage(P1, G1), Coverage(P2, G2), ..., Coverage(Pn, Gn)) = Domain(T)
```

where `T` is the type of expression `e` and `Domain(T)` is the set of all possible values of type `T`.

**Rule 3: Catch-All Pattern Coverage**

A catch-all pattern `_: T` covers all values:

```
Coverage(_: T, true) = Domain(T)
```

**Rule 4: Guarded Pattern Coverage**

For a guarded pattern `P if G`, coverage is the intersection:

```
Coverage(P if G) = Coverage(P, true) ∩ { v | G(v) = true }
```

**Rule 5: Exhaustiveness Checking Algorithm**

```pseudocode
function check_exhaustiveness(value_type, patterns):
    // Initialize coverage set
    total_coverage = empty_set()
    
    // Process each pattern
    for each pattern in patterns:
        if is_catch_all_pattern(pattern):
            // Catch-all covers everything
            total_coverage = Domain(value_type)
            break
        else if has_guard(pattern):
            // Guarded pattern: coverage is intersection
            pattern_coverage = compute_pattern_coverage(pattern.pattern, value_type)
            guard_coverage = compute_guard_coverage(pattern.guard, value_type)
            pattern_guard_coverage = intersection(pattern_coverage, guard_coverage)
            total_coverage = union(total_coverage, pattern_guard_coverage)
        else:
            // Unguarded pattern: covers all matching values
            pattern_coverage = compute_pattern_coverage(pattern.pattern, value_type)
            total_coverage = union(total_coverage, pattern_coverage)
        end if
    end for
    
    // Check if coverage is complete
    if total_coverage == Domain(value_type):
        return EXHAUSTIVE
    else:
        uncovered = difference(Domain(value_type), total_coverage)
        return NOT_EXHAUSTIVE(uncovered)
    end if
end function
```

**Rule 6: Guard Coverage Computation**

For a guard expression `G`, the guard coverage is:

```
GuardCoverage(G) = { v | G(v) evaluates to true }
```

**Guard Expression Evaluation:**

Guard expressions can contain any valid Silica expression, including:
- Arithmetic operations: `n > 0`, `x + y < 100`
- Comparison operations: `n == 42`, `str != ""`
- Logical operations: `n > 0 and n < 100`, `x or y`
- Function calls: `is_positive(n)`, `check_valid(value)`

**Practical Guard Exhaustiveness Examples:**

**Example 1: Integer Range Guards**

```silica
fn process_number(n: int64) -> int64 {
    case n of {
        n: int64 if n < 0 -> -1           // Negative numbers
        n: int64 if n >= 0 and n < 100 -> n * 2  // Range [0, 100)
        n: int64 if n >= 100 -> n + 100   // Range [100, infinity)
        // Exhaustive: covers all int64 values
    }
}
```

This case expression is exhaustive because:
- First pattern covers all negative values (`n < 0`)
- Second pattern covers range [0, 100) (`n >= 0 and n < 100`)
- Third pattern covers range [100, infinity) (`n >= 100`)
- Union covers all possible int64 values

**Example 2: Guard Exhaustiveness with Catch-All**

```silica
fn process_with_guard(x: int64) -> int64 {
    case x of {
        x: int64 if x > 0 -> x * 2        // Positive numbers only
        x: int64 if x < 0 -> x * -1       // Negative numbers only
        _: int64 -> 0                     // Catch-all: covers x == 0
        // Exhaustive: guarded patterns + catch-all cover all values
    }
}
```

This case expression is exhaustive because:
- First pattern covers positive values (`x > 0`)
- Second pattern covers negative values (`x < 0`)
- Catch-all pattern covers zero (`x == 0`)
- Union covers all possible int64 values

**Example 3: Non-Exhaustive Guards (Error Case)**

```silica
fn incomplete_guards(x: int64) -> int64 {
    case x of {
        x: int64 if x > 0 -> x * 2        // Positive numbers
        x: int64 if x < 0 -> x * -1      // Negative numbers
        // ERROR: Not exhaustive - missing case for x == 0
        // Compiler error: pattern match is not exhaustive
    }
}
```

**Example 4: Guard Exhaustiveness with Function Calls**

```silica
fn is_even(n: int64) -> boolean {
    (n % 2) == 0
}

fn process_even_odd(n: int64) -> int64 {
    case n of {
        n: int64 if is_even(n) -> n / 2      // Even numbers
        n: int64 if not is_even(n) -> n * 3 + 1  // Odd numbers
        // Exhaustive: is_even(n) and not is_even(n) cover all values
    }
}
```

**Note**: When guards use function calls, the compiler conservatively assumes the guard may not cover all cases unless it can prove exhaustiveness. In this example, `is_even(n)` and `not is_even(n)` are complementary and cover all values.

**Example 5: Complex Guard Conditions**

```silica
fn categorize_age(age: int64) -> string {
    case age of {
        age: int64 if age < 0 -> "invalid"           // Negative ages
        age: int64 if age >= 0 and age < 13 -> "child"     // [0, 13)
        age: int64 if age >= 13 and age < 18 -> "teen"     // [13, 18)
        age: int64 if age >= 18 and age < 65 -> "adult"    // [18, 65)
        age: int64 if age >= 65 -> "senior"          // [65, infinity)
        // Exhaustive: covers all int64 values with non-overlapping ranges
    }
}
```

**Example 6: Non-Exhaustive with Overlapping Guards**

```silica
fn overlapping_guards(x: int64) -> int64 {
    case x of {
        x: int64 if x > 10 -> x * 2        // x > 10
        x: int64 if x > 5 -> x + 1         // x > 5 (overlaps with x > 10)
        _: int64 -> 0                      // Catch-all for x <= 5
        // Exhaustive: overlapping guards are allowed, catch-all covers remainder
    }
}
```

**Note**: Overlapping guards are allowed - the first matching guard is used. The catch-all pattern ensures exhaustiveness.

**Example 7: Boolean Exhaustiveness**

```silica
fn process_boolean(b: boolean) -> int64 {
    case b of {
        b: boolean if b == true -> 1          // true case
        b: boolean if b == false -> 0         // false case
        // Exhaustive: covers all boolean values
    }
}
```

**Alternative (simpler) for booleans:**

```silica
fn process_boolean_simple(b: boolean) -> int64 {
    case b of {
        true -> 1
        false -> 0
        // Exhaustive: pattern matching without guards covers all boolean values
    }
}
```

**Example 8: Guard Exhaustiveness Error Messages**

When guards are not exhaustive, the compiler reports:

```
❌ Compilation error: PatternMatchError at example.silica:15:5 [E3000]

Pattern match is not exhaustive: missing case for x == 0
Guards cover: x < 0, x > 0
Missing: x == 0
See specification: spec:§3.6.3
    }
}
```

This case expression is NOT exhaustive because:
- First pattern covers positive values (`x > 0`)
- Second pattern covers negative values (`x < 0`)
- Missing: zero case (`x == 0`)
- Union does NOT cover all possible int64 values

**Example 4: Complex Guard Conditions**

```silica
fn complex_guards(n: int64) -> string {
    case n of {
        n: int64 if n < 0 -> "negative"
        n: int64 if n == 0 -> "zero"
        n: int64 if n > 0 and n <= 10 -> "small positive"
        n: int64 if n > 10 and n <= 100 -> "medium positive"
        n: int64 if n > 100 -> "large positive"
        // Exhaustive: covers all int64 values with overlapping guards
    }
}
```

This case expression is exhaustive because:
- Guards cover all possible ranges: (-infinity, 0), [0, 0], (0, 10], (10, 100], (100, infinity)
- Union covers all possible int64 values

**Example 5: Guard with Pattern and Type Matching**

```silica
fn guarded_pattern_matching(msg: Message) -> int {
    case msg of {
        AllocateMsg {size} if size > 0 and size < 1000 -> size * 2
        AllocateMsg {size} if size >= 1000 -> size
        AllocateMsg {size} if size <= 0 -> 0
        PrintMsg {text} if text != "" -> length_chars(text)
        PrintMsg {text} if text == "" -> 0
        _: Message -> -1
        // Exhaustive: all Message variants covered with guards
    }
}
```

This case expression is exhaustive because:
- `AllocateMsg` variants covered by three guards (size > 0 and < 1000, size >= 1000, size <= 0)
- `PrintMsg` variants covered by two guards (text != "", text == "")
- Catch-all covers any other Message variants

**Example 6: Guard Edge Cases**

```silica
fn guard_edge_cases(n: int64) -> int64 {
    case n of {
        n: int64 if n == 0 -> 0           // Exact match
        n: int64 if n != 0 -> n * 2       // All other values
        // Exhaustive: n == 0 and n != 0 cover all values
    }
}
```

This case expression is exhaustive because:
- First pattern covers exactly zero (`n == 0`)
- Second pattern covers all non-zero values (`n != 0`)
- Union covers all possible int64 values

**Example 7: Guard Performance Implications**

Guards are evaluated at runtime, and complex guards may impact performance:

```silica
fn performance_aware_guards(x: int64) -> int64 {
    case x of {
        // Simple guard: fast evaluation (single comparison)
        x: int64 if x < 0 -> -1
        
        // Complex guard: slower evaluation (multiple comparisons)
        x: int64 if x >= 0 and x < 100 and is_prime(x) -> x * 2
        
        // Catch-all: fastest (no guard evaluation)
        _: int64 -> 0
    }
}
```

Performance considerations:
- Simple guards (`x < 0`) evaluate in 1-2 cycles
- Complex guards (`x >= 0 and x < 100 and is_prime(x)`) evaluate in 10-100+ cycles depending on function calls
- Catch-all patterns have no guard evaluation overhead

**Example 8: Guard Exhaustiveness with Boolean Patterns**

```silica
fn boolean_guards(b: boolean) -> int {
    case b of {
        b: boolean if b == true -> 1         // Redundant guard (pattern already matches true)
        b: boolean if b == false -> 0        // Redundant guard (pattern already matches false)
        // Exhaustive but redundant: guards are unnecessary for boolean patterns
    }
}
```

For boolean patterns, guards are often redundant:
- `b: boolean if b == true` is equivalent to `true ->`
- `b: boolean if b == false` is equivalent to `false ->`
- Guards are useful when combining with other conditions

**Example 9: Guard Exhaustiveness Checking Algorithm**

The compiler uses the following algorithm to check guard exhaustiveness:

```pseudocode
function check_guard_exhaustiveness(value_type, patterns):
    // Build coverage sets for each pattern
    coverage_sets = []
    
    for each pattern in patterns:
        if is_catch_all(pattern):
            // Catch-all covers everything
            return EXHAUSTIVE
        else if has_guard(pattern):
            // Compute guard coverage
            pattern_coverage = compute_pattern_coverage(pattern.pattern, value_type)
            guard_coverage = compute_guard_coverage(pattern.guard, value_type)
            pattern_guard_coverage = intersection(pattern_coverage, guard_coverage)
            coverage_sets.append(pattern_guard_coverage)
        else:
            // Unguarded pattern covers all matching values
            pattern_coverage = compute_pattern_coverage(pattern.pattern, value_type)
            coverage_sets.append(pattern_coverage)
        end if
    end for
    
    // Check if union covers all values
    total_coverage = union_all(coverage_sets)
    if total_coverage == Domain(value_type):
        return EXHAUSTIVE
    else:
        uncovered = difference(Domain(value_type), total_coverage)
        return NOT_EXHAUSTIVE(uncovered)
    end if
end function
```

**Example 10: Common Guard Patterns**

Common guard patterns that ensure exhaustiveness:

```silica
// Pattern 1: Range partitioning
case n of {
    n: int64 if n < 0 -> ...
    n: int64 if n >= 0 and n < 100 -> ...
    n: int64 if n >= 100 -> ...
}

// Pattern 2: Equality partitioning
case x of {
    x: int64 if x == 0 -> ...
    x: int64 if x != 0 -> ...
}

// Pattern 3: Guarded + catch-all
case x of {
    x: int64 if x > 0 -> ...
    _: int64 -> ...  // Covers x <= 0
}

// Pattern 4: Multiple conditions
case msg of {
    AllocateMsg {size} if size > 0 -> ...
    AllocateMsg {size} if size <= 0 -> ...
    _: Message -> ...
}
```
- Pattern-matched variables: Variables bound in the pattern can be used in guards
- Nested expressions: Complex expressions combining multiple operations

**Conservative Guard Coverage Algorithm:**

The compiler uses a conservative approach to compute guard coverage:

```pseudocode
function compute_guard_coverage(guard_expr, value_type):
    // Initialize coverage set
    coverage = empty_set()
    
    // Analyze guard expression structure
    case guard_expr of:
        // Simple comparisons: can analyze statically
        comparison_op(pattern_var, literal):
            if comparison_op == ">" or ">=":
                coverage = { v | v > literal }
            else if comparison_op == "<" or "<=":
                coverage = { v | v < literal }
            else if comparison_op == "==":
                coverage = { literal }  // Exact match
            else if comparison_op == "!=":
                coverage = Domain(value_type) \ { literal }  // All except literal
            end if
        
        // Function calls: conservative assumption
        function_call(func_name, args):
            // Cannot statically determine coverage for arbitrary functions
            // Assume function may not cover all cases
            coverage = unknown_coverage()  // Conservative: assume partial coverage
        
        // Logical operations: combine coverage sets
        logical_and(guard1, guard2):
            coverage1 = compute_guard_coverage(guard1, value_type)
            coverage2 = compute_guard_coverage(guard2, value_type)
            coverage = intersection(coverage1, coverage2)  // Both must be true
        
        logical_or(guard1, guard2):
            coverage1 = compute_guard_coverage(guard1, value_type)
            coverage2 = compute_guard_coverage(guard2, value_type)
            coverage = union(coverage1, coverage2)  // Either can be true
        
        logical_not(guard1):
            coverage1 = compute_guard_coverage(guard1, value_type)
            coverage = difference(Domain(value_type), coverage1)  // Complement
        
        // Arithmetic operations: conservative assumption
        arithmetic_op(expr1, expr2):
            // Cannot statically determine coverage for complex arithmetic
            coverage = unknown_coverage()  // Conservative: assume partial coverage
        
        // Default: unknown coverage
        default:
            coverage = unknown_coverage()  // Conservative: assume partial coverage
    end case
    
    return coverage
end function
```

**Unknown Coverage Handling:**

When guard coverage cannot be determined statically (e.g., function calls, complex arithmetic), the compiler treats the guard as having `unknown_coverage()`. This means:

- **Exhaustiveness Check**: Guards with unknown coverage do not contribute to exhaustiveness verification
- **Catch-All Requirement**: A catch-all pattern (`_: type`) is required when guards have unknown coverage
- **Conservative Safety**: The compiler assumes guarded patterns may not cover all cases

**Function Calls in Guards:**

Any Silica function can be used in guard expressions. Function calls in guards are handled conservatively:

```silica
fn is_positive(x: int64) -> boolean {
    x > 0
}

fn categorize(x: int64) -> string {
    case x of {
        n: int64 if is_positive(n) -> "positive";  // Function call in guard
        n: int64 if n < 0 -> "negative";
        _: int64 -> "zero"  // Required: catch-all for unknown coverage
    }
}
```

**Guard Coverage Examples:**

**Example 1: Simple Comparison (Known Coverage)**
```silica
case x of {
    n: int64 if n > 0 -> n * 2;      // Coverage: {1, 2, 3, ...}
    n: int64 if n < 0 -> -n * 2;     // Coverage: {..., -3, -2, -1}
    _: int64 -> 0                    // Coverage: {0} (exhaustive)
}
```
Coverage analysis: `{n > 0} ∪ {n < 0} ∪ {0} = Domain(int64)` ✓ Exhaustive

**Example 2: Function Call (Unknown Coverage)**
```silica
fn check_valid(n: int64) -> boolean {
    // Complex logic that cannot be statically analyzed
    n % 2 == 0 and n > 10
}

case x of {
    n: int64 if check_valid(n) -> n;  // Unknown coverage
    _: int64 -> 0                     // Required: catch-all
}
```
Coverage analysis: `unknown_coverage() ∪ Domain(int64) = Domain(int64)` ✓ Exhaustive (catch-all covers all)

**Example 3: Multiple Guards (Combined Coverage)**
```silica
case x of {
    n: int64 if n > 100 -> "large";           // Coverage: {101, 102, ...}
    n: int64 if n > 10 and n <= 100 -> "medium";  // Coverage: {11, 12, ..., 100}
    n: int64 if n > 0 and n <= 10 -> "small";     // Coverage: {1, 2, ..., 10}
    _: int64 -> "zero or negative"            // Coverage: {0, -1, -2, ...}
}
```
Coverage analysis: `{n > 100} ∪ {n > 10 and n <= 100} ∪ {n > 0 and n <= 10} ∪ {rest} = Domain(int64)` ✓ Exhaustive

**Guard Exhaustiveness Verification:**

The exhaustiveness checker verifies guards as follows:

1. **Compute Coverage**: For each guarded pattern, compute `Coverage(P, G)`
2. **Handle Unknown Coverage**: If `Coverage(P, G) = unknown_coverage()`, require catch-all pattern
3. **Union Coverage**: Compute union of all coverage sets
4. **Check Completeness**: Verify union equals `Domain(T)` or catch-all pattern exists

**Note**: Guard coverage computation is conservative - if the compiler cannot determine guard coverage statically (e.g., for function calls or complex expressions), it assumes the guard may not cover all cases and requires a catch-all pattern for exhaustiveness.

**Rule 7: Integer Guard Exhaustiveness**

For integer types with guards, exhaustiveness requires:

1. **Complete Guard Coverage**: Guards must cover all integer values, OR
2. **Catch-All Pattern**: A catch-all pattern must be provided

**Example: Exhaustive Integer Guards**

```silica
case x of {
    n: int64 if n > 0 -> expr1;    // Covers positive integers
    n: int64 if n < 0 -> expr2;    // Covers negative integers
    _: int64 -> expr3               // Covers zero (catch-all)
}
```

**Coverage Analysis:**

```
Coverage(n: int64 if n > 0) = { n | n ∈ int64 and n > 0 } = {1, 2, 3, ...}
Coverage(n: int64 if n < 0) = { n | n ∈ int64 and n < 0 } = {..., -3, -2, -1}
Coverage(_: int64) = {0} ∪ (all other unmatched values)

Total Coverage = {1, 2, 3, ...} ∪ {..., -3, -2, -1} ∪ {0} = Domain(int64)
```

**Result**: EXHAUSTIVE

**Example: Non-Exhaustive Integer Guards**

```silica
case x of {
    n: int64 if n > 0 -> expr1;    // Covers positive integers
    n: int64 if n < 0 -> expr2;    // Covers negative integers
    // ERROR: Zero is not covered
}
```

**Coverage Analysis:**

```
Coverage(n: int64 if n > 0) = {1, 2, 3, ...}
Coverage(n: int64 if n < 0) = {..., -3, -2, -1}

Total Coverage = {1, 2, 3, ...} ∪ {..., -3, -2, -1} ≠ Domain(int64)
Uncovered = {0}
```

**Result**: NOT_EXHAUSTIVE (missing coverage for 0)

**Rule 8: Boolean Guard Exhaustiveness**

For boolean types, guards can be exhaustive:

```silica
case x of {
    true if condition1 -> expr1;
    true if condition2 -> expr2;
    false -> expr3;
}
```

**Coverage Analysis:**

```
Coverage(true if condition1) = {true} ∩ {v | condition1(v)} = {true} if condition1(true)
Coverage(true if condition2) = {true} ∩ {v | condition2(v)} = {true} if condition2(true)
Coverage(false) = {false}

// If condition1(true) or condition2(true) is true, then true is covered
// If condition1(true) and condition2(true) are both false, then true is not covered
```

**Rule 9: Overlapping Guard Coverage**

When guards overlap, the first matching pattern wins:

```silica
case x of {
    n: int64 if n > 100 -> expr1;   // Covers {101, 102, ...}
    n: int64 if n > 10 -> expr2;     // Covers {11, 12, ..., 100, 101, ...}
    // First pattern matches first, so {101, 102, ...} goes to expr1
    // {11, 12, ..., 100} goes to expr2
    _: int64 -> expr3                // Covers {..., 10}
}
```

**Coverage Analysis:**

```
Coverage(n: int64 if n > 100) = {101, 102, ...}
Coverage(n: int64 if n > 10) = {11, 12, ..., 100}  // Excluding {101, 102, ...} (already matched)
Coverage(_: int64) = {..., 10}

Total Coverage = {101, 102, ...} ∪ {11, 12, ..., 100} ∪ {..., 10} = Domain(int64)
```

**Result**: EXHAUSTIVE (with overlapping guards)

**Formal Exhaustiveness Theorem:**

A case expression `case e of { P1 if G1 -> E1; ...; Pn -> En }` is exhaustive if and only if:

```
∀v ∈ Domain(T). ∃i ∈ {1, ..., n}. (v matches Pi) ∧ (Gi(v) = true ∨ Gi is absent)
```

where `T` is the type of `e`, `Pi` are patterns, and `Gi` are guard expressions (or `true` if absent).

**Example - Exhaustive with guards:**
```silica
fn classify(x: int64) -> string {
    case x of {
        n: int64 if n > 0 -> "positive";
        n: int64 if n < 0 -> "negative";
        _: int64 -> "zero"  // Catch-all covers zero and any other unmatched cases
    }
}
```
This is exhaustive because:
- Positive integers are covered by `n: int64 if n > 0`
- Negative integers are covered by `n: int64 if n < 0`
- Zero and all other cases are covered by `_: int64`

**Example - Non-exhaustive (missing catch-all):**
```silica
fn classify(x: int64) -> string {
    case x of {
        n: int64 if n > 0 -> "positive";
        n: int64 if n < 0 -> "negative";
        // ERROR: Zero is not covered - exhaustiveness check fails
    }
}
```

**Example - Exhaustive with overlapping guards:**
```silica
fn categorize(x: int64) -> string {
    case x of {
        n: int64 if n > 100 -> "large";
        n: int64 if n > 10 -> "medium";  // Covers 11-100 (overlaps with n > 100, but first match wins)
        n: int64 if n > 0 -> "small";    // Covers 1-10
        _: int64 -> "zero or negative"   // Covers 0 and negative
    }
}
```
This is exhaustive because:
- Values > 100 are covered by the first branch
- Values 11-100 are covered by the second branch (first match wins)
- Values 1-10 are covered by the third branch
- Values <= 0 are covered by the catch-all

**Guard Evaluation Order and Semantics:**

Guard evaluation follows a strict order that ensures predictable behavior and efficient execution:

**Evaluation Sequence:**

1. **Pattern Matching First**: Pattern matching is always attempted before guard evaluation
   - The compiler first checks if the value matches the pattern structure
   - If pattern matching fails, the guard is never evaluated
   - Pattern variables are bound only if pattern matching succeeds
   
2. **Guard Evaluation After Pattern Match**: Guards are evaluated only after pattern matching succeeds
   - Guards can use variables bound by pattern matching
   - Guard evaluation occurs in the context where pattern variables are available
   - Guard expressions have access to all pattern-bound variables
   
3. **Guard Result Determines Branch Selection**: Branch selection depends on guard evaluation result
   - If guard evaluates to `true`, the branch is taken and expression is evaluated
   - If guard evaluates to `false`, matching continues to the next branch
   - Guard evaluation is deterministic - same value and guard always produce same result
   
4. **Short-Circuit Behavior**: Guard evaluation uses short-circuit evaluation for logical operations
   - Logical `and` operations short-circuit on first `false` value
   - Logical `or` operations short-circuit on first `true` value
   - Logical `not` operations evaluate their operand completely
   
5. **Catch-All Pattern Evaluation**: Catch-all patterns are evaluated only when no previous pattern matches or all guards fail
   - Catch-all patterns are reached when pattern matching fails for all previous patterns
   - Catch-all patterns are also reached when pattern matching succeeds but all guards evaluate to `false`
   - Catch-all patterns never have guards (they match all values by definition)

**Guard Evaluation Guarantees:**

The compiler and runtime provide the following guarantees for guard evaluation:

- **Deterministic Evaluation**: Guards are evaluated deterministically - same inputs produce same outputs
- **No Side Effects Required**: Guards should be pure expressions without side effects (side effects are allowed but not recommended)
- **Exception Handling**: If guard evaluation raises an exception, the exception propagates normally (does not fall through to next pattern)
- **Performance**: Guard evaluation is optimized - expensive guards may be hoisted or optimized by the compiler

**Guard Evaluation Examples:**

**Example 1: Simple Guard Evaluation**
```silica
case x of {
    n: int64 if n > 0 -> n * 2;    // Pattern matches, guard evaluates n > 0
    n: int64 if n < 0 -> -n * 2;   // Pattern matches, guard evaluates n < 0
    _: int64 -> 0                   // Catch-all: reached if n == 0
}
```
Evaluation order for `x = 5`:
1. Pattern `n: int64` matches, binds `n = 5`
2. Guard `n > 0` evaluates to `true`
3. Branch taken, expression `n * 2` evaluates to `10`

**Example 2: Guard with Short-Circuit Evaluation**
```silica
case x of {
    n: int64 if n > 0 and n < 100 -> "small positive";
    n: int64 if n >= 100 -> "large";
    _: int64 -> "other"
}
```
Evaluation order for `x = -5`:
1. Pattern `n: int64` matches, binds `n = -5`
2. Guard `n > 0 and n < 100` evaluates:
   - `n > 0` evaluates to `false`
   - Short-circuit: `n < 100` is not evaluated
   - Guard result: `false`
3. Matching continues to next pattern
4. Pattern `n: int64` matches, binds `n = -5`
5. Guard `n >= 100` evaluates to `false`
6. Matching continues to catch-all
7. Catch-all pattern matches, expression evaluates to `"other"`

**Example 3: Guard with Function Calls**
```silica
fn is_valid(n: int64) -> boolean {
    n > 0 and n < 1000
}

case x of {
    n: int64 if is_valid(n) -> "valid";
    _: int64 -> "invalid"
}
```
Evaluation order for `x = 500`:
1. Pattern `n: int64` matches, binds `n = 500`
2. Guard `is_valid(n)` evaluates:
   - Function `is_valid(500)` is called
   - Function evaluates `500 > 0 and 500 < 1000` → `true`
   - Guard result: `true`
3. Branch taken, expression evaluates to `"valid"`

**Guard Evaluation Performance:**

Guard evaluation performance characteristics:

- **Pattern Match Cost**: Pattern matching cost depends on pattern complexity (O(1) for simple patterns, O(depth) for nested patterns)
- **Guard Evaluation Cost**: Guard evaluation cost depends on guard expression complexity (O(1) for simple comparisons, O(n) for function calls)
- **Short-Circuit Optimization**: Short-circuit evaluation minimizes guard evaluation cost when possible
- **Compiler Optimization**: Compiler may optimize guard evaluation through hoisting, common subexpression elimination, and other optimizations

**Cross-References:**
- See Section 3.6.5 (AArch64-Specific Pattern Matching Optimizations) for guard evaluation optimizations
- See Section 3.6.4 (Pattern Compilation Strategy) for guard compilation strategies
- See Section 7.5 (Logical Expressions) for logical operator semantics used in guards
- See Section 3.6.5 (AArch64-Specific Pattern Matching Optimizations) for register allocation in guarded pattern matching
- See Section 3.6.3 (Exhaustiveness Checking) for formal exhaustiveness rules

**Exhaustiveness Algorithm:**

The compiler performs exhaustiveness checking as follows:

1. **Collect all patterns**: Identify all patterns in the case expression
2. **Identify uncovered values**: For each pattern type, determine which values are not covered
3. **Check guard coverage**: For patterns with guards, verify that guards cover all cases or catch-all exists
4. **Verify completeness**: Ensure all possible values are covered by either:
   - A pattern without guards
   - A pattern with a guard that evaluates to `true` for that value
   - A catch-all pattern

**Formal Exhaustiveness Judgment:**

```
exhaustive(case e of { p1 if g1 -> e1; ...; pn if gn -> en; _ -> e_default }, T)
```

This judgment holds when:
- All values of type `T` are covered by patterns `p1 ... pn` or the catch-all pattern
- For each value `v` of type `T`:
  - Either `v` matches some `pi` and `gi` evaluates to `true` for `v`
  - Or `v` matches no `pi`, or all matching `pi` have `gi` evaluating to `false`, and the catch-all pattern exists

#### 3.3.4 List Pattern Matching

Lists support pattern matching with explicit element type annotations. The pattern matching syntax for lists is:

```silica
case list_expression of {
    []: List[ElementType] -> expression;                    // Empty list pattern
    [head: ElementType, tail: List[ElementType]] -> expression;  // Non-empty pattern
    _: List[ElementType] -> expression;                     // Catch-all pattern
}
```

**Pattern Matching Rules:**

1. The `ElementType` in the pattern must match the list's element type exactly
2. Pattern matching is exhaustive - all cases must be covered
3. The `head` variable has type `ElementType` (the list's element type)
4. The `tail` variable has type `List[ElementType]` with the same element type
5. All patterns must explicitly specify `List[ElementType]` - `List` alone is not valid
6. The non-empty pattern matches a list with at least one element, binding the first element to `head` and the remaining elements to `tail`

**Examples:**

```silica
// Pattern matching on list of strings
case my_strings: List[string] of {
    []: List[string] -> "empty";
    [first: string, rest: List[string]] -> first;
    _: List[string] -> "other";
}

// Pattern matching on list of functions
case my_functions: List[(int64 -> int64)] of {
    []: List[(int64 -> int64)] -> fn(x: int64) -> int64 { 0 };
    [f: (int64 -> int64), rest: List[(int64 -> int64)]] -> f;
    _: List[(int64 -> int64)] -> fn(x: int64) -> int64 { x };
}

// Recursive list processing
fn sum_list(list: List[int64]) -> int64 {
    case list of {
        []: List[int64] -> 0;
        [head: int64, tail: List[int64]] -> head + sum_list(tail);
        _: List[int64] -> 0;
    }
}

// Building lists recursively
fn build_list(count: int64) -> List[int64] {
    case count of {
        0: int64 -> empty[int64]();
        n: int64 -> {
            rest: List[int64] <- build_list(n - 1);
            prepend[int64](n, rest)
        };
        _: int64 -> empty[int64]();
    }
}
```

#### 3.3.4 Sequence Expressions
```
sequence_expression ::= "sequence" ["proc" "[" effect_list "]"] {statement} "produces" "pure" expression "end"
statement          ::= pattern "<-" expression ";"
                     | expression ";"
```

A sequence block runs steps in order and returns a value. The `produces` keyword marks the result boundary; `pure` asserts the returned expression introduces no new effects. Effectful sequences must declare effects: `sequence proc[ε]`.

Example (effect-free):
```silica
fn main() -> int64 {
    sequence
        p: Point <- Point { x: 10, y: 20 };
        result: int64 <- p.x + p.y;
    produces
        pure result
    end
}
```

Example (effectful):
```silica
fn read_and_process(path: string) -> int64 {
    sequence proc[DeviceIO]
        content: string <- read_lines(path);
        trimmed: string <- trim_leading(content);
    produces
        pure len(trimmed)
    end
}
```

### 3.4 Declarations

#### 3.4.1 Function Declarations
```
function_declaration ::= "fn" identifier parameter_list [":" type] "{" statement_list "}"

parameter_list ::= "(" [parameter {"," parameter}] ")"
parameter      ::= identifier ":" type
```

**Restriction: Function declarations are only allowed at the top level of a program.** Nested function declarations inside other function bodies are not permitted in Silica. This restriction ensures:

1. **Clear program structure**: All functions are visible at the module level, making dependencies and call graphs explicit
2. **Simplified compilation**: Top-level functions simplify code generation and optimization
3. **Consistent scoping**: Function names are always in the module scope, avoiding complex nested scoping rules

If you need to define a helper function that is only used within another function, you have two options:
- **Move the function to top-level**: Declare it at the module level and pass any needed context as parameters
- **Use function literals (lambdas)**: Anonymous functions created with `fn(...) { ... }` can be used within expressions and can capture variables from their enclosing scope

**Example - Invalid (nested function):**
```silica
fn outer() -> int {
    fn inner(x: int) -> int {  // ERROR: Nested function declarations are not allowed
        x + 1
    }
    inner(5)
}
```

**Example - Valid (top-level function):**
```silica
fn helper(x: int) -> int {
    x + 1
}

fn outer() -> int {
    helper(5)
}
```

**Example - Valid (function literal):**
```silica
fn outer() -> int {
    inner: (int) -> int <- fn(x: int) -> int { x + 1 };
    inner(5)
}
```

**Example - Function with effects:**
```silica
fn main() -> int64 {
    sequence proc[device_io, concurrency]
        echo_ref: actor_ref <- spawn(
            EchoState { received: 0 },
            fn(msg: Response, state: EchoState) -> EchoState {
                print_string("Received message: ");
                EchoState { received: state.received + 1 }
            }
        );
    produces
        pure 0
    end
}
```

**Function Literals and Effects:**

Function literals do not declare effects. When a function literal uses effectful operations, it must contain a sequence block that declares the required effects (e.g., `sequence proc[device_io]`).

**Syntax:**
```
function_literal ::= "fn" parameter_list [":" type] "{" statement_list "}"
```

**Example - Function literal with device_io effect (effects declared on sequence inside):**
```silica
fn main() -> int {
    sequence proc[concurrency, device_io]
        echo_ref: actor_ref <- spawn(
            EchoState { received: 0 },
            fn(msg: Response, state: EchoState) -> EchoState {
                sequence proc[device_io]
                    print_string("Received message: ");
                    print_int(msg.result);
                    print_ln("");
                produces
                    pure EchoState { received: state.received + msg.result }
                end
            }
        );
    produces
        pure 0
    end
}
```

**Example - Function literals in compound types (tuples):**
```silica
fn create_function_tuple(x: int64) -> (fn(int64) -> int64, fn(int64) -> int64) {
    (
        fn(n: int64) -> int64 { n + x },        // Adder function
        fn(n: int64) -> int64 { n * x }         // Multiplier function
    )
}

fn apply_tuple_first_func(funcs: (fn(int64) -> int64, fn(int64) -> int64), value: int64) -> int64 {
    (f1: fn(int64) -> int64, f2: fn(int64) -> int64) <- funcs;
    part: int64 <- f1(value);
    f2(part)
}
```

**Example - Function literals passed as parameters:**
```silica
fn accept_function(func: fn(int64) -> int64, value: int64) -> int64 {
    func(value)
}

fn make_multiplier(factor: int64) -> fn(int64) -> int64 {
    fn(x: int64) -> int64 { x * factor }
}

fn main() -> int64 {
    // Pass function literal directly as parameter
    result1: int64 <- accept_function(fn(x: int64) -> int64 { x * 2 }, 10);
    
    // Create and pass a multiplier function
    tripler: fn(int64) -> int64 <- make_multiplier(3);
    result2: int64 <- accept_function(tripler, 7);
    
    result1 + result2
}
```

**Note:** If a function literal uses operations requiring effects (such as `print_string`, `print_int`, `cast`, `spawn`, etc.), it must contain a sequence block that declares those effects (e.g., `sequence proc[device_io]`). Effectful operations may not appear outside sequence blocks.

#### 3.4.2 Type Declarations
```
type_declaration ::= "type" identifier "=" type ";"
```

#### 3.4.3 Effect Declarations
```
effect_declaration ::= "effect" identifier "=" effect ";"
```

#### 3.4.4 Import Declarations
```
import_declaration ::= "use" module_list ";"

module_list ::= identifier {"," identifier}
```

#### 3.4.5 Export Declarations
```
export_declaration ::= "export" export_list ";"

export_list ::= export_item {"," export_item}
export_item ::= identifier "/" integer_literal
```

#### 3.4.6 Struct Declarations
```
struct_declaration ::= "struct" identifier "{" struct_fields "}" ";"

struct_fields ::= [struct_field {"," struct_field}]
struct_field  ::= identifier ":" type
```

Example:
```silica
struct Point {
    x: int,
    y: int
}
```

#### 3.4.7 Enum Declarations
```
enum_declaration ::= "enum" identifier "{" enum_variants "}" ";"

enum_variants ::= enum_variant {"," enum_variant}
enum_variant  ::= identifier [variant_payload]

variant_payload ::= "(" [type {"," type}] ")"
                  | "{" struct_fields "}"
```

Examples:
```silica
enum Option {
    Some(int),
    None
}

enum Result {
    Ok(int),
    Error(string)
}

enum Message {
    Text { content: string },
    Number(int)
}
```

#### 3.4.8 Trait Declarations
```
trait_declaration ::= "trait" identifier "{" trait_items "}" ";"

trait_items ::= [trait_item]
trait_item  ::= associated_type
              | trait_method

associated_type ::= "type" identifier [type_bounds] ";"
type_bounds     ::= ":" identifier {"," identifier}

trait_method ::= "fn" identifier parameter_list [":" type] ";"
```

Examples:
```silica
trait Display {
    fn show(self) -> string;
}

trait Debug {
    fn debug(self) -> string;
}

trait Comparable {
    type Output;
    fn compare(self, other: Self) -> Output;
}
```

#### 3.4.9 Implementation Declarations
```
impl_declaration ::= "impl" [trait_name] "for" type "{" impl_items "}" ";"

trait_name ::= identifier

impl_items ::= [impl_item]
impl_item  ::= associated_type_def
             | method_implementation

associated_type_def ::= "type" identifier "=" type ";"
method_implementation ::= function_declaration
```

Examples:
```silica
impl Display for Point {
    fn show(self) -> string {
        "Point"
    }
}

impl Comparable for Point {
    type Output = int;
    fn compare(self, other: Point) -> int {
        0
    }
}
```

#### 3.4.10 Type Alias Declarations
```
type_alias_declaration ::= "type" identifier "=" type ";"
```

Example:
```silica
type IntPair = (int, int);
type StringMap = { keys: list<string>, values: list<int> };
```

#### 3.4.11 Module Declarations
```
module_declaration ::= "module" identifier ";"
```

Note: Modules are typically inferred from filenames, but explicit module declarations are also supported.

### 3.5 Patterns
```
pattern ::= literal_pattern
          | atom_pattern
          | identifier_pattern
          | wildcard_pattern
          | tuple_pattern
          | record_pattern
          | variant_pattern
          | pattern_alternative

literal_pattern  ::= literal
atom_pattern     ::= atom_literal
identifier_pattern ::= identifier ":" type
wildcard_pattern  ::= "_" ":" type
tuple_pattern     ::= "(" typed_pattern {"," typed_pattern} ")"
typed_pattern     ::= identifier ":" type
record_pattern    ::= "{" identifier ":" pattern {"," identifier ":" pattern} "}"
variant_pattern   ::= identifier [pattern]
pattern_alternative ::= pattern "|" pattern {"|" pattern}
```

#### 3.3.5 If Expressions
```
if_expression ::= "if" expression "then" expression "else" expression
```

Example:
```silica
if x > 0 then x else -x
```

#### 3.3.6 Function Literals
```
function_literal ::= "fn" parameter_list [":" type] "{" statement_list "}"
```

Example:
```silica
fn(x: int, y: int) -> int { x + y }
fn(msg: Message) -> atom {
    sequence proc[device_io]
        print(msg)
    produces
        pure :ok
    end
}
```

#### 3.3.7 Struct Literals
```
struct_literal ::= identifier "{" [field_initializer {"," field_initializer}] "}"

field_initializer ::= identifier ":" expression
```

Example:
```silica
Point { x: 10, y: 20 }
```

#### 3.3.8 Field Access
```
field_access ::= expression "." identifier
```

Example:
```silica
point.x
point.y
```

#### 3.3.9 Tuple Literals
```
tuple_literal ::= "(" expression {"," expression} ")"
```

Example:
```silica
(1, 2, 3)
(42, "hello")
```

#### 3.3.10 Constructor Calls
```
constructor_call ::= identifier "::" identifier ["(" expression ")"]
```

Example:
```silica
OptionInt::Some(42)
ResultIntString::Ok(100)
```

**Note**: Constructor calls do not use generic type parameters. Types are inferred from the variant type name (e.g., `OptionInt` for an option containing `int`, `ResultIntString` for a result with `int` success and `string` error).

#### 3.3.11 Cast Expressions
```
cast_expression ::= "cast" "(" expression "," expression ")"
```

Example:
```silica
cast(actor_ref, message)
```

### 3.6 Pattern Matching Semantics

#### 3.6.1 Pattern Matching Judgment

The pattern matching judgment has the form:

```
ρ ⊢ p ⇓ v ⇒ ρ'

Where:
- ρ is the current environment
- p is the pattern to match
- v is the value to match against
- ρ' is the extended environment with pattern bindings
```

#### 3.6.2 Pattern Matching Rules

**Literal Pattern:**
```
─────────────────────────────
ρ ⊢ n ⇓ n ⇒ ρ          (integer literal match)
ρ ⊢ true ⇓ true ⇒ ρ    (boolean literal match)
ρ ⊢ false ⇓ false ⇒ ρ  (boolean literal match)
ρ ⊢ 'c' ⇓ 'c' ⇒ ρ      (character literal match)
ρ ⊢ "s" ⇓ "s" ⇒ ρ      (string literal match)
ρ ⊢ () ⇓ () ⇒ ρ        (unit literal match)
```

**Typed Identifier Pattern:**
```
ρ ⊢ x:τ ⇓ v ⇒ ρ[x → v]    (bind identifier to typed value)
  where typeof(v) ≡ τ
```

**Wildcard Pattern:**
```
ρ ⊢ _:τ ⇓ v ⇒ ρ           (match any value of type τ, no binding)
  where typeof(v) ≡ τ
```

**Note**: Wildcard patterns must include a type annotation. The pattern `_: type` matches any value of the specified type without binding it to a variable.

**Typed Tuple Pattern:**
```
ρ ⊢ x₁:τ₁ ⇓ v₁ ⇒ ρ₁    ρ₁ ⊢ x₂:τ₂ ⇓ v₂ ⇒ ρ₂    ...    ρₙ₋₁ ⊢ xₙ:τₙ ⇓ vₙ ⇒ ρₙ
  where typeof(vᵢ) ≡ τᵢ for each i
─────────────────────────────────────────────────────────────────────
ρ ⊢ (x₁:τ₁, x₂:τ₂, ..., xₙ:τₙ) ⇓ (v₁, v₂, ..., vₙ) ⇒ ρₙ
```

**Record Pattern:**
```
For each field fᵢ: pᵢ in the record pattern,
ρ ⊢ pᵢ ⇓ v.fᵢ ⇒ ρᵢ    (where ρ₀ = ρ, ρᵢ extends ρᵢ₋₁)
─────────────────────────────────────────────────────
ρ ⊢ {f₁: p₁, f₂: p₂, ..., fₙ: pₙ} ⇓ {f₁: v₁, f₂: v₂, ..., fₙ: vₙ} ⇒ ρₙ
```

**Variant Pattern:**
```
ρ ⊢ p ⇓ v ⇒ ρ'    (where Constructor(v) is the input value)
─────────────────────────────────────────────────────────────
ρ ⊢ Constructor(p) ⇓ Constructor(v) ⇒ ρ'
```

#### 3.6.3 Exhaustiveness Checking

Pattern matches must be exhaustive - every possible value must be matched.

**Type Coverage Analysis:**
For a type τ, a set of patterns P covers τ if:
- Every possible value of type τ matches at least one pattern in P
- No pattern in P matches impossible values

**Exhaustiveness Algorithm:**
1. **Literal Types**: Check that all possible literal values are covered
2. **Variant Types**: Check that all constructors are present
3. **Tuple/Record Types**: Check that destructuring covers all components
4. **Wildcard Patterns**: `_: type` covers all remaining cases of the specified type

**Non-Exhaustive Match Detection:**
If a match is not exhaustive, the compiler reports an error with:
- The uncovered cases
- Suggestions for additional patterns to add

#### 3.6.4 Pattern Compilation Strategy

Pattern matching is compiled to efficient code using decision trees, jump tables, and guard evaluation. The compiler analyzes patterns to determine the most efficient compilation strategy.

**Compilation Strategies:**

The compiler selects compilation strategies based on pattern characteristics:

1. **Decision Tree Compilation**: For complex pattern matching with multiple branches
2. **Jump Table Optimization**: For dense integer/enum pattern matching
3. **Guard Compilation**: Pattern match followed by guard evaluation
4. **Exhaustiveness Checking**: Compile-time verification of pattern coverage

**Decision Tree Compilation:**

For pattern matching with multiple branches, the compiler builds a decision tree:

```silica
case x of {
    {a: int64, b: int64} -> expr1;
    {a: int64} -> expr2;
    _: RecordType -> expr3;
}
```

**Compilation Strategy:**

1. **Constructor Splitting**: First test variant constructors
2. **Field Extraction**: Extract tuple/record fields
3. **Value Testing**: Test literal values
4. **Binding Assignment**: Create environment bindings

**Optimization Techniques:**
- **Common Subexpression Elimination**: Share pattern tests across branches
- **Guard Hoisting**: Move expensive tests earlier in the tree
- **Redundancy Elimination**: Remove unreachable pattern branches
- **Backtracking Minimization**: Prefer deterministic patterns over backtracking
- **Early Exit**: Check most common patterns first
- **Type Specialization**: Generate specialized code for common types (int64, boolean, etc.)

**Jump Table Optimization:**

For dense integer or enum pattern matching, the compiler uses jump tables:

```silica
case x of {
    0 -> expr0;
    1 -> expr1;
    2 -> expr2;
    // ... many cases ...
    100 -> expr100;
    _: int64 -> default_expr;
}
```

**Compilation Strategy:**

- **Density Check**: Use jump tables when pattern density > 50% (e.g., 50+ consecutive integer values)
- **Bounds Check**: Generate bounds check before jump table access
- **Fallback**: Use decision tree for sparse patterns

**Guard Compilation:**

Patterns with guards are compiled in two phases: pattern matching followed by guard evaluation:

```silica
case x of {
    n: int64 if n > 0 -> expr1;
    n: int64 if n < 0 -> expr2;
    _: int64 -> expr3;
}
```

**Compilation Strategy:**

1. **Pattern Match First**: Match pattern structure (bind variables)
2. **Guard Evaluation**: Evaluate guard condition after pattern match succeeds
3. **Fallback**: If guard fails, continue to next pattern
4. **Catch-All**: Catch-all pattern covers all unmatched cases

**Exhaustiveness Checking Algorithm:**

The compiler verifies pattern exhaustiveness at compile time:

1. **Build Coverage Set**: Compute values covered by each pattern
2. **Check Completeness**: Verify all possible values are covered
3. **Guard Handling**: Guarded patterns don't contribute to exhaustiveness (guards may not cover all cases)
4. **Catch-All**: Catch-all patterns (`_: type`) cover all unmatched values

**Performance Characteristics:**
- **O(1)** for simple literal matches (direct branch)
- **O(1)** for jump table patterns (single indirect jump)
- **O(depth)** for nested pattern matching (decision tree traversal)
- **O(guards)** for guarded patterns (pattern match + guard evaluation)
- **Optimal Branching**: Decision tree minimizes comparisons

#### 3.6.5 AArch64-Specific Pattern Matching Optimizations

The compiler generates highly optimized pattern matching code specifically tailored for AArch64 architecture, leveraging hardware features that are not available on other architectures.

**Conditional Execution Optimization:**

AArch64 provides conditional execution instructions that eliminate branch misprediction penalties for simple pattern matches. The compiler uses conditional select (`CSEL`), conditional set (`CSET`), and conditional compare (`CCMP`, `CCMN`) instructions to optimize pattern matching:

- **Conditional Select (CSEL)**: For simple two-way pattern matches, the compiler generates `CSEL` instructions that select between values based on condition flags without branching
- **Conditional Set (CSET)**: For boolean pattern matching results, `CSET` instructions set register values based on condition flags
- **Conditional Compare (CCMP, CCMN)**: For pattern matching with multiple conditions, `CCMP` and `CCMN` instructions chain comparisons without intermediate branches

**Jump Table Generation with PC-Relative Addressing:**

For dense integer and enum pattern matching, the compiler generates jump tables using AArch64's PC-relative addressing capabilities:

- **ADR Instruction**: Generates PC-relative addresses for jump table base addresses
- **ADRP + ADD**: For large jump tables, uses `ADRP` to load page-aligned base address, then `ADRP` + `ADD` for final address calculation
- **Indirect Branch Optimization**: Uses `BR` instruction with register-indirect addressing for jump table dispatch
- **Bounds Checking**: Generates efficient bounds checks using `CMP` and conditional branches before jump table access

**Pattern Matching with Guards Using Conditional Instructions:**

Pattern matching with guard expressions leverages AArch64's conditional execution capabilities:

- **Guard Evaluation**: Guards are evaluated using conditional compare instructions (`CCMP`, `CCMN`) that set condition flags without branching
- **Pattern-Guard Fusion**: For patterns with simple guards (e.g., `n: int64 if n > 0`), the compiler fuses pattern matching and guard evaluation into a single conditional instruction sequence
- **Guard Short-Circuiting**: Complex guards use conditional branches (`B.cond`) that leverage AArch64's branch prediction hardware
- **Guard Hoisting**: Expensive guard evaluations are hoisted before pattern matching when possible, using AArch64's out-of-order execution capabilities

**Guard Evaluation Code Generation:**

The compiler generates optimized guard evaluation code using AArch64 conditional instructions. The following pseudocode illustrates the guard evaluation algorithm:

```pseudocode
function generate_guard_evaluation(pattern_var, guard_expr, next_pattern_label, fallthrough_label):
    // Evaluate guard expression and branch based on result
    guard_result_reg = evaluate_guard_expression(pattern_var, guard_expr)
    
    // Branch to next pattern if guard fails, fallthrough if guard succeeds
    CBNZ guard_result_reg, next_pattern_label  // Branch if guard fails (non-zero = false)
    // Fallthrough: guard succeeded, execute pattern branch
    return fallthrough_label
end function

function evaluate_guard_expression(pattern_var, guard_expr):
    case guard_expr.type:
        // Simple comparison: use conditional compare
        comparison_op(pattern_var, literal):
            if comparison_op == ">":
                CMP pattern_var, literal
                CSET result_reg, GT  // Set result_reg = 1 if pattern_var > literal
            else if comparison_op == ">=":
                CMP pattern_var, literal
                CSET result_reg, GE  // Set result_reg = 1 if pattern_var >= literal
            else if comparison_op == "<":
                CMP pattern_var, literal
                CSET result_reg, LT  // Set result_reg = 1 if pattern_var < literal
            else if comparison_op == "<=":
                CMP pattern_var, literal
                CSET result_reg, LE  // Set result_reg = 1 if pattern_var <= literal
            else if comparison_op == "==":
                CMP pattern_var, literal
                CSET result_reg, EQ  // Set result_reg = 1 if pattern_var == literal
            else if comparison_op == "!=":
                CMP pattern_var, literal
                CSET result_reg, NE  // Set result_reg = 1 if pattern_var != literal
            end if
            return result_reg
        
        // Function call: evaluate function and check result
        function_call(func_name, args):
            // Allocate registers for function arguments
            arg_regs = allocate_argument_registers(args)
            // Call function
            MOV X0, pattern_var  // First argument
            BL func_name  // Branch and link to function
            // Function result in X0
            // Check if result is true (non-zero)
            CBNZ X0, guard_succeeds_label
            MOV result_reg, #0  // Guard fails
            B guard_done_label
            guard_succeeds_label:
            MOV result_reg, #1  // Guard succeeds
            guard_done_label:
            return result_reg
        
        // Logical AND: evaluate both guards, combine results
        logical_and(guard1, guard2):
            result1_reg = evaluate_guard_expression(pattern_var, guard1)
            // Short-circuit: if first guard fails, skip second guard
            CBZ result1_reg, guard_fails_label
            result2_reg = evaluate_guard_expression(pattern_var, guard2)
            // Both guards must succeed
            AND result_reg, result1_reg, result2_reg
            B guard_done_label
            guard_fails_label:
            MOV result_reg, #0
            guard_done_label:
            return result_reg
        
        // Logical OR: evaluate guards, combine results
        logical_or(guard1, guard2):
            result1_reg = evaluate_guard_expression(pattern_var, guard1)
            // Short-circuit: if first guard succeeds, skip second guard
            CBNZ result1_reg, guard_succeeds_label
            result2_reg = evaluate_guard_expression(pattern_var, guard2)
            // Either guard can succeed
            ORR result_reg, result1_reg, result2_reg
            B guard_done_label
            guard_succeeds_label:
            MOV result_reg, #1
            guard_done_label:
            return result_reg
        
        // Logical NOT: negate guard result
        logical_not(guard1):
            result1_reg = evaluate_guard_expression(pattern_var, guard1)
            // Negate: if guard1 is true (1), result is false (0), and vice versa
            EOR result_reg, result1_reg, #1  // XOR with 1 to negate
            return result_reg
        
        // Complex arithmetic: evaluate expression, compare result
        arithmetic_op(expr1, expr2):
            // Evaluate arithmetic expression
            result_reg = evaluate_arithmetic_expression(pattern_var, arithmetic_op)
            // Compare result (typically against zero or literal)
            CMP result_reg, #0
            CSET result_reg, NE  // Set result_reg = 1 if result != 0
            return result_reg
    end case
end function
```

**Guard Evaluation in Decision Trees:**

When guards are used in decision tree pattern matching, guards are integrated into the decision tree structure:

```pseudocode
function generate_decision_tree_with_guards(value_reg, patterns_with_guards):
    // Build decision tree with guard evaluation at each node
    for each pattern_with_guard in patterns_with_guards:
        pattern = pattern_with_guard.pattern
        guard = pattern_with_guard.guard
        
        // Generate pattern matching code
        pattern_match_label = generate_pattern_match(value_reg, pattern)
        
        // Generate guard evaluation code after pattern match succeeds
        guard_eval_label = generate_guard_evaluation(pattern_var, guard, next_pattern_label, guard_succeeds_label)
        
        // Guard succeeds: execute pattern branch
        guard_succeeds_label:
        generate_pattern_branch_code(pattern_with_guard.branch_expr)
        B pattern_matching_done_label
        
        // Guard fails: continue to next pattern
        next_pattern_label:
        // Continue decision tree traversal
    end for
    
    // No pattern matched: exhaustiveness error or catch-all
    pattern_matching_done_label:
    return
end function
```

**Guard Hoisting Optimization:**

The compiler hoists expensive guard evaluations when possible to improve performance:

```pseudocode
function hoist_guards(patterns_with_guards):
    // Identify guards that can be hoisted (no dependencies on pattern bindings)
    hoistable_guards = []
    for each pattern_with_guard in patterns_with_guards:
        guard = pattern_with_guard.guard
        if is_hoistable(guard):
            // Guard only depends on input value, not pattern bindings
            hoistable_guards.append(guard)
        end if
    end for
    
    // Evaluate hoisted guards before pattern matching
    for each guard in hoistable_guards:
        guard_result = evaluate_guard_before_pattern_matching(guard)
        // Store result for later use in pattern matching
        store_guard_result(guard, guard_result)
    end for
    
    // Pattern matching can now use pre-evaluated guard results
    return patterns_with_guards
end function
```

**Guard Performance Characteristics:**

Guard evaluation performance varies based on guard complexity:

- **Simple Comparisons** (`n > 0`, `x == 42`): 1-2 cycles (single `CMP` + `CSET`)
- **Function Calls** (`is_positive(n)`): Function call overhead + guard check (~10-50 cycles depending on function complexity)
- **Logical Operations** (`n > 0 and n < 100`): 2-4 cycles (two comparisons + logical operation)
- **Complex Arithmetic** (`n * 2 < 100`): 3-10 cycles (arithmetic + comparison)

**Guard Evaluation Register Usage:**

Guards use temporary registers that are freed after evaluation:

- **Simple Guards**: 1-2 temporary registers (comparison result)
- **Function Call Guards**: 2-8 argument registers (X0-X7) + 1 result register
- **Complex Guards**: 3-5 temporary registers (intermediate results)

Guards do not interfere with pattern matching register allocation since guard registers are allocated separately and freed immediately after guard evaluation completes.

**Decision Tree Optimization for AArch64:**

Decision tree compilation is optimized for AArch64's branch prediction and instruction pipeline:

- **Branch Prediction Hints**: The compiler uses AArch64's branch prediction hints (`B.cond` with hint bits) to improve prediction accuracy for common pattern paths
- **Instruction Scheduling**: Pattern matching code is scheduled to maximize AArch64's dual-issue capabilities (two instructions per cycle on many cores)
- **Register Pressure Management**: Decision tree code generation considers AArch64's 32 general-purpose registers to minimize register spills
- **Cache-Aware Code Layout**: Pattern matching code is laid out to maximize instruction cache utilization, leveraging AArch64's instruction prefetch capabilities

**Variant Pattern Matching Optimization:**

Variant (enum) pattern matching uses AArch64-specific optimizations:

- **Tag Extraction**: Variant tags are extracted using efficient bit manipulation instructions (`UBFX`, `SBFX`) that leverage AArch64's flexible bitfield extraction
- **Tag Comparison**: Multiple tag comparisons use `CMP` with immediate values, optimized for AArch64's immediate encoding capabilities
- **Variant Dispatch**: Variant dispatch uses jump tables with PC-relative addressing for dense variant sets, or decision trees with conditional branches for sparse sets

**Record Pattern Matching Optimization:**

Record pattern matching leverages AArch64's load/store capabilities:

- **Field Extraction**: Record fields are extracted using efficient load instructions (`LDR`, `LDP`) that leverage AArch64's load-pair capabilities
- **Field Comparison**: Field comparisons use `CMP` instructions optimized for AArch64's comparison semantics
- **Nested Record Matching**: Nested record patterns use AArch64's register windows and callee-saved registers to minimize stack operations

**Tuple Pattern Matching Optimization:**

Tuple pattern matching uses AArch64's multiple register load/store capabilities:

- **Tuple Decomposition**: Tuple elements are extracted using `LDP` (Load Pair) instructions that load two 64-bit values in a single instruction
- **Register Allocation**: Tuple elements are allocated to AArch64's 32 general-purpose registers to minimize memory accesses
- **Nested Tuple Matching**: Nested tuple patterns leverage AArch64's register renaming and out-of-order execution capabilities

**Register Allocation Strategies for Pattern Matching:**

The compiler implements sophisticated register allocation strategies specifically optimized for AArch64's 32 general-purpose registers when generating pattern matching code.

**AArch64 Register Architecture:**

AArch64 provides 32 general-purpose registers (X0-X31) that are used for pattern matching:

- **X0-X7**: Parameter/result registers (caller-saved)
- **X8**: Indirect result location register
- **X9-X15**: Temporary registers (caller-saved)
- **X16-X17**: IP0/IP1 (intra-procedure call registers)
- **X18**: Platform register (reserved)
- **X19-X28**: Callee-saved registers
- **X29**: Frame pointer (FP)
- **X30**: Link register (LR)
- **X31/SP**: Stack pointer

**Pattern Matching Register Allocation Algorithm:**

The compiler uses the following algorithm for register allocation in pattern matching:

```pseudocode
function allocate_registers_for_pattern_matching(pattern_expr, available_registers):
    // Phase 1: Allocate registers for pattern value
    pattern_value_reg = allocate_register(available_registers)
    
    // Phase 2: Allocate registers for pattern matching temporaries
    temp_registers = []
    for each subpattern in pattern_expr:
        temp_reg = allocate_register(available_registers)
        temp_registers.append(temp_reg)
    end for
    
    // Phase 3: Allocate registers for guard evaluation (if guards present)
    guard_registers = []
    if has_guards(pattern_expr):
        for each guard in pattern_expr:
            guard_temp_reg = allocate_register(available_registers)
            guard_registers.append(guard_temp_reg)
        end for
    end if
    
    // Phase 4: Spill strategy when register pressure is high
    if register_pressure > REGISTER_PRESSURE_THRESHOLD:
        spilled_registers = spill_to_stack(least_used_registers)
    end if
    
    return RegisterAllocation(pattern_value_reg, temp_registers, guard_registers)
end function
```

**Register Allocation for Different Pattern Types:**

**Literal Pattern Register Allocation:**

For literal patterns, minimal registers are needed:

- **Pattern Value**: 1 register (X0-X7 for parameter passing)
- **Comparison Result**: 1 register (X9-X15 for temporary)
- **Total**: 2 registers

```assembly
// Literal pattern: case x of { 42 -> ... }
LDR   X0, [X1]        // Load pattern value (X0 = x)
MOV   X9, #42         // Load literal (X9 = 42)
CMP   X0, X9          // Compare (uses condition flags)
B.EQ  match_42        // Branch if match
```

**Variant Pattern Register Allocation:**

For variant patterns, tag extraction and dispatch require registers:

- **Pattern Value**: 1 register (X0-X7)
- **Tag Extraction**: 1 register (X9-X15 for tag)
- **Tag Comparison**: Uses condition flags (no additional register)
- **Dispatch Address**: 1 register (X16-X17 for indirect branch)
- **Total**: 3 registers

```assembly
// Variant pattern: case x of { Some(n) -> ...; None -> ... }
LDR   X0, [X1]        // Load pattern value (X0 = x)
UBFX  X9, X0, #0, #8  // Extract tag (X9 = variant tag)
CMP   X9, #0          // Compare tag (Some = 0, None = 1)
B.EQ  match_some      // Branch if Some
B     match_none      // Branch if None
```

**Record Pattern Register Allocation:**

For record patterns, multiple field extractions require registers:

- **Pattern Value**: 1 register (X0-X7)
- **Field Extraction**: N registers for N fields (X9-X15, X19-X28)
- **Field Comparison**: Uses condition flags (no additional registers)
- **Total**: 1 + N registers (where N is number of fields)

```assembly
// Record pattern: case x of { {a, b} -> ... }
LDR   X0, [X1]        // Load pattern value (X0 = x)
LDP   X9, X10, [X0]   // Load pair: X9 = a, X10 = b (load-pair optimization)
// Field comparisons use X9, X10 directly
```

**Tuple Pattern Register Allocation:**

For tuple patterns, tuple decomposition uses load-pair instructions:

- **Pattern Value**: 1 register (X0-X7)
- **Tuple Elements**: N/2 load-pair operations (X9-X15, X19-X28)
- **Total**: 1 + ceil(N/2) registers (load-pair optimization)

```assembly
// Tuple pattern: case x of { (a, b, c) -> ... }
LDR   X0, [X1]        // Load pattern value (X0 = x)
LDP   X9, X10, [X0]   // Load pair: X9 = a, X10 = b
LDR   X11, [X0, #16]  // Load: X11 = c
// Tuple elements in X9, X10, X11
```

**Guarded Pattern Register Allocation:**

For guarded patterns, guard evaluation requires additional registers:

- **Pattern Matching**: Same as pattern type (see above)
- **Guard Evaluation**: M registers for M guard expressions (X9-X15, X19-X28)
- **Guard Result**: 1 register for guard boolean result (X9-X15)
- **Total**: Pattern registers + M + 1 registers

```assembly
// Guarded pattern: case x of { n: int64 if n > 0 -> ... }
LDR   X0, [X1]        // Load pattern value (X0 = x)
MOV   X9, X0          // Copy to X9 for guard evaluation
CMP   X9, #0          // Guard: n > 0
B.LE  next_pattern    // Branch if guard fails
// Pattern match and guard both passed
```

**Register Pressure Management:**

When register pressure is high, the compiler implements spill strategies:

**Spill Strategy 1: Stack Spilling**

When registers are exhausted, values are spilled to the stack:

```pseudocode
function spill_to_stack(register):
    // Save register to stack
    stack_offset = allocate_stack_slot()
    STR register, [SP, #stack_offset]
    
    // Mark register as available
    mark_register_available(register)
    
    return stack_offset
end function

function reload_from_stack(stack_offset, target_register):
    // Reload register from stack
    LDR target_register, [SP, #stack_offset]
    
    // Free stack slot
    free_stack_slot(stack_offset)
end function
```

**Spill Strategy 2: Register Prioritization**

The compiler prioritizes registers based on usage frequency:

- **High Priority**: Pattern value register (used throughout matching)
- **Medium Priority**: Frequently used temporaries (tag extraction, field access)
- **Low Priority**: Rarely used temporaries (guard intermediates, comparison results)

**Spill Strategy 3: Live Range Analysis**

The compiler analyzes register live ranges to minimize spills:

- **Short Live Ranges**: Spill registers with short live ranges first
- **Long Live Ranges**: Keep registers with long live ranges in registers
- **Dead Registers**: Immediately free registers when they become dead

**Register Allocation Performance Characteristics:**

| Pattern Type | Registers Used | Spill Probability | Performance Impact |
|-------------|---------------|------------------|-------------------|
| Literal | 2 | Low | Minimal overhead |
| Variant | 3 | Low | Minimal overhead |
| Record (2 fields) | 3 | Low | Load-pair optimization |
| Record (4+ fields) | 5+ | Medium | May require spills |
| Tuple (2 elements) | 2 | Low | Load-pair optimization |
| Tuple (4+ elements) | 3+ | Medium | May require spills |
| Guarded (simple) | +2 | Low | Guard evaluation overhead |
| Guarded (complex) | +4+ | Medium | May require spills |

**Register Allocation Optimization Techniques:**

**Technique 1: Register Coalescing**

The compiler coalesces registers when possible:

- **Pattern Value Reuse**: Reuse pattern value register for extracted values when possible
- **Temporary Reuse**: Reuse temporary registers across different pattern branches
- **Guard Reuse**: Reuse guard evaluation registers across multiple guards

**Technique 2: Register Renaming**

The compiler uses register renaming to avoid false dependencies:

- **Out-of-Order Execution**: Leverages AArch64's out-of-order execution capabilities
- **Register Renaming**: Hardware register renaming eliminates false dependencies
- **Pipeline Efficiency**: Maximizes instruction-level parallelism

**Technique 3: Callee-Saved Register Usage**

The compiler uses callee-saved registers for long-lived values:

- **X19-X28**: Used for values that span multiple pattern branches
- **Frame Pointer**: X29 used for stack frame management
- **Link Register**: X30 used for function calls within pattern matching

**Register Allocation Examples:**

**Example 1: Simple Pattern (Low Register Pressure)**

```silica
case x of {
    42 -> result1
    100 -> result2
    _: int64 -> result3
}
```

Register allocation:
- X0: Pattern value (x)
- X9: Literal comparison temporary
- Total: 2 registers (low pressure, no spills)

**Example 2: Complex Pattern (High Register Pressure)**

```silica
case x of {
    {a: int64, b: int64, c: int64, d: int64} if a > 0 and b > 0 -> result1
    {a: int64, b: int64, c: int64, d: int64} if c > 0 -> result2
    _: RecordType -> result3
}
```

Register allocation:
- X0: Pattern value (x)
- X9-X12: Field extraction (a, b, c, d)
- X13-X14: Guard evaluation temporaries
- Total: 7 registers (medium pressure, may require spills for guard evaluation)

**Performance Characteristics on AArch64:**

Pattern matching performance on AArch64 exceeds that of traditional architectures:

- **Simple Literal Matches**: O(1) with single conditional instruction (`CSEL` or `CSET`), typically 1-2 cycles
- **Jump Table Patterns**: O(1) with PC-relative addressing, typically 2-3 cycles including bounds check
- **Decision Tree Patterns**: O(depth) with optimized branch prediction, typically 1-2 cycles per level
- **Guarded Patterns**: O(1) pattern match + O(1) guard evaluation with conditional instructions, typically 2-4 cycles total
- **Variant Patterns**: O(1) tag extraction + O(1) dispatch, typically 2-3 cycles
- **Record Patterns**: O(fields) with load-pair optimization, typically 1 cycle per field pair
- **Tuple Patterns**: O(elements) with load-pair optimization, typically 1 cycle per element pair

**Cross-References:**
- See Section 27.1.1 (Region-Aware Code Generation) for register allocation strategies used in pattern matching
- See Section 27.2.1 (Asymmetric Core Utilization) for core-specific pattern matching optimizations
- See Section 3.6.4 (Pattern Compilation Strategy) for general pattern matching compilation strategies
- See Section 18.2.3 (AArch64 Memory Model Mapping) for memory ordering implications of pattern matching
- See Section 15.1.2.1 (AArch64 Runtime Integration) for actor state register allocation
- See Section 21.1.3.1 (SVE Runtime Behavior) for SIMD register allocation

### 3.7 Types
```
type ::= type_identifier
       | function_type
       | tuple_type
       | record_type
       | variant_type
       | effect_type

type_identifier ::= identifier

function_type   ::= "(" [type {"," type}] ")" "->" type
tuple_type      ::= "(" type {"," type} ")"
record_type     ::= "struct" identifier "{" identifier ":" type {"," identifier ":" type} "}"
variant_type    ::= identifier {"|" identifier}

effect_type     ::= "proc" "[" effect_list "]" type
effect_list     ::= effect {"," effect}
effect          ::= effect_identifier
```

### 3.8 Effects
```
effect ::= effect_identifier
```

### 3.9 Operator Precedence and Associativity

From highest to lowest precedence:

1. Function application (left associative)
2. Unary operators: `not` (right associative)
3. Binary operators:
   - `*`, `/`, `%` (left associative)
   - `+`, `-` (left associative)
   - `<`, `<=`, `>`, `>=` (non-associative)
   - `==`, `!=` (non-associative)
   - `and` (left associative)
   - `or` (left associative)

Parentheses can be used to override precedence.

## 4. Built-in Types

### 4.1 Primitive Types

#### 4.1.1 Unit Type
The `unit` type has a single value, written as `()`. It represents the absence of meaningful data.

```
type unit = ()
```

#### 4.1.2 Boolean Type
The `boolean` type represents boolean values.

```
type boolean = true | false
```

#### 4.1.3 Integer Types
Silica provides multiple integer types with different bit widths:

```
type int8   // 8-bit signed integer (-128 to 127)
type int16  // 16-bit signed integer (-32,768 to 32,767)
type int32  // 32-bit signed integer (-2,147,483,648 to 2,147,483,647)
type int64  // 64-bit signed integer (-9,223,372,036,854,775,808 to 9,223,372,036,854,775,807)
```

#### 4.1.4 Floating-Point Types
Silica provides multiple floating-point types with different precisions:

```
type float16  // 16-bit half precision floating-point (IEEE 754 binary16)
type float32  // 32-bit single precision floating-point (IEEE 754 binary32)
type float64  // 64-bit double precision floating-point (IEEE 754 binary64)
```

#### 4.1.5 Character Type
The `char` type represents Unicode scalar values.

```
type char
```

#### 4.1.6 String Type
The `string` type represents UTF-8 encoded strings.

```
type string
```

#### 4.1.7 Atom Type
The `atom` type represents symbolic constants that evaluate to themselves. Atoms are prefixed with a colon (`:`); all characters except the atom delimiters (see §2.2.3 Atom Literals) are valid in atom names, including Unicode such as emojis and accented letters.

```
type atom
```

Atoms are interned at compile time into a global atom table. Each unique atom name maps to a fixed integer index, so atom equality is a single integer comparison rather than a string comparison. Atoms require no runtime heap allocation.

**Atom Literal Syntax:**
```silica
:ok                // atom representing success
:error             // atom representing failure
:not_found         // atom representing absence
:pending           // atom representing a waiting state
```

**Usage in Variable Bindings:**
```silica
status: atom <- :ok
reason: atom <- :not_found
```

**Usage in Case Expressions:**
```silica
fn describe_status(s: atom) -> string {
    case s of {
        :ok -> "success";
        :error -> "failure";
        :pending -> "waiting";
        _: atom -> "unknown"
    }
}
```

**Usage as Function Parameters and Return Types:**
```silica
fn validate(input: int64) -> atom {
    case input > 0 of {
        true -> :valid;
        false -> :invalid
    }
}
```

**Usage in Tuples (Tagged Values):**
Atoms combine naturally with tuples to create tagged values, providing a lightweight alternative to defining variant types:
```silica
fn safe_divide(x: int64, y: int64) -> (atom, int64) {
    case y == 0 of {
        true -> (:error, 0);
        false -> (:ok, x / y)
    }
}

fn handle_result(result: (atom, int64)) -> string {
    case result of {
        (:ok, v: int64) -> "result: " + int_to_string(v);
        (:error, _: int64) -> "division by zero"
    }
}
```

**Atom Identity Semantics:**
Two atoms are equal if and only if they have the same name. Atom comparison is O(1) since the compiler maps each atom to a unique integer index at compile time.

```silica
:ok == :ok           // true
:ok == :error        // false
:ok != :error        // true
```

**AArch64 Representation:**
Atoms are represented as 64-bit integer indices into the atom table. The atom table is generated at compile time and stored in a read-only data section. Atom comparison compiles to a single `CMP` instruction on the integer index.

### 4.2 Compound Types

#### 4.2.1 Function Types
Function types have the form `(ParamTypes...) -> ReturnType`.

Examples:
```
(int, int) -> int                    // binary function
() -> atom                           // nullary function returning atom
(string) -> int proc[mem(normal)]    // function returning a process
```

#### 4.2.2 Tuple Types
Tuple types have the form `(Type1, Type2, ..., TypeN)`.

Examples:
```
(int, boolean)                          // pair of int and boolean
(char, char, char)                   // triple of characters
()                                   // unit (empty tuple)
```

**Recursive Tuple Types (Extension):**

Tuple types may contain the keyword `rec` in type positions. `rec` refers to the enclosing tuple type. Recursive positions are represented as `ref(R, Space, rec) | :none` (optional region reference). The shorthand `ref?(R, Space, T)` denotes `ref(R, Space, T) | :none`.

Examples:
```
(int64, ref?(R, normal, (int64, rec, rec)), ref?(R, normal, (int64, rec, rec)))   // BST node
(int64, ref?(R, normal, (int64, rec)))                                            // linked list node
```

- **Base case:** The atom `:none` denotes an empty recursive position.
- **Construction:** `alloc_rec(region, (value, ...))` allocates a recursive tuple in a region; returns `ref(R, Space, tuple_type)`. Effect: `mem(Space)`.
- **Decomposition:** Use `read_ref(ref)` to obtain the tuple value, then decompose with `(pattern) <- expr`. Pattern match on `ref?` slots: `case slot of { :none -> ...; ref_var: ref(R, rec) -> ... }`.
- **Type checking:** Structural equality with `rec`-scoped occurs check. `rec` is valid only inside a tuple type.

See [recursive_tuple_specification.md](recursive_tuple_specification.md) for full design.

#### 4.2.3 Record Types
Record types have the form `{field1: Type1, field2: Type2, ..., fieldN: TypeN}`.

Example:
```
{ name: string, age: int, active: boolean }
```

#### 4.2.4 List Types
List types are parameterized by their element type and have the form `List[ElementType]` where `ElementType` must implement the `Collectable` trait.

```
List[ElementType: Collectable]
```

Lists are immutable - all operations return new lists rather than modifying existing lists. The language implementation uses structural sharing to make head operations (prepend, remove_head) efficient without copying all elements.

Examples:
- `List[string]` - list of strings
- `List[int64]` - list of integers
- `List[(int64 -> int64)]` - list of functions
- `List[List[int64]]` - nested list (list of lists of integers)

**Type Annotation Requirement**: All list variables and expressions must explicitly specify the element type using `List[ElementType]` syntax. Type inference is not performed for lists - the element type must always be explicitly provided. The type `List` alone (without element type) is not valid.

**Immutability**: Lists in Silica are immutable. All list operations return new lists rather than modifying existing lists. The language implementation uses structural sharing to make these operations efficient - when prepending an element or removing the head, the original list structure is reused rather than copying all elements.

**Head Operations Only**: All list modification operations work only on the head of the list. There are no operations to remove elements from the middle or end of a list.

**Internal Representation**: Lists are composed of sets of buffers. The buffers are sized to match the vector processing units found on the target chip architecture (e.g., 128-bit or 256-bit buffers for NEON/SVE on AArch64). When a non-primitive type is stored in a list, a reference to a Silica region is placed in the buffer at the correct location relative to existing region references. This design enables efficient vectorized operations on list elements and aligns with AArch64 vector processing capabilities.

#### 4.2.5 Variant Types
Variant types represent sum types with the form `Constructor1 [Type1] | Constructor2 [Type2] | ...`.

Examples:
```
type status = Ok | Error
```

### 4.3 Process Types
Process types represent monadic computations and have the form `proc[Effects] ResultType`.

Examples:
```
proc[] int                           // pure computation returning int
proc[mem(normal)] ref(region, int)   // computation allocating memory
proc[concurrency] actor_ref(msg)     // computation spawning an actor
```

### 4.4 Region and Memory Types

#### 4.4.1 Lifetime Type and Lifetime Identifiers

**Lifetime Type:** The built-in type `lifetime` represents a lifetime identifier. Values of type `lifetime` are obtained from the built-in function `fresh_lifetime()` (see §22.1). Each call to `fresh_lifetime()` returns a unique lifetime value.

**Lifetime Identifiers (L*):** Region types use lifetime identifiers (e.g. `L1`, `L2`) to track region scope. The first parameter of `region`, `ref`, `buf`, and `atomic_ref` types is a lifetime identifier. Programmers obtain lifetime values via `fresh_lifetime()` and pass them to region-allocating functions. This ensures unique lifetimes when composing multiple regions in the same scope.

**Allocation Scope:** Region allocation (`alloc_region`, `alloc_ref`, `alloc_buf`, `alloc_atomic`) is permitted only within sequence blocks (`sequence ... produces ... end`), not in function scope or plain `do` blocks.

#### 4.4.2 Region Types
Region types represent memory regions: `region(L, Space)` where L is a lifetime identifier and Space is a memory space.

**Memory Space Types:**

```
normal                              // Normal memory (write-back cacheable, default)
normal_writeback                    // Normal memory with write-back caching (default normal)
normal_writethrough                 // Normal memory with write-through caching
normal_noncacheable                // Normal memory, non-cacheable
atomic                              // Atomic memory region (for atomic operations)
device                              // Device memory (reserved for future driver library)
```

**Normal Memory Variants:**
- **normal** or **normal_writeback**: Write-back cacheable memory. Writes go to cache first, flushed to memory on eviction. Best performance for most use cases. This is the default when `normal` is specified.
- **normal_writethrough**: Write-through cacheable memory. Writes update both cache and memory immediately. Useful when you need data to be immediately visible to other cores or DMA devices.
- **normal_noncacheable**: Non-cacheable memory. All accesses go directly to memory, bypassing cache. Use for DMA buffers, shared memory with devices, or when cache coherency overhead is undesirable.

**Atomic Memory:**
- **atomic**: Memory space specifically for atomic operations. Provides hardware support for atomic read-modify-write operations.

**Device Memory:**
- **device**: Memory-mapped I/O space for device registers. Reserved for future device driver library. Application code should use normal memory variants.

```
region(L1, normal)                  // normal memory region (write-back), lifetime L1
region(L1, normal_writeback)       // explicit write-back (same as normal)
region(L1, normal_writethrough)     // write-through cacheable
region(L1, normal_noncacheable)     // non-cacheable
region(L1, atomic)                  // atomic memory region
region(L1, device)                  // device memory (driver library only)
```

The lifetime identifier (e.g. `L1`) is obtained from `fresh_lifetime()` and must be unique among region allocations in the same or composable scope.

**Region Handle Move Semantics:** Region handles are move-only; they cannot be copied. When a region handle is passed to any function, ownership transfers (the handle is moved). The function must return the handle to the caller. This ensures exactly one handle exists per region at any time. See §12.1.5 for the special case of `spawn`.

#### 4.4.3 Reference Types
Reference types represent pointers to memory: `ref(L, Space, T)`.

```
ref(L1, normal, int)                  // reference to int in region with lifetime L1
```

#### 4.4.4 Buffer Types
Buffer types represent contiguous arrays: `buf(L, Space, T, N)`.

```
buf(L1, normal, int, 1024)            // buffer of 1024 ints
```

#### 4.4.5 Atomic Types
Atomic reference types: `atomic_ref(L, Space, T)`.

```
atomic_ref(L1, normal, int)           // atomic reference to int
```

### 4.5 Actor Types

#### 4.5.1 Actor Reference Types
Actor references are a primitive type (like `int` or `boolean`):

```
actor_ref                            // actor reference (primitive type)
```

The `actor_ref` type is not parameterized by message type. It is a primitive type that represents a reference to an actor, created by the `spawn()` function.

### 4.6 Core Affinity Types

Silica provides types for specifying CPU core affinity for actor placement:

```
core_id                              // Single CPU core identifier (primitive type)
core_set                             // Set of CPU cores (primitive type)
performance_cores                     // Built-in: high-performance cores
efficiency_cores                      // Built-in: low-power efficiency cores
```

These types are used with the `spawn()` function to control actor placement:

```silica
spawn(initial_state, behavior, performance_cores)
spawn(initial_state, behavior, core_id(0))
spawn(initial_state, behavior, core_set([0, 1, 2]))
```

### 4.7 SIMD Vector Types

Silica provides first-class SIMD vector types for AArch64 architectures using concrete types and marker traits:

#### 4.7.1 Marker Traits for Vector Elements

```
// Marker trait for NEON 128-bit vector elements
trait Vec128Element {
    // Marker trait, no methods needed
}

// Marker trait for SVE scalable vector elements
trait VecElement {
    // Marker trait, no methods needed
}

// Implement traits for supported types
impl Vec128Element for int8;
impl Vec128Element for int16;
impl Vec128Element for int32;
impl Vec128Element for int64;
impl Vec128Element for float32;

impl VecElement for int8;
impl VecElement for int16;
impl VecElement for int32;
impl VecElement for int64;
impl VecElement for float16;
impl VecElement for float32;
impl VecElement for float64;
```

#### 4.7.2 NEON 128-bit Vector Types
Fixed-width 128-bit vectors using NEON instructions:

```
Vec128Int8    // 16 × int8 elements
Vec128Int16   // 8 × int16 elements
Vec128Int32   // 4 × int32 elements
Vec128Int64   // 2 × int64 elements
Vec128Float32 // 4 × float32 elements
Vec128Boolean    // Boolean vector for comparisons
```

**Note**: All NEON vector types use concrete element types. The `Vec128Element` trait marks types that can be used as NEON vector elements.

#### 4.7.3 SVE Scalable Vector Types
Scalable vectors using SVE/SVE2 instructions (size determined at runtime):

```
VecInt8       // Scalable vector of int8
VecInt16      // Scalable vector of int16
VecInt32      // Scalable vector of int32
VecInt64      // Scalable vector of int64
VecFloat16    // Scalable vector of float16
VecFloat32    // Scalable vector of float32
VecFloat64    // Scalable vector of float64
VecBoolean       // Scalable boolean vector
```

**Note**: All SVE vector types use concrete element types. The `VecElement` trait marks types that can be used as SVE vector elements.

#### 4.7.4 SVE Predicate Type
```
Pred          // SVE predicate mask for conditional operations
```

The `Pred` type represents a predicate mask used for conditional vector operations in SVE.

## 5. Built-in Functions and Primitives

Silica provides a comprehensive set of built-in functions and language primitives for common operations. These are always available without requiring imports.

### 5.1 Print Functions

All print functions require the `device_io` effect. The `device_io` effect is limited to: print (stdout), read from file, write to file, and read from console.

#### 5.1.1 String Printing
```
print(value: string) -> atom proc[device_io]
println(value: string) -> atom proc[device_io]
```

Print a string to stdout. `println` appends a newline. Returns `:ok` on success.

#### 5.1.2 Numeric Printing
```
print_int8(value: int8) -> atom proc[device_io]
print_int16(value: int16) -> atom proc[device_io]
print_int32(value: int32) -> atom proc[device_io]
print_int64(value: int64) -> atom proc[device_io]
```

Print integer values to stdout. Returns `:ok` on success.

#### 5.1.3 Floating-Point Printing
```
print_float16(value: float16) -> atom proc[device_io]
print_float32(value: float32) -> atom proc[device_io]
print_float64(value: float64) -> atom proc[device_io]
```

Print floating-point values to stdout. Returns `:ok` on success.

#### 5.1.4 Other Type Printing
```
print_boolean(value: boolean) -> atom proc[device_io]
print_char(value: char) -> atom proc[device_io]
```

Print boolean and character values to stdout. Returns `:ok` on success.

### 5.2 File I/O Functions

All file I/O functions (read, write, directory operations) require the `device_io` effect.

#### 5.2.1 File Reading
```
read_file(path: string) -> string proc[device_io]
read_lines(path: string) -> list<string> proc[device_io]
```

Read entire file contents or lines from a file.

#### 5.2.2 File Writing
```
write_file(path: string, content: string) -> atom proc[device_io]
append_file(path: string, content: string) -> atom proc[device_io]
```

Write or append content to a file.

#### 5.2.3 File Operations
```
file_exists(path: string) -> boolean proc[device_io]
delete_file(path: string) -> atom proc[device_io]
get_file_size(path: string) -> int64 proc[device_io]
```

Check file existence, delete files, and get file sizes.

#### 5.2.4 Directory Operations
```
create_directory(path: string) -> atom proc[device_io]
remove_directory(path: string) -> atom proc[device_io]
list_directory(path: string) -> list<string> proc[device_io]
```

Create, remove, and list directory contents.

### 5.3 String Operations

String operations are pure functions (no effects required).

#### 5.3.1 String Length
```
length_bytes(s: string) -> int64
length_chars(s: string) -> int64
```

These are the only user-available functions for finding the length of strings. `length_bytes` returns the byte length (UTF-8 encoded size); `length_chars` returns the character count (number of Unicode scalar values).

#### 5.3.2 String Manipulation
```
concat(a: string, b: string) -> string
substring(s: string, start: int64, end: int64) -> string
substring_until_char(s: string, start: int64, char: char) -> string
```

Concatenate strings and extract substrings. `substring` uses **character-based** indices (UTF-8 code points), not byte offsets. E.g. `substring("Hi🙂!", 2, 3)` returns `"🙂"`.

#### 5.3.3 String Predicates
```
starts_with(s: string, prefix: string) -> boolean
ends_with(s: string, suffix: string) -> boolean
contains(s: string, substr: string) -> boolean
```

Check string prefixes, suffixes, and containment.

### 5.4 Process Execution

```
exec_command(command: string, args: list<string>) -> string proc[device_io]
```

Execute a system command and return its output. Requires the `device_io` effect.

### 5.5 System Information

```
get_cpu_topology_info() -> string proc[device_io]
```

Get CPU topology information as a JSON string. Requires the `device_io` effect.

### 5.6 List Operations

All list operations work with the `List[ElementType]` type where `ElementType` must implement `Collectable`. Lists are immutable - all operations return new lists rather than modifying existing lists.

#### 5.6.1 List Construction

```silica
// Create empty list with explicit element type
fn empty[ElementType: Collectable]() -> List[ElementType]
```

#### 5.6.2 Head Operations

```silica
// Prepend an element to the front of a list (returns new list)
// Efficient: uses structural sharing, does not copy existing elements
fn prepend[ElementType: Collectable](item: ElementType, list: List[ElementType]) -> List[ElementType]

// Remove the head element from a list (returns new list)
// Efficient: uses structural sharing, does not copy remaining elements
// Returns empty list if input list is empty
fn remove_head[ElementType: Collectable](list: List[ElementType]) -> List[ElementType]

// Get the head element of a list
// Returns the first element
fn head[ElementType: Collectable](list: List[ElementType]) -> ElementType
// Runtime error if list is empty

// Get the tail of a list (all elements except the head)
// Returns a new list containing all elements except the first
fn tail[ElementType: Collectable](list: List[ElementType]) -> List[ElementType]
// Returns empty list if input list has 0 or 1 elements
```

#### 5.6.3 List Utilities

```silica
// Reverse a list (returns new list with elements in reverse order)
fn reverse[ElementType: Collectable](list: List[ElementType]) -> List[ElementType]

// Get list length
fn length[ElementType: Collectable](list: List[ElementType]) -> int64

// Check if list is empty
fn is_empty[ElementType: Collectable](list: List[ElementType]) -> boolean
```

**Operation Notes:**
- **Head operations only**: All list modification operations work only on the head of the list. There are no operations to remove elements from the middle or end of a list.
- **Immutability**: All operations return new lists; they never modify existing lists.
- **Structural sharing**: When prepending or removing the head, Silica uses structural sharing to avoid copying all elements. The original list structure is reused efficiently.

**Examples:**

```silica
// Create a list
my_list: List[string] <- ["hello", "world"]: List[string];

// Prepend an element (returns new list)
new_list: List[string] <- prepend[string]("new", my_list);
// my_list is unchanged, new_list is ["new", "hello", "world"]

// Remove head element (returns new list)
shorter_list: List[string] <- remove_head[string](my_list);
// my_list is unchanged, shorter_list is ["world"]

// Get head element
first: string <- head[string](my_list);  // "hello"

// Get tail
rest: List[string] <- tail[string](my_list);  // ["world"]

// Reverse list
reversed: List[string] <- reverse[string](my_list);  // ["world", "hello"]

// Check properties
len: int64 <- length[string](my_list);  // 2
empty_flag: boolean <- is_empty[string](my_list);  // false

// Create empty list
empty_list: List[string] <- empty[string]();
```

### 5.5 System Information

```
get_cpu_topology_info() -> string proc[device_io]
```

Get CPU topology information as a JSON string. Requires the `device_io` effect.

## 6. Language Features

### 6.1 Advanced Pattern Matching

#### 6.1.1 Record Patterns
Record patterns allow destructuring record values:

```silica
struct Point {
    x: int,
    y: int
}

fn distance_from_origin(p: Point) -> int {
    case p.x == 0 && p.y == 0 of {
        true -> 0
        false -> p.x * p.x + p.y * p.y  // Simplified for now
    }
}
```

#### 6.1.2 Variant Patterns
Variant patterns match against sum type constructors:

```silica
// Note: Sum types and variant patterns not yet implemented in experiments
// This shows the intended future syntax

fn handle_result(success: boolean, message: string) -> string {
    case success of {
        true -> "Success: " + message
        false -> "Error: " + message
    }
}
```

### 6.2 Exception Handling

#### 6.2.1 Exception Types
Silica provides structured exception handling:

```silica
type exception =
    DivisionByZero
  | InvalidArgument(string)
  | FileNotFound(string)
```

#### 6.2.2 Throwing Exceptions
Safe division using case expressions:

```silica
fn safe_divide(x: int, y: int) -> int {
    case y == 0 of {
        true -> 0  // Return 0 for division by zero
        false -> x / y
    }
}
```

#### 6.2.3 Result Handling
Error handling through return values (exceptions not yet implemented):

```silica
fn main() -> int {
    do
        result:int <- safe_divide(10, 2);
        // In future: proper error handling with Result types
        result
    end
}
```

#### 6.2.4 AArch64-Specific Exception Handling Patterns

Silica integrates with AArch64 hardware exception mechanisms to provide efficient and safe error handling. The language exposes AArch64 exception handling capabilities through structured error handling patterns.

**AArch64 Exception Types:**

AArch64 hardware generates exceptions for various error conditions:

- **Synchronous Exceptions**: Generated by instruction execution (e.g., division by zero, memory access violations)
- **Asynchronous Exceptions**: Generated by external events (e.g., interrupts, system calls)
- **System Errors**: Generated by system-level conditions (e.g., MTE tag faults, pointer authentication failures)

**Hardware Exception Integration:**

Silica integrates with AArch64 exception handling mechanisms:

```pseudocode
// AArch64 exception handling flow
function handle_hardware_exception(exception_type, exception_info):
    case exception_type of {
        DIVISION_BY_ZERO -> {
            // AArch64 generates synchronous exception for division by zero
            // Silica runtime converts to language-level error handling
            raise_exception(DivisionByZero)
        }
        MEMORY_ACCESS_VIOLATION -> {
            // AArch64 generates synchronous exception for invalid memory access
            // Silica runtime converts to language-level error handling
            raise_exception(InvalidMemoryAccess(exception_info.address))
        }
        MTE_TAG_FAULT -> {
            // AArch64 MTE generates tag fault exception
            // Silica runtime converts to language-level error handling
            raise_exception(MemoryTagMismatch(exception_info.address, exception_info.tag))
        }
        POINTER_AUTHENTICATION_FAILURE -> {
            // AArch64 PAC generates authentication failure exception
            // Silica runtime converts to language-level error handling
            raise_exception(PointerAuthenticationFailed(exception_info.address))
        }
        _: ExceptionType -> {
            // Other AArch64 exceptions
            raise_exception(SystemError(exception_type, exception_info))
        }
    }
end function
```

**Exception Handling in Actor Contexts:**

Actors handle exceptions independently, providing isolation:

```silica
type ActorMessage = Request | Response;

type Request = {
    command: string,
    value: int64
}

type Response = {
    result: int64,
    error: OptionException
}

impl ActorMessage for Request;
impl ActorMessage for Response;

// Actor behavior with exception handling
fn safe_calculator(msg: Request, state: int64) -> int64 {
    case msg.command of {
        "divide" -> {
            // Division with error handling
            divisor: int64 <- msg.value;
            if divisor == 0 {
                // Return error response (no exception thrown)
                Response {result: 0, error: Some(DivisionByZero)}
            } else {
                // Successful division
                result: int64 <- state / divisor;
                Response {result: result, error: None}
            }
        }
        "multiply" -> {
            // Multiplication (no error possible)
            result: int64 <- state * msg.value;
            Response {result: result, error: None}
        }
        _: string -> {
            // Unknown command
            Response {result: 0, error: Some(InvalidArgument(msg.command))}
        }
    }
}
```

**AArch64 Hardware Exception to Silica Error Mapping:**

The runtime maps AArch64 hardware exceptions to Silica error types:

| AArch64 Exception | Silica Exception Type | Handling Strategy |
|------------------|----------------------|------------------|
| Division by zero (`DIV`) | `DivisionByZero` | Check divisor before division, return error |
| Memory access violation (`SEGV`) | `InvalidMemoryAccess(address)` | Bounds checking, region validation |
| MTE tag fault (`SEGV_MTESERR`) | `MemoryTagMismatch(address, tag)` | Tag validation, memory safety checks |
| Pointer authentication failure (`SEGV_PACERR`) | `PointerAuthenticationFailed(address)` | PAC validation, pointer integrity checks |
| Illegal instruction (`SIGILL`) | `IllegalInstruction(opcode)` | Instruction validation, feature detection |
| Floating-point exception (`SIGFPE`) | `FloatingPointError(operation)` | FP operation validation, NaN handling |

**Exception Handling Best Practices:**

**Practice 1: Check Before Operations**

Always check conditions before operations that may generate hardware exceptions:

```silica
fn safe_divide(x: int64, y: int64) -> ResultInt64String {
    if y == 0 {
        // Check before division to avoid hardware exception
        Error("Division by zero")
    } else {
        // Safe division (no hardware exception)
        Ok(x / y)
    }
}
```

**Practice 2: Use Result Types for Error Propagation**

Use Result types to propagate errors without exceptions:

```silica
type ResultInt64String = Ok(int64) | Error(string)

fn process_with_errors(x: int64, y: int64) -> ResultInt64String {
    do
        // Chain operations with error handling
        result1: ResultInt64String <- safe_divide(x, y);
        case result1 of {
            Ok(value) -> {
                // Continue processing
                Ok(value * 2)
            }
            Error(msg) -> {
                // Propagate error
                Error(msg)
            }
        }
    end
}
```

**Practice 3: Actor Exception Isolation**

Actors handle exceptions independently, providing isolation:

```silica
// Actor with robust error handling
fn robust_actor(msg: Request, state: State) -> State {
    do
        // Attempt operation with error handling
        result: ResultInt64String <- process_with_errors(msg.value1, msg.value2);
        case result of {
            Ok(value) -> {
                // Update state on success
                State {value: value, error_count: state.error_count}
            }
            Error(msg) -> {
                // Handle error without crashing actor
                // Log error, update error count, continue processing
                State {value: state.value, error_count: state.error_count + 1}
            }
        }
    end
}
```

**AArch64 Exception Handling Performance:**

Exception handling has performance implications:

- **Hardware Exception Overhead**: Hardware exceptions incur significant overhead (~1000-10000 cycles)
- **Software Error Checking**: Software checks (e.g., `if divisor == 0`) have minimal overhead (~1-2 cycles)
- **Result Type Overhead**: Result type pattern matching has minimal overhead (~2-5 cycles)
- **Best Practice**: Prefer software checks over hardware exceptions for performance

**Exception Handling Examples:**

**Example 1: Division by Zero Prevention**

```silica
fn safe_division(x: int64, y: int64) -> ResultInt64String {
    // Check before division (avoids hardware exception)
    if y == 0 {
        Error("Division by zero")
    } else {
        // Safe division (no hardware exception generated)
        Ok(x / y)
    }
}
```

**Example 2: Memory Access Validation**

```silica
fn safe_memory_access(ref: ref(R, normal, int), index: int) -> ResultInt64String {
    // Validate index before access (avoids hardware exception)
    if index < 0 or index >= buffer_length(ref) {
        Error("Index out of bounds")
    } else {
        // Safe access (no hardware exception generated)
        Ok(read_ref(ref, index))
    }
}
```

**Example 3: MTE Tag Validation**

```silica
fn safe_tagged_access(ptr: ref(R, normal, NodeData)) -> ResultNodeDataString {
    // Check tag before access (avoids MTE hardware exception)
    tag_valid: boolean <- check_tag_nodedata(ptr);
    if not tag_valid {
        Error("Tag mismatch - potential use-after-free")
    } else {
        // Safe access (no MTE hardware exception generated)
        Ok(read_ref(ptr))
    }
}
```

**Cross-References:**
- See Section 20.1.2 (Result Type) for Result type usage
- See Section 15.3 (Actor Failure and Supervision) for actor exception handling
- See Section 21.3.3 (MTE Runtime Integration) for MTE exception handling
- See Section 21.4.5 (PAC Authentication Failure Handling) for PAC exception handling
- See Section 12.4 (Memory Safety Guarantees) for memory access exception prevention
- See Section 15.1.2.1 (AArch64 Runtime Integration) for hardware exception integration
- See Section 6.2.1 (Exception Types) for exception type definitions
- See Section 6.2.2 (Throwing Exceptions) for exception throwing semantics

### 6.3 Advanced Effects

#### 6.3.1 Effect Composition
Effects can be combined:

```silica
effect io_and_mem = [device_io, mem(normal)]

fn combined_operation() -> int {
    do
        // Operations that require both I/O and memory effects
        42
    end
}
```

## 7. Basic Expressions

### 7.1 Literals
Literal expressions evaluate to their corresponding values:

```
42          // evaluates to integer 42
true        // evaluates to boolean true
'a'         // evaluates to character 'a'
"hello"     // evaluates to string "hello"
()          // evaluates to unit value
:ok         // evaluates to atom :ok
```

### 7.2 Identifiers
Identifier expressions evaluate to the value bound to that identifier in the current scope.

```
x           // evaluates to the value of variable x
my_function // evaluates to the function bound to my_function
```

### 7.3 Arithmetic Expressions
Arithmetic operators work on integers:

```
x + y       // integer addition
a - b       // integer subtraction
m * n       // integer multiplication
p / q       // integer division (truncates toward zero)
r % s       // integer modulo
```

### 7.4 Comparison Expressions
Comparison operators return boolean values:

```
x == y      // equality
a != b      // inequality
p < q       // less than
r <= s      // less than or equal
m > n       // greater than
u >= v      // greater than or equal
```

### 7.5 Logical Expressions
Logical operators work on booleans:

```
not p       // logical negation
p and q     // logical conjunction
r or s      // logical disjunction
```

### 7.6 Function Application
Function application has the form `function(arg1, arg2, ..., argN)`:

```
add(3, 4)           // applies add function to 3 and 4
length_bytes("hello")   // applies length_bytes built-in to string (byte length)
f()                 // applies nullary function
```

### 7.7 Grouping
Parentheses can be used to group expressions and override precedence:

```
(2 + 3) * 4         // evaluates to 20, not 14
not (p and q)       // equivalent to (not p) or (not q)
```

## 8. Type System

### 8.1 Type Constructors
Type constructors define how to build complex types from simpler ones.

Built-in type constructors:
- `ref<R, S, T>` - reference in region R, space S, to type T
- `buf<R, S, T, N>` - buffer in region R, space S, of N elements of type T

User-defined types are declared with concrete types:

```
type int_stack = { elements: list<int>, size: int }
type string_map = { data: list<pair<string, string>>, size: int }
```

### 8.2 Type Equivalence and Subtyping

#### 8.2.1 Structural Equivalence
Types are equivalent if they have the same structure:

```
int ≡ int                                   // primitive types
(int, boolean) ≡ (int, boolean)                   // tuple types
{a: int, b: boolean} ≡ {a: int, b: boolean}       // record types
```

#### 8.2.2 Nominal Equivalence for User Types
User-defined types are equivalent only if they have the same name:

```
type my_int = int
type your_int = int

my_int ≢ your_int    // different names, not equivalent
my_int ≢ int         // user type vs primitive
```

#### 8.2.3 Subtyping Rules
Silica has no structural subtyping - all types must match exactly. However, polymorphism is achieved through trait composition, where types implementing the same trait can be used interchangeably in trait-constrained contexts.

```
// No structural subtyping:
type MyInt = int64
type YourInt = int64
MyInt ≢ YourInt    // Different names, not equivalent

// Trait-based polymorphism (not subtyping):
trait Display { fn show(self) -> string; }
impl Display for int64;
impl Display for string;

// Types implementing Display can be used where Display is required
fn print_value(x: Display) -> atom { ... }
// int64 and string can both be passed to print_value
```

#### 8.2.4 Collectable Trait

The `Collectable` trait is a marker trait indicating that a type can be collected in lists. This trait has no methods; it serves as a type-level marker.

```silica
trait Collectable {
    // Marker trait indicating that a type can be collected in lists
    // This trait has no methods; it serves as a type-level marker
}
```

**Automatic Collectable Implementation**

The following types automatically implement `Collectable` without requiring explicit `impl` declarations. These automatic implementations are language rules, not explicit code. The compiler treats these types as `Collectable` by default. No explicit `impl Collectable for ...` declarations are needed or allowed for these built-in cases.

**Primitive Types (Automatic Collectable):**
- `unit`
- `boolean`
- `int8`
- `int16`
- `int32`
- `int64`
- `float16`
- `float32`
- `float64`
- `char`
- `string`
- `atom`

**Function Types (Automatic Collectable):**
All function types of the form `(T1 -> T2)` where `T1` and `T2` are concrete types automatically implement `Collectable`. This includes:
- Simple function types: `(int64 -> int64)`, `(string -> boolean)`, etc.
- Higher-order function types: `((int64 -> int64) -> string)`, `((string -> boolean) -> (int64 -> int64))`, etc.
- Functions with any arity: `(int64, string -> boolean)`, `(int64, int64, int64 -> int64)`, etc.

**Tuple Types (Automatic Collectable):**
All tuple types of the form `(T1, T2, ...)` where all `Ti` are concrete types automatically implement `Collectable`. This includes:
- `(int64, string)`
- `(boolean, int64, string)`
- `((int64 -> int64), string)` (tuples containing functions)
- Any combination of concrete types

**Struct Types (Automatic Collectable):**
All struct types automatically implement `Collectable`:
```silica
struct Point {
    x: int64,
    y: int64
}
// Point automatically implements Collectable
```

**Enum Types (Automatic Collectable):**
All enum types automatically implement `Collectable`:
```silica
enum OptionInt {
    None,
    Some(int64)
}
// OptionInt automatically implements Collectable
```

**Type Aliases (Automatic Collectable):**
Type aliases inherit `Collectable` from their underlying type:
```silica
type MyInt = int64;
// MyInt automatically implements Collectable (inherited from int64)
```

**List Type (Automatic Collectable):**
The `List` type itself automatically implements `Collectable`, enabling nested lists:
```silica
// List[ElementType] automatically implements Collectable for any ElementType
// This allows: List[List[string]], List[List[List[int64]]], etc.
```

**Type Checking Rules for Lists:**
- All elements in a list literal must have exactly the same type
- The explicit type annotation `List[ElementType]` must match the types of all elements exactly
- Mixing different types in a list literal is a compile-time error, even if both types implement `Collectable`
- The element type must implement `Collectable`
- All list variables must have an explicit element type - `List` alone is not a valid type

**Note**: While many types implement `Collectable`, a list can only contain elements of a single, specific type. For example, a `List[string]` cannot contain `int64` values, even though both `string` and `int64` implement `Collectable`.

**Trait-Based Polymorphism Examples:**

**Example 1: Basic Trait Polymorphism**

```silica
// Define trait
trait Display {
    fn show(self: Self) -> string;
}

// Define types
type Point2D = {x: int64, y: int64};
type Rectangle = {width: int64, height: int64};

// Implement trait for different types
impl Display for Point2D {
    fn show(self: Point2D) -> string {
        // Format point as string
        format("Point({}, {})", self.x, self.y)
    }
}

impl Display for Rectangle {
    fn show(self: Rectangle) -> string {
        // Format rectangle as string
        format("Rectangle({}x{})", self.width, self.height)
    }
}

// Function using trait constraint
fn print_display(value: Display) -> atom {
    str: string <- value.show();
    print_string(str)
}

// Usage: Both Point2D and Rectangle can be passed to print_display
fn example() -> atom {
    point: Point2D <- Point2D {x: 3, y: 7};
    rect: Rectangle <- Rectangle {width: 4, height: 5};
    
    // Both types implement Display, so both can be used
    print_display(point);  // Calls Point2D.show()
    print_display(rect);   // Calls Rectangle.show()
}
```

**Example 2: Multiple Trait Constraints**

```silica
// Define multiple traits
trait Display {
    fn show(self: Self) -> string;
}

trait Math {
    fn compute(self: Self, other: int64) -> int64;
}

// Type implementing multiple traits
type Point2D = {x: int64, y: int64};

impl Display for Point2D {
    fn show(self: Point2D) -> string {
        format("Point({}, {})", self.x, self.y)
    }
}

impl Math for Point2D {
    fn compute(self: Point2D, other: int64) -> int64 {
        (self.x + self.y) * other
    }
}

// Function requiring multiple trait constraints
fn print_and_compute(value: Display, math_value: Math, scale: int64) -> atom {
    // Use Display trait
    str: string <- value.show();
    print_string(str);
    
    // Use Math trait
    result: int64 <- math_value.compute(scale);
    print_int64(result)
}

// Usage: Point2D implements both traits
fn example() -> atom {
    point: Point2D <- Point2D {x: 2, y: 3};
    
    // Point2D can be used for both Display and Math constraints
    print_and_compute(point, point, 4)
}
```

**Example 3: Trait Constraints in Function Parameters**

```silica
// Define trait
trait Comparable {
    fn equals(self: Self, other: Self) -> boolean;
    fn less_than(self: Self, other: Self) -> boolean;
}

// Implement trait for different types
type Person = {name: string, age: int64};

impl Comparable for Person {
    fn equals(self: Person, other: Person) -> boolean {
        self.name == other.name and self.age == other.age
    }
    
    fn less_than(self: Person, other: Person) -> boolean {
        self.age < other.age
    }
}

// Function using trait constraint
fn find_maximum(a: Comparable, b: Comparable) -> Comparable {
    if a.less_than(b) {
        b
    } else {
        a
    }
}

// Usage: Person implements Comparable
fn example() -> Person {
    person1: Person <- Person {name: "Alice", age: 30};
    person2: Person <- Person {name: "Bob", age: 25};
    
    // Person can be used where Comparable is required
    max_person: Person <- find_maximum(person1, person2);
    max_person
}
```

**Example 4: Trait-Based Collections**

```silica
// Define trait for collection elements
trait CollectionElement {
    fn to_string(self: Self) -> string;
}

// Implement trait for different types
impl CollectionElement for int64 {
    fn to_string(self: int64) -> string {
        format("{}", self)
    }
}

impl CollectionElement for string {
    fn to_string(self: string) -> string {
        self
    }
}

// Function operating on collections of trait-constrained elements
fn print_collection_element(elem: CollectionElement) -> atom {
    str: string <- elem.to_string();
    print_string(str)
}

// Usage: Both int64 and string can be used
fn example() -> atom {
    num: int64 <- 42;
    text: string <- "Hello";
    
    // Both types implement CollectionElement
    print_collection_element(num);   // Calls int64.to_string()
    print_collection_element(text);  // Calls string.to_string()
}
```

**Key Points:**

1. **No Structural Subtyping**: `Point2D` and `Rectangle` are not subtypes of each other, even if they have similar structure
2. **Trait-Based Polymorphism**: Types implementing the same trait can be used interchangeably in trait-constrained contexts
3. **Explicit Trait Constraints**: Function parameters specify trait requirements (e.g., `value: Display`)
4. **Multiple Trait Implementation**: Types can implement multiple traits
5. **Trait Method Dispatch**: Method calls on trait-constrained values dispatch to the appropriate implementation based on the actual type

**Difference from Structural Subtyping:**

- **Structural Subtyping**: Types with same structure are automatically compatible
- **Trait-Based Polymorphism**: Types must explicitly implement traits to be compatible
- **Explicit vs. Implicit**: Trait implementation is explicit, structural subtyping is implicit

#### 8.2.4 Trait Composition and Design Patterns

Trait-based polymorphism enables powerful design patterns through trait composition and combination.

**Trait Composition Patterns:**

Types can implement multiple traits to create rich interfaces:

**Example 1: Multiple Trait Implementation**
```silica
trait Display {
    fn show(self: Self) -> string;
}

trait Debug {
    fn debug(self: Self) -> string;
}

// Type implementing multiple traits
type Point2D = {x: int64, y: int64};

impl Display for Point2D {
    fn show(self: Point2D) -> string {
        format("Point({}, {})", self.x, self.y)
    }
}

impl Debug for Point2D {
    fn debug(self: Point2D) -> string {
        format("Point2D {{ x: {}, y: {} }}", self.x, self.y)
    }
}

// Function requiring a specific trait
fn print_debug(value: Debug) -> atom {
    debug_str: string <- value.debug();
    print_string(debug_str)
}
```

**Example 2: Multiple Trait Constraints**
```silica
// Define independent traits
trait Serialize {
    fn serialize(self: Self) -> string;
}

trait Deserialize {
    fn deserialize(data: string) -> Self;
}

// Type implementing both traits
type User = {id: int64, name: string};

impl Serialize for User {
    fn serialize(self: User) -> string {
        format("{{id: {}, name: \"{}\"}}", self.id, self.name)
    }
}

impl Deserialize for User {
    fn deserialize(data: string) -> User {
        // Parse JSON-like string (simplified)
        User {id: 1, name: "Alice"}
    }
}

// Function requiring both traits
fn round_trip(value: Serialize, deserializer: Deserialize) -> atom {
    // Serialize value
    serialized: string <- value.serialize();
    
    // Deserialize using different type's deserializer
    // (Note: This is a simplified example - actual usage would be type-specific)
}
```

**Example 3: Trait-Based Type Erasure**
```silica
// Trait for type-erased values
trait AnyValue {
    fn type_name(self: Self) -> string;
    fn to_string(self: Self) -> string;
}

// Implement for different types
impl AnyValue for int64 {
    fn type_name(self: int64) -> string {
        "int64"
    }
    
    fn to_string(self: int64) -> string {
        format("{}", self)
    }
}

impl AnyValue for string {
    fn type_name(self: string) -> string {
        "string"
    }
    
    fn to_string(self: string) -> string {
        self
    }
}

// Function accepting any type-erased value
fn process_any_value(value: AnyValue) -> atom {
    type_name: string <- value.type_name();
    str_value: string <- value.to_string();
    print_string(type_name);
    print_string(str_value)
}
```

**Common Trait Design Patterns:**

**Pattern 1: Marker Traits**
```silica
// Marker trait (no methods, just type safety)
trait Sendable {
    // No methods - just marks types as safe to send between actors
}

// Implement for types that can be sent
impl Sendable for int64;
impl Sendable for string;

// Function requiring marker trait
fn send_message(msg: Sendable) -> atom {
    // Can send any Sendable type
}
```

**Pattern 2: Builder Pattern with Traits**
```silica
// Trait for buildable types
trait Buildable {
    fn build(self: Self) -> Self;
}

// Type with builder methods
type Config = {host: string, port: int64};

impl Buildable for Config {
    fn build(self: Config) -> Config {
        // Validate and build configuration
        self
    }
}

// Builder function using trait
fn create_config(builder: Buildable) -> Buildable {
    builder.build()
}
```

**Pattern 3: Strategy Pattern with Traits**
```silica
// Strategy trait
trait SortStrategy {
    fn sort(self: Self, data: list<int64>) -> list<int64>;
}

// Different sorting strategies
type QuickSort = unit;
type MergeSort = unit;

impl SortStrategy for QuickSort {
    fn sort(self: QuickSort, data: list<int64>) -> list<int64> {
        // Quick sort implementation
        data
    }
}

impl SortStrategy for MergeSort {
    fn sort(self: MergeSort, data: list<int64>) -> list<int64> {
        // Merge sort implementation
        data
    }
}

// Function using strategy trait
fn sort_data(strategy: SortStrategy, data: list<int64>) -> list<int64> {
    strategy.sort(data)
}
```

**Trait-Based Type Checking:**

While Silica requires explicit type annotations for all variable bindings, function parameters, and return types, trait constraints enable flexible type usage through trait-based type checking. The type checker verifies that types implement required traits rather than inferring types.

**Example: Trait-Based Type Checking**
```silica
// Trait for numeric operations
trait Numeric {
    fn add(self: Self, other: Self) -> Self;
    fn multiply(self: Self, other: Self) -> Self;
}

impl Numeric for int64 {
    fn add(self: int64, other: int64) -> int64 {
        self + other
    }
    
    fn multiply(self: int64, other: int64) -> int64 {
        self * other
    }
}

// Function with trait constraint - type checker verifies trait implementation
fn compute(value: Numeric, factor: Numeric) -> Numeric {
    // Trait constraint: return type must match input type's trait implementation
    value.multiply(factor)
}

// Usage: Type is explicit but constrained by trait
fn example() -> int64 {
    x: int64 <- 10;
    y: int64 <- 5;
    result: int64 <- compute(x, y);  // Type must be explicit, trait checked at compile time
    result
}
```

**Cross-References:**
- See Section 3.4.8 (Trait Declarations) for trait declaration syntax
- See Section 3.4.9 (Implementation Declarations) for trait implementation syntax
- See Section 10.1.4.1 (Trait-Constrained Function Application) for trait constraint resolution
- See Section 19.3.2 (Name Resolution) for trait method resolution

### 8.3 Type Declarations

#### 8.3.1 Explicit Type Requirements
All types must be explicitly declared in Silica. There is no type inference - every variable binding, function parameter, and return type must have an explicit type annotation. However, when using trait-constrained function parameters (e.g., `x: Display`), the type checker verifies trait implementations rather than inferring types. This trait-based type checking enables polymorphism while maintaining explicit type annotations.

#### 8.3.2 Type Annotation Rules
1. Function parameters must have explicit types: `fn add(x: int64, y: int64) -> int64`
2. Variable bindings must have explicit types: `value: int64 <- 42`
3. Function return types must be explicit: `-> int64`
4. Pattern matching uses typed patterns: `n: int64 if n > 0 -> ...`

#### 8.3.3 Type Annotation Examples

```
fn add(x: int64, y: int64) -> int64 { x + y }        // explicit types

fn example() -> int64 {
    do
        x: int64 <- 42;                               // variable binding with type
        x
    end
}
```

## 9. Effect System

### 9.1 Effect Types

#### 9.1.1 Built-in Effects
Silica defines several built-in effects that track different kinds of side effects:

- `mem(Space)` - Memory allocation/deallocation in space `Space`
  - `mem(normal)` or `mem(normal_writeback)` - Normal write-back cacheable memory
  - `mem(normal_writethrough)` - Normal write-through cacheable memory
  - `mem(normal_noncacheable)` - Normal non-cacheable memory
  - `mem(atomic)` - Atomic memory operations
  - `mem(device)` - Device memory (reserved for driver library)
- `mailbox` - Message passing (messages must implement `ActorMessage` trait)
- `concurrency` - Actor spawning and scheduling
- `atomic` - Atomic memory operations
- `device_io` - Limited to: print (stdout), read from file, write to file, read from console
- `network_io` - Network communications of all kinds (sockets, HTTP, etc.)
- `hot_swap` - Code loading (dynamic loading, JIT, self-modifying code). On AArch64, requires `ISB` barrier to ensure instruction fetch sees code writes.
- `register_rwr` - Direct device register access via mmap (read and write). On AArch64, requires `DSB SY` before and `ISB` after for device ordering.

**Note**: The `mailbox` effect is untyped. Type safety for actor messages is ensured through:
1. Function parameter types (behavior functions receive typed messages)
2. The `ActorMessage` trait (all message types must implement this trait)
3. Compile-time type checking

#### 9.1.2 Effect Aliases
Effects can be aliased for convenience and abstraction:

```
effect actor_eff = [mailbox, concurrency]
effect io_eff = [mem(normal), device_io]
effect atomic_eff = [mem(atomic), atomic]
effect network_eff = [network_io]
effect hot_swap_eff = [hot_swap]
effect register_rwr_eff = [register_rwr]
```

**Note**: Effect aliases use untyped `mailbox` effect. Message type safety is ensured through function parameter types and the `ActorMessage` trait.

#### 9.1.3 User-Defined Effects
New effects can be declared for domain-specific side effects:

```
effect logging = []        // pure effect for logging framework
effect database = [mem(normal), device_io]  // database operations
```

### 9.2 Effect Composition

#### 9.2.1 Effect Sets
Effects are combined in sets: `proc[effect1, effect2, ...] Result`

```
proc[mem(normal), atomic] int           // memory + atomic operations
proc[concurrency] actor_ref<msg>        // actor spawning
proc[] unit                             // pure computation
```

#### 9.2.2 Built-in Memory Operations
Silica provides built-in memory operations as primitive language constructs that return processes:

```
alloc_region(Space) -> region(R, Space) proc[mem(Space)]
// R is taken from the binding's explicit type annotation (e.g. region(R1, Space)); no implicit typing
alloc_ref(Region, Value) -> ref(R, Space, T) proc[mem(Space)]
read_ref(Ref) -> T proc[mem(Space)]
write_ref(Ref, Value) -> atom proc[mem(Space)]
```

These operations are not function calls but fundamental language primitives for memory management.

#### 9.2.3 Process Composition
Processes compose through monadic binding:

```
// Creates a process value (not executed)
process_value: proc[mem(normal), mem(atomic)] (region(R1, normal), region(R2, atomic)) <- sequence proc[mem(normal), mem(atomic)]
    x: region(R1, normal) <- alloc_region(normal)     // creates process value
    y: region(R2, atomic) <- alloc_region(atomic)     // creates process value
produces
    pure (x, y)
end

// Execute the process using spawn
result: (region(R1, normal), region(R2, atomic)) <- spawn(process_value)
```

#### 9.2.4 Effect Subeffecting
Effects form a subeffecting lattice:

- `mem(normal) <: mem(atomic)` - atomic space includes normal operations
- `mem(normal_writeback) <: mem(normal)` - write-back is the default normal
- `mem(normal_writethrough) <: mem(normal)` - write-through is a normal variant
- `mem(normal_noncacheable) <: mem(normal)` - non-cacheable is a normal variant
- `[] <: E` - pure computations can be used where effects are expected
- `[e1] <: [e1, e2]` - subset relation for effect sets

**Note**: All normal memory variants (`normal_writeback`, `normal_writethrough`, `normal_noncacheable`) are subtypes of `mem(normal)` for effect checking purposes. However, regions allocated with different normal variants are not interchangeable - a region allocated as `normal_noncacheable` cannot be used where `normal_writeback` is expected.

### 9.3 Effect Tracking Rules

#### 9.3.1 Effect Declaration on Sequences
**Function declarations do not contain effect declarations.** Effects are declared only on sequence blocks using `sequence proc[effect_list]`.

**Effect Declaration Rules:**

1. **Pure Functions**: Functions with no effects need no sequence block:
   ```
   fn add(x: int64, y: int64) -> int64 {
       x + y
   }
   ```

2. **Single Effect**: Effects are declared on the sequence block:
   ```
   fn allocate(region: region(R, normal)) -> ref(R, normal, int) {
       sequence proc[mem(normal)]
           ref: ref(R, normal, int) <- alloc_ref(region, 0);
       produces
           pure ref
       end
   }
   ```

3. **Multiple Effects**: All effects are declared on the sequence:
   ```
   fn allocate_and_print(region: region(R, normal)) -> ref(R, normal, int) {
       sequence proc[mem(normal), device_io]
           ref: ref(R, normal, int) <- alloc_ref(region, 0);
           print_string("Allocated");
       produces
           pure ref
       end
   }
   ```

4. **Effect Propagation**: Effects propagate through sequence blocks - if a sequence calls effectful functions, those effects must be declared on the sequence:
   ```
   fn helper() -> ref(R, normal, int) { ... }  // uses sequence proc[mem(normal)] internally
   
   fn caller() -> ref(R, normal, int) {
       sequence proc[mem(normal)]
           ref: ref(R, normal, int) <- helper();
       produces
           pure ref
       end
   }
   ```

**Effect Set Union Algorithm:**

When multiple effects are combined (e.g., from multiple function calls or operations), the compiler computes the union of effect sets using the following algorithm:

```pseudocode
function union_effect_sets(effect_set1, effect_set2):
    // Initialize result set
    result = empty_set()
    
    // Add all effects from first set
    for each effect in effect_set1:
        result = result ∪ {effect}
    
    // Add all effects from second set
    for each effect in effect_set2:
        result = result ∪ {effect}
    
    // Apply subeffecting: if e1 <: e2, keep only e2 (more general)
    // Remove redundant effects based on subeffecting lattice
    result = minimize_effect_set(result)
    
    return result
end function

function minimize_effect_set(effect_set):
    // Remove redundant effects based on subeffecting
    minimized = effect_set
    
    // Check for subeffecting relationships
    for each effect1 in minimized:
        for each effect2 in minimized:
            if effect1 != effect2 and effect1 <: effect2:
                // effect1 is subsumed by effect2, remove effect1
                minimized = minimized \ {effect1}
            end if
        end for
    end for
    
    return minimized
end function
```

**Subeffecting in Effect Union:**

The effect union algorithm respects the subeffecting lattice:

- **Subeffecting Rules**:
  - `mem(normal) <: mem(atomic)` - atomic space includes normal operations
  - `mem(normal_writeback) <: mem(normal)` - write-back is a normal variant
  - `mem(normal_writethrough) <: mem(normal)` - write-through is a normal variant
  - `mem(normal_noncacheable) <: mem(normal)` - non-cacheable is a normal variant
  - `[] <: E` - pure computations can be used where effects are expected
  - `[e1] <: [e1, e2]` - subset relation for effect sets

**Effect Union Examples:**

**Example 1: Simple Union**
```silica
// Function with mem(normal) - effects on sequence
fn alloc() -> ref(R, normal, int) { sequence proc[mem(normal)] ... }

// Function with device_io - effects on sequence
fn print() -> atom { sequence proc[device_io] ... }

// Combined effects: union declared on sequence
fn alloc_and_print() -> ref(R, normal, int) {
    sequence proc[mem(normal), device_io]
        ref: ref(R, normal, int) <- alloc();  // mem(normal)
        print();                              // device_io
    produces
        pure ref                              // Combined: [mem(normal), device_io]
    end
}
```

**Example 2: Subeffecting in Union**
```silica
// Function with mem(normal) - effects on sequence
fn alloc_normal() -> ref(R, normal, int) { ... }

// Function with mem(atomic) - effects on sequence
fn alloc_atomic() -> ref(R, atomic, int) { ... }

// Combined: mem(normal) <: mem(atomic), effects on sequence
fn alloc_both() -> (ref(R, normal, int), ref(R, atomic, int)) {
    sequence proc[mem(atomic)]
        ref1: ref(R, normal, int) <- alloc_normal();  // mem(normal)
        ref2: ref(R, atomic, int) <- alloc_atomic();  // mem(atomic)
    produces
        pure (ref1, ref2)                             // Union: [mem(atomic)] (normal subsumed)
    end
}
```

**Example 3: Multiple Effect Union**
```silica
// Multiple effects declared on sequence
fn complex() -> atom {
    sequence proc[mem(normal), device_io, concurrency]
        alloc();        // mem(normal)
        print();        // device_io
        spawn(...);     // concurrency
    produces
        pure ()         // Union: [mem(normal), device_io, concurrency]
    end
}
```

**Example 4: Effect Composition Through Nested Calls**

Effects propagate through nested function calls, requiring all effects to be declared:

```silica
// Helper function with mem(normal) - effects on sequence
fn allocate_helper() -> ref(R, normal, int) {
    sequence proc[mem(normal)]
        r: region(R, normal) <- alloc_region(normal);
        ref: ref(R, normal, int) <- alloc_ref(r, 42);
    produces
        pure ref
    end
}

// Helper function with device_io - effects on sequence
fn print_helper() -> atom {
    sequence proc[device_io]
        print_string("Helper called")
    produces
        pure :ok
    end
}

// Function that calls both helpers - effects on sequence
fn composed_operation() -> ref(R, normal, int) {
    sequence proc[mem(normal), device_io]
        ref: ref(R, normal, int) <- allocate_helper();  // mem(normal) propagates
        print_helper();                                  // device_io propagates
    produces
        pure ref                                        // Combined: [mem(normal), device_io]
    end
}
```

**Example 5: Effect Composition with Subeffecting**

When effects have subeffecting relationships, the compiler minimizes the effect set:

```silica
// Function with mem(normal) - effects on sequence
fn alloc_normal() -> ref(R, normal, int) { ... }

// Function with mem(atomic) - effects on sequence (mem(normal) <: mem(atomic))
fn alloc_atomic() -> ref(R, atomic, int) { ... }

// Combined: mem(normal) is subsumed by mem(atomic)
fn alloc_both() -> (ref(R, normal, int), ref(R, atomic, int)) {
    sequence proc[mem(atomic)]
        ref1: ref(R, normal, int) <- alloc_normal();   // mem(normal)
        ref2: ref(R, atomic, int) <- alloc_atomic();   // mem(atomic)
    produces
        pure (ref1, ref2)                              // Union: [mem(atomic)] (normal subsumed)
    end
}
```

**Example 6: Effect Composition in Sequence Expressions**

Sequence expressions compose effects from multiple statements:

```silica
fn complex_operation() -> int {
    sequence proc[mem(normal), device_io, concurrency]
        // Statement 1: mem(normal)
        r: region(R, normal) <- alloc_region(normal);
        ref: ref(R, normal, int) <- alloc_ref(r, 42);
        
        // Statement 2: device_io
        print_string("Allocated: ");
        print_int(read_ref(ref));
        
        // Statement 3: concurrency
        actor_ref: actor_ref <- spawn(initial_state, behavior_fn);
        send(actor_ref, SomeMessage {});
    produces
        pure read_ref(ref)  // Combined: [mem(normal), device_io, concurrency]
    end
}
```

**Example 7: Effect Composition with Function Literals**

Function literals do not declare effects; the sequence inside declares them. Effects compose with the enclosing sequence:

```silica
fn actor_with_effects() -> actor_ref {
    sequence proc[mem(normal), device_io, concurrency]
        // Function literal - effects on sequence inside
        behavior_fn: fn(msg: Message, state: State) -> State = 
            fn(msg: Message, state: State) -> State {
                sequence proc[mem(normal), device_io]
                    r: region(R, normal) <- alloc_region(normal);
                    print_string("Processing message");
                produces
                    pure State { value: 42 }
                end
            };
        
        // Spawn actor (concurrency effect)
    produces
        pure spawn(initial_state, behavior_fn)  // Combined: [mem(normal), device_io, concurrency]
    end
}
```

**Example 8: Effect Composition Error Cases**

When effects are not properly declared, compilation errors occur:

```silica
// ERROR: Function uses mem(normal) but doesn't declare it
fn allocate_missing_effect(region: region(R, normal)) -> ref(R, normal, int) {
    alloc_ref(region, 0)  // Error: mem(normal) effect not declared
}

// ERROR: Function calls helper with effects but doesn't declare them
fn caller_missing_effects() -> int {
    allocate_helper()  // Error: mem(normal) effect not declared in caller
}

// CORRECT: All effects properly declared
fn allocate_correct(region: region(R, normal)) -> ref(R, normal, int) {
    alloc_ref(region, 0)  // Correct: mem(normal) declared
}

fn caller_correct() -> ref(R, normal, int) {
    allocate_helper()  // Correct: mem(normal) declared in caller
}
```

**Example 9: Effect Composition with Conditional Execution**

Effects compose correctly even with conditional execution:

```silica
fn conditional_effects(condition: boolean) -> int {
    do
        if condition {
            // Branch 1: mem(normal) and device_io
            r: region(R, normal) <- alloc_region(normal);
            ref: ref(R, normal, int) <- alloc_ref(r, 42);
            print_string("Branch 1");
            read_ref(ref)
        } else {
            // Branch 2: device_io only
            print_string("Branch 2");
            0
        }
        // Combined: [mem(normal), device_io] (union of both branches)
    end
}
```

**Example 10: Effect Composition with Pattern Matching**

Effects compose correctly in pattern matching expressions:

```silica
fn pattern_matching_effects(msg: Message) -> int {
    do
        result: int <- case msg of {
            AllocateMsg {size} -> {
                // Pattern branch: mem(normal)
                r: region(R, normal) <- alloc_region(normal);
                alloc_ref(r, size);
                0
            }
            PrintMsg {text} -> {
                // Pattern branch: device_io
                print_string(text);
                0
            }
            _: Message -> {
                // Catch-all: no effects
                0
            }
        };
        // Combined: [mem(normal), device_io] (union of all branches)
        result
    end
}
```

**Minimal Effect Set Computation:**

The compiler computes the minimal effect set by removing redundant effects:

```pseudocode
function compute_minimal_effects(function_body):
    // Collect all effects from function body
    all_effects = empty_set()
    
    // Traverse function body and collect effects
    for each operation in function_body:
        op_effects = get_operation_effects(operation)
        all_effects = union_effect_sets(all_effects, op_effects)
    end for
    
    // Minimize effect set (remove redundant effects)
    minimal_effects = minimize_effect_set(all_effects)
    
    return minimal_effects
end function
```

**Effect Union in Function Calls:**

When a function calls another function, the caller's effect set must include all effects from the callee:

```pseudocode
function check_function_call(caller_effects, callee_effects):
    // Verify callee effects are subsumed by caller effects
    for each effect in callee_effects:
        if not is_subsumed_by(effect, caller_effects):
            error("Caller must declare effect: " + effect)
        end if
    end for
end function

function is_subsumed_by(effect, effect_set):
    // Check if effect is subsumed by any effect in effect_set
    for each e in effect_set:
        if effect <: e:
            return true  // Effect is subsumed
        end if
    end for
    
    // Check if effect is directly in the set
    if effect in effect_set:
        return true
    end if
    
    return false  // Effect not subsumed
end function
```

**Effect Checking:**

The type checker verifies that:
1. All effects used in the function body are declared on sequence blocks
2. All effects required by called functions are declared on the enclosing sequence
3. Effect mismatches result in type errors (E2011)

**No Effect Inference:**

Silica does not infer effects. If a function uses an effect but does not declare it, a compilation error occurs:

```
// ERROR: Function uses mem(normal) but doesn't declare it
fn allocate(region: region(R, normal)) -> ref(R, normal, int) {
    alloc_ref(region, 0)  // Error: mem(normal) effect not declared
}
```

**Function Literals and Effects:**

Function literals (anonymous functions) can also declare effects using the same `proc[...]` syntax. This is required when the function literal uses operations that require specific effects:

```
// Function literal with device_io effect
fn(msg: Response, state: EchoState) -> EchoState {
    print_string("Received message: ");
    print_int(msg.result);
    EchoState { received: state.received + msg.result }
}
```

Function literals must declare all effects they use, just like regular function declarations. If a function literal uses an effect but doesn't declare it, a type error (E2011) will occur.

#### 9.3.3 Effect Error Examples

Silica's effect system requires explicit effect declarations. When effects are missing or mismatched, the compiler generates specific error messages with suggestions for fixing the errors. This section provides examples of common effect errors and their error messages.

**Effect Error Suggestions:**

The compiler may include suggestions in effect error messages to help developers fix missing effect declarations:

- **Suggestion Format**: Error messages may include a "Suggestion:" section showing the corrected function signature
- **Automatic Detection**: The compiler analyzes the function body to determine which effects should be declared
- **Signature Generation**: The compiler generates a suggested function signature with the correct effect declarations
- **Optional Feature**: Effect suggestions are optional - compilers may choose to include or omit them based on implementation preferences

**Error Message Format:**

Effect errors follow the standard Silica error message format with LLM-parseable metadata:

```
❌ Compilation error: [EffectError] error at [file]:[line]:[column] [[ErrorCode]]

[Human-readable error description]
See specification: spec:[section]

<!-- SILICA-ERROR-METADATA
{
  "@context": "https://aalang.dev/silica-dev/error/",
  "errorCode": "[ErrorCode]",
  "errorType": "error",
  "severity": "error",
  "location": {
    "file": "[file]",
    "line": [line],
    "column": [column],
    "offset": [offset]
  },
  "specification": {
    "section": "[section]"
  },
  "effect": {
    "missing": ["effect1", "effect2"],
    "declared": ["effect3"],
    "required": ["effect1", "effect2", "effect3"]
  }
}
-->
```

**Example 1: Missing Effect Declaration**

**Code:**
```silica
fn allocate_int(region: region(R, normal)) -> ref(R, normal, int) {
    alloc_ref(region, 0)  // Error: mem(normal) effect not declared
}
```

**Error Message:**
```
❌ Compilation error: EffectError error at example.silica:2:5 [E3001]

Function uses mem(normal) effect but does not declare it on sequence.
Required effects: [mem(normal)]
Declared effects: []
See specification: spec:§9.3.1

Suggestion: Use a sequence block with `sequence proc[mem(normal)]` in the function body:
  fn allocate_int(region: region(R, normal)) -> ref(R, normal, int) {
      sequence proc[mem(normal)]
          ref: ref(R, normal, int) <- alloc_ref(region, 0);
      produces
          pure ref
      end
  }

<!-- SILICA-ERROR-METADATA
{
  "@context": "https://aalang.dev/silica-dev/error/",
  "errorCode": "E3001",
  "errorType": "error",
  "severity": "error",
  "location": {
    "file": "example.silica",
    "line": 2,
    "column": 5,
    "offset": 45
  },
  "specification": {
    "section": "§9.3.1"
  },
  "effect": {
    "missing": ["mem(normal)"],
    "declared": [],
    "required": ["mem(normal)"]
  },
  "suggestion": {
    "type": "add_effect_declaration",
    "suggested_signature": "fn allocate_int(region: region(R, normal)) -> ref(R, normal, int) — use sequence proc[mem(normal)] in body"
  }
}
-->
```

**Fix:**
```silica
fn allocate_int(region: region(R, normal)) -> ref(R, normal, int) {
    alloc_ref(region, 0)
}
```

**Example 2: Missing Effect in Function Call Chain**

**Code:**
```silica
fn helper() -> ref(R, normal, int) {
    region: region(R, normal) <- alloc_region(normal);
    alloc_ref(region, 0)
}

fn caller() -> ref(R, normal, int) {  // Error: missing mem(normal) effect
    helper()
}
```

**Error Message:**
```
❌ Compilation error: EffectError error at example.silica:6:5 [E3002]

Function calls helper() which requires mem(normal) effect, but the sequence does not declare it.
Required effects: [mem(normal)]
Declared effects: []
See specification: spec:§9.3.1

Suggestion: Use `sequence proc[mem(normal)]` in the function body:
  fn caller() -> ref(R, normal, int) {
      sequence proc[mem(normal)]
          ref: ref(R, normal, int) <- helper();
      produces
          pure ref
      end
  }

<!-- SILICA-ERROR-METADATA
{
  "@context": "https://aalang.dev/silica-dev/error/",
  "errorCode": "E3002",
  "errorType": "error",
  "severity": "error",
  "location": {
    "file": "example.silica",
    "line": 6,
    "column": 5,
    "offset": 78
  },
  "specification": {
    "section": "§9.3.1"
  },
  "effect": {
    "missing": ["mem(normal)"],
    "declared": [],
    "required": ["mem(normal)"],
    "called_function": "helper",
    "called_function_effects": ["mem(normal)"]
  },
  "suggestion": {
    "type": "add_effect_declaration",
    "suggested_signature": "fn caller() -> ref(R, normal, int) — use sequence proc[mem(normal)] in body"
  }
}
-->
```

**Fix:**
```silica
fn caller() -> ref(R, normal, int) {
    sequence proc[mem(normal)]
        ref: ref(R, normal, int) <- helper();
    produces
        pure ref
    end
}
```

**Example 3: Effect Mismatch in Function Call**

**Code:**
```silica
fn high_level() -> atom {
    do
        print_string("Hello")
    end
}

fn low_level() -> atom {
    high_level()  // Error: device_io effect mismatch
}
```

**Error Message:**
```
❌ Compilation error: EffectError error at example.silica:8:5 [E3003]

Function calls high_level() which requires [mem(normal), device_io] effects, but low_level() only declares [mem(normal)].
Required effects: [mem(normal), device_io]
Declared effects: [mem(normal)]
Missing effects: [device_io]
See specification: spec:§9.3.1

<!-- SILICA-ERROR-METADATA
{
  "@context": "https://aalang.dev/silica-dev/error/",
  "errorCode": "E3003",
  "errorType": "error",
  "severity": "error",
  "location": {
    "file": "example.silica",
    "line": 8,
    "column": 5,
    "offset": 112
  },
  "specification": {
    "section": "§9.3.1"
  },
  "effect": {
    "missing": ["device_io"],
    "declared": ["mem(normal)"],
    "required": ["mem(normal)", "device_io"],
    "called_function": "high_level",
    "called_function_effects": ["mem(normal)", "device_io"]
  }
}
-->
```

**Fix:**
```silica
fn low_level() -> atom {
    high_level()
}
```

**Example 4: Missing Effect in Function Literal**

**Code:**
```silica
fn create_actor() -> actor_ref {
    behavior_fn: (Msg, State) -> State = fn(msg: Msg, state: State) -> State {
        print_string("Received")  // Error: device_io effect not declared
        state
    };
    spawn(initial_state, behavior_fn)
}
```

**Error Message:**
```
❌ Compilation error: EffectError error at example.silica:3:9 [E3004]

Function literal uses device_io effect but does not declare it on sequence.
Required effects: [device_io]
Declared effects: []
See specification: spec:§9.3.1

<!-- SILICA-ERROR-METADATA
{
  "@context": "https://aalang.dev/silica-dev/error/",
  "errorCode": "E3004",
  "errorType": "error",
  "severity": "error",
  "location": {
    "file": "example.silica",
    "line": 3,
    "column": 9,
    "offset": 67
  },
  "specification": {
    "section": "§9.3.1"
  },
  "effect": {
    "missing": ["device_io"],
    "declared": [],
    "required": ["device_io"],
    "context": "function_literal"
  }
}
-->
```

**Fix:**
```silica
fn create_actor() -> actor_ref {
    behavior_fn: (Msg, State) -> State = fn(msg: Msg, state: State) -> State {
        print_string("Received")
        state
    };
    spawn(initial_state, behavior_fn)
}
```

**Example 5: Multiple Missing Effects**

**Code:**
```silica
fn complex_operation() -> atom {
    do
        region: region(R, normal) <- alloc_region(normal);
        print_string("Allocated")
    end
}
```

**Error Message:**
```
❌ Compilation error: EffectError error at example.silica:2:5 [E3005]

Function uses [mem(normal), device_io] effects but does not declare them on sequence.
Required effects: [mem(normal), device_io]
Declared effects: []
See specification: spec:§9.3.1

<!-- SILICA-ERROR-METADATA
{
  "@context": "https://aalang.dev/silica-dev/error/",
  "errorCode": "E3005",
  "errorType": "error",
  "severity": "error",
  "location": {
    "file": "example.silica",
    "line": 2,
    "column": 5,
    "offset": 23
  },
  "specification": {
    "section": "§9.3.1"
  },
  "effect": {
    "missing": ["mem(normal)", "device_io"],
    "declared": [],
    "required": ["mem(normal)", "device_io"]
  }
}
-->
```

**Fix:**
```silica
fn complex_operation() -> atom {
    do
        region: region(R, normal) <- alloc_region(normal);
        print_string("Allocated")
    end
}
```

**Effect Error Recovery:**

When encountering effect errors, consider:

1. **Add Missing Effects**: Declare all effects used in the function body
2. **Propagate Effects**: If calling functions with effects, declare those effects in the caller
3. **Effect Composition**: Use effect aliases to simplify effect declarations
4. **Refactor**: Consider splitting functions to separate effectful and pure operations

**Effect Error Codes:**

- **E3001**: Missing effect declaration on sequence
- **E3002**: Missing effect in function call chain
- **E3003**: Effect mismatch in function call
- **E3004**: Missing effect in function literal
- **E3005**: Multiple missing effects

#### 9.3.2 Effect Composition
Effect polymorphism is achieved through concrete functions for each effect combination, preserving all effect information.

**Effect Composition in Sequence Expressions:**
Effects compose naturally in `sequence...produces pure...end` blocks. When binding multiple expressions with effects, the resulting process type includes all effects from all bound expressions:

```silica
fn example() -> atom {
    sequence proc[mem(normal), device_io]
        // Creates process with mem(normal) effect
        region: region(R, normal) <- alloc_region(normal);
        
        // Creates process with device_io effect
        print_string("Region allocated");
        
        // Creates process with mem(normal) effect
        ref: ref(R, normal, int64) <- alloc_ref(region, 42);
        
        // Result process has both mem(normal) and device_io effects
    produces
        pure ()
    end
}
```

**Nested Sequence Expressions:**
Nested `sequence...produces pure...end` blocks compose effects from inner to outer:

```silica
fn nested_example() -> atom {
    sequence proc[mem(normal), device_io, concurrency]
        // Outer sequence block
        region: region(R, normal) <- alloc_region(normal);
        
        inner_result: unit <- sequence proc[device_io]
            // Inner sequence block with device_io
            print_string("Inner block");
        produces
            pure ()
        end;
        
        // Spawn actor (requires concurrency)
        actor_ref: actor_ref <- spawn(initial_state, behavior);
        
        // Result includes all effects: mem(normal), device_io, concurrency
    produces
        pure ()
    end
}
```

**Effect Propagation:**
Effects propagate through function calls and bindings. The type checker ensures all required effects are declared:

```
// Pure computation with logging (preserves pure effect)
fn with_logging_pure(action: proc[] int) -> int {
    sequence proc[logging]
        log("Starting action");
        result: int <- action;
        log("Action complete");
    produces
        pure result
    end
}

// Memory normal computation with logging (preserves mem(normal))
fn with_logging_mem_normal(action: proc[mem(normal)] int) -> int {
    sequence proc[mem(normal), logging]
        log("Starting action");
        result: int <- action;
        log("Action complete");
    produces
        pure result
    end
}

// Concurrency computation with logging (preserves concurrency)
fn with_logging_concurrency(action: proc[concurrency] int) -> int {
    sequence proc[concurrency, logging]
        log("Starting action");
        result: int <- action;
        log("Action complete");
    produces
        pure result
    end
}

// Combined effects: mem(normal) + concurrency (preserves both)
fn with_logging_mem_normal_concurrency(action: proc[mem(normal), concurrency] int) 
    -> int {
    sequence proc[mem(normal), concurrency, logging]
        log("Starting action");
        result: int <- action;
        log("Action complete");
    produces
        pure result
    end
}
```

**Note**: Silica does not support generic type parameters or effect variables. Effect polymorphism is achieved through:
1. **Effect aliases** (for common effect combinations)
2. **Concrete functions** (for specific effect combinations)
3. **Trait-based composition** (for non-effect polymorphism)

All effects are declared on sequence blocks - no effect erasure occurs.

**Cross-References:**
- See Section 9.3.1 (Function Effects) for effect declaration and propagation rules
- See Section 9.2 (Effect Composition) for effect set composition semantics
- See Section 9.2.4 (Effect Subeffecting) for effect subtyping relationships
- See Section 10.1.4 (Function Application) for effect checking in function calls
- See Section 9.3.3 (Effect Error Examples) for effect error handling

### 9.4 Effect Safety

#### 9.4.1 Effect Checking
Effect capability enforcement is done at compile time. The type checker ensures:
- All effects in a function body are declared on sequence blocks
- Effect requirements are properly propagated through call chains
- Effect subeffecting is respected
- Functions requiring effects cannot be called without appropriate effect declarations

**Compile-Time Enforcement:**
```silica
fn requires_io() -> atom {
    print_string("Hello")
}

fn pure_function() -> atom {
    requires_io()  // ERROR: pure_function() doesn't declare device_io effect
}

fn caller() -> atom {
    requires_io()  // OK: caller() declares device_io effect
}
```

#### 9.4.2 Effect Type Safety
The type system ensures effect safety at compile time. All effect violations are caught during type checking, not at runtime. Well-typed programs cannot violate effect requirements.

## 10. Type Checking

### 10.1 Type Checking Rules

#### 10.1.1 Expression Typing
Every expression has a type and effect:

```
Γ ⊢ e : τ ! ε

Where:
- Γ is the type environment (variable bindings)
- e is the expression
- τ is the result type
- ε is the effect set
```

#### 10.1.2 Literal Typing
```
Γ ⊢ n : int ! []          where n is an integer literal
Γ ⊢ true : boolean ! []
Γ ⊢ false : boolean ! []
Γ ⊢ 'c' : char ! []
Γ ⊢ "s" : string ! []
Γ ⊢ () : unit ! []
```

#### 10.1.3 Variable Typing
```
Γ ⊢ x : Γ(x) ! []         if x ∈ dom(Γ)
```

#### 10.1.4 Function Application
```
Γ ⊢ f : (τ₁, τ₂, ..., τₙ) → τ ! ε
Γ ⊢ e₁ : τ₁ ! ε₁
...
Γ ⊢ eₙ : τₙ ! εₙ
─────────────────────────────────────────
Γ ⊢ f(e₁, ..., eₙ) : τ ! ε ∪ ε₁ ∪ ... ∪ εₙ
```

#### 10.1.4.1 Trait-Constrained Function Application

When a function parameter has a trait constraint (e.g., `x: Display`), the type checker verifies that the argument type implements the required trait.

**Trait Constraint Checking:**

For a function with trait-constrained parameters:
```
fn f(x: TraitName) -> τ ! ε
```

The type checking rule is:
```
Γ ⊢ e : τ_e ! ε_e
τ_e implements TraitName
─────────────────────────────────────────
Γ ⊢ f(e) : τ ! ε ∪ ε_e
```

**Trait Implementation Lookup:**

The type checker looks up trait implementations using the following algorithm:

1. **Direct Implementation**: Check if `impl TraitName for τ_e` exists

**Trait Constraint Examples:**

```silica
// Define trait
trait Display {
    fn show(self: Self) -> string;
}

// Implement trait for int64
impl Display for int64 {
    fn show(self: int64) -> string {
        format("{}", self)
    }
}

// Function with trait constraint
fn print_value(x: Display) -> atom {
    str: string <- x.show();
    print_string(str)
}

// Type checking: int64 implements Display
fn example() -> atom {
    value: int64 <- 42;
    print_value(value)  // ✓ Valid: int64 implements Display
}
```

**Multiple Trait Constraints:**

When a function requires multiple trait constraints, all constraints must be satisfied:

```silica
trait Display {
    fn show(self: Self) -> string;
}

trait Comparable {
    fn equals(self: Self, other: Self) -> boolean;
}

// Function requiring both traits
fn print_and_compare(x: Display, y: Comparable) -> boolean {
    str: string <- x.show();
    print_string(str);
    y.equals(y, y)
}

// Type checking: Both arguments must implement their respective traits
```

**Trait Constraint Error:**

If an argument type does not implement the required trait, a type error is generated:

```
Type τ_e does not implement trait TraitName
Required trait: TraitName
Argument type: τ_e
```

**Example Error:**

```silica
trait Display {
    fn show(self: Self) -> string;
}

// int64 does not implement Display (no impl declaration)
fn print_value(x: Display) -> atom {
    str: string <- x.show();
    print_string(str)
}

fn example() -> atom {
    value: int64 <- 42;
    print_value(value)  // ERROR: int64 does not implement Display
}
```

**Trait Method Resolution:**

When calling a method on a trait-constrained value, the compiler resolves the method implementation:

1. **Lookup Implementation**: Find `impl TraitName for τ_e`
2. **Method Binding**: Bind method call to the implementation's method
3. **Type Checking**: Verify method signature matches trait definition

**Trait Constraint Propagation:**

Trait constraints propagate through function calls:

```silica
fn helper(x: Display) -> string {
    x.show()  // Trait constraint propagates: x must implement Display
}

fn caller(value: int64) -> string {
    helper(value)  // int64 must implement Display for this to type-check
}
```

#### 10.1.5 Process Creation
```
Γ ⊢ e : τ ! ε
─────────────────
Γ ⊢ proc { e } : proc[ε] τ ! []
```

### 10.2 Declaration Type Checking

#### 10.2.1 Function Declaration
```
Γ ⊢ body : τ_body ! ε_body
τ_body ≡ τ_return
parameters define new bindings in Γ
─────────────────
Γ ⊢ fn f(params): τ_return { body } : () ! []
```

#### 10.2.2 Type Declaration
```
Type declaration is well-formed
─────────────────
Γ ⊢ type T<α₁, ..., αₙ> = τ : () ! []
```

#### 10.2.3 Effect Declaration
```
Effect declaration is well-formed
─────────────────
Γ ⊢ effect E<α₁, ..., αₙ> = [effects] : () ! []
```

### 10.3 Type Errors

#### 10.3.1 Type Mismatch
```
Expected type τ_expected, but got τ_actual
```

#### 10.3.2 Effect Mismatch
```
Process requires effects [ε_required] but declares [ε_declared]
```

#### 10.3.3 Unbound Variable
```
Variable x is not in scope
```

#### 10.3.4 Arity Mismatch
```
Function expects n arguments, but got m
```

### 10.4 Effect Checking Examples

```
fn pure_add(x: int, y: int) -> int {
    x + y        // Type: int ! []
}
// Function type: (int, int) -> int ! []

fn allocate_pair(r: region(R, normal))
 -> (ref(R, normal, int), ref(R, normal, int)) proc[mem(normal)] {
    do
        x: ref(R, normal, int) <- alloc_ref(r, 1)    // creates process value
        y: ref(R, normal, int) <- alloc_ref(r, 2)    // creates process value
        (x, y)                                         // returns composed process value
    end
}
// Effects properly declared: mem(normal)
// Returns a process value that must be spawned to execute

fn bad_alloc(r: region(R, normal)) -> ref(R, normal, int) {
    alloc_ref(r, 42)    // ERROR: requires mem(normal) but declares []
}
```

## 11. Process Semantics and Execution

### 11.1 Process Monad Structure

#### 11.1.1 Process Type
A process `proc[ε] τ` represents a computation that:
- Produces a value of type `τ`
- May perform effects in the set `ε`
- Is executed sequentially within its context

#### 11.1.2 Monadic Operations
Processes support monadic binding (`<-`) and implicit return:

```
sequence proc[ε]
    p <- m    // bind: create process m, bind to p
produces
    pure q    // value produced by the block
end
```

**Note**: Binding creates process values but does not execute them. Processes are executed using the `spawn()` function. The `produces pure` line declares the block's return value; the expression must be effect-free.

#### 11.1.3 Sequential Execution
When processes are executed via `spawn()`, execution is strictly sequential:

```
computation: proc[] (int, int) <- sequence
    x: int <- computation1    // creates process for computation1
    y: int <- computation2    // creates process for computation2
produces
    pure (x, y)
end

// Execution happens when spawned
result: (int, int) <- spawn(computation)
```

### 11.2 Effect Execution Model

#### 11.2.1 Effect Capabilities
Each effect requires runtime capability checking:

- `mem(S)` - Access to memory space S
- `mailbox` - Message queue (messages must implement `ActorMessage` trait)
- `concurrency` - Actor spawning and scheduling
- `atomic` - Atomic memory operations
- `device_io` - Device access permissions

#### 9.2.2 Effect Tracking
Effects are tracked through the execution stack:

```
Execution Stack:
┌─────────────────────────────────┐
│ Process: proc[mem(normal)] ref  │  // current process
├─────────────────────────────────┤
│ Process: proc[concurrency] unit │  // caller
├─────────────────────────────────┤
│ Process: proc[] int            │  // root
└─────────────────────────────────┘

Active Effects: [mem(normal), concurrency]
```

#### 11.2.3 Effect Safety
Runtime enforces effect capabilities when processes are executed via `spawn()`:

```
// Creates process value (not executed yet)
process: proc[mem(normal)] ref(R, normal, int) <- alloc_ref(region, value)
// ✓ Allowed: mem(normal) effect declared

// Execute the process
ref: ref(R, normal, int) <- spawn(process)
// ✓ Allowed: mem(normal) capability active during execution

// Invalid process (missing required effect)
bad_process: proc[] ref(R, normal, int) <- alloc_ref(region, value)
// ✗ Type Error: process requires mem(normal) but declares []
```

### 11.3 Process Lifecycle

#### 11.3.1 Process Creation
Processes are created when bound but not executed:

```
p: proc[mem(normal)] ref(R, normal, int) <- alloc_ref(r, 42)    // creates process value, does not execute
```

Processes are executed using the built-in `spawn()` function. Binding creates a process value that can be passed around or executed later.

#### 11.3.2 Process Execution
Processes are executed using the `spawn()` function:

```
// Process creation (not executed)
computation: proc[mem(normal)] (ref(R, normal, int), ref(R, normal, int)) <- do
    x: ref(R, normal, int) <- alloc_ref(region, 1)
    y: ref(R, normal, int) <- alloc_ref(region, 2)
    (x, y)
end

// Process execution via spawn
result: (ref(R, normal, int), ref(R, normal, int)) <- spawn(computation)
```

#### 11.3.3 Process Composition
Processes compose through monadic binding:

```
fn allocate_pair(r: region(R, normal))
 -> (ref(R, normal, int), ref(R, normal, int)) proc[mem(normal)] {
    x: ref(R, normal, int) <- alloc_ref(r, 1)    // creates process value
    y: ref(R, normal, int) <- alloc_ref(r, 2)    // creates process value
    return (x, y)                                 // returns composed process value
}

// Usage: the function returns a process value that must be spawned
pair_process: proc[mem(normal)] (ref(R, normal, int), ref(R, normal, int)) <- allocate_pair(region)
result: (ref(R, normal, int), ref(R, normal, int)) <- spawn(pair_process)

fn allocate_quad(r: region(R, normal))
    -> (ref(R, normal, int), ref(R, normal, int),
        ref(R, normal, int), ref(R, normal, int)) proc[mem(normal)] {
    do
        (a: ref(R, normal, int), b: ref(R, normal, int)) <- allocate_pair(r)    // creates process value
        (c: ref(R, normal, int), d: ref(R, normal, int)) <- allocate_pair(r)    // creates process value
        (a, b, c, d)                                                              // returns composed process value
    end
}
```

## 12. Memory Model

### 12.1 Region-Based Memory Management

#### 12.1.1 Region Allocation
Regions are allocated explicitly and provide memory pools:

```
alloc_region(normal) -> region(R, normal) proc[mem(normal)]
alloc_region(normal_writeback) -> region(R, normal_writeback) proc[mem(normal_writeback)]
alloc_region(normal_writethrough) -> region(R, normal_writethrough) proc[mem(normal_writethrough)]
alloc_region(normal_noncacheable) -> region(R, normal_noncacheable) proc[mem(normal_noncacheable)]
alloc_region(atomic) -> region(R, atomic) proc[mem(atomic)]
alloc_region(device) -> region(R, device) proc[mem(device)]  // Driver library only
```

**Memory Space Selection Guidelines:**

- **normal** (write-back): Use for general application data, actor state, most buffers. Provides best performance through caching.
- **normal_writethrough**: Use when data must be immediately visible to other cores or DMA devices without explicit cache flushing. Slightly slower than write-back but ensures coherency.
- **normal_noncacheable**: Use for DMA buffers shared with network cards or other devices, memory-mapped shared memory, or when cache overhead is undesirable. All accesses bypass cache.
- **atomic**: Use for memory regions containing atomic references that will be accessed by multiple actors concurrently.
- **device**: Reserved for future device driver library. Application code should not use device memory directly.

#### 12.1.1.1 AArch64 Memory Attribute Mapping

Silica memory spaces map directly to AArch64 memory attributes configured in the `MAIR_EL1` (Memory Attribute Indirection Register). The compiler and runtime configure these attributes to ensure correct hardware behavior.

**MAIR_EL1 Runtime Initialization:**

The runtime initializes `MAIR_EL1` during program startup to configure memory attributes:

1. **Initialization Timing**: `MAIR_EL1` is configured once during program initialization, before any memory regions are allocated
2. **Privilege Level**: `MAIR_EL1` configuration requires EL1 privilege level (kernel mode). User-space programs rely on the kernel to configure `MAIR_EL1` or use system calls to request memory attribute configuration
3. **Configuration Method**: The runtime uses system calls (e.g., `mmap()` with `MAP_ANONYMOUS` and memory attribute hints) or kernel interfaces to configure memory attributes
4. **Error Handling**: If `MAIR_EL1` configuration fails, the runtime falls back to default memory attributes (typically write-back cacheable)
5. **Per-Process Configuration**: `MAIR_EL1` is typically configured per-process, allowing different processes to use different memory attribute mappings
6. **Attribute Persistence**: Once configured, `MAIR_EL1` attributes persist for the lifetime of the process

**System Register Access:**

Access to AArch64 system registers (e.g., `MAIR_EL1`, `MPIDR_EL1`, `CLIDR_EL1`) requires appropriate privilege levels. For detailed information about system register access patterns, privilege requirements, and fallback strategies, see Section 21.0.2 (AArch64 System Register Access).

**Memory Attribute Register (MAIR_EL1) Mapping:**

| Silica Memory Space | MAIR_EL1 Attr Index | Outer Attributes | Inner Attributes | Cache Policy | Shareability |
|---------------------|---------------------|-------------------|------------------|--------------|--------------|
| `normal` | Attr0 | WBRAWA | WBRAWA | Write-Back, Read-Allocate, Write-Allocate | Inner Shareable |
| `normal_writeback` | Attr0 | WBRAWA | WBRAWA | Write-Back, Read-Allocate, Write-Allocate | Inner Shareable |
| `normal_writethrough` | Attr1 | WTRAWA | WTRAWA | Write-Through, Read-Allocate, Write-Allocate | Inner Shareable |
| `normal_noncacheable` | Attr2 | NCNB | NCNB | Non-Cacheable, Non-Bufferable | Inner Shareable |
| `atomic` | Attr0 | WBRAWA | WBRAWA | Write-Back, Read-Allocate, Write-Allocate | Inner Shareable |
| `device` | Attr3 | DEV_nGnRE | DEV_nGnRE | Device, Non-Gathering, Non-Reordering, Early Write Acknowledgment | Outer Shareable |

**Attribute Encoding:**

Each MAIR_EL1 attribute byte encodes cache and memory properties:

- **WBRAWA** (Write-Back, Read-Allocate, Write-Allocate): `0xFF` (11111111)
  - Outer: Write-Back, Read-Allocate, Write-Allocate
  - Inner: Write-Back, Read-Allocate, Write-Allocate
  
- **WTRAWA** (Write-Through, Read-Allocate, Write-Allocate): `0x44` (01000100)
  - Outer: Write-Through, Read-Allocate, Write-Allocate
  - Inner: Write-Through, Read-Allocate, Write-Allocate
  
- **NCNB** (Non-Cacheable, Non-Bufferable): `0x00` (00000000)
  - Outer: Non-Cacheable, Non-Bufferable
  - Inner: Non-Cacheable, Non-Bufferable
  
- **DEV_nGnRE** (Device, Non-Gathering, Non-Reordering, Early Write Acknowledgment): `0x04` (00000100)
  - Outer: Device, Non-Gathering, Non-Reordering, Early Write Acknowledgment
  - Inner: Device, Non-Gathering, Non-Reordering, Early Write Acknowledgment

**MAIR_EL1 Register Structure:**

The MAIR_EL1 (Memory Attribute Indirection Register) is a 64-bit system register that defines memory attributes used by page table entries. The register is divided into 8 attribute bytes, each 8 bits wide:

```
MAIR_EL1 Register Layout:
┌─────────────────────────────────────────────────────────────┐
│ 63:56 │ 55:48 │ 47:40 │ 39:32 │ 31:24 │ 23:16 │ 15:8  │ 7:0  │
│ Attr7 │ Attr6 │ Attr5 │ Attr4 │ Attr3 │ Attr2 │ Attr1 │ Attr0 │
└─────────────────────────────────────────────────────────────┘
```

Each attribute byte (Attr0 through Attr7) encodes cache and memory properties:

**Attribute Byte Encoding:**

Each 8-bit attribute byte is divided into two 4-bit fields:

```
┌─────────┬─────────┐
│ Outer   │ Inner   │
│ [7:4]   │ [3:0]   │
└─────────┴─────────┘
```

**Outer and Inner Attribute Field Encoding:**

Each 4-bit field encodes memory attributes:

| Bits | Meaning | Description |
|------|---------|-------------|
| [3] | Outer/Inner Shareable | 0 = Non-shareable, 1 = Shareable |
| [2:0] | Memory Type | 000 = Device, 001-111 = Normal Memory (cache policy) |

**Normal Memory Cache Policy Encoding:**

For normal memory (not device), bits [2:0] encode cache policy:

| Bits [2:0] | Cache Policy | Description |
|------------|--------------|-------------|
| 000 | Non-cacheable | No caching |
| 001 | Write-Back, Read-Allocate, Write-Allocate | WBRAWA |
| 010 | Write-Through, Read-Allocate, Write-Allocate | WTRAWA |
| 011 | Write-Back, Read-Allocate, No Write-Allocate | WBRA |
| 100-111 | Reserved | Reserved for future use |

**Silica MAIR_EL1 Configuration:**

At program startup, the runtime configures MAIR_EL1 with the following attribute assignments:

```
MAIR_EL1 Configuration:
Attr0 = 0xFF (WBRAWA)  // normal, normal_writeback, atomic
Attr1 = 0x44 (WTRAWA)  // normal_writethrough
Attr2 = 0x00 (NCNB)    // normal_noncacheable
Attr3 = 0x04 (DEV_nGnRE) // device
Attr4-Attr7 = Reserved (0x00)
```

**Runtime MAIR_EL1 Initialization:**

The runtime initializes MAIR_EL1 during program startup:

```pseudocode
// Initialize MAIR_EL1 register
MAIR_EL1 = 0x0000000000000000  // Clear register

// Set Attr0: Write-Back, Read-Allocate, Write-Allocate (WBRAWA)
// Outer: 1111 (WBRAWA, Inner Shareable)
// Inner: 1111 (WBRAWA, Inner Shareable)
MAIR_EL1[7:0] = 0xFF

// Set Attr1: Write-Through, Read-Allocate, Write-Allocate (WTRAWA)
// Outer: 0100 (WTRAWA, Inner Shareable)
// Inner: 0100 (WTRAWA, Inner Shareable)
MAIR_EL1[15:8] = 0x44

// Set Attr2: Non-Cacheable, Non-Bufferable (NCNB)
// Outer: 0000 (Non-cacheable, Inner Shareable)
// Inner: 0000 (Non-cacheable, Inner Shareable)
MAIR_EL1[23:16] = 0x00

// Set Attr3: Device, Non-Gathering, Non-Reordering, Early Write Acknowledgment
// Outer: 0000 (Device nGnRE, Outer Shareable)
// Inner: 0100 (Device nGnRE, Outer Shareable)
MAIR_EL1[31:24] = 0x04

// Write MAIR_EL1 register
MSR MAIR_EL1, MAIR_EL1
```

**Page Table Entry (PTE) Configuration:**

When allocating regions, the runtime configures page table entries with the appropriate memory attribute index:

```
PTE[MAIR_INDEX] = memory_space_to_mair_index(memory_space)
```

The `atomic` memory space uses the same cache attributes as `normal` but with additional hardware guarantees for atomic operations (LDAR/STLR instructions).

**Runtime Behavior:**

- **Cache Coherency**: All memory spaces except `normal_noncacheable` and `device` participate in AArch64 cache coherency protocols (MESI/MOESI)
- **Write Visibility**: 
  - `normal` and `normal_writeback`: Writes become visible to other cores when cache lines are evicted or explicitly flushed
  - `normal_writethrough`: Writes are immediately written to main memory, visible to other cores and DMA devices
  - `normal_noncacheable`: All accesses bypass cache, immediately visible
  - `atomic`: Uses acquire/release semantics for cross-core visibility
- **DMA Visibility**: `normal_writethrough` and `normal_noncacheable` ensure DMA devices see writes without explicit cache maintenance operations

#### 12.1.1.2 Memory Space Runtime Guarantees

**Note on Latency Specifications**: The latency ranges specified in this section represent **typical/average latencies** under normal system load conditions. Actual latencies may vary significantly based on hardware, system load, NUMA topology, cache state, and other factors. These values are provided for guidance and should not be interpreted as performance guarantees.

Each memory space provides specific runtime guarantees for write visibility, cache coherency, and DMA access.

**normal (Write-Back) Runtime Guarantees:**

- **Write Visibility to Other Cores**: 
  - Writes are initially stored in the local core's cache
  - Writes become visible to other cores when:
    1. Cache line is evicted (LRU replacement)
    2. Cache line is explicitly flushed via cache maintenance operations
    3. Another core requests the same cache line (cache coherency protocol)
  - **Visibility Latency**:
    - **Cache-to-cache transfer**: 10-100 nanoseconds (typical)
      - Same NUMA node: 10-50 nanoseconds
      - Cross-NUMA node: 50-200 nanoseconds
      - Cache coherency protocol overhead: ~5-20 nanoseconds
    - **Hardware-specific variations**:
      - **Apple Silicon (M-series)**: 5-30 nanoseconds (optimized cache hierarchy)
      - **Generic AArch64**: 10-100 nanoseconds (standard cache coherency)
      - **High-end servers**: 20-150 nanoseconds (larger cache hierarchies)
  
- **Cache Coherency**: 
  - Participates in AArch64 MESI/MOESI cache coherency protocols
  - Cache lines transition through Modified/Exclusive/Shared/Invalid states
  - Hardware ensures all cores see a consistent view of memory
  - **Coherency latency**: 5-20 nanoseconds per cache line transition
  
- **DMA Visibility**: 
  - DMA devices may not see writes immediately (writes are in cache)
  - Explicit cache flushing required: `DC CIVAC` (Cache Invalidate by Virtual Address to Point of Coherency)
  - **Flush latency**: 100-500 nanoseconds (depends on cache size)
  - Use `normal_writethrough` or `normal_noncacheable` for DMA buffers
  
- **Performance**: 
  - Best performance for most use cases
  - Write-back caching minimizes memory traffic
  - **Cache hit latency**: 1-5 nanoseconds (L1), 5-15 nanoseconds (L2), 15-50 nanoseconds (L3)
  - **Cache miss latency**: 50-300 nanoseconds (depends on memory hierarchy)

**normal_writethrough Runtime Guarantees:**

- **Write Visibility to Other Cores**: 
  - Writes are immediately written to both cache and main memory
  - Other cores see writes immediately (no cache coherency delay)
  - **Visibility Latency**:
    - **Memory write latency**: 50-200 nanoseconds (typical)
      - Same NUMA node: 50-150 nanoseconds
      - Cross-NUMA node: 150-300 nanoseconds
    - **Hardware-specific variations**:
      - **Apple Silicon (M-series)**: 30-100 nanoseconds (unified memory architecture)
      - **Generic AArch64**: 50-200 nanoseconds (standard DDR memory)
      - **High-end servers**: 100-300 nanoseconds (larger memory hierarchies)
  
- **Cache Coherency**: 
  - Participates in cache coherency protocols
  - Cache always contains valid data (no Modified state)
  - Cache lines are always in Shared or Exclusive state
  - **Coherency overhead**: Minimal (no Modified state transitions)
  
- **DMA Visibility**: 
  - DMA devices see writes immediately (writes go directly to memory)
  - No explicit cache flushing required
  - **DMA visibility latency**: Same as memory write latency (50-200 nanoseconds)
  - Suitable for DMA buffers shared with network cards or other devices
  
- **Performance**: 
  - Slightly slower than write-back (every write goes to memory)
  - **Write latency**: 50-200 nanoseconds (memory access time)
  - **Read latency**: 1-5 nanoseconds (L1 cache hit), 5-15 nanoseconds (L2 cache hit)
  - Still benefits from read caching
  - Good balance between performance and visibility

**normal_noncacheable Runtime Guarantees:**

- **Write Visibility to Other Cores**: 
  - All accesses bypass cache entirely
  - Writes go directly to main memory
  - Immediate visibility to all cores and devices
  - **Visibility Latency**:
    - **Memory access latency**: 100-300 nanoseconds (typical)
      - Same NUMA node: 100-200 nanoseconds
      - Cross-NUMA node: 200-400 nanoseconds
    - **Hardware-specific variations**:
      - **Apple Silicon (M-series)**: 50-150 nanoseconds (unified memory, no NUMA)
      - **Generic AArch64**: 100-300 nanoseconds (standard DDR memory)
      - **High-end servers**: 150-400 nanoseconds (NUMA-aware systems)
  
- **Cache Coherency**: 
  - Does not participate in cache coherency (no caching)
  - All accesses are uncached memory operations
  - No cache state to maintain
  - **No coherency overhead**: Zero (no cache operations)
  
- **DMA Visibility**: 
  - DMA devices see all accesses immediately
  - No cache coherency concerns
  - **DMA visibility latency**: Same as memory access latency (100-300 nanoseconds)
  - Ideal for DMA buffers, memory-mapped I/O, shared memory
  
- **Performance**: 
  - Slowest option (no cache benefits)
  - **Access latency**: 100-300 nanoseconds (predictable, no cache misses)
  - **No cache hit benefits**: All accesses go to memory
  - Use only when cache is undesirable or DMA visibility is critical

**atomic Runtime Guarantees:**

- **Write Visibility to Other Cores**: 
  - Uses acquire/release semantics for cross-core visibility
  - Store-release (`STLR`) ensures prior writes are visible before the store
  - Load-acquire (`LDAR`) ensures subsequent reads see the store-release
  - **Visibility Latency**:
    - **Acquire/release operations**: 100-200 nanoseconds (typical)
      - Same NUMA node: 100-150 nanoseconds
      - Cross-NUMA node: 150-250 nanoseconds
      - Barrier overhead: ~5-20 nanoseconds (instruction semantics)
    - **Sequential consistency (seq_cst)**: 150-300 nanoseconds
      - Additional barrier overhead: ~10-50 nanoseconds (`DMB ISH`)
    - **Hardware-specific variations**:
      - **Apple Silicon (M-series)**: 50-150 nanoseconds (optimized atomic operations)
      - **Generic AArch64**: 100-200 nanoseconds (standard atomic instructions)
      - **High-end servers**: 150-300 nanoseconds (larger memory hierarchies)
  
- **Cache Coherency**: 
  - Participates in cache coherency protocols
  - Atomic operations use cache-coherent memory
  - Hardware ensures atomicity across cores
  - **Coherency latency**: 5-20 nanoseconds per atomic operation
  
- **DMA Visibility**: 
  - Atomic operations are cache-coherent
  - DMA devices see atomic operations after cache coherency completes
  - **DMA visibility latency**: 150-300 nanoseconds (cache coherency + memory access)
  - May require explicit cache flushing for DMA visibility
  
- **Performance**: 
  - Similar to `normal` for non-atomic operations
  - **Atomic operation overhead**:
    - **LDAR/STLR**: Minimal overhead (~5-10 nanoseconds vs. normal load/store)
    - **LDXR/STXR loops**: 50-200 nanoseconds (depends on contention)
      - Low contention: 50-100 nanoseconds
      - High contention: 100-500 nanoseconds (retry loops)
  - Optimal for lock-free data structures

**device Runtime Guarantees:**

- **Write Visibility**: 
  - Device memory is not cacheable
  - Writes go directly to device registers
  - Visibility is device-dependent (some devices have write buffers)
  
- **Cache Coherency**: 
  - Does not participate in cache coherency
  - Device memory is marked as Device type (non-cacheable, non-bufferable)
  
- **DMA Visibility**: 
  - Device memory is directly accessible to devices
  - No cache coherency concerns
  
- **Performance**: 
  - Device-dependent latency (typically slower than normal memory)
  - Reserved for device driver library use

**Summary Table:**

*Latency values are typical ranges under normal conditions. Actual values may vary.*

| Memory Space | Write Visibility Latency (typical) | Cache Coherency | DMA Visibility Latency (typical) | Performance |
|--------------|-----------------------------------|-----------------|----------------------------------|-------------|
| `normal` | 10-100ns (cache-to-cache) | Yes (MESI/MOESI) | 100-500ns (with flush) | Best |
| `normal_writethrough` | 50-200ns (memory write) | Yes (Shared/Exclusive) | 50-200ns (immediate) | Good |
| `normal_noncacheable` | 100-300ns (memory access) | N/A (no cache) | 100-300ns (immediate) | Slowest |
| `atomic` | 100-200ns (acquire/release) | Yes (MESI/MOESI) | 150-300ns (coherency) | Good |
| `device` | Device-dependent | N/A | Direct | Device-dependent |

**Detailed Latency Specification Notes:**

The latency ranges specified throughout this section represent **typical/average latencies** under normal system load conditions. The following notes provide additional context:

- **Typical Latency**: The ranges shown (e.g., 10-100ns) represent typical latencies observed under normal operating conditions
- **Best-Case Scenarios**: Actual latencies may be faster than the lower bound in optimal conditions:
  - Cache hits may be faster (1-5ns for L1 cache)
  - Low system load may reduce contention
  - Optimal memory alignment may improve performance
- **Worst-Case Scenarios**: Actual latencies may be slower than the upper bound under adverse conditions:
  - Cache pressure may increase cache miss rates
  - High system load may increase contention
  - NUMA distance may increase cross-node latency
  - Memory controller congestion may delay operations

**Hardware-Specific Latency Notes:**

- **Apple Silicon (M-series)**: Generally 2-3x faster than generic AArch64 due to unified memory architecture and optimized cache hierarchy. Typical latencies are often at the lower end of ranges.
- **Generic AArch64**: Standard latency ranges apply (typical server/workstation hardware). Latencies are typically in the middle of the specified ranges.
- **High-end servers**: May have higher latency due to larger NUMA hierarchies and memory subsystems. Latencies may approach the upper bounds, especially for cross-NUMA operations.
- **Latency Variability**: Actual latency depends on:
  - System load and contention
  - Memory controller state
  - Cache state and pressure
  - NUMA topology and distance
  - Hardware-specific optimizations

#### 12.1.1.3 Write-Back vs Write-Through Performance Implications

Understanding the performance trade-offs between write-back (`normal`) and write-through (`normal_writethrough`) memory spaces is crucial for optimizing Silica programs.

**Performance Characteristics Comparison:**

**Write-Back (`normal`) Performance:**

- **Write Performance**: 
  - **Cache Hit**: Writes complete in 1-5 nanoseconds (L1 cache write)
  - **Cache Miss**: Writes trigger cache line allocation, adding 50-300 nanoseconds overhead
  - **Write Buffering**: Multiple writes to same cache line are buffered, reducing memory traffic
  - **Best Case**: Sequential writes to same cache line achieve near-L1 cache performance
  
- **Read Performance**:
  - **Cache Hit**: Reads complete in 1-5 nanoseconds (L1), 5-15 nanoseconds (L2), 15-50 nanoseconds (L3)
  - **Cache Miss**: Reads trigger cache line fetch, adding 50-300 nanoseconds overhead
  - **Prefetching**: Hardware prefetching improves sequential read performance
  
- **Memory Traffic**:
  - **Reduced Traffic**: Write-back minimizes memory writes by batching writes in cache lines
  - **Write Coalescing**: Multiple writes to same cache line are coalesced into single memory write
  - **Bandwidth Efficiency**: Write-back uses memory bandwidth more efficiently than write-through
  
- **Cache Coherency Overhead**:
  - **Modified State**: Cache lines in Modified state require invalidation before other cores can read
  - **Coherency Protocol**: MESI/MOESI protocol adds 5-20 nanoseconds overhead per cache line transition
  - **False Sharing**: Multiple cores writing to same cache line cause frequent coherency traffic

**Write-Through (`normal_writethrough`) Performance:**

- **Write Performance**:
  - **Immediate Memory Write**: Every write goes to memory, adding 50-200 nanoseconds latency
  - **No Write Buffering**: Writes cannot be buffered or coalesced (each write goes to memory)
  - **Predictable Latency**: Write latency is predictable (no cache miss variability)
  - **Worst Case**: Every write pays full memory access cost
  
- **Read Performance**:
  - **Cache Hit**: Reads benefit from caching (same as write-back: 1-50 nanoseconds)
  - **Cache Miss**: Reads trigger cache line fetch (same as write-back: 50-300 nanoseconds)
  - **Read Caching**: Read performance is identical to write-back (both use read caching)
  
- **Memory Traffic**:
  - **Increased Traffic**: Write-through doubles memory write traffic compared to write-back
  - **No Write Coalescing**: Each write generates separate memory transaction
  - **Bandwidth Consumption**: Write-through consumes more memory bandwidth
  
- **Cache Coherency Overhead**:
  - **Shared/Exclusive State**: Cache lines are always in Shared or Exclusive state (no Modified state)
  - **Reduced Coherency**: No Modified-to-Shared transitions, reducing coherency overhead
  - **Immediate Visibility**: Writes are immediately visible, reducing coherency protocol complexity

**Performance Trade-Offs:**

**When to Use Write-Back (`normal`):**

- **High Write Frequency**: Applications with frequent writes benefit from write-back caching
- **Sequential Writes**: Sequential writes to same cache line achieve best performance with write-back
- **Memory Bandwidth Constraints**: Write-back reduces memory bandwidth consumption
- **General-Purpose Data**: Most application data benefits from write-back performance
- **Actor State**: Actor state and local data structures benefit from write-back caching

**When to Use Write-Through (`normal_writethrough`):**

- **DMA Visibility Requirements**: When DMA devices must see writes immediately
- **Cross-Core Visibility**: When other cores must see writes immediately (reduced coherency overhead)
- **Predictable Latency**: When write latency predictability is more important than peak performance
- **Shared Buffers**: Buffers shared between CPU and DMA devices
- **Debugging**: When immediate memory visibility aids debugging (writes visible in memory immediately)

**Performance Impact Scenarios:**

**Scenario 1: High Write Frequency**
- **Write-Back**: Excellent performance - writes are buffered in cache, memory traffic minimized
- **Write-Through**: Poor performance - every write pays full memory access cost
- **Recommendation**: Use write-back for high write frequency scenarios

**Scenario 2: DMA Buffer Sharing**
- **Write-Back**: Requires explicit cache flushing, adding 100-500 nanoseconds overhead per flush
- **Write-Through**: Immediate DMA visibility, no flushing required
- **Recommendation**: Use write-through for DMA buffers

**Scenario 3: Cross-Core Communication**
- **Write-Back**: Cache coherency protocol ensures visibility, but may have 10-100 nanoseconds delay
- **Write-Through**: Immediate visibility, but higher write latency (50-200 nanoseconds)
- **Recommendation**: Use write-back for general cross-core communication, write-through only if immediate visibility is critical

**Scenario 4: Read-Heavy Workloads**
- **Write-Back**: Excellent read performance (same as write-through)
- **Write-Through**: Excellent read performance (same as write-back)
- **Recommendation**: Both perform equally well for read-heavy workloads

**Scenario 5: Write-Heavy Workloads**
- **Write-Back**: Excellent performance - writes are buffered and coalesced
- **Write-Through**: Poor performance - every write pays full memory cost
- **Recommendation**: Use write-back for write-heavy workloads

**Cache Coherency Implications:**

**Write-Back Cache Coherency:**
- **Modified State Transitions**: Cache lines transition through Modified state, requiring invalidation
- **Coherency Traffic**: Modified-to-Shared transitions generate coherency protocol messages
- **False Sharing Impact**: False sharing causes frequent coherency traffic with write-back
- **Coherency Overhead**: 5-20 nanoseconds per cache line state transition

**Write-Through Cache Coherency:**
- **No Modified State**: Cache lines never enter Modified state (always Shared or Exclusive)
- **Reduced Coherency Traffic**: No Modified-to-Shared transitions, reducing coherency messages
- **False Sharing Impact**: False sharing has less impact (no Modified state transitions)
- **Coherency Overhead**: Minimal (no Modified state management)

**Memory Bandwidth Considerations:**

**Write-Back Bandwidth Usage:**
- **Efficient Bandwidth Usage**: Write-back uses memory bandwidth efficiently
- **Write Coalescing**: Multiple writes to same cache line coalesce into single memory write
- **Bandwidth Savings**: 50-90% bandwidth savings compared to write-through for write-heavy workloads

**Write-Through Bandwidth Usage:**
- **Higher Bandwidth Usage**: Write-through consumes more memory bandwidth
- **No Write Coalescing**: Each write generates separate memory transaction
- **Bandwidth Impact**: 2x bandwidth consumption compared to write-back for write-heavy workloads

**Cross-References:**
- See Section 12.1.1.1 (AArch64 Memory Attribute Mapping) for memory attribute configuration
- See Section 12.1.1.2 (Memory Space Runtime Guarantees) for detailed runtime guarantees
- See Section 18.2.3 (AArch64 Memory Model Mapping) for memory ordering implications
- See Section 15.1.2.2 (Actor Migration Overhead and Behavior) for cache performance during migration

**Performance Guarantees:**

The specification provides latency ranges as **guidelines** rather than strict guarantees:
- Applications should not rely on exact latency values
- Performance-critical code should benchmark on target hardware
- Latency ranges are intended to help developers choose appropriate memory spaces

#### 12.1.2 Region Lifetime and Allocation Strategy

**Allocation Strategy:** Regions are heap-allocated with scope-tied deallocation. When a sequence block exits, all regions allocated within that scope are deallocated (unless returned). This enables returning regions and references when the lifetime analysis permits.

**Region Lifetime:** Regions exist until scope exit or process termination:

```
sequence proc[mem(normal)]
    L1: lifetime <- fresh_lifetime();
    r: region(L1, normal) <- alloc_region(normal)    // region created (heap)
    ref1: ref(L1, normal, int64) <- alloc_ref(r, 42)
    // implicit deallocation when sequence exits (unless r is returned)
produces pure () end
```

**Region Return Lifetime Extension:** A region is *not* deallocated when a region reference is returned. When a sequence, block, or function returns a value of type `region(L, Space)`, the region's lifetime extends to the caller's scope. The region remains allocated as long as the returned value is in use.

#### 12.1.3 Region Isolation
Regions provide memory isolation:

```
L1: lifetime <- fresh_lifetime();
L2: lifetime <- fresh_lifetime();
r1: region(L1, normal) <- alloc_region(normal)
r2: region(L2, normal) <- alloc_region(normal)
// r1 and r2 are separate memory pools
// no aliasing between different regions
```

#### 12.1.4 Static Region Lifetime Analysis
The compiler performs static analysis to verify region lifetimes and ensure memory safety:

**Lifetime Tracking:**
- Regions are tracked through their lexical scope (sequence blocks)
- Lifetime identifiers are obtained via `fresh_lifetime()`; functions may take `L: lifetime` as a parameter for polymorphism
- Each distinct region allocation must use a distinct lifetime; duplicate lifetimes across composable scopes are a compile error
- References cannot outlive their containing region
- Region deallocation is verified to occur after all references are no longer accessible

**Formal Lifetime Analysis Rules:**

The lifetime analysis uses a formal judgment system to track region lifetimes and reference scope dependencies.

**Lifetime Environment:**
```
L ::= ∅ | L, Lᵢ:scope
```
A lifetime environment `L` maps lifetime identifiers `Lᵢ` to their lexical scopes.

**Scope Dependency Set (ScopDep):**
```
ScopDep ::= ∅ | ScopDep, ref(Lᵢ, Space, T):scope
```
The scope dependency set ScopDep tracks all references and their creation scopes.

**Lifetime Analysis Judgment:**
```
Γ; L; ScopDep ⊢ e : T; L'; ScopDep'
```
This judgment states that expression `e` has type `T` in type environment `Γ`, lifetime environment `L`, and scope dependency set `ScopDep`, producing updated environments `L'` and `ScopDep'`.

**Region Allocation Rule:**
```
Γ; L; ScopDep ⊢ x: region(Lᵢ, Space) <- alloc_region(Space) : proc[mem(Space)] region(Lᵢ, Space); L, Lᵢ:scope_current; ScopDep
```
When a region is allocated, the lifetime identifier Lᵢ is taken from the explicit type annotation on the binding (e.g. `region(L1, Space)`). Lᵢ is added to the lifetime environment with the current scope. The same identifier must not be used for a different allocation in an overlapping or composable scope.

**Reference Allocation Rule:**
```
Γ; L, Lᵢ:scope_r; ScopDep ⊢ alloc_ref(r, v) : proc[mem(Space)] ref(Lᵢ, Space, T); L, Lᵢ:scope_r; ScopDep, ref(Lᵢ, Space, T):scope_current
```
When a reference is allocated in region `Lᵢ`, the reference is added to the scope dependency set. The region `Lᵢ` must exist in the lifetime environment.

**Reference Usage Rule:**
```
Γ; L, Lᵢ:scope_r; ScopDep, ref(Lᵢ, Space, T):scope_ref ⊢ read_ref(ref) : proc[mem(Space)] T; L, Lᵢ:scope_r; ScopDep, ref(Lᵢ, Space, T):scope_ref
```
A reference can only be used if:
1. The region `Lᵢ` exists in the lifetime environment
2. The reference exists in the scope dependency set
3. The current scope is within the region's scope: `scope_current ≤ scope_r`
4. The current scope is within the reference's creation scope: `scope_current ≤ scope_ref`

**Scope Exit Rule:**
```
Γ; L, Lᵢ:scope_r; ScopDep, ref(Lᵢ, Space, T):scope_ref ⊢ end_scope : unit; L'; ScopDep'
```
When exiting a scope:
1. All regions allocated in the current scope are removed, *except* those that are returned (escape to the caller): `L' = L \ {Lᵢ | scope_r = scope_current and Lᵢ ∉ returned_regions}`. Returned regions have their scope extended to the caller.
2. All references allocated in the current scope are removed: `ScopDep' = ScopDep \ {ref(Lᵢ, Space, T) | scope_ref = scope_current}`
3. Safety check: No references to deallocated regions remain: `∀ ref(Lᵢ, Space, T) ∈ ScopDep'. Lᵢ ∈ L'`

A region is considered *returned* when the scope's return value has type `region(Lᵢ, Space)` or contains such a type (e.g., in a tuple or record).

**Region Isolation Enforcement:** The type checker verifies L matching at creation (`alloc_ref`, `alloc_buf`) and at use (`read_ref`, `write_ref`, `buf_load`, `buf_store`). A `ref(L1, ...)` cannot be used with a `region(L2, ...)` when L1 ≠ L2.

**Cross-Function Analysis:**

**Function Parameter Lifetime Extension:**
```
fn f(L1: lifetime, r: region(L1, Space)) -> T proc[mem(Space)]
```
When a region is passed as a function parameter, its lifetime extends to at least the function's return scope.

**Function Return Lifetime Constraint:**
```
fn f(...) -> ref(Lᵢ, Space, T) proc[mem(Space)]
```
When a function returns a reference, the region `Lᵢ` must outlive the function call:
```
scope_return ≤ scope_r
```

**Region Return Lifetime Extension:**
```
fn f(...) -> region(Lᵢ, Space) proc[mem(Space)]
```
When a function or sequence returns a region reference, the region is *not* deallocated at scope exit. Its lifetime extends to the caller's scope. The region remains allocated as long as the returned value is in use.

**Function Call Lifetime Propagation:**
```
Γ; L; ScopDep ⊢ f(args) : T proc[E]; L'; ScopDep'
```
Function calls propagate lifetime constraints:
- Regions passed as arguments extend their lifetimes to the call site
- References returned from functions require their regions to outlive the call
- Regions returned from functions or sequences extend their lifetimes to the caller (they are not deallocated at scope exit)

**Analysis Algorithm:**

The lifetime analysis algorithm processes expressions and statements to compute updated lifetime environments `L'` and scope dependency sets `ScopDep'`:

**Algorithm: Compute Lifetime Environments**

```pseudocode
function analyze_lifetime(expr, Γ, L, ScopDep):
    // Initialize result environments
    L' = L
    ScopDep' = ScopDep
    
    case expr of:
        // Region allocation: Lᵢ from explicit type annotation on binding (no implicit)
        alloc_region(Space):
            Lᵢ = region_id_from_binding_type()  // From binding's type annotation, e.g. region(L1, Space)
            scope_current = current_scope()
            L' = L ∪ {Lᵢ:scope_current}
            ScopDep' = ScopDep
            return (region(Lᵢ, Space), L', ScopDep')
        
        // Reference allocation: add reference to ScopDep, verify region exists
        alloc_ref(r, v):
            // Analyze region parameter
            (τ_r, L_r, ScopDep_r) = analyze_lifetime(r, Γ, L, ScopDep)
            // Verify r is a region type
            if τ_r != region(Lᵢ, Space):
                error("Expected region type")
            // Verify region exists in lifetime environment
            if Lᵢ ∉ L_r:
                error("Region Lᵢ not in scope")
            // Analyze value
            (τ_v, L_v, ScopDep_v) = analyze_lifetime(v, Γ, L_r, ScopDep_r)
            // Add reference to scope dependency set
            scope_current = current_scope()
            ScopDep' = ScopDep_v ∪ {ref(Lᵢ, Space, τ_v):scope_current}
            L' = L_v
            return (ref(Lᵢ, Space, τ_v), L', ScopDep')
        
        // Reference read: verify reference and region exist
        read_ref(ref_expr):
            // Analyze reference expression
            (τ_ref, L_ref, ScopDep_ref) = analyze_lifetime(ref_expr, Γ, L, ScopDep)
            // Verify ref_expr is a reference type
            if τ_ref != ref(Lᵢ, Space, T):
                error("Expected reference type")
            // Verify region exists
            if Lᵢ ∉ L_ref:
                error("Region Lᵢ not in scope")
            // Verify reference exists in scope dependency set
            if ref(Lᵢ, Space, T) ∉ ScopDep_ref:
                error("Reference not in scope dependency set")
            // Verify scope constraints
            scope_current = current_scope()
            scope_r = L_ref[Lᵢ]
            scope_ref = ScopDep_ref[ref(Lᵢ, Space, T)]
            if scope_current > scope_r:
                error("Reference used after region deallocation")
            if scope_current > scope_ref:
                error("Reference used after creation scope")
            // Reference read doesn't modify environments
            return (T, L_ref, ScopDep_ref)
        
        // Reference write: verify reference and region exist
        write_ref(ref_expr, value):
            // Analyze reference (same as read_ref)
            (τ_ref, L_ref, ScopDep_ref) = analyze_lifetime(ref_expr, Γ, L, ScopDep)
            if τ_ref != ref(Lᵢ, Space, T):
                error("Expected reference type")
            if Lᵢ ∉ L_ref or ref(Lᵢ, Space, T) ∉ ScopDep_ref:
                error("Reference or region not in scope")
            // Verify scope constraints (same as read_ref)
            scope_current = current_scope()
            scope_r = L_ref[Lᵢ]
            scope_ref = ScopDep_ref[ref(Lᵢ, Space, T)]
            if scope_current > scope_r or scope_current > scope_ref:
                error("Reference used after deallocation")
            // Analyze value
            (τ_v, L_v, ScopDep_v) = analyze_lifetime(value, Γ, L_ref, ScopDep_ref)
            // Verify value type matches reference type
            if τ_v != T:
                error("Type mismatch in write_ref")
            return (atom, L_v, ScopDep_v)
        
        // Function call: propagate lifetime constraints
        function_call(f, args):
            // Analyze function type
            τ_f = lookup_function_type(f, Γ)
            // Analyze arguments and collect lifetime constraints
            L_args = L
            ScopDep_args = ScopDep
            for each arg in args:
                (τ_arg, L_arg, ScopDep_arg) = analyze_lifetime(arg, Γ, L_args, ScopDep_args)
                L_args = L_arg
                ScopDep_args = ScopDep_arg
            // Function call doesn't extend lifetimes (regions passed as args extend via parameter rules)
            return (return_type(τ_f), L_args, ScopDep_args)
        
        // Scope exit: remove scoped regions and references (except returned regions)
        end_scope(scope_id):
            scope_current = scope_id
            // Remove regions allocated in this scope; do NOT remove regions that are returned
            // (returned regions extend to caller scope — see Region Return Lifetime Extension)
            L' = {Lᵢ:scope_r | Lᵢ:scope_r ∈ L and (scope_r != scope_current or Lᵢ ∈ returned_regions)}
            // returned_regions = lifetime IDs in the scope's return value type
            // Remove references allocated in this scope
            ScopDep' = {ref(Lᵢ, Space, T):scope_ref | ref(Lᵢ, Space, T):scope_ref ∈ ScopDep and scope_ref != scope_current}
            // Safety check: verify no references to deallocated regions
            for each ref(Lᵢ, Space, T) in ScopDep':
                if Lᵢ ∉ L':
                    error("Reference to deallocated region")
            return (atom, L', ScopDep')
        
        // Do expression: sequential composition
        do_expr(stmts):
            L_current = L
            ScopDep_current = ScopDep
            for each stmt in stmts:
                (τ_stmt, L_stmt, ScopDep_stmt) = analyze_lifetime(stmt, Γ, L_current, ScopDep_current)
                L_current = L_stmt
                ScopDep_current = ScopDep_stmt
            return (return_type(last_stmt), L_current, ScopDep_current)
        
        // Other expressions: no lifetime effects
        default:
            return (type_of(expr), L, ScopDep)
    end case
end function
```

**Detailed Algorithm Steps:**

1. **Region Creation Points**: Identify all `alloc_region()` calls and their lexical scopes
   - For each `alloc_region(Space)` at scope `s`, add `Lᵢ:s` to `L`
   - Algorithm: `L' = L ∪ {Lᵢ:current_scope()}`

2. **Reference Dependencies**: Track all `alloc_ref()` calls and their region parameters
   - For each `alloc_ref(r, v)` at scope `s`, verify `r` exists in `L` and add `ref(Lᵢ, Space, T):s` to `ScopDep`
   - Algorithm: Verify `Lᵢ ∈ L`, then `ScopDep' = ScopDep ∪ {ref(Lᵢ, Space, T):current_scope()}`

3. **Lifetime Bounds**: Compute the minimum lifetime for each region based on reference usage
   - For each reference `ref(Lᵢ, Space, T)` in `ScopDep`, ensure `scope_ref ≤ scope_r` where `Lᵢ:scope_r ∈ L`
   - Algorithm: At each reference use, check `current_scope() ≤ L[Lᵢ]` and `current_scope() ≤ ScopDep[ref(Lᵢ, Space, T)]`

4. **Safety Verification**: Ensure no references are used after region deallocation
   - At each scope exit, verify: `∀ ref(Lᵢ, Space, T) ∈ ScopDep'. Lᵢ ∈ L'`
   - At each reference use, verify: `scope_current ≤ scope_r` and `scope_current ≤ scope_ref`
   - Algorithm: Check constraints before allowing reference operations

**Environment Update Rules:**

The algorithm computes `L'` and `ScopDep'` as follows:

- **L' (Updated Lifetime Environment)**:
  - **Region Allocation**: `L' = L ∪ {Lᵢ:current_scope()}`
  - **Scope Exit**: `L' = {Lᵢ:scope_r | Lᵢ:scope_r ∈ L and scope_r != exited_scope}` for regions not returned; returned regions extend to caller scope and are not removed
  - **Other Operations**: `L' = L` (no change)

- **ScopDep' (Updated Scope Dependency Set)**:
  - **Reference Allocation**: `ScopDep' = ScopDep ∪ {ref(Lᵢ, Space, T):current_scope()}`
  - **Scope Exit**: `ScopDep' = {ref(Lᵢ, Space, T):scope_ref | ref(Lᵢ, Space, T):scope_ref ∈ ScopDep and scope_ref != exited_scope}`
  - **Reference Use**: `ScopDep' = ScopDep` (no change, but verify constraints)
  - **Other Operations**: `ScopDep' = ScopDep` (no change)

**Example (region not returned):**
```silica
fn example() -> int64 proc[mem(normal)] {
    sequence proc[mem(normal)]
        L1: lifetime <- fresh_lifetime();
        r: region(L1, normal) <- alloc_region(normal);  // L = {L1:scope_seq}
        ref1: ref(L1, normal, int64) <- alloc_ref(r, 42);  // ScopDep = {ref1:scope_seq}
        // ref1 is valid here: scope_current = scope_seq, scope_r = scope_seq, scope_ref = scope_seq
        value: int64 <- read_ref(ref1);  // ✓ Valid: scope_seq ≤ scope_seq
        // Region r and ref1 are deallocated at end of scope (r not returned)
    produces pure value end  // L' = ∅, ScopDep' = ∅, safety check: ✓ No references remain
    // ERROR: Cannot use ref1 here - region r is deallocated
    // read_ref(ref1) would fail: L1 ∉ L'
}
```

**Example (region returned — lifetime extends):**
```silica
fn alloc_and_return() -> region(L1, normal) proc[mem(normal)] {
    sequence proc[mem(normal)]
        L1: lifetime <- fresh_lifetime();
        r: region(L1, normal) <- alloc_region(normal);
    produces
        pure r   // r is returned: region is NOT deallocated, lifetime extends to caller
    end
}
```

**Example (multiple regions — fresh_lifetime for distinct identifiers):**
```silica
// Correct: use fresh_lifetime() for each region when composing
sequence proc[mem(normal)]
    L1: lifetime <- fresh_lifetime();
    L2: lifetime <- fresh_lifetime();
    x: region(L1, normal) <- alloc_region(normal);
    y: region(L2, normal) <- alloc_region(normal);
    // x and y have distinct types region(L1, normal) and region(L2, normal)
produces pure () end

// Error: using same L for both would cause confusion
// L1: lifetime <- fresh_lifetime();
// x: region(L1, normal) <- alloc_region(normal);
// y: region(L1, normal) <- alloc_region(normal);  // ERROR: duplicate L1 for different regions
```

**Recursive Data Structures:**

For recursive data structures containing references, lifetime analysis tracks references through the structure:

```
struct Node {
    value: int64,
    next: OptionRefNode
}

type OptionRefNode = Some(ref(L, normal, Node)) | None
```

When analyzing recursive structures:
- References within the structure must satisfy: `scope_ref ≤ scope_r`
- The structure's lifetime is bounded by the minimum lifetime of all contained references

**Cross-Function Analysis:**
The compiler analyzes region lifetimes across function boundaries:
- Function parameters containing regions extend the region lifetime
- Return values containing references must ensure the region outlives the return
- Return values containing regions extend the region lifetime to the caller (regions are not deallocated when returned)
- Function calls propagate region lifetime constraints

#### 12.1.5 Region Handles and Actor Spawn

When a region handle is passed to an actor via `spawn(initial_state, behavior_fn)`, the handle is moved (see §4.4.2). The actor may outlive the spawner, so the spawner must receive the handle back. `spawn` obeys the same rule as any function: the handle is moved in and must be returned.

**Region Copy and Return Semantics:** When `spawn` is invoked with an `initial_state` that contains a region handle:

1. **Handle is moved**: Ownership of the handle transfers to `spawn`.
2. **Copy the region**: Allocate a new region and copy its contents (all allocated cells and their values) into the new region.
3. **Actor receives handle to the copy**: The actor's `initial_state` receives a handle to the newly allocated copy.
4. **Spawn returns the original handle**: `spawn` returns the original handle to the spawner (along with the `actor_ref`). The spawner regains ownership.

This ensures that both the spawner and the actor have valid handles to distinct regions. For spawning multiple actors with the same region, the spawner passes the handle to each `spawn` call and receives it back each time; each actor receives its own handle to its own copy.

**Trade-offs:**
- **Correctness**: No dangling handles; each owner has a distinct region.
- **Cost**: Each spawn performs a region copy (allocation plus copying all contents).
- **Semantics**: Regions diverge after spawn; changes in one do not affect the others.
- **Use case**: Use when each actor should have its own independent copy of the data.

See §15.1.1 (Actor Creation) for the `spawn` function signature.

### 12.2 Reference Semantics

#### 12.2.1 Reference Creation
References are allocated within regions:

```
alloc_ref(region, initial_value) -> ref(R, Space, T) proc[mem(Space)]
```

#### 12.2.2 Reference Operations
References support reading and writing:

```
read_ref(reference) -> T proc[mem(Space)]
write_ref(reference, value) -> atom proc[mem(Space)]
```

#### 12.2.3 Reference Identity
References are identity-based:

```
r1: ref(R, normal, int) <- alloc_ref(region, 42)
r2: ref(R, normal, int) <- alloc_ref(region, 42)
r1 ≠ r2    // different references, even with same value
```

### 12.3 Buffer Semantics

#### 12.3.1 Buffer Types
Buffers represent contiguous memory arrays:

```
buf(R, Space, T, N)    // buffer of N elements of type T
```

#### 12.3.2 Buffer Operations
Buffers support indexed access:

```
read_buf(buffer, index) -> T proc[mem(Space)]
write_buf(buffer, index, value) -> atom proc[mem(Space)]
```

#### 12.3.3 Bounds Checking
Buffer access is bounds-checked. The compiler emits bounds checks before every `buf_load` and `buf_store`:

- **MTE (Memory Tagging Extensions)**: When MTE hardware is available (see §21.3), the implementation may use hardware-assisted bounds checking.
- **Software fallback**: When MTE is not available, the compiler emits a comparison and conditional branch before each load/store: compare index against buffer size N (from `buf(L, Space, T, N)`), trap on out-of-bounds.

```
buf: buf(L1, normal, int, 10) <- alloc_buf(region, 10)  // buffer of size 10
x: int <- read_buf(buf, 5)           // ✓ valid index
y: int <- read_buf(buf, 15)          // ✗ runtime bounds error (traps)
```

### 12.4 Memory Safety Guarantees

#### 12.4.1 Region Isolation
No cross-region references. The type checker enforces region isolation at two points:

1. **At creation**: When type-checking `alloc_ref(region_expr, value)` or `alloc_buf(region_expr, n)`, the region expression's lifetime L must match the result type's L.
2. **At use**: When type-checking `read_ref`, `write_ref`, `buf_load`, or `buf_store`, the ref/buf's L must be in the lifetime environment and must match the region it was allocated in.

```
L1: lifetime <- fresh_lifetime();
L2: lifetime <- fresh_lifetime();
r1: region(L1, normal) <- alloc_region(normal)
r2: region(L2, normal) <- alloc_region(normal)
ref1: ref(L1, normal, int) <- alloc_ref(r1, 42)

// Cannot create reference in r2 pointing to r1's memory
// Type system prevents: ref(L2, normal, ref(L1, normal, int))
// alloc_ref(r2, ref1) would fail: region has L2, ref has L1
```

#### 12.4.2 Lifetime Safety
References cannot outlive their regions:

```
sequence proc[mem(normal)]
    L1: lifetime <- fresh_lifetime();
    r: region(L1, normal) <- alloc_region(normal)
    ref: ref(L1, normal, int) <- alloc_ref(r, 42)
    // ref is valid here
produces pure () end
// r deallocated here
// ref is now invalid (use would be memory error)
```

#### 12.4.3 Type Safety
Memory operations preserve types:

```
ref_int: ref(L1, normal, int) <- alloc_ref(r, 42)
ref_str: ref(L1, normal, string) <- alloc_ref(r, "hello")

x: int <- read_ref(ref_int)    // x : int
y: string <- read_ref(ref_str)    // y : string
```

## 13. Operational Semantics

### 13.1 Evaluation Judgment

The evaluation judgment has the form:

```
ρ; σ; κ ⊢ e ⇓ v; σ'; κ'

Where:
- ρ is the environment (variable bindings)
- σ is the memory state (regions, references, buffers)
- κ is the capability context (active effects)
- e is the expression to evaluate
- v is the result value
- σ' is the updated memory state
- κ' is the updated capability context
```

### 13.2 Expression Evaluation Rules

#### 13.2.1 Literal Evaluation
```
ρ; σ; κ ⊢ n ⇓ n; σ; κ          (integer literal)
ρ; σ; κ ⊢ true ⇓ true; σ; κ     (boolean literal)
ρ; σ; κ ⊢ 'c' ⇓ 'c'; σ; κ       (character literal)
ρ; σ; κ ⊢ "s" ⇓ "s"; σ; κ       (string literal)
ρ; σ; κ ⊢ () ⇓ (); σ; κ         (unit literal)
```

#### 13.2.2 Variable Lookup
```
ρ(x) = v
─────────────────────────────
ρ; σ; κ ⊢ x ⇓ v; σ; κ
```

#### 13.2.3 Arithmetic Operations
```
ρ; σ; κ ⊢ e₁ ⇓ n₁; σ₁; κ₁
ρ; σ₁; κ₁ ⊢ e₂ ⇓ n₂; σ₂; κ₂
─────────────────────────────
ρ; σ; κ ⊢ e₁ + e₂ ⇓ n₁ + n₂; σ₂; κ₂
```

Similar rules for `-`, `*`, `/`, `%`.

#### 13.2.4 Function Application
```
ρ; σ; κ ⊢ f ⇓ <λx.e, ρ'>; σ₁; κ₁
ρ; σ₁; κ₁ ⊢ e_arg ⇓ v_arg; σ₂; κ₂
ρ'[x → v_arg]; σ₂; κ₂ ⊢ e ⇓ v_result; σ₃; κ₃
────────────────────────────────────────────────
ρ; σ; κ ⊢ f(e_arg) ⇓ v_result; σ₃; κ₃
```

#### 13.2.5 Process Creation
```
─────────────────────────────
ρ; σ; κ ⊢ proc { e } ⇓ <proc e ρ>; σ; κ
```

#### 13.2.6 Process Creation (Binding)
Binding creates a process value but does not execute it:

```
────────────────────────────────────────────
ρ; σ; κ ⊢ x: proc[ε] τ <- e_proc ⇓ <proc e_proc ρ>; σ; κ
```

#### 13.2.7 Process Execution (Spawn)
Processes are executed using the `spawn()` function:

```
ρ; σ; κ ⊢ e_proc ⇓ <proc e_body ρ_proc>; σ₁; κ₁
ρ_proc; σ₁; κ₁ ⊢ e_body ⇓ v_result; σ₂; κ₂
────────────────────────────────────────────
ρ; σ; κ ⊢ x: τ <- spawn(e_proc) ⇓ v_result; σ₂; κ₂
```

### 13.3 Memory Operation Semantics

#### 13.3.1 Region Allocation
```
κ contains mem(Space)
σ' = σ[r ↦ new_region(Space)]
─────────────────────────────
ρ; σ; κ ⊢ alloc_region(Space) ⇓ r; σ'; κ
```

#### 13.3.2 Reference Allocation
```
κ contains mem(Space)
σ(region) = region_state
σ' = σ[region ↦ region_state[ref ↦ initial_value]]
─────────────────────────────
ρ; σ; κ ⊢ alloc_ref(region, initial_value) ⇓ ref; σ'; κ
```

#### 13.3.3 Reference Read
```
κ contains mem(Space)
σ(region)(ref) = value
─────────────────────────────
ρ; σ; κ ⊢ read_ref(ref) ⇓ value; σ; κ
```

#### 13.3.4 Reference Write
```
κ contains mem(Space)
region_state' = σ(region)[ref → new_value]
σ' = σ[region ↦ region_state']
─────────────────────────────
ρ; σ; κ ⊢ write_ref(ref, new_value) ⇓ (); σ'; κ
```

### 13.4 Control Flow Semantics

#### 13.4.1 Case Expression
```
ρ; σ; κ ⊢ e_scrut ⇓ v; σ₁; κ₁
pattern_match(v, p₁) = bindings₁
ρ₁ = ρ ∪ bindings₁
ρ₁; σ₁; κ₁ ⊢ e₁ ⇓ v_result; σ₂; κ₂
─────────────────────────────────
ρ; σ; κ ⊢ case e_scrut of { p₁ -> e₁; ... } ⇓ v_result; σ₂; κ₂
```

### 13.5 Sequence Expression Semantics

The `sequence ... produces pure ... end` expression creates a process value through monadic binding:

```
sequence [proc[ε]]
    x <- e1    // creates process e1, binds result to x
    y <- e2    // creates process e2, binds result to y
produces
    pure result    // value the block returns (must be effect-free)
end
```

This creates a process value that must be executed using `spawn()` to actually run the computations.

**Note**: The `produces pure` line declares the block's return value. The expression after `pure` must be effect-free; all effects must appear in the steps above `produces`.

## 14. Safety Properties

### 14.1 Memory Safety

#### 14.1.1 No Null Pointers
All references are guaranteed to be valid:

- References are created by explicit allocation
- No implicit null values
- Type system prevents uninitialized references

#### 14.1.2 No Dangling Pointers
Reference lifetimes are bounded by region lifetimes:

```
{
    r: region(R, normal) <- alloc_region(normal)
    ref: ref(R, normal, int) <- alloc_ref(r, 42)
    // ref is valid here
}
// r and ref are deallocated together (when r is not returned)
```

When a region reference is returned, the region is *not* deallocated at scope exit; its lifetime extends to the caller.

#### 14.1.3 No Use-After-Free
Attempting to use a reference after its region is deallocated is a type error:

```
fn bad_lifetime() {
    r: region(R, normal) <- alloc_region(normal)
    ref: ref(R, normal, int) <- alloc_ref(r, 42)
    return ref    // ERROR: ref would outlive region r
}
```

### 14.2 Type Safety

#### 14.2.1 Type Preservation
Well-typed programs don't go wrong:

```
If ⊢ program : τ then either:
- program evaluates to value of type τ, or
- program encounters runtime effect violation
```

#### 14.2.2 Effect Safety
Effect violations are caught at runtime:

```
proc[] int = alloc_ref(r, 42)    // Type checks!
// But fails at runtime: missing mem(normal) capability
```

#### 14.2.3 Pattern Match Exhaustiveness
Case expressions must cover all possible values:

```
type OptionInt = Some(int64) | None

case opt of {
    Some(x: int64) -> x
    // Missing None case: compilation error
}
```

### 14.3 Concurrency Safety

#### 14.3.1 Actor Isolation
Actors have isolated state and communication:

- No shared mutable state between actors
- All communication through message passing
- Actor failures don't corrupt other actors

#### 14.3.2 Message Ordering
Messages maintain happens-before relationships:

```
send(actor1, msg1)
send(actor1, msg2)
// actor1 receives msg1 before msg2
```

#### 14.3.3 Atomicity Guarantees
Atomic operations provide strong guarantees:

```
atomic_compare_exchange(ref, expected, new)
// Either succeeds completely or fails completely
// No partial updates visible to other threads
```

### 14.4 Runtime Safety

#### 14.4.1 Bounds Checking
Array/buffer access is bounds-checked:

```
buf: buf(R, normal, int, 10) <- alloc_buf(r, 10)
x: int <- read_buf(buf, 15)    // Runtime error: index out of bounds
```

#### 14.4.2 Division by Zero
Integer division checks for zero divisor:

```
x / 0    // Runtime error: division by zero
```

#### 14.4.3 Effect Violations
Missing capabilities cause runtime errors:

```
// Without mem(normal) capability:
alloc_ref(r, 42)    // Runtime error: capability violation
```

## 15. Actor Model Semantics

### 15.1 Actor Lifecycle

#### 15.1.1 Actor Creation
Actors are created with initial state and behavior function:

```
spawn(initial_state, behavior_fn [, core_affinity]) -> actor_ref proc[concurrency]
```

When `initial_state` contains a region handle, `spawn` returns `(actor_ref, initial_state)` so the caller receives the moved handle back. The actor's state receives a handle to a copy of the region.

The `spawn()` function is the execution point for actor creation. It returns an `actor_ref` handle that can be used with other functions such as `cast()`, `send()`, and `pin_actor_to_core()`. The actor begins executing immediately when `spawn()` is called.

The behavior function has type: `(Msg, State) -> State`. The effects required by the behavior function are declared on sequence blocks inside it.

The `initial_state` parameter must implement the `ActorState` trait (for named types only). The `actor_ref` return type is a primitive type (like `int` or `boolean`), not parameterized by message type.

**Region handles in initial state:** When `initial_state` contains a region handle, the handle is moved into `spawn` and must be returned (see §4.4.2, §12.1.5). The runtime copies the region; the actor receives a handle to the copy. `spawn` returns the original handle to the spawner (along with the `actor_ref`). Each actor has its own handle to its own copy.

**Important**: `spawn()` creates and immediately starts executing the actor. The returned `actor_ref` is a handle to the running actor that can be used for message passing and control operations.

#### 15.1.2 Actor Execution Model
Each actor executes as an infinite loop in the runtime system:

```
actor_loop(state, behavior) {
    message <- recv()           // runtime receives message from mailbox
    new_state <- behavior(message, state)  // user behavior processes message
    actor_loop(new_state, behavior)        // continue with new state
}
```

**Important**: The `recv()` operation is performed by the actor runtime system outside of the user-defined behavior function. User-defined behavior functions only receive the message and current state as parameters - they never call `recv()` directly. The `recv()` function is not user-callable and is an internal runtime operation. The runtime calls `recv()` to retrieve messages from the actor's mailbox, then passes the received message to the behavior function along with the current state.

**Actor Migration Policy:**

Actor migration between cores is **manual-only** - there is no automatic migration. Actors remain on their initial core unless explicitly migrated by application code using the `migrate_actor()` function. The runtime does not perform automatic load balancing or thermal migration of actors.

**Migration Control:**

- **Default Behavior**: Actors remain on the core where they were spawned unless explicitly migrated
- **Manual Migration**: Application code must call `migrate_actor()` to move actors between cores
- **Core Affinity**: Actors can be pinned to specific cores using `pin_actor_to_core()`, preventing migration
- **Migration API**: `migrate_actor(actor_ref, target_core) -> atom` - explicitly migrates an actor to a target core proc[concurrency]

#### 15.1.2.1 AArch64 Runtime Integration

The actor runtime integrates with AArch64 hardware features to provide efficient scheduling, core affinity, and power management.

**Actor Scheduling on AArch64 Cores:**

The runtime schedules actors on AArch64 cores using the following mechanisms:

1. **Core Affinity Enforcement**: Actors with specified core affinity are pinned to their designated cores using `pthread_setaffinity_np()` or equivalent system calls
2. **NUMA Awareness**: When actors are spawned or migrated, the runtime schedules them on cores within the same NUMA node as their data regions to minimize cross-NUMA memory access latency
3. **Initial Core Assignment**: Actors without explicit affinity are assigned to available cores at spawn time, but remain on that core unless explicitly migrated
4. **Core Topology Detection**: The runtime queries AArch64 CPU topology via system registers and `/proc/cpuinfo`/`sysfs` to determine:
   - Core IDs and their physical/logical mapping
   - NUMA node assignments for each core
   - Cache hierarchy (L1/L2/L3 cache relationships)
   - Big.LITTLE core types (performance vs efficiency cores)

**System Register Access for Topology:**

Access to topology registers (`MPIDR_EL1`, `CLIDR_EL1`, `CCSIDR_EL1`) requires EL1 privilege. For detailed information about system register access patterns, privilege requirements, and fallback strategies, see Section 21.0.2 (AArch64 System Register Access).

**Topology Detection Timing:**

- **Initialization Timing**: Topology detection occurs during runtime initialization, before actor scheduling begins
- **Error Handling**: Failed topology detection results in fallback to default topology assumptions (single NUMA node, uniform cores)

**AArch64 Core Topology Registers:**

**MPIDR_EL1 (Multiprocessor Affinity Register):**

The `MPIDR_EL1` register provides core identification and affinity information:

```
MPIDR_EL1 Register Layout:
┌─────────────────────────────────────────────────────────────┐
│ 63:40 │ 39:32 │ 31:24 │ 23:16 │ 15:8  │ 7:0   │
│ Reserved│ Aff3 │ Aff2 │ Aff1 │ Aff0 │ U     │
└─────────────────────────────────────────────────────────────┘
```

**Key Fields:**

- **U (bit 0)**: Uniprocessor bit (0 = multiprocessor system)
- **Aff0 [7:0]**: Affinity level 0 (CPU ID within cluster)
- **Aff1 [15:8]**: Affinity level 1 (Cluster ID)
- **Aff2 [23:16]**: Affinity level 2 (NUMA node ID, if supported)
- **Aff3 [31:24]**: Affinity level 3 (Socket ID, if supported)

**Core ID Extraction:**

```pseudocode
// Read MPIDR_EL1 for current core
MRS MPIDR_EL1, MPIDR_EL1

// Extract core identification
CPU_ID = MPIDR_EL1[7:0]        // Affinity level 0 (CPU within cluster)
CLUSTER_ID = MPIDR_EL1[15:8]   // Affinity level 1 (cluster)
NUMA_NODE = MPIDR_EL1[23:16]   // Affinity level 2 (NUMA node)
SOCKET_ID = MPIDR_EL1[31:24]   // Affinity level 3 (socket)

// Construct logical core ID
LOGICAL_CORE_ID = (SOCKET_ID << 24) | (NUMA_NODE << 16) | (CLUSTER_ID << 8) | CPU_ID
```

**Cache ID Registers:**

**CLIDR_EL1 (Cache Level ID Register):**

The `CLIDR_EL1` register describes the cache hierarchy:

```
CLIDR_EL1 Register Layout:
┌─────────────────────────────────────────────────────────────┐
│ 63:30 │ 29:27 │ 26:24 │ 23:21 │ 20:18 │ 17:15 │ 14:12 │ 11:9 │ 8:6 │ 5:3 │ 2:0 │
│ Reserved│ LoC  │ LoUIS│ LoUU │ Ctype7│ Ctype6│ Ctype5│ Ctype4│ Ctype3│ Ctype2│ Ctype1│
└─────────────────────────────────────────────────────────────┘
```

**Key Fields:**

- **Ctype1 [2:0]**: Cache type for level 1 (0 = no cache, 1 = instruction only, 2 = data only, 3 = unified)
- **Ctype2 [5:3]**: Cache type for level 2
- **Ctype3 [8:6]**: Cache type for level 3
- **LoC [29:27]**: Level of Coherency (outermost cache level that maintains coherency)
- **LoUIS [26:24]**: Level of Unification Inner Shareable
- **LoUU [23:21]**: Level of Unification Uniprocessor

**CCSIDR_EL1 (Cache Size ID Register):**

The `CCSIDR_EL1` register provides cache size information for a specific cache level:

```
CCSIDR_EL1 Register Layout:
┌─────────────────────────────────────────────────────────────┐
│ 63:32 │ 31:29 │ 28:27 │ 26:13 │ 12:3 │ 2:0 │
│ Reserved│ WT   │ WB   │ NumSets│ Associativity│ LineSize│
└─────────────────────────────────────────────────────────────┘
```

**Key Fields:**

- **LineSize [2:0]**: Cache line size (log2(bytes) - 4, e.g., 4 = 64 bytes)
- **Associativity [12:3]**: Cache associativity (log2(ways) - 1)
- **NumSets [26:13]**: Number of sets (log2(sets) - 1)
- **WB [28:27]**: Write-Back support
- **WT [31:29]**: Write-Through support

**Cache Size Calculation:**

```pseudocode
// Select cache level (via CSSELR_EL1)
CSSELR_EL1[3:1] = cache_level  // Select cache level (1, 2, or 3)
IS = 0  // Instruction cache (0) or Data cache (1)
CSSELR_EL1[0] = IS
MSR CSSELR_EL1, CSSELR_EL1

// Read cache size information
MRS CCSIDR_EL1, CCSIDR_EL1

// Calculate cache parameters
LINE_SIZE = 2^(CCSIDR_EL1[2:0] + 4)  // Cache line size in bytes
ASSOCIATIVITY = 2^(CCSIDR_EL1[12:3] + 1)  // Number of ways
NUM_SETS = 2^(CCSIDR_EL1[26:13] + 1)  // Number of sets

// Calculate total cache size
CACHE_SIZE = LINE_SIZE * ASSOCIATIVITY * NUM_SETS
```

**Runtime Topology Detection Algorithm:**

```pseudocode
function detect_cpu_topology():
    topology = {}
    
    // Query all cores
    for each core in system:
        // Set core affinity
        set_cpu_affinity(core)
        
        // Read MPIDR_EL1
        MRS MPIDR_EL1, MPIDR_EL1
        cpu_id = MPIDR_EL1[7:0]
        cluster_id = MPIDR_EL1[15:8]
        numa_node = MPIDR_EL1[23:16]
        socket_id = MPIDR_EL1[31:24]
        
        // Read cache hierarchy
        MRS CLIDR_EL1, CLIDR_EL1
        l1_type = CLIDR_EL1[2:0]
        l2_type = CLIDR_EL1[5:3]
        l3_type = CLIDR_EL1[8:6]
        
        // Query L1 cache size
        CSSELR_EL1 = (1 << 1) | 0  // Level 1, Data cache
        MSR CSSELR_EL1, CSSELR_EL1
        MRS CCSIDR_EL1, CCSIDR_EL1
        l1_size = calculate_cache_size(CCSIDR_EL1)
        
        // Query L2 cache size
        CSSELR_EL1 = (2 << 1) | 0  // Level 2, Data cache
        MSR CSSELR_EL1, CSSELR_EL1
        MRS CCSIDR_EL1, CCSIDR_EL1
        l2_size = calculate_cache_size(CCSIDR_EL1)
        
        // Query L3 cache size (if present)
        if l3_type != 0:
            CSSELR_EL1 = (3 << 1) | 0  // Level 3, Data cache
            MSR CSSELR_EL1, CSSELR_EL1
            MRS CCSIDR_EL1, CCSIDR_EL1
            l3_size = calculate_cache_size(CCSIDR_EL1)
        end if
        
        // Store topology information
        topology[core] = {
            cpu_id: cpu_id,
            cluster_id: cluster_id,
            numa_node: numa_node,
            socket_id: socket_id,
            l1_cache_size: l1_size,
            l2_cache_size: l2_size,
            l3_cache_size: l3_size,
            l1_shared_with: find_shared_l1_cores(cpu_id),
            l2_shared_with: find_shared_l2_cores(cluster_id),
            l3_shared_with: find_shared_l3_cores(numa_node)
        }
    end for
    
    return topology
end function
```

**Big.LITTLE Core Detection:**

For systems with heterogeneous cores (performance vs. efficiency), the runtime detects core types:

```pseudocode
function detect_core_types():
    // Read MIDR_EL1 (Main ID Register) for each core
    for each core in system:
        set_cpu_affinity(core)
        MRS MIDR_EL1, MIDR_EL1
        
        // Extract implementer and part number
        IMPLEMENTER = MIDR_EL1[31:24]
        PART_NUM = MIDR_EL1[15:4]
        
        // Determine core type based on implementer/part number
        // (implementation-specific, e.g., ARM Cortex-A78 vs Cortex-A55)
        if is_performance_core(IMPLEMENTER, PART_NUM):
            core_type[core] = PERFORMANCE_CORE
        else:
            core_type[core] = EFFICIENCY_CORE
        end if
    end for
end function
```

**NUMA Topology Detection:**

NUMA node information is extracted from MPIDR_EL1 and cross-referenced with system information:

```pseudocode
function detect_numa_topology():
    // Read MPIDR_EL1 for NUMA node assignment
    for each core in system:
        set_cpu_affinity(core)
        MRS MPIDR_EL1, MPIDR_EL1
        numa_node = MPIDR_EL1[23:16]
        
        // Cross-reference with /proc/cpuinfo or sysfs
        // /sys/devices/system/node/node{N}/cpulist
        numa_nodes[numa_node].cores.append(core)
    end for
    
    // Query NUMA node memory information
    for each numa_node in numa_nodes:
        // Read from /sys/devices/system/node/node{N}/meminfo
        numa_nodes[numa_node].memory_size = read_numa_memory_size(numa_node)
    end for
end function
```

**Core Affinity Mechanisms:**

When an actor is spawned with core affinity:

```silica
// Pin actor to performance cores
actor_ref: actor_ref <- spawn(initial_state, behavior, performance_cores)

// Pin actor to specific core
actor_ref: actor_ref <- spawn(initial_state, behavior, core_id(0))

// Pin actor to core set
actor_ref: actor_ref <- spawn(initial_state, behavior, core_set([0, 1, 2]))
```

The runtime:
1. **Thread Creation**: Creates a pthread bound to the specified core(s)
2. **Affinity Setting**: Uses `pthread_setaffinity_np()` to set CPU affinity mask
3. **Migration Prevention**: Prevents OS scheduler from migrating the thread to other cores
4. **Core Validation**: Verifies that specified cores exist and are available

**NUMA-Aware Actor Placement:**

The runtime optimizes actor placement for NUMA architectures:

1. **NUMA Node Detection**: Queries NUMA topology via `numa_available()` and `numa_node_of_cpu()`
2. **Region-to-NUMA Mapping**: Tracks which NUMA node each region is allocated on
3. **Actor-to-Region Affinity**: Places actors on cores within the same NUMA node as their primary data regions
4. **Cross-NUMA Minimization**: Minimizes cross-NUMA memory accesses by co-locating actors with their data

**Interaction with AArch64 Power Management:**

The runtime interacts with AArch64 power management features to balance performance and energy efficiency. Power management affects actor scheduling, core selection, and runtime behavior.

**1. CPU Frequency Scaling (DVFS - Dynamic Voltage and Frequency Scaling):**

The runtime interacts with CPU frequency scaling:

**Behavior:**
- **Performance Cores**: Actors pinned to performance cores may trigger frequency scaling
  - High-priority actors → CPU frequency increases (up to maximum)
  - Low-priority actors → CPU frequency may decrease (power saving)
- **Efficiency Cores**: Frequency scaling is more aggressive on efficiency cores
  - Efficiency cores scale down more quickly during idle periods
  - Efficiency cores scale up when load increases

**Runtime Adaptation:**
```pseudocode
function schedule_actor_with_frequency_scaling(actor, core):
    // Pin actor to core
    pin_actor_to_core(actor, core)
    
    // Monitor actor priority and load
    if actor.priority == HIGH_PRIORITY:
        // Request maximum frequency for performance cores
        if is_performance_core(core):
            set_cpu_frequency(core, MAX_FREQUENCY)
        end if
    else if actor.priority == LOW_PRIORITY:
        // Allow frequency scaling down for power saving
        set_cpu_frequency(core, SCALABLE)
    end if
end function
```

**Frequency Scaling Impact:**
- **Performance**: Higher frequency → faster execution, higher power consumption
- **Latency**: Frequency changes take ~10-100 microseconds (transition time)
- **Actor Execution**: Actors continue executing during frequency transitions (no interruption)

**2. Core Parking and Unparking:**

Efficiency cores may be parked/unparked by the OS based on system load:

**Behavior:**
- **Parking**: OS parks efficiency cores when system load is low
  - Parked cores are powered down (no execution)
  - Actors on parked cores are migrated to active cores
- **Unparking**: OS unparks efficiency cores when system load increases
  - Unparked cores become available for actor scheduling
  - Runtime can schedule new actors on unparked cores

**Runtime Handling:**
```pseudocode
function handle_core_parking(core):
    if core_is_parked(core):
        // Find actors on parked core
        actors_on_core = get_actors_on_core(core)
        
        // Migrate actors to active cores
        for each actor in actors_on_core:
            // Find suitable active core
            target_core = find_active_core(actor.affinity)
            
            // Migrate actor
            migrate_actor(actor, target_core)
        end for
    end if
end function

function handle_core_unparking(core):
    if core_is_unparked(core):
        // Core is now available
        // Runtime can schedule new actors on this core
        available_cores.add(core)
        
        // Prefer unparked efficiency cores for low-priority actors
        schedule_low_priority_actors_on_efficiency_cores()
    end if
end function
```

**Core Parking Impact:**
- **Actor Migration**: Actors are migrated transparently (no actor code changes)
- **Latency**: Migration takes ~1-10 milliseconds (context switch overhead)
- **State Preservation**: Actor state is preserved during migration

**3. Thermal Management:**

The runtime respects thermal throttling and adapts actor scheduling:

**Behavior:**
- **Thermal Throttling**: Cores throttle when temperature exceeds limits
  - Throttled cores reduce frequency or pause execution
  - Actors on throttled cores experience performance degradation
- **Thermal Migration**: Runtime may migrate actors from hot cores to cooler cores
  - Prevents further temperature increase
  - Distributes heat load across cores

**Runtime Adaptation:**
```pseudocode
function monitor_thermal_state():
    for each core in system:
        temperature = get_core_temperature(core)
        
        if temperature > THROTTLE_THRESHOLD:
            // Core is throttling
            if temperature > MIGRATION_THRESHOLD:
                // Migrate actors to cooler cores
                migrate_actors_from_hot_core(core)
            else:
                // Reduce actor priority on hot core
                reduce_actor_priority_on_core(core)
            end if
        end if
    end for
end function

function migrate_actors_from_hot_core(hot_core):
    // Find cooler cores
    cool_cores = find_cooler_cores(hot_core)
    
    // Migrate high-priority actors first
    actors = get_actors_on_core(hot_core)
    sort_by_priority(actors)
    
    for each actor in actors:
        // Find suitable cool core
        target_core = find_suitable_cool_core(actor, cool_cores)
        
        // Migrate actor
        migrate_actor(actor, target_core)
    end for
end function
```

**Thermal Management Impact:**
- **Performance**: Throttled cores execute slower (frequency reduction)
- **Migration Overhead**: Actor migration adds latency (~1-10 milliseconds)
- **Heat Distribution**: Migration distributes heat load, preventing hotspots

**4. Energy Efficiency Optimization:**

The runtime optimizes for energy efficiency by placing actors on appropriate cores:

**Behavior:**
- **Low-Priority Actors**: Scheduled on efficiency cores
  - Efficiency cores consume less power
  - Lower performance but adequate for low-priority work
- **High-Priority Actors**: Scheduled on performance cores
  - Performance cores provide maximum performance
  - Higher power consumption but necessary for performance-critical work
- **Load Balancing**: Distributes load to minimize power consumption

**Runtime Strategy:**
```pseudocode
function optimize_energy_efficiency():
    // Classify actors by priority
    high_priority_actors = filter_actors_by_priority(HIGH)
    low_priority_actors = filter_actors_by_priority(LOW)
    
    // Schedule high-priority actors on performance cores
    for each actor in high_priority_actors:
        performance_core = find_available_performance_core()
        schedule_actor(actor, performance_core)
    end for
    
    // Schedule low-priority actors on efficiency cores
    for each actor in low_priority_actors:
        efficiency_core = find_available_efficiency_core()
        schedule_actor(actor, efficiency_core)
    end for
    
    // Balance load to minimize power consumption
    balance_load_for_power_efficiency()
end function
```

**Energy Efficiency Impact:**
- **Power Consumption**: Efficiency cores consume ~30-50% less power than performance cores
- **Performance Trade-off**: Efficiency cores provide ~50-70% of performance core performance
- **Battery Life**: Better battery life on mobile devices (Apple Silicon, ARM laptops)

**Power Management Runtime Behavior Summary:**

| Feature | Behavior | Impact on Actors | Runtime Response |
|---------|----------|------------------|-------------------|
| **Frequency Scaling** | CPU frequency adjusts based on load | Performance varies with frequency | Request max frequency for high-priority actors |
| **Core Parking** | Efficiency cores powered down | Actors migrated to active cores | Transparent migration, no actor changes |
| **Thermal Throttling** | Cores throttle when hot | Performance degradation on hot cores | Migrate actors to cooler cores |
| **Energy Optimization** | Load balanced for efficiency | Low-priority actors on efficiency cores | Automatic core selection based on priority |

**Power Management Integration:**

The runtime integrates with OS power management via:

- **sysfs interfaces**: `/sys/devices/system/cpu/cpu{N}/cpufreq/` for frequency control
- **thermal interfaces**: `/sys/devices/virtual/thermal/` for temperature monitoring
- **core parking**: `/sys/devices/system/cpu/cpu{N}/online` for core state
- **cgroup integration**: CPU cgroups for power-aware scheduling

**OS Scheduler Integration:**

The actor runtime integrates with the Linux kernel scheduler:

1. **SCHED_FIFO/SCHED_RR**: High-priority actors may use real-time scheduling policies
2. **SCHED_NORMAL**: Most actors use normal scheduling with nice values for priority
3. **Cgroup Integration**: Actors can be placed in cgroups for resource limits
4. **Scheduler Hints**: The runtime provides scheduler hints via `sched_setaffinity()` and `sched_setscheduler()`

**Cache-Aware Scheduling:**

The runtime optimizes for cache locality:

1. **L1 Cache Affinity**: Actors accessing the same data are scheduled on cores sharing L1 cache (typically not possible, as L1 is per-core)
2. **L2 Cache Affinity**: Actors are scheduled on cores sharing L2 cache when possible (some AArch64 chips share L2 between cores)
3. **L3 Cache Affinity**: Actors are scheduled within the same L3 cache domain (typically per-NUMA node)
4. **Cache Line Alignment**: The runtime ensures actor data structures are cache-line aligned (64-byte alignment)

**Runtime Actor State:**

Each actor maintains runtime state for AArch64 integration:

```
actor_runtime_state {
    core_id: int,              // Core the actor is pinned to
    numa_node: int,            // NUMA node of the actor's core
    cache_domain: int,          // Cache domain (L2/L3 sharing)
    priority: int,              // Scheduling priority
    affinity_mask: cpu_set_t,   // CPU affinity mask
}
```

**Example: NUMA-Aware Actor Placement**

```silica
// Allocate region on NUMA node 0
region: region(R, normal) <- alloc_region_on_numa(0, normal);

// Spawn actor on same NUMA node
actor_ref: actor_ref <- spawn_on_numa(initial_state, behavior, 0);

// Runtime ensures actor runs on a core within NUMA node 0
// minimizing cross-NUMA memory access latency
```

#### 15.1.2.2 Actor Migration Overhead and Behavior

The runtime may migrate actors between cores for load balancing, thermal management, power optimization, and core availability. Actor migration is transparent to user code but has performance implications that developers should understand.

**Migration Triggers:**

Actors are migrated between cores in the following scenarios:

- **Thermal Throttling**: When a core exceeds thermal limits, actors are migrated to cooler cores to prevent performance degradation and hardware damage
  - **Trigger Threshold**: Core temperature exceeds thermal throttling threshold (typically 80-90°C)
  - **Migration Target**: Actors are migrated to cores with lower temperatures
  - **Priority**: High-priority actors are migrated first to maintain performance
  
- **Load Balancing**: When core load becomes imbalanced, actors are redistributed to balance workload across available cores
  - **Trigger Threshold**: Core load imbalance exceeds threshold (typically 20-30% difference)
  - **Migration Target**: Actors are migrated from overloaded cores to underutilized cores
  - **Frequency**: Load balancing occurs periodically (typically every 100-1000 milliseconds)
  
- **Core Parking/Unparking**: When efficiency cores are parked or unparked by the OS, actors are migrated accordingly
  - **Core Parking**: Actors on parked cores are migrated to active cores
  - **Core Unparking**: New actors may be scheduled on newly unparked cores
  - **Transparency**: Migration is transparent to actors - no code changes required
  
- **Affinity Changes**: When actor affinity is changed via runtime APIs, actors are migrated to new cores
  - **Explicit Migration**: Programmers can explicitly migrate actors using runtime APIs
  - **Affinity Updates**: Changes to actor affinity trigger migration to new cores
  - **Validation**: Runtime validates that target cores are available before migration
  
- **NUMA Optimization**: Actors may be migrated to cores within the same NUMA node as their data regions
  - **NUMA Awareness**: Runtime migrates actors to minimize cross-NUMA memory access
  - **Data Locality**: Actors are placed on cores with local access to their data regions
  - **Performance**: Reduces memory access latency by avoiding cross-NUMA communication

**Migration Process:**

Actor migration follows a well-defined process to ensure correctness and minimize overhead:

1. **Migration Decision**: Runtime determines that migration is necessary based on triggers
2. **Target Selection**: Runtime selects target core based on load, temperature, NUMA affinity, and actor requirements
3. **State Capture**: Actor's current state is captured (including in-flight message processing)
4. **Migration Preparation**: Runtime prepares target core for actor execution
5. **State Transfer**: Actor state is transferred to target core (memory remains in place, only execution context moves)
6. **Execution Resume**: Actor resumes execution on target core
7. **Cleanup**: Source core resources are cleaned up

**Migration Overhead:**

Actor migration incurs overhead that affects performance:

- **Context Switch Overhead**: Migration requires context switching between cores, adding ~1-5 microseconds overhead
- **Cache Invalidation**: Actor's cache lines may be invalidated on source core, requiring cache refill on target core
- **TLB Flush**: Translation Lookaside Buffer (TLB) entries may need to be flushed and refilled on target core
- **Memory Access Latency**: Cross-NUMA migration may increase memory access latency if data regions are not migrated
- **Message Queue Transfer**: Actor's message queue is transferred to target core, adding ~100-500 nanoseconds overhead

**Migration Latency Guarantees:**

The runtime provides latency guarantees for actor migration:

- **Migration Time**: Actor migration completes within 1-10 milliseconds for typical actors
- **Message Delivery**: Messages continue to be delivered during migration (queued until migration completes)
- **State Consistency**: Actor state remains consistent during migration (no partial updates visible)
- **Execution Continuity**: Actor execution resumes on target core without loss of progress

**Message Delivery During Migration:**

Message delivery behavior during migration ensures correctness:

- **Pre-Migration Messages**: Messages received before migration starts are processed on source core
- **Migration-In-Flight Messages**: Messages received during migration are queued and delivered after migration completes
- **Post-Migration Messages**: Messages received after migration completes are delivered normally on target core
- **Message Ordering**: Message ordering is preserved across migration (FIFO ordering maintained)

**Cache Performance Impact:**

Migration affects cache performance:

- **Cache Warm-Up**: Target core's caches are cold after migration, requiring cache warm-up
- **Cache Miss Rate**: Cache miss rate increases immediately after migration (typically 20-30% increase)
- **Cache Recovery**: Cache performance recovers as actor executes on target core (typically within 100-1000 operations)
- **NUMA Cache Effects**: Cross-NUMA migration may cause cache misses if data is not local to target core

**Migration Control:**

Actor migration is **manual-only** - application programmers must explicitly migrate actors using the `migrate_actor()` function. The runtime does not perform automatic migration for load balancing, thermal management, or core parking.

**Migration API:**

```silica
// Explicitly migrate an actor to a target core
migrate_actor(actor_ref: actor_ref, target_core: int) -> atom proc[concurrency]

// Pin an actor to a specific core (prevents migration)
pin_actor_to_core(actor_ref: actor_ref, core_id: int) -> atom proc[concurrency]
```

**Migration Policy:**

- **No Automatic Migration**: The runtime never automatically migrates actors between cores
- **Manual Migration Only**: Actors only migrate when explicitly requested via `migrate_actor()`
- **Core Affinity**: Actors can be pinned to cores using `pin_actor_to_core()` to prevent migration
- **Migration Responsibility**: Application programmers are responsible for implementing migration policies (load balancing, thermal management, etc.) if needed

**NUMA-Aware Migration Strategies:**

The runtime implements sophisticated NUMA-aware migration strategies to optimize actor placement and minimize cross-NUMA memory access latency. These strategies leverage AArch64 topology detection to make intelligent migration decisions.

**NUMA Topology Detection:**

Before making migration decisions, the runtime detects NUMA topology using AArch64 system registers and kernel interfaces:

```pseudocode
function detect_numa_topology():
    // Read MPIDR_EL1 for NUMA node information
    MRS MPIDR_EL1, MPIDR_EL1
    NUMA_NODE = MPIDR_EL1[23:16]  // Affinity level 2 (NUMA node)
    
    // Query kernel interfaces for NUMA topology
    // /sys/devices/system/node/node{N}/cpulist provides core-to-NUMA mapping
    // /proc/cpuinfo provides core identification
    
    // Build NUMA topology map
    for each core in system:
        core_numa_node = query_numa_node(core)
        numa_topology[core] = core_numa_node
    end for
    
    return numa_topology
end function
```

**NUMA-Aware Migration Decision Algorithm:**

The runtime uses the following algorithm to determine optimal actor placement:

```pseudocode
function decide_actor_migration(actor, current_core, target_candidates):
    // Get actor's data region NUMA affinity
    actor_data_numa = get_actor_data_numa_node(actor)
    
    // Get current core NUMA node
    current_numa = numa_topology[current_core]
    
    // Evaluate migration candidates
    best_core = current_core
    best_score = evaluate_placement_score(actor, current_core, actor_data_numa)
    
    for each candidate_core in target_candidates:
        candidate_numa = numa_topology[candidate_core]
        score = evaluate_placement_score(actor, candidate_core, actor_data_numa)
        
        if score > best_score:
            best_score = score
            best_core = candidate_core
        end if
    end for
    
    // Migration decision: migrate if significantly better placement available
    if best_core != current_core and best_score > best_score + MIGRATION_THRESHOLD:
        return MIGRATE_TO(best_core)
    else:
        return STAY_ON_CURRENT_CORE
    end if
end function

function evaluate_placement_score(actor, core, data_numa):
    core_numa = numa_topology[core]
    
    // Same NUMA node: highest score (local memory access)
    if core_numa == data_numa:
        return 100
    
    // Different NUMA node: lower score (cross-NUMA access)
    // Score decreases with NUMA distance
    numa_distance = get_numa_distance(core_numa, data_numa)
    return 100 - (numa_distance * 20)  // Penalty for cross-NUMA access
end function
```

**NUMA Migration Scenarios:**

The runtime handles different NUMA migration scenarios with specific behavioral guarantees:

**Scenario 1: Same-NUMA Migration**

When migrating an actor within the same NUMA node:

- **Memory Access Latency**: No change - memory access remains local to NUMA node
- **Cache Performance**: Cache may be cold on target core, but cache warm-up is fast (same NUMA node)
- **Migration Overhead**: Minimal - only execution context moves, memory stays local
- **Behavioral Guarantee**: Actor execution continues normally with no observable performance degradation after cache warm-up

**Scenario 2: Cross-NUMA Migration**

When migrating an actor to a different NUMA node:

- **Memory Access Latency**: Increases - memory access now crosses NUMA boundaries
- **Cache Performance**: Cache is cold on target core, and cache misses may access remote NUMA memory
- **Migration Overhead**: Higher - execution context moves, and memory access patterns change
- **Behavioral Guarantee**: Actor execution continues correctly, but memory access latency increases by 2-3x for remote NUMA access

**Scenario 3: NUMA-Optimized Placement**

When placing a new actor or migrating for NUMA optimization:

- **Placement Strategy**: Runtime selects core on same NUMA node as actor's data regions
- **Data Locality**: Maximizes data locality by keeping actor and data on same NUMA node
- **Performance**: Optimal memory access latency (local NUMA access)
- **Behavioral Guarantee**: Actor is placed to minimize cross-NUMA memory access

**NUMA Migration Behavioral Guarantees:**

The runtime provides the following behavioral guarantees for NUMA-aware migration:

1. **Correctness**: Actor execution remains correct regardless of NUMA placement - migration does not affect program semantics
2. **Message Delivery**: Messages continue to be delivered correctly during and after migration, regardless of NUMA placement
3. **State Consistency**: Actor state remains consistent during migration - no partial state visible across NUMA boundaries
4. **Memory Access**: Memory access remains valid, but latency may increase for cross-NUMA access
5. **Cache Coherency**: Cache coherency is maintained across NUMA boundaries via AArch64 cache coherency protocols

**NUMA Migration Decision Factors:**

The runtime considers the following factors when making NUMA-aware migration decisions:

- **Data Region NUMA Affinity**: NUMA node where actor's data regions are allocated
- **Current Core NUMA Node**: NUMA node of the core where actor currently executes
- **Target Core NUMA Node**: NUMA node of potential migration target cores
- **NUMA Distance**: Distance between NUMA nodes (affects memory access latency)
- **Core Load**: Load on source and target cores (affects migration benefit)
- **Actor Priority**: Priority of actor (high-priority actors get better NUMA placement)

**NUMA Migration Timing:**

NUMA-aware migration occurs at specific times:

- **Actor Spawn**: New actors are placed on cores with same NUMA node as their data regions
- **Explicit Migration**: When `migrate_actor()` is called, runtime selects NUMA-optimal target core
- **Data Region Allocation**: When actor allocates new data regions, runtime may migrate actor to optimize NUMA placement
- **Load Balancing**: When load balancing triggers migration, runtime considers NUMA placement in target selection

**NUMA Migration Examples:**

**Example 1: Same-NUMA Migration**

```silica
// Actor with data on NUMA node 0
actor_ref: actor_ref <- spawn(initial_state, behavior_fn);

// Data region allocated on NUMA node 0
do
    r: region(R, normal) <- alloc_region(normal);  // Allocated on NUMA node 0
    // Actor is placed on core in NUMA node 0 (optimal placement)
end

// Migration within NUMA node 0 (minimal overhead)
migrate_actor(actor_ref, target_core_in_numa_0);  // Same NUMA, fast migration
```

**Example 2: Cross-NUMA Migration**

```silica
// Actor with data on NUMA node 0
actor_ref: actor_ref <- spawn(initial_state, behavior_fn);

// Data region allocated on NUMA node 0
do
    r: region(R, normal) <- alloc_region(normal);  // Allocated on NUMA node 0
    // Actor is placed on core in NUMA node 0
end

// Migration to NUMA node 1 (cross-NUMA, higher latency)
migrate_actor(actor_ref, target_core_in_numa_1);  
// Memory access latency increases (cross-NUMA access)
// Actor execution remains correct, but performance may degrade
```

**Example 3: NUMA-Optimized Placement**

```silica
// Allocate data region on specific NUMA node
do
    // Allocate region on NUMA node 0
    r: region(R, normal) <- alloc_region_on_numa(normal, numa_node_0);
    
    // Spawn actor - runtime places on core in NUMA node 0
    actor_ref: actor_ref <- spawn(initial_state, behavior_fn);
    // Actor automatically placed on NUMA node 0 core (optimal)
end
```

**NUMA Migration Performance Characteristics:**

| Migration Type | Memory Access Latency Change | Cache Warm-Up Time | Migration Overhead | Performance Impact |
|---------------|------------------------------|-------------------|-------------------|-------------------|
| Same-NUMA | No change (local access) | ~100-1000 operations | ~1-5 microseconds | Minimal after warm-up |
| Cross-NUMA | 2-3x increase (remote access) | ~1000-10000 operations | ~5-20 microseconds | Moderate (increased latency) |
| NUMA-Optimal Placement | Optimal (local access) | ~100-1000 operations | ~1-5 microseconds | Best performance |

**Migration Best Practices:**

Developers implementing manual migration should consider:

- **NUMA Awareness**: Allocate data regions on the same NUMA node as target cores to minimize cross-NUMA migration overhead
- **Affinity Pinning**: Pin performance-critical actors to specific cores to prevent accidental migration
- **Cache-Friendly Data**: Use cache-friendly data structures to minimize cache miss impact after migration
- **Migration Timing**: Migrate actors during low-activity periods to minimize performance impact
- **NUMA Placement**: When spawning actors, allocate data regions first, then spawn actors to ensure NUMA-optimal placement
- **Cross-NUMA Avoidance**: Avoid cross-NUMA migration for performance-critical actors - prefer same-NUMA migration

**Cross-References:**
- See Section 15.1.2.1 (AArch64 Runtime Integration) for core topology and scheduling details
- See Section 12.1.1.1 (AArch64 Memory Attribute Mapping) for NUMA memory configuration
- See Section 15.1.2.2.1 (Message Delivery During Migration) for message delivery guarantees during migration
- See Section 23.1.3 (CPU Scheduling and Affinity) for scheduling and affinity controls
- See Section 18.2.3.2 (Memory Barrier Instruction Selection Strategy) for NUMA-aware barrier selection

#### 15.1.2.2.1 Message Delivery During Migration

When an actor migrates between cores, message delivery guarantees ensure correct actor semantics and message ordering.

**Message Queue Preservation:**

During actor migration, the actor's message queue is preserved:

1. **Queue Transfer**: The message queue is atomically transferred from source core to target core
2. **Queue Integrity**: All messages in the queue remain intact during migration
3. **No Message Loss**: No messages are lost during migration - all queued messages are preserved
4. **Atomic Transfer**: Queue transfer is atomic - messages are either all on source core or all on target core, never split

**FIFO Ordering Guarantees:**

Message ordering is preserved across migration:

1. **Pre-Migration Messages**: Messages received before migration starts are processed before migration begins
2. **Migration Messages**: Messages received during migration are queued and processed after migration completes
3. **Post-Migration Messages**: Messages received after migration completes are processed in order
4. **FIFO Preservation**: The relative order of all messages is preserved across migration

**Message Delivery Atomicity:**

Message delivery operations are atomic with respect to migration:

1. **Send Atomicity**: A `send()` operation either completes before migration starts or after migration completes - never during migration
2. **Receive Atomicity**: A `recv()` operation processes messages atomically - either all messages from a batch or none
3. **Migration Barrier**: Migration acts as a barrier - no message operations occur during the migration window

**Migration Window:**

The migration window is the period during which an actor is transitioning between cores:

1. **Window Start**: Migration window begins when runtime decides to migrate actor
2. **Window Duration**: Migration window duration is typically 1-20 microseconds (depends on NUMA topology)
3. **Window End**: Migration window ends when actor is fully operational on target core
4. **Message Handling**: During migration window, incoming messages are queued but not processed

**Example: Message Delivery During Migration**

```silica
// Actor A sends messages to Actor B
send(actor_b, msg1)  // Delivered before migration
send(actor_b, msg2)  // Delivered before migration

// Runtime decides to migrate Actor B (migration window starts)
// Migration occurs (1-20 microseconds)

send(actor_b, msg3)  // Queued during migration, delivered after migration completes
send(actor_b, msg4)  // Delivered after migration completes

// Actor B processes messages in order: msg1, msg2, msg3, msg4
```

**Cross-Core Message Delivery:**

When messages are sent from one core to an actor on another core:

1. **Cross-Core Send**: `send()` operation may complete before message reaches target core
2. **Message Routing**: Runtime routes messages to correct core based on actor's current location
3. **Location Tracking**: Runtime tracks actor location and updates routing when migration occurs
4. **Delivery Guarantee**: Messages are guaranteed to reach the actor regardless of migration timing

**Migration and Message Ordering:**

Actor migration preserves message ordering semantics:

1. **Per-Actor Ordering**: Messages to a single actor maintain FIFO order across migration
2. **Cross-Actor Ordering**: Messages between different actors maintain ordering relative to synchronization points
3. **Happens-Before**: Migration preserves happens-before relationships established by message passing

**Error Handling:**

If migration fails:

1. **Migration Rollback**: Actor remains on source core, migration is rolled back
2. **Message Queue**: Message queue remains on source core, no messages are lost
3. **Retry Strategy**: Runtime may retry migration after a delay
4. **Error Reporting**: Migration failure is reported to runtime monitoring systems

**Cross-References:**
- See Section 16.1.3 (Message Ordering) for general message ordering guarantees
- See Section 15.1.2.2 (Actor Migration Overhead and Behavior) for migration performance characteristics
- See Section 18.1.1 (Actor Message Ordering) for happens-before relationships

#### 15.1.3 Actor Identity
Each actor has a unique identity:

```
self() -> actor_ref proc[mailbox, concurrency]
```

The `actor_ref` type is a primitive type (like `int` or `boolean`), representing a reference to an actor.

### 15.2 Actor Behavior Functions

#### 15.2.1 Behavior Function Signature
Behavior functions transform messages and state:

```
type Request = {command: string, reply_to: actor_ref};
type Response = {result: int};
impl ActorMessage for Request;
impl ActorMessage for Response;

fn counter(msg: Request, state: int)
 -> int proc[mailbox, concurrency] {

    case msg of
        {command: "increment", reply_to} -> return state + 1
        {command: "get", reply_to} ->
            // Send response back using cast
            cast(reply_to, Response {result: state})
            return state
        {command: "reset", reply_to} -> return 0
    end
}
```

#### 15.2.2 State Encapsulation
Actor state is private and can only be modified by the actor itself:

```
// External code cannot access or modify actor state
actor_ref: actor_ref <- spawn(0, counter)
// No way to read or write the counter value directly
```

#### 15.2.3 Behavior Hot-Swapping
Actors can change their behavior by returning a different behavior function type (future extension).

### 15.3 Actor Failure and Supervision

#### 15.3.1 Actor Termination
Actors terminate when their behavior function cannot handle a message:

```
fn fragile_behavior(msg: string, state: unit) -> atom proc[mailbox] {
    case msg of
        "quit" -> // terminate actor (no return)
        other -> return ()  // continue
    end
}
```

#### 15.3.2 Failure Isolation
Actor failures don't affect other actors:

```
actor1: actor_ref <- spawn((), fragile_behavior)
actor2: actor_ref <- spawn((), robust_behavior)

send(actor1, "quit")    // actor1 terminates
send(actor2, "ping")    // actor2 continues normally
```

## 16. Message Passing

### 16.1 Message Send Semantics

#### 16.1.1 Asynchronous Send
Messages are sent asynchronously:

```
send(actor: actor_ref, message: ActorMessage) -> atom proc[concurrency]
```

Send never blocks - messages are queued in the actor's mailbox. The `message` parameter must be a type that implements the `ActorMessage` trait (for named types only).

#### 16.1.2 Asynchronous Cast
Messages can be sent asynchronously without blocking, with success/failure indication:

```
cast(actor: actor_ref, message: ActorMessage) -> boolean proc[concurrency]
```

Cast never blocks - messages are queued in the actor's mailbox and the function returns immediately. Returns `true` if the message was successfully enqueued, `false` if the actor doesn't exist. Mailboxes are unbounded queues that can grow without limit, so messages are never rejected due to mailbox capacity. The `message` parameter must be a type that implements the `ActorMessage` trait (for named types only).

#### 16.1.3 Message Ordering
Each actor's mailbox is a queue containing all messages from all actors sending it messages. Messages are processed in standard queue ordering: first-received, first-processed (FIFO). Messages from the same sender maintain order relative to each other, but messages from different senders are interleaved in the order they arrive at the actor's mailbox.

```
send(actor, msg1)
send(actor, msg2)
// actor receives msg1, then msg2 (maintains sender order)
// Messages from other actors may be interleaved between msg1 and msg2
```

#### 16.1.4 Message Delivery
Messages are delivered exactly once, in FIFO order. The actor runtime processes messages from the mailbox queue sequentially, passing each message to the behavior function along with the current state.

**Message Delivery to Terminated Actors:**

When a message is sent to an actor that has terminated, the following behavior applies:

- **Messages are Dropped**: Messages sent to terminated actors are dropped (not queued) and will never be delivered
- **No Error Raised**: The `send()` and `cast()` operations do not raise errors when targeting terminated actors (asynchronous send semantics)
- **Optional Logging**: The runtime may optionally log warnings about messages sent to terminated actors, but this is implementation-dependent and does not affect program behavior
- **No Delivery Guarantee**: There is no guarantee that messages sent to an actor will be delivered if the actor terminates before message processing

**Termination Detection:**

An actor is considered terminated when:
- The actor's behavior function explicitly terminates (e.g., returns a termination signal)
- The actor encounters an unrecoverable error and the runtime terminates it
- The actor is explicitly terminated by the runtime (e.g., via supervisor actions)

**Message Delivery Semantics:**

```silica
// Actor A sends message to Actor B
actor_b_ref: actor_ref <- spawn(initial_state, behavior_fn);

// Actor B terminates (for any reason)
// ... Actor B terminates ...

// Actor A sends message to terminated Actor B
send(actor_b_ref, SomeMessage {});  // Message is dropped, no error raised

// Optional: Runtime may log warning (implementation-dependent)
// Warning: "Message sent to terminated actor"
```

**Race Conditions:**

There is a potential race condition between actor termination and message sending:

- **Message Sent Before Termination**: If a message is sent before the actor terminates, it will be delivered normally
- **Message Sent After Termination**: If a message is sent after the actor terminates, it will be dropped
- **Message Sent During Termination**: If a message is sent while the actor is terminating, delivery behavior is implementation-dependent (typically dropped)

**Best Practices:**

To avoid message loss:
- Use actor supervision and monitoring to detect actor termination
- Implement application-level acknowledgments for critical messages
- Use actor references that are known to be alive before sending messages

### 16.2 Message Receive Semantics

#### 16.2.1 Runtime Message Reception
Message reception is handled automatically by the actor runtime system. The `recv()` operation is not available for direct use in user code:

```
recv() -> Msg proc[mailbox, concurrency]  // Runtime internal function
```

User behavior functions receive messages as parameters rather than calling `recv()` directly.

#### 16.2.2 Mailbox Semantics
Each actor has a single mailbox that queues incoming messages:

```
Mailbox State:
┌─────────────────────────────────┐
│ Message 3 (newest)             │
├─────────────────────────────────┤
│ Message 2                      │
├─────────────────────────────────┤
│ Message 1 (oldest)             │
└─────────────────────────────────┘

recv() returns Message 1, removes it from queue
```

**Unbounded Mailbox Queues:**

Actor mailboxes are implemented as unbounded queues that can grow without limit:

- **No Size Limits**: Mailboxes have no fixed size limits and can grow to accommodate any number of messages
- **Dynamic Growth**: Mailboxes dynamically allocate memory as needed to store incoming messages
- **No Overflow**: Messages are never rejected due to mailbox capacity - all messages are accepted and queued
- **Memory Management**: The runtime manages mailbox memory allocation and deallocation automatically
- **Unlimited Capacity**: Mailbox capacity is limited only by available system memory, not by fixed queue sizes

**Memory Considerations:**

While mailboxes are unbounded, developers should be aware of memory implications:

- **Memory Consumption**: Unbounded mailboxes can consume significant memory if message production exceeds consumption
- **Memory Pressure**: Very large mailboxes may cause system memory pressure, potentially affecting other actors
- **Backpressure**: Applications should implement application-level backpressure if message production consistently exceeds consumption
- **Monitoring**: Runtime may provide monitoring capabilities to track mailbox sizes and memory usage

**Message Acceptance Guarantees:**

Both `send()` and `cast()` guarantee message acceptance:

- **`send()` Function**: Always accepts messages - never fails due to mailbox capacity
  - Messages are always queued successfully
  - No blocking - returns immediately after queuing message
  - No capacity errors - mailboxes never reject messages
  
- **`cast()` Function**: Always accepts messages from existing actors - returns `false` only if actor doesn't exist
  - Returns `true` if message was successfully enqueued (actor exists)
  - Returns `false` only if actor doesn't exist (terminated or invalid actor reference)
  - Never returns `false` due to mailbox capacity - mailboxes are unbounded

**Performance Characteristics:**

Unbounded mailboxes provide predictable performance characteristics:

- **Enqueue Performance**: Message enqueueing is O(1) amortized - constant time per message
- **Memory Allocation**: Memory allocation occurs as needed, with minimal overhead per allocation
- **Queue Growth**: Queue growth is transparent to senders - no performance degradation as queue grows
- **Processing Latency**: Message processing latency may increase as queue size grows (more messages to process)

**Cross-References:**
- See Section 16.1.2 (Asynchronous Cast) for `cast()` return value semantics
- See Section 16.1.1 (Asynchronous Send) for `send()` behavior
- See Section 15.1.2.2 (Actor Migration Overhead and Behavior) for message delivery during migration
- See Section 12.1 (Region-Based Memory Management) for memory management details

#### 16.2.3 Message Patterns in Behavior Functions
Behavior functions receive messages as parameters and can use pattern matching on them:

```
fn selective_receiver(msg: msg_type, state: unit) -> atom proc[mailbox] {
    case msg of
        {request, data} -> handle_request(data)
        ping -> handle_ping()
        quit -> terminate()
    end
    return ()
}
```

The message parameter is automatically provided by the actor runtime when a message is received.

### 16.2.4 Cast vs Send
Both `cast()` and `send()` send messages asynchronously without blocking:

- **`send()`**: Returns `atom` - fire-and-forget message sending
  - Always succeeds (messages are always accepted by unbounded mailboxes)
  - No return value to check - suitable when message delivery is guaranteed
  
- **`cast()`**: Returns `boolean` - indicates whether actor exists
  - Returns `true` if message was successfully enqueued (actor exists)
  - Returns `false` only if actor doesn't exist (terminated or invalid actor reference)
  - Never returns `false` due to mailbox capacity (mailboxes are unbounded)

Use `cast()` when you need to detect if an actor exists before sending messages. Use `send()` for simple fire-and-forget messaging when actor existence is guaranteed.

### 16.3 Message Types and Serialization

#### 16.3.1 ActorState Trait
The `ActorState` trait is a marker trait that must be implemented by types used as actor initial state:

```
trait ActorState {
    // No methods required - marker trait for type safety
}
```

Only the `initial_state` parameter in `spawn(initial_state, ...)` must implement `ActorState`. The trait is only implemented for named types (structs, type aliases) - no blanket implementations for primitive types.

#### 16.3.2 ActorMessage Trait
The `ActorMessage` trait is a marker trait that must be implemented by types used as messages:

```
trait ActorMessage {
    // No methods required - marker trait for type safety
}
```

All types used in `send()` or `cast()` must implement `ActorMessage`. The trait is only implemented for named types (structs, type aliases) - no blanket implementations for primitive types.

#### 16.3.3 Message Type Safety
Messages must implement the `ActorMessage` trait:

```
type Request = {data: int, reply_to: actor_ref};
impl ActorMessage for Request;

actor_ref: actor_ref <- spawn(0, handler)
cast(actor_ref, Request {data: 42, reply_to: some_actor})  // ✓ correct type
cast(actor_ref, 42)  // ✗ type error: int doesn't implement ActorMessage
```

#### 16.3.4 Cast-Back Pattern
Messages can include a `reply_to` field containing an `actor_ref` for sending responses back:

```
type Request = {data: int, reply_to: actor_ref};
type Response = {result: int};
impl ActorMessage for Request;
impl ActorMessage for Response;

fn handler(msg: Request, state: State) -> State {
    case msg of
        {data, reply_to} ->
            // Process request and send response back
            cast(reply_to, Response {result: data * 2})
            // ... update state ...
    end
}
```

The `reply_to` field is optional - messages without it cannot be used for cast-back, but this is enforced at compile time through field access checks. Attempting to access `reply_to` on a message type that doesn't have it results in a compile-time error.

#### 16.3.5 Message Passing Guarantees
- **Type Safety**: Messages are type-checked at compile time - must implement `ActorMessage` trait
- **Trait Checking**: All type inference and trait checking happens at compile time, not runtime
- **Compile-Time Verification**: Field access (e.g., `reply_to`) is verified at compile time - attempting to access a field that doesn't exist in the message type results in a compile-time error
- **Immutability**: Message data cannot be mutated after sending
- **Isolation**: Message contents are copied between actors
- **Cast Success Indication**: `cast()` returns `boolean` indicating success/failure of message enqueueing
- **Actor Reference Type**: `actor_ref` is a primitive type (like `int` or `boolean`), not parameterized by message type

## 17. Atomic Operations

### 17.1 Atomic Types and Memory Spaces

#### 17.1.1 Atomic References
Atomic references provide thread-safe shared memory:

```
atomic_ref(R, Space, T)    // atomic reference to type T
```

#### 17.1.2 Atomic Memory Spaces
Atomic operations work in designated memory spaces:

```
alloc_atomic(region, initial_value) -> atomic_ref(R, Space, T) proc[mem(Space), atomic]
```

### 17.2 Memory Ordering Semantics

#### 17.2.1 Ordering Levels
Silica supports standard memory orderings that control how atomic operations synchronize memory accesses across actors:

```
type order = relaxed | acquire | release | acq_rel | seq_cst
```

**When to Use Each Ordering:**
- **relaxed**: Counters, statistics, or when ordering doesn't matter
- **acquire/release**: Producer-consumer patterns, flag-based coordination
- **acq_rel**: Read-modify-write operations that need both acquire and release
- **seq_cst**: When you need the strongest ordering guarantees (default if unsure)

#### 17.2.2 Detailed Ordering Guarantees

**relaxed**: No ordering constraints beyond atomicity
- Provides atomicity guarantee only - no synchronization with other operations
- Fastest option, but provides no ordering guarantees
- Use for: counters, statistics, or when ordering truly doesn't matter

```silica
// Example: Simple counter (ordering doesn't matter)
counter: atomic_ref(R, normal, int64) <- alloc_atomic(region, 0);

fn increment() -> atom proc[mem(normal), atomic] {
    atomic_fetch_add(counter, 1, relaxed)  // Fast, no ordering needed
}
```

**acquire**: Synchronizes with release operations
- Establishes happens-before relationship with prior release operations on the same atomic
- All memory operations after an acquire load are guaranteed to see effects from operations before the matching release store
- Use for: reading shared data that was published with release

```silica
// Example: Reading published data
data: atomic_ref(R, normal, Data) <- alloc_atomic(region, initial_data);
ready: atomic_ref(R, normal, boolean) <- alloc_atomic(region, false);

// Publisher (Actor A)
fn publish(new_data: Data) -> atom proc[mem(normal), atomic] {
    write_ref(data, new_data);                    // Write data
    atomic_store(ready, true, release);           // Publish with release
}

// Consumer (Actor B)
fn consume() -> Data proc[mem(normal), atomic] {
    ready_flag: boolean <- atomic_load(ready, acquire);  // Acquire synchronization
    if ready_flag {
        read_ref(data)  // Guaranteed to see the data written before release
    } else {
        initial_data
    }
}
```

**release**: Synchronizes with acquire operations
- Establishes happens-before relationship with future acquire operations on the same atomic
- All memory operations before a release store are guaranteed to be visible to operations after a matching acquire load
- Use for: publishing shared data that will be read with acquire

```silica
// Example: Publishing initialization complete
init_complete: atomic_ref(R, normal, boolean) <- alloc_atomic(region, false);

// Initializer (Actor A)
fn initialize() -> atom proc[mem(normal), atomic] {
    // ... perform initialization ...
    atomic_store(init_complete, true, release);  // Release: all prior writes visible
}

// Waiter (Actor B)
fn wait_for_init() -> atom proc[mem(normal), atomic] {
    complete: boolean <- atomic_load(init_complete, acquire);  // Acquire: see release
    // All initialization work is now guaranteed visible
}
```

**acq_rel**: Both acquire and release semantics
- Combines acquire (for reading) and release (for writing) in a single operation
- Use for: read-modify-write operations that both read and write shared state

```silica
// Example: Atomic counter with ordering
shared_counter: atomic_ref(R, normal, int64) <- alloc_atomic(region, 0);

fn increment_with_ordering() -> int64 proc[mem(normal), atomic] {
    // acq_rel ensures this operation synchronizes both ways
    old_value: int64 <- atomic_fetch_add(shared_counter, 1, acq_rel);
    // All prior increments are visible (acquire)
    // This increment is visible to future operations (release)
    old_value
}
```

**seq_cst**: Sequential consistency (strongest ordering)
- All seq_cst operations appear in a single total order visible to all actors
- Provides the strongest ordering guarantees - easiest to reason about
- Slightly slower than acquire/release, but provides global ordering
- Use when: you need the strongest guarantees or are unsure which ordering to use

```silica
// Example: Global flag coordination
global_flag: atomic_ref(R, normal, boolean) <- alloc_atomic(region, false);

// Actor A
fn set_flag() -> atom proc[mem(normal), atomic] {
    atomic_store(global_flag, true, seq_cst);  // Participates in global order
}

// Actor B
fn check_flag() -> boolean proc[mem(normal), atomic] {
    atomic_load(global_flag, seq_cst)  // Sees all seq_cst operations in order
}
```

#### 17.2.3 Practical Examples

**Producer-Consumer Pattern:**
```silica
// Shared buffer with atomic head/tail pointers
type ring_buffer = {
    data: buf(R, normal, int64, 100),
    head: atomic_ref(R, normal, int64),
    tail: atomic_ref(R, normal, int64)
}

// Producer: publish items
fn produce(buffer: ring_buffer, item: int64) -> boolean proc[mem(normal), atomic] {
    head: int64 <- atomic_load(buffer.head, acquire);
    tail: int64 <- atomic_load(buffer.tail, relaxed);
    
    next_head: int64 <- (head + 1) % 100;
    if next_head == tail {
        false  // Buffer full
    } else {
        write_buf(buffer.data, head, item);
        atomic_store(buffer.head, next_head, release);  // Release: item visible
        true
    }
}

// Consumer: consume items
fn consume(buffer: ring_buffer) -> OptionInt64 proc[mem(normal), atomic] {
    tail: int64 <- atomic_load(buffer.tail, acquire);  // Acquire: see producer releases
    head: int64 <- atomic_load(buffer.head, relaxed);
    
    if tail == head {
        None  // Buffer empty
    } else {
        item: int64 <- read_buf(buffer.data, tail);
        next_tail: int64 <- (tail + 1) % 100;
        atomic_store(buffer.tail, next_tail, release);  // Release: consumption visible
        Some(item)
    }
}
```

**Flag-Based Coordination:**
```silica
// Coordination flag between actors
ready_flag: atomic_ref(R, normal, boolean) <- alloc_atomic(region, false);
data: ref(R, normal, SharedData) <- alloc_ref(region, initial_data);

// Actor A: Prepare and signal
fn prepare_and_signal(new_data: SharedData) -> atom proc[mem(normal), atomic] {
    write_ref(data, new_data);                    // Prepare data
    atomic_store(ready_flag, true, release);      // Signal with release
    // All writes before release are visible to acquire loads
}

// Actor B: Wait and read
fn wait_and_read() -> SharedData proc[mem(normal), atomic] {
    ready: boolean <- atomic_load(ready_flag, acquire);  // Acquire: see release
    if ready {
        read_ref(data)  // Guaranteed to see data written before release
    } else {
        initial_data
    }
}
```

**Lock-Free Stack:**
```silica
type stack_node = {
    value: int64,
    next: ref(R, normal, stack_node)
}

type lock_free_stack = {
    top: atomic_ref(R, normal, ref(R, normal, stack_node))
}

fn push(stack: lock_free_stack, value: int64) -> atom proc[mem(normal), atomic] {
    new_node: ref(R, normal, stack_node) <- alloc_ref(region, {value: value, next: null});
    
    loop {
        old_top: ref(R, normal, stack_node) <- atomic_load(stack.top, acquire);
        write_ref(new_node.next, old_top);
        
        // Compare-and-swap with acq_rel for both read and write ordering
        result: {ok, ref(R, normal, stack_node)} | {fail, ref(R, normal, stack_node)} 
            <- atomic_compare_exchange(stack.top, old_top, new_node, acq_rel);
        
        case result of {
            {ok, _} -> return;  // Success
            {fail, current_top} -> write_ref(new_node.next, current_top);  // Retry
        }
    }
}
```

**Actor Synchronization with Atomics:**
```silica
// Shared counter accessed by multiple actors
shared_count: atomic_ref(R, normal, int64) <- alloc_atomic(region, 0);

// Actor behavior that increments counter
fn counter_actor(msg: IncrementMsg, state: unit) -> atom proc[mem(normal), atomic] {
    // Use seq_cst for global ordering across all actors
    atomic_fetch_add(shared_count, 1, seq_cst);
    ()
}

// Reader actor that reads the counter
fn reader_actor(msg: ReadMsg, state: unit) -> int64 proc[mem(normal), atomic] {
    // seq_cst ensures we see all increments in order
    atomic_load(shared_count, seq_cst)
}
```

#### 17.2.4 Actor and Atomic Interaction

Atomic operations provide synchronization between actors, complementing message passing:

**When to Use Atomics vs Messages:**
- **Use atomics**: Shared counters, flags, lock-free data structures, performance-critical shared state
- **Use messages**: Most actor communication, complex state updates, when you need backpressure

**Combining Both:**
```silica
// Use atomics for fast shared state, messages for coordination
shared_stats: atomic_ref(R, normal, Stats) <- alloc_atomic(region, initial_stats);

// Fast path: atomic update
fn update_stats_fast(increment: int64) -> atom proc[mem(normal), atomic] {
    atomic_fetch_add(shared_stats.count, increment, relaxed);
}

// Coordination path: message passing
fn request_detailed_report(reply_to: actor_ref) -> atom proc[concurrency] {
    current_stats: Stats <- atomic_load(shared_stats, acquire);
    send(reply_to, ReportMsg {stats: current_stats});
}
```

**Happens-Before with Actors:**
```silica
// Atomic release in one actor, message send, atomic acquire in another
flag: atomic_ref(R, normal, boolean) <- alloc_atomic(region, false);

// Actor A
fn actor_a_behavior(msg: StartMsg, state: unit) -> atom proc[mem(normal), atomic, concurrency] {
    // Prepare data
    prepare_data();
    
    // Release: all prior writes visible
    atomic_store(flag, true, release);
    
    // Send message (happens after release)
    send(actor_b_ref, DataReadyMsg {});
    ()
}

// Actor B
fn actor_b_behavior(msg: DataReadyMsg, state: unit) -> atom proc[mem(normal), atomic] {
    // Acquire: guaranteed to see release from Actor A
    ready: boolean <- atomic_load(flag, acquire);
    // All data prepared by Actor A is now visible
    process_data();
    ()
}
```

#### 17.2.5 Performance Characteristics

**Ordering Performance (fastest to slowest):**
1. **relaxed**: Fastest - no memory barriers, just atomicity
2. **acquire/release**: Fast - minimal barriers, one-way synchronization
3. **acq_rel**: Moderate - combines acquire and release barriers
4. **seq_cst**: Slowest - requires global ordering, more barriers

**Performance Guidelines:**
- Use `relaxed` when ordering doesn't matter (counters, statistics)
- Use `acquire/release` pairs for producer-consumer patterns (optimal performance with correct ordering)
- Use `seq_cst` when you need global ordering or are unsure (safest, slightly slower)
- Avoid `seq_cst` in hot paths if `acquire/release` is sufficient

**AArch64 Performance Notes:**
- AArch64's acquire/release operations map efficiently to hardware
- `seq_cst` may require additional barriers compared to acquire/release
- Modern AArch64 processors optimize atomic operations well

#### 17.2.6 AArch64 Implementation Details

**Instruction Mapping:**

Each ordering level maps to specific AArch64 instructions:

**relaxed:**
- Load: `LDR <Rt>, [<Xn|SP>, #<imm>]` (regular load)
- Store: `STR <Rt>, [<Xn|SP>, #<imm>]` (regular store)
- RMW: `LDXR <Rt>, [<Xn|SP>]` / `STXR <Ws>, <Rt>, [<Xn|SP>]` loop without barriers
- **Hardware guarantee**: Atomicity only, no ordering guarantees
- **Instruction encoding**: Standard load/store instructions, no special ordering semantics

**acquire:**
- Load: `LDAR <Rt>, [<Xn|SP>]` (load-acquire)
  - Acquire semantics: All subsequent memory operations are ordered after this load
  - Instruction variant: `LDARB` (byte), `LDARH` (halfword), `LDAR` (word/doubleword)
- Store: `STLR <Rt>, [<Xn|SP>]` (store-release) - release semantics for stores
  - Release semantics: All prior memory operations are ordered before this store
  - Instruction variant: `STLRB` (byte), `STLRH` (halfword), `STLR` (word/doubleword)
- RMW: `LDAXR <Rt>, [<Xn|SP>]` / `STLXR <Ws>, <Rt>, [<Xn|SP>]` loop
  - Load-acquire-exclusive / Store-release-exclusive
  - Retry loop: `STLXR` returns status in `Ws` register (0 = success, 1 = failure)
- **Hardware guarantee**: All memory operations after the load-acquire see effects from operations before matching release stores
- **Synchronization**: One-way synchronization barrier (acquire barrier)

**release:**
- Load: `LDAR <Rt>, [<Xn|SP>]` (load-acquire) - acquire semantics for loads
  - When used for release ordering on the read side, acquire ensures visibility
- Store: `STLR <Rt>, [<Xn|SP>]` (store-release)
  - Release semantics: All prior memory operations are ordered before this store
- RMW: `LDAXR <Rt>, [<Xn|SP>]` / `STLXR <Ws>, <Rt>, [<Xn|SP>]` loop
  - Combines acquire (read) and release (write) semantics
- **Hardware guarantee**: All memory operations before the store-release are visible to operations after matching acquire loads
- **Synchronization**: One-way synchronization barrier (release barrier)

**acq_rel:**
- Load: `LDAR <Rt>, [<Xn|SP>]` (acquire)
  - Acquire semantics on the read side
- Store: `STLR <Rt>, [<Xn|SP>]` (release)
  - Release semantics on the write side
- RMW: `LDAXR <Rt>, [<Xn|SP>]` / `STLXR <Ws>, <Rt>, [<Xn|SP>]` loop with both acquire and release semantics
  - Load-acquire-exclusive provides acquire semantics
  - Store-release-exclusive provides release semantics
- **Hardware guarantee**: Combines acquire (read-side) and release (write-side) guarantees
- **Synchronization**: Two-way synchronization barrier (acquire + release)

**seq_cst:**
- Load: `DMB ISH` + `LDAR <Rt>, [<Xn|SP>]` (memory barrier before load-acquire)
  - `DMB ISH` (Inner Shareable domain barrier) ensures ordering
  - `LDAR` provides acquire semantics
- Store: `DMB ISH` + `STLR <Rt>, [<Xn|SP>]` (memory barrier before store-release)
  - `DMB ISH` ensures all prior operations complete
  - `STLR` provides release semantics
- RMW: `DMB ISH` + `LDAXR <Rt>, [<Xn|SP>]` / `STLXR <Ws>, <Rt>, [<Xn|SP>]` + `DMB ISH` loop with full barriers
  - Barriers before and after the RMW operation
- **Hardware guarantee**: All seq_cst operations participate in a single total order visible to all cores
- **Synchronization**: Full memory barrier (both acquire and release, plus global ordering)

**Memory Barrier Instructions:**

AArch64 provides several memory barrier instructions:

- **DMB (Data Memory Barrier)**: Ensures memory operations complete in order
  - `DMB ISH` (Inner Shareable): Barrier for inner shareable domain (all cores)
  - `DMB OSH` (Outer Shareable): Barrier for outer shareable domain (includes I/O)
  - `DMB SY` (Full System): Full system barrier
  - Variants: `DMB LD` (load barrier), `DMB ST` (store barrier), `DMB` (full barrier)

- **DSB (Data Synchronization Barrier)**: Stronger than DMB, ensures completion
  - `DSB ISH`, `DSB OSH`, `DSB SY` variants
  - Used when completion (not just ordering) is required

- **ISB (Instruction Synchronization Barrier)**: Flushes pipeline, ensures instruction fetch
  - Used when code modification requires visibility

**Atomic RMW Instruction Details:**

The `LDXR`/`STXR` (Load-Exclusive/Store-Exclusive) pair provides atomic read-modify-write:

```
LDXR <Rt>, [<Xn|SP>]     // Load-exclusive: marks memory location as exclusive
// ... modify <Rt> ...
STXR <Ws>, <Rt>, [<Xn|SP>] // Store-exclusive: stores if location still exclusive
```

- **Exclusive Monitor**: Hardware tracks exclusive access per core
- **Store Success**: `STXR` sets `Ws = 0` if store succeeds (location still exclusive)
- **Store Failure**: `STXR` sets `Ws = 1` if store fails (another core modified location)
- **Retry Loop**: Compiler generates retry loop on failure:
  ```
  loop:
      LDXR Rt, [Xn]
      // modify Rt
      STXR Ws, Rt, [Xn]
      CBNZ Ws, loop  // retry if Ws != 0
  ```

**Instruction Variants:**

AArch64 provides size-specific variants for all atomic operations:

- **Byte**: `LDARB`, `STLRB`, `LDAXRB`, `STLXRB`
- **Halfword**: `LDARH`, `STLRH`, `LDAXRH`, `STLXRH`
- **Word**: `LDAR`, `STLR`, `LDAXR`, `STLXR` (32-bit)
- **Doubleword**: `LDAR`, `STLR`, `LDAXR`, `STLXR` (64-bit, register size determines)

**AArch64 Hardware Guarantees:**

1. **Cache Coherency**: AArch64 provides cache-coherent shared memory via MESI/MOESI protocols - atomic operations work correctly across cores
2. **Load-Acquire/Store-Release**: Hardware-supported acquire/release semantics, very efficient (single instruction, no barrier overhead)
3. **Memory Barriers**: `DMB` (Data Memory Barrier) instructions provide explicit ordering control with configurable scope (ISH/OSH/SY)
4. **Atomic RMW**: `LDXR`/`STXR` (Load-Exclusive/Store-Exclusive) provide atomic read-modify-write with hardware-managed exclusive monitor and automatic retry on conflict
5. **Total Store Ordering**: AArch64 provides TSO-like guarantees for store-release operations - stores to the same location are observed in the same order by all cores
6. **Weakly Ordered Model**: AArch64 uses a weakly ordered memory model - only explicit synchronization (acquire/release, barriers) provides ordering guarantees

**Compiler Optimizations:**

- The compiler may optimize atomic operations based on ordering requirements
- Redundant barriers may be removed when ordering is already guaranteed by acquire/release semantics
- Acquire/release pairs are optimized to use efficient AArch64 instructions (`LDAR`/`STLR`) without additional barriers
- Sequential consistency operations may be optimized to use acquire/release when total ordering is not strictly required
- The compiler generates efficient retry loops for `LDXR`/`STXR` operations, minimizing overhead on contention

### 17.3 Atomic Primitives

#### 17.3.1 Load and Store
```
atomic_load(aref, order) -> T proc[mem(Space), atomic]
atomic_store(aref, value, order) -> atom proc[mem(Space), atomic]
```

#### 17.3.2 Read-Modify-Write Operations
```
atomic_fetch_add(aref, delta, order) -> T proc[mem(Space), atomic]
atomic_fetch_sub(aref, delta, order) -> T proc[mem(Space), atomic]
atomic_fetch_and(aref, mask, order) -> T proc[mem(Space), atomic]
atomic_fetch_or(aref, mask, order) -> T proc[mem(Space), atomic]
atomic_fetch_xor(aref, mask, order) -> T proc[mem(Space), atomic]
```

#### 17.3.3 Compare and Exchange
```
atomic_compare_exchange(aref, expected, new_val, order)
 -> {ok, T} | {fail, T} proc[mem(Space), atomic]
```

Returns `{ok, old_value}` if successful, `{fail, current_value}` if the value wasn't expected.

### 17.4 Lock-Free Data Structures

#### 17.4.1 SPSC Queue Example
```
type spsc_queue<R, T> = {
    buf: buf(R, normal, T, Capacity),
    capacity: int,
    head: atomic_ref(R, normal, int),
    tail: atomic_ref(R, normal, int)
}

fn spsc_send(queue, item) -> boolean proc[mem(normal), atomic] {
    tail: int <- atomic_load(queue.tail, acquire)
    head: int <- atomic_load(queue.head, acquire)

    next_tail = (tail + 1) % queue.capacity
    if next_tail == head {
        return false    // queue full
    }

    write_buf(queue.buf, tail, item)
    atomic_store(queue.tail, next_tail, release)
    return true
}

fn spsc_recv(queue) -> option<T> proc[mem(normal), atomic] {
    head: int <- atomic_load(queue.head, acquire)
    tail: int <- atomic_load(queue.tail, acquire)

    if head == tail {
        return None     // queue empty
    }

    item: T <- read_buf(queue.buf, head)
    next_head = (head + 1) % queue.capacity
    atomic_store(queue.head, next_head, release)
    return Some(item)
}
```

## 18. Synchronization Guarantees

### 18.1 Happens-Before Relationships

#### 18.1.1 Actor Message Ordering
```
send(actorA, msg1)
send(actorA, msg2)
```
establishes: `msg1` happens-before `msg2` in actorA

#### 18.1.2 Atomic Synchronization
```
atomic_store(ref, value, release)  // in actor A
atomic_load(ref, acquire)          // in actor B
```
establishes: store happens-before load

#### 18.1.3 Transitive Ordering
Happens-before is transitive:
```
A → B and B → C implies A → C
```

### 18.2 Memory Consistency Models

#### 18.2.1 Per-Actor Sequential Consistency
Within a single actor, all operations appear sequentially consistent:

```
// Inside actor, this appears atomic to external observers
x: int <- read_ref(ref1)
y: int <- read_ref(ref2)
write_ref(ref3, x + y)
```

#### 18.2.2 Cross-Actor Ordering
Between actors, only explicit synchronization establishes ordering:

```
// Actor 1
atomic_store(flag, true, release)
send(actor2, data)

// Actor 2 behavior function
fn process_message(msg: Data, state: unit) -> atom {
    flag_value: boolean <- atomic_load(flag, acquire)
    // flag_value is guaranteed to be true
}
```

#### 18.2.3 AArch64 Memory Model Mapping

Silica's memory consistency model maps directly to the AArch64 weakly ordered memory model. The following table shows how Silica guarantees map to AArch64 hardware guarantees:

**AArch64 Memory Model Characteristics:**

- **Weakly Ordered**: AArch64 uses a weakly ordered memory model - memory operations can be reordered unless explicitly synchronized
- **Cache Coherent**: AArch64 provides cache-coherent shared memory via MESI/MOESI protocols
- **Acquire/Release Semantics**: Hardware-supported via `LDAR` (load-acquire) and `STLR` (store-release) instructions
- **Total Store Ordering**: Store-release operations provide TSO-like guarantees (stores to same location observed in same order)

**Silica-to-AArch64 Mapping:**

| Silica Guarantee | AArch64 Hardware Guarantee | Implementation |
|------------------|---------------------------|-----------------|
| Per-actor sequential consistency | Single-core sequential consistency | Natural single-core ordering |
| Cross-actor ordering via acquire/release | Load-acquire/store-release ordering | `LDAR`/`STLR` instructions |
| Cross-actor ordering via seq_cst | Global ordering with barriers | `LDAR`/`STLR` + `DMB ISH` barriers |
| Actor message ordering | Per-actor sequential consistency + message queue ordering | Single-core ordering + FIFO queue |
| Atomic operation ordering | Hardware atomic ordering | `LDXR`/`STXR` with acquire/release |

**Weak Ordering Implications:**

AArch64's weak ordering means:

1. **No Guaranteed Ordering**: Without synchronization, memory operations can be reordered:
   ```
   // These may be reordered on AArch64:
   write_ref(x, 1)  // May appear after write_ref(y, 2) to other cores
   write_ref(y, 2)
   ```

2. **Acquire/Release Ordering**: Acquire/release pairs establish ordering:
   ```
   // Actor 1: Release
   write_ref(data, value)
   atomic_store(flag, true, release)  // STLR ensures prior writes visible
   
   // Actor 2: Acquire
   ready: boolean <- atomic_load(flag, acquire)  // LDAR ensures sees release
   value: int <- read_ref(data)  // Guaranteed to see write_ref(data, value)
   ```

3. **Sequential Consistency**: Sequential consistency requires full barriers:
   ```
   // seq_cst operations use DMB ISH + LDAR/STLR
   atomic_store(x, 1, seq_cst)  // DMB ISH + STLR
   atomic_load(y, seq_cst)      // LDAR + DMB ISH
   ```

**Cache Coherency Guarantees:**

AArch64 provides cache-coherent shared memory:

- **MESI Protocol**: Modified/Exclusive/Shared/Invalid states ensure cache coherency
- **MOESI Extensions**: Some AArch64 implementations use MOESI (adds Owned state)
- **Automatic Coherency**: Cache coherency is automatic - no explicit cache flushing required for normal memory
- **Cross-Core Visibility**: Writes become visible to other cores via cache coherency protocols

**Release Consistency Model:**

AArch64 implements a release consistency model:

- **Acquire Operations**: `LDAR` (load-acquire) ensures all subsequent memory operations see effects from operations before matching release stores
- **Release Operations**: `STLR` (store-release) ensures all prior memory operations are visible before the store completes
- **Acquire-Release Pairs**: Establish happens-before relationships between cores
- **One-Way Synchronization**: Acquire/release provides efficient one-way synchronization (no full barrier required)

**Silica Guarantees on AArch64:**

1. **Per-Actor Sequential Consistency**: 
   - Guaranteed by single-core sequential consistency (natural AArch64 behavior)
   - No explicit synchronization required within an actor

2. **Cross-Actor Ordering**:
   - Requires explicit synchronization (acquire/release or seq_cst)
   - Implemented via `LDAR`/`STLR` instructions
   - Hardware ensures ordering guarantees

3. **Actor Message Ordering**:
   - Per-actor sequential consistency (single-core ordering)
   - Message queue FIFO ordering (runtime guarantee)
   - Combined: Messages are processed in order within each actor

4. **Atomic Operation Ordering**:
   - Hardware atomic operations via `LDXR`/`STXR`
   - Acquire/release semantics via `LDAXR`/`STLXR`
   - Hardware ensures atomicity and ordering

**Example: AArch64 Ordering Guarantees**

```silica
// Actor 1: Producer
fn producer_behavior(msg: unit, state: ProducerState) -> ProducerState proc[mem(normal), atomic] {
    // Prepare data
    write_ref(state.data, 42);
    
    // Release: ensures data write is visible before flag write
    atomic_store(state.ready_flag, true, release);  // STLR instruction
    
    // Send message (happens after release)
    send(consumer_ref, DataReadyMsg {});
    state
}

// Actor 2: Consumer
fn consumer_behavior(msg: DataReadyMsg, state: ConsumerState) -> ConsumerState proc[mem(normal), atomic] {
    // Acquire: guaranteed to see release from producer
    ready: boolean <- atomic_load(shared_flag, acquire);  // LDAR instruction
    
    // All data prepared by producer is now visible
    // This read is guaranteed to see write_ref(state.data, 42)
    value: int <- read_ref(shared_data);
    
    state
}
```

The AArch64 hardware guarantees:
- `STLR` in producer ensures `write_ref(state.data, 42)` is visible before `atomic_store(state.ready_flag, true, release)`
- `LDAR` in consumer ensures it sees the `STLR` and all prior writes
- `read_ref(shared_data)` is guaranteed to see `write_ref(state.data, 42)`

#### 18.2.3.1 Memory Barrier Instruction Mapping

Silica's memory ordering guarantees are implemented using AArch64 memory barrier instructions. The compiler maps language-level memory orderings to specific AArch64 barrier instructions and atomic instruction variants.

**Memory Ordering to AArch64 Instruction Mapping:**

| Silica Ordering | AArch64 Instructions | Barrier Type | Description |
|----------------|---------------------|--------------|-------------|
| `relaxed` | `LDXR`/`STXR` (no barriers) | None | Atomic operations only, no ordering guarantees |
| `acquire` | `LDAR` or `LDAXR` | Load-acquire | Ensures subsequent operations see prior release stores |
| `release` | `STLR` or `STLXR` | Store-release | Ensures prior operations visible before store completes |
| `acq_rel` | `LDAXR`/`STLXR` | Load-acquire + Store-release | Combines acquire and release in RMW operations |
| `seq_cst` | `LDAR`/`STLR` + `DMB ISH` | Full barrier | Global ordering with full memory barrier |

**AArch64 Memory Barrier Instructions:**

**DMB (Data Memory Barrier):**

The `DMB` instruction ensures memory operations before the barrier complete before operations after the barrier:

- **DMB SY**: Full system barrier (all memory operations)
- **DMB ISH**: Inner Shareable domain barrier (all cores in inner shareable domain)
- **DMB ISHST**: Inner Shareable domain store barrier (stores only)
- **DMB ISHLD**: Inner Shareable domain load barrier (loads only)
- **DMB OSH**: Outer Shareable domain barrier
- **DMB OSHST**: Outer Shareable domain store barrier
- **DMB OSHLD**: Outer Shareable domain load barrier

**DSB (Data Synchronization Barrier):**

The `DSB` instruction ensures all memory operations complete before subsequent instructions execute:

- **DSB SY**: Full system synchronization barrier
- **DSB ISH**: Inner Shareable domain synchronization barrier
- **DSB ISHST**: Inner Shareable domain store synchronization barrier
- **DSB ISHLD**: Inner Shareable domain load synchronization barrier

**ISB (Instruction Synchronization Barrier):**

The `ISB` instruction ensures all previous instructions complete before subsequent instructions are fetched:

- **ISB**: Instruction synchronization barrier (flushes pipeline)

**Atomic Instruction Variants:**

AArch64 provides atomic instruction variants with built-in acquire/release semantics:

- **LDAR** (Load-Acquire Register): Load with acquire semantics
- **STLR** (Store-Release Register): Store with release semantics
- **LDAXR** (Load-Acquire Exclusive Register): Load-acquire for atomic RMW operations
- **STLXR** (Store-Release Exclusive Register): Store-release for atomic RMW operations
- **LDXR** (Load Exclusive Register): Load for atomic RMW (no ordering)
- **STXR** (Store Exclusive Register): Store for atomic RMW (no ordering)

**Compiler Code Generation Strategy:**

The compiler generates AArch64 instructions based on memory ordering requirements:

**1. Relaxed Ordering (`relaxed`):**

```silica
atomic_store(ptr, value, relaxed)
```

Generated code:
```assembly
// No barriers, just atomic store
LDXR  W0, [X1]      // Load exclusive (no ordering)
STXR  W2, W0, [X1]  // Store exclusive (no ordering)
CBNZ  W2, retry      // Retry if store failed
```

**2. Acquire Ordering (`acquire`):**

```silica
value: int <- atomic_load(ptr, acquire)
```

Generated code:
```assembly
// Load-acquire instruction provides acquire semantics
LDAR  W0, [X1]      // Load-acquire: ensures subsequent operations see prior releases
```

**3. Release Ordering (`release`):**

```silica
atomic_store(ptr, value, release)
```

Generated code:
```assembly
// Store-release instruction provides release semantics
STLR  W0, [X1]      // Store-release: ensures prior operations visible before store
```

**4. Acquire-Release Ordering (`acq_rel`):**

```silica
old_value: int <- atomic_fetch_add(ptr, 1, acq_rel)
```

Generated code:
```assembly
loop:
  LDAXR W0, [X1]    // Load-acquire: see prior releases
  ADD   W0, W0, #1  // Modify value
  STLXR W2, W0, [X1] // Store-release: make modification visible
  CBNZ  W2, loop     // Retry if store failed
```

**5. Sequential Consistency (`seq_cst`):**

```silica
atomic_store(ptr, value, seq_cst)
```

Generated code:
```assembly
// Full barrier before store-release for global ordering
DMB   ISH           // Ensure all prior operations complete
STLR  W0, [X1]      // Store-release with global ordering
DMB   ISH           // Ensure store completes before subsequent operations
```

For `atomic_load` with `seq_cst`:

```assembly
LDAR  W0, [X1]      // Load-acquire
DMB   ISH           // Ensure load completes before subsequent operations
```

**Barrier Selection Guidelines:**

The compiler selects barriers based on:

1. **Ordering Requirements**: `seq_cst` requires full barriers (`DMB ISH`), while `acquire`/`release` use instruction semantics
2. **Domain Scope**: Inner Shareable (`ISH`) for normal multi-core systems, Outer Shareable (`OSH`) for system-level synchronization
3. **Performance**: Minimize barriers - use instruction semantics (`LDAR`/`STLR`) when possible instead of explicit barriers
4. **Correctness**: Always ensure ordering guarantees match language semantics

#### 18.2.3.2 Memory Barrier Instruction Selection Strategy

The compiler implements a sophisticated strategy for selecting appropriate memory barrier instructions based on synchronization requirements, performance constraints, and AArch64 hardware characteristics.

**DMB vs DSB vs ISB Selection:**

The compiler selects between DMB (Data Memory Barrier), DSB (Data Synchronization Barrier), and ISB (Instruction Synchronization Barrier) based on synchronization requirements:

- **DMB (Data Memory Barrier)**: Used when ordering memory operations is required but subsequent instructions can execute before memory operations complete
  - **Use Case**: Ordering memory operations for acquire/release semantics, sequential consistency
  - **Performance**: Minimal overhead - allows instruction execution to continue while memory operations complete
  - **When to Use**: Most common barrier type for memory ordering in multi-core systems
  
- **DSB (Data Synchronization Barrier)**: Used when all memory operations must complete before subsequent instructions execute
  - **Use Case**: Ensuring memory operations complete before device access, cache maintenance, or system configuration
  - **Performance**: Higher overhead - blocks instruction execution until memory operations complete
  - **When to Use**: Device driver code, cache maintenance operations, system configuration changes
  
- **ISB (Instruction Synchronization Barrier)**: Used when instruction fetch must see effects of prior operations
  - **Use Case**: Ensuring instruction fetch sees memory writes (e.g., self-modifying code, code loading)
  - **Performance**: Highest overhead - flushes instruction pipeline
  - **When to Use**: Code loading (`hot_swap` effect), self-modifying code, security-sensitive instruction updates

**Effect-to-Barrier Mapping:**

- `device_io` (print, file I/O, console read): Syscall-based I/O; kernel handles ordering; typically no explicit barriers required in user space
- `network_io`: Network communications; kernel handles device access; typically no explicit barriers required in user space
- `hot_swap`: Code loading requires `ISB` to ensure instruction fetch sees code writes
- `register_rwr`: Direct device register access via mmap requires `DSB SY` before and `ISB` after for device ordering

**Barrier Scope Selection:**

The compiler selects barrier scope based on synchronization domain requirements:

- **SY (System)**: Full system barrier - synchronizes with all system components (cores, devices, DMA)
  - **Use Case**: System-level synchronization, device driver code, DMA synchronization
  - **Performance**: Highest overhead - synchronizes with entire system
  - **When to Use**: Device access, DMA operations, system configuration
  
- **ISH (Inner Shareable)**: Inner shareable domain barrier - synchronizes with cores in inner shareable domain (typical multi-core systems)
  - **Use Case**: Most common barrier scope for application-level synchronization
  - **Performance**: Moderate overhead - synchronizes with cores in inner shareable domain
  - **When to Use**: Normal multi-core synchronization, actor message passing, atomic operations
  
- **OSH (Outer Shareable)**: Outer shareable domain barrier - synchronizes with cores in outer shareable domain (system-level)
  - **Use Case**: System-level synchronization, hypervisor coordination
  - **Performance**: Higher overhead - synchronizes with outer shareable domain
  - **When to Use**: System-level coordination, hypervisor interactions
  
- **NSH (Non-Shareable)**: Non-shareable domain barrier - synchronizes within single core
  - **Use Case**: Single-core synchronization, instruction ordering
  - **Performance**: Lowest overhead - synchronizes within single core only
  - **When to Use**: Single-core instruction ordering, rarely used in multi-core systems

**Barrier Type Selection (Load/Store):**

The compiler selects barrier type based on operation direction:

- **Full Barrier (DMB SY/ISH/OSH)**: Orders both loads and stores
  - **Use Case**: General memory ordering, sequential consistency
  - **Performance**: Moderate overhead - orders all memory operations
  - **When to Use**: When both load and store ordering is required
  
- **Store Barrier (DMB SYST/ISHST/OSHST)**: Orders stores only
  - **Use Case**: Ensuring stores are visible before subsequent operations
  - **Performance**: Lower overhead - orders stores only
  - **When to Use**: When only store ordering is required (less common)
  
- **Load Barrier (DMB SYLD/ISHLD/OSHLD)**: Orders loads only
  - **Use Case**: Ensuring loads see prior stores
  - **Performance**: Lower overhead - orders loads only
  - **When to Use**: When only load ordering is required (less common)

**Memory Ordering to Barrier Mapping:**

The compiler maps Silica memory orderings to specific barrier instructions:

- **relaxed**: No barriers - uses `LDXR`/`STXR` without ordering guarantees
- **acquire**: No explicit barriers - uses `LDAR`/`LDAXR` with built-in acquire semantics
- **release**: No explicit barriers - uses `STLR`/`STLXR` with built-in release semantics
- **acq_rel**: No explicit barriers - uses `LDAXR`/`STLXR` with built-in acquire-release semantics
- **seq_cst**: Full barriers - uses `DMB ISH` + `LDAR`/`STLR` for global ordering

**Barrier Selection for Different Use Cases:**

The compiler selects barriers based on specific use cases:

- **Actor Message Passing**: Uses `LDAR`/`STLR` (acquire/release) for efficient one-way synchronization
- **Atomic Operations**: Uses `LDAXR`/`STLXR` (acquire-release) for read-modify-write operations
- **Sequential Consistency**: Uses `DMB ISH` + `LDAR`/`STLR` for global ordering
- **Device Access**: Uses `DSB SY` to ensure device writes complete before subsequent operations
- **Cache Maintenance**: Uses `DSB ISH` to ensure cache operations complete before memory access
- **Code Loading**: Uses `ISB` to ensure instruction fetch sees code writes

**Performance Optimization Strategies:**

The compiler optimizes barrier usage for performance:

- **Barrier Elimination**: Eliminates redundant barriers when ordering is already guaranteed by instruction semantics
- **Barrier Fusion**: Combines multiple barriers into single barrier when possible
- **Barrier Hoisting**: Moves barriers outside loops when ordering requirements permit
- **Barrier Scope Minimization**: Uses narrowest scope (ISH) when possible instead of system scope (SY)

**Barrier Selection Best Practices:**

The compiler follows best practices for barrier selection:

- **Minimize Barriers**: Use instruction semantics (`LDAR`/`STLR`) instead of explicit barriers when possible
- **Narrow Scope**: Use inner shareable (ISH) scope instead of system (SY) scope when possible
- **Correct Ordering**: Ensure barrier ordering matches language semantics requirements
- **Performance Awareness**: Consider performance impact when selecting barrier type and scope

**Detailed Barrier Selection Tables:**

The following tables provide comprehensive guidance for selecting appropriate barrier instructions based on synchronization requirements:

**Table 1: Barrier Type Selection**

| Synchronization Requirement | Barrier Type | Instruction | Use Case | Performance Impact |
|---------------------------|--------------|------------|---------|-------------------|
| Order memory operations, allow instruction execution to continue | DMB | `DMB ISH` | Normal acquire/release semantics, actor message passing | ~10-50 cycles |
| Ensure all memory operations complete before continuing | DSB | `DSB ISH` | Device access, cache maintenance, system configuration | ~50-200 cycles |
| Ensure instruction fetch sees prior memory writes | ISB | `ISB` | Code loading, self-modifying code, security updates | ~100-500 cycles (pipeline flush) |
| Acquire semantics (load ordering) | Load-Acquire | `LDAR`, `LDAXR` | Read synchronization, actor message reception | Minimal (hardware-supported) |
| Release semantics (store ordering) | Store-Release | `STLR`, `STLXR` | Write synchronization, actor message sending | Minimal (hardware-supported) |

**Table 2: Barrier Scope Selection**

| Synchronization Domain | Scope | Instruction Variant | Use Case | Performance Impact |
|----------------------|-------|-------------------|---------|-------------------|
| Full system (cores, devices, DMA) | SY | `DMB SY`, `DSB SY` | Device drivers, DMA synchronization, system configuration | Highest overhead |
| Inner shareable (typical multi-core) | ISH | `DMB ISH`, `DSB ISH` | Application-level synchronization, actors, atomics | Moderate overhead (most common) |
| Outer shareable (system-level) | OSH | `DMB OSH`, `DSB OSH` | Hypervisor coordination, system-level synchronization | Higher overhead |
| Single core only | NSH | `DMB NSH` | Single-core instruction ordering | Lowest overhead (rarely used) |

**Table 3: Barrier Direction Selection**

| Operation Direction | Barrier Variant | Instruction | Use Case | Performance Impact |
|-------------------|----------------|-------------|---------|-------------------|
| Both loads and stores | Full barrier | `DMB ISH` | General memory ordering, sequential consistency | Moderate overhead |
| Stores only | Store barrier | `DMB ISHST` | Ensure stores visible before subsequent operations | Lower overhead |
| Loads only | Load barrier | `DMB ISHLD` | Ensure loads see prior stores | Lower overhead |

**Table 4: Memory Ordering to Barrier Mapping**

| Silica Memory Ordering | AArch64 Instruction Sequence | Barrier Requirements | Performance Characteristics |
|----------------------|----------------------------|---------------------|--------------------------|
| `relaxed` | `LDXR`/`STXR` | No barriers | Minimal overhead, no ordering guarantees |
| `acquire` | `LDAR`/`LDAXR` | No explicit barriers (built-in acquire) | Minimal overhead, hardware-supported |
| `release` | `STLR`/`STLXR` | No explicit barriers (built-in release) | Minimal overhead, hardware-supported |
| `acq_rel` | `LDAXR`/`STLXR` | No explicit barriers (built-in acquire-release) | Minimal overhead, hardware-supported |
| `seq_cst` | `DMB ISH` + `LDAR`/`STLR` | Full barrier for global ordering | Moderate overhead (~10-50 cycles for barrier) |

**Table 5: Use Case to Barrier Selection**

| Use Case | Recommended Barrier | Alternative | Rationale |
|---------|-------------------|-------------|----------|
| Actor message passing | `LDAR`/`STLR` | `DMB ISH` | Hardware-supported acquire/release, minimal overhead |
| Atomic read-modify-write | `LDAXR`/`STLXR` | `DMB ISH` + `LDXR`/`STXR` | Hardware-supported acquire-release semantics |
| Sequential consistency | `DMB ISH` + `LDAR`/`STLR` | `DSB ISH` + `LDAR`/`STLR` | Full ordering with minimal barrier overhead |
| Device register access | `DSB SY` | `DMB SY` | Ensure device writes complete before continuing |
| Cache maintenance | `DSB ISH` | `DMB ISH` | Ensure cache operations complete before memory access |
| Code loading (`hot_swap` effect) | `ISB` | `DSB SY` + `ISB` | Ensure instruction fetch sees code writes |
| Cross-NUMA synchronization | `DMB ISH` | `DMB SY` | Inner shareable sufficient for NUMA domains |
| Single-core ordering | `DMB NSH` | No barrier (if ordering not required) | Minimal overhead for single-core scenarios |

**Table 6: Barrier Performance Characteristics**

| Barrier Instruction | Typical Latency | Pipeline Impact | Cache Impact | When to Use |
|-------------------|----------------|----------------|--------------|-------------|
| `LDAR`/`STLR` | 1-2 cycles | None | Minimal | Most common synchronization |
| `DMB ISH` | 10-50 cycles | Pipeline stall | Cache coherency | General memory ordering |
| `DSB ISH` | 50-200 cycles | Pipeline stall until memory completes | Cache coherency | Device access, cache maintenance |
| `ISB` | 100-500 cycles | Full pipeline flush | Instruction cache | Code loading, security updates |
| `DMB SY` | 20-100 cycles | Pipeline stall | System-wide coherency | Device drivers, DMA |
| `DSB SY` | 100-500 cycles | Pipeline stall until system memory completes | System-wide coherency | Critical device operations |

**Table 7: Barrier Elimination Opportunities**

| Scenario | Can Eliminate Barrier? | Reason | Alternative |
|---------|----------------------|--------|------------|
| `LDAR` followed by `LDAR` | Yes | Second `LDAR` already has acquire semantics | Use single `LDAR` |
| `STLR` followed by `STLR` | Yes | First `STLR` already provides release ordering | Use single `STLR` |
| `LDAR` followed by `DMB ISH` | Yes | `LDAR` already provides acquire ordering | Remove `DMB ISH` |
| `DMB ISH` followed by `LDAR` | Partial | `LDAR` provides acquire, but `DMB ISH` may be needed for store ordering | Evaluate if store ordering needed |
| `STLR` followed by `DMB ISH` | Yes | `STLR` already provides release ordering | Remove `DMB ISH` |
| `DMB ISH` followed by `STLR` | Partial | `STLR` provides release, but `DMB ISH` may be needed for load ordering | Evaluate if load ordering needed |

**Cross-References:**
- See Section 17.2.1 (Ordering Levels) for memory ordering semantics
- See Section 17.2.6 (AArch64 Implementation Details) for atomic instruction mapping
- See Section 18.2.3.1 (Memory Barrier Instruction Mapping) for instruction mapping details
- See Section 12.1.1.1 (AArch64 Memory Attribute Mapping) for memory space configuration
- See Section 15.1.2.1 (AArch64 Runtime Integration) for actor synchronization barrier usage
- See Section 16.1.1 (Asynchronous Send) for message passing barrier requirements

**Example: Producer-Consumer with Barriers**

```silica
// Producer (Actor 1)
fn producer(msg: unit, state: ProducerState) -> ProducerState proc[mem(normal), atomic] {
    write_ref(state.data, 42);
    atomic_store(state.ready_flag, true, release);  // STLR instruction
    state
}
```

Generated code for producer:
```assembly
// write_ref(state.data, 42)
STR   W2, [X1]      // Store data (normal store, no ordering)

// atomic_store(state.ready_flag, true, release)
MOV   W0, #1
STLR  W0, [X3]      // Store-release: ensures data store visible before flag store
```

```silica
// Consumer (Actor 2)
fn consumer(msg: unit, state: ConsumerState) -> ConsumerState proc[mem(normal), atomic] {
    ready: boolean <- atomic_load(state.ready_flag, acquire);  // LDAR instruction
    if ready {
        value: int <- read_ref(state.data);  // Guaranteed to see producer's write
    }
    state
}
```

Generated code for consumer:
```assembly
// atomic_load(state.ready_flag, acquire)
LDAR  W0, [X3]      // Load-acquire: ensures sees producer's release
CBNZ  W0, read_data // Branch if ready

read_data:
// read_ref(state.data)
LDR   W1, [X1]      // Load data (guaranteed to see producer's write due to LDAR)
```

**Performance Considerations:**

- **LDAR/STLR Overhead**: Minimal overhead compared to explicit barriers (hardware-supported)
- **DMB ISH Overhead**: ~10-50 cycles per barrier (pipeline flush)
- **Barrier Minimization**: Compiler optimizes by using instruction semantics instead of explicit barriers when possible
- **Cache Effects**: Barriers ensure cache coherency but may cause cache line invalidation

### 18.3 Race Condition Prevention

#### 18.3.1 Atomic Operations
Atomic operations prevent data races:

```
counter: atomic_ref(R, normal, int) <- alloc_atomic(region, 0)

// Multiple actors can safely increment
fn increment_counter() {
    atomic_fetch_add(counter, 1, seq_cst)
}
```

#### 18.3.2 Actor Isolation
Actors cannot directly share mutable state:

```
// This is not possible in Silica
actor1_state = actor2.state.field  // ✗ No shared state access
```

#### 18.3.3 Message Immutability
Messages cannot be mutated after sending:

```
mutable_data = {value: 42}
send(actor, mutable_data)
// Cannot modify mutable_data.value here
// Actor receives immutable copy
```

### 18.4 Deadlock Freedom

#### 18.4.1 No Blocking Sends
Send operations never block - no send-side deadlocks.

#### 18.4.2 Actor Autonomy
Actors process messages independently - no circular wait conditions.

#### 18.4.3 Atomic Operation Atomicity
Atomic RMW operations are indivisible - no partial update deadlocks.

### 18.5 Performance Guarantees

#### 18.5.1 Lock-Free Algorithms
Atomic operations enable lock-free data structures:

```
// SPSC queue: no locks, wait-free for single producer/consumer
// MPSC queue: lock-free, wait-free for producers
```

#### 16.5.2 Composable Concurrency
Actors compose without synchronization overhead:

```
// Independent actors scale linearly
// No global locks or shared state bottlenecks
```

#### 18.5.3 Hardware Utilization
Direct mapping to AArch64 concurrency features:

```
load acquire  → LDAR (load-acquire)
store release → STLR (store-release)
RMW operations → LDXR/STXR loops with barriers
```

## 19. Module System

### 19.1 Module Structure

#### 19.1.1 Filename-Based Modules
Modules are implicitly created from source file names. A file named `math_utils.silica` automatically creates a module named `math_utils`. No explicit module declarations are required in the source code.

#### 19.1.2 Module Naming
- Module names are derived from the filename (without the `.silica` extension)
- Files must have the `.silica` extension
- Module names follow identifier rules: letters, digits, underscores, starting with a letter
- Examples:
  - `math_utils.silica` → module `math_utils`
  - `collections.silica` → module `collections`
  - `io_network.silica` → module `io_network`

#### 17.1.3 File Organization
Modules are organized through file system structure and search paths:

```
project/
├── main.silica          // module 'main'
├── math_utils.silica    // module 'math_utils'
└── utils/
    ├── string.silica    // module 'string'
    └── list.silica      // module 'list'
```

### 19.2 Export System

#### 19.2.1 Export Declarations
Functions are exported using the `export` keyword with function name and arity:

```
export add/2, multiply/2, factorial/1;
```

- Functions must be defined in the same module to be exported
- Arity specifies the number of parameters (e.g., `add/2` for binary addition)
- Only exported functions are visible to importing modules
- All exported symbols are available to importers (no selective imports)

#### 19.2.2 Export Validation
The compiler validates that:
- All exported symbols exist in the module
- Arities match the actual function definitions
- No duplicate exports in the same module

### 19.3 Import System

#### 19.3.1 Module Imports
Import modules using the `use` keyword with comma-separated module names:

```
use math_utils;                    // import single module
use collections, io, string;      // import multiple modules
```

- All exported functions from imported modules become available in the current scope
- No selective imports - all exports are imported
- No module renaming - imported modules are accessed by their original names
- Imports must appear at the top level of a module (before any function definitions)

#### 19.3.2 Name Resolution
Imported functions are accessed directly by name:

```
use math_utils;

fn main() -> int {
    do
        result:int <- add(3, 4);   // 'add' from math_utils module
        multiply(result, 2)        // 'multiply' from math_utils module
    end
}
```

#### 19.3.3 Name Conflicts

**Conflict Detection:**

- If two imported modules export functions with the same name, it's a compiler error
- Variable shadowing is not allowed; attempting to shadow a variable causes a compilation error
- Explicit qualification is not supported - conflicts must be resolved by renaming or restructuring

**Conflict Detection Order:**

When multiple modules are imported in a single `use` statement, conflicts are detected as follows:

1. **Module Processing Order**: Modules are processed in the order they appear in the `use` statement:
   ```silica
   use module_a, module_b, module_c;
   ```
   Processing order: `module_a` → `module_b` → `module_c`

2. **Conflict Detection**: All conflicts are detected and reported together, not just the first conflict:
   - The compiler collects all exported symbols from all imported modules
   - It identifies all name conflicts across all modules
   - All conflicts are reported in a single error message

3. **Conflict Reporting**: When conflicts are detected, the error message includes:
   - All conflicting function names
   - The modules that export each conflicting name
   - The order in which modules were processed

**Conflict Detection Algorithm:**

```pseudocode
function detect_name_conflicts(imported_modules):
    // Collect all exported symbols from all modules
    all_symbols = {}
    module_symbols = {}
    
    for each module in imported_modules:
        exports = get_module_exports(module)
        module_symbols[module] = exports
        
        for each symbol in exports:
            if symbol.name not in all_symbols:
                all_symbols[symbol.name] = []
            all_symbols[symbol.name].append({module: module, symbol: symbol})
        end for
    end for
    
    // Detect conflicts
    conflicts = []
    for each symbol_name, symbol_list in all_symbols:
        if length(symbol_list) > 1:
            // Multiple modules export the same name
            conflicts.append({
                name: symbol_name,
                modules: [s.module for s in symbol_list],
                symbols: symbol_list
            })
        end if
    end for
    
    // Report all conflicts together
    if conflicts:
        error("Name conflicts detected: " + format_conflicts(conflicts))
    end if
end function
```

**Conflict Detection Example:**

```silica
// math.silica
export add/2, multiply/2;

// string.silica
export add/2, concat/2;

// collections.silica
export add/2, remove/1;

// main.silica
use math, string, collections;  // Error: add/2 conflict across 3 modules
```

**Error Message Format:**

```
❌ Compilation error: NameConflict error at main.silica:1:1 [E4001]

Name conflict detected: function 'add/2' is exported by multiple modules.
Conflicting modules (in import order):
  - math
  - string
  - collections

See specification: spec:§19.3.3

<!-- SILICA-ERROR-METADATA
{
  "@context": "https://aalang.dev/silica-dev/error/",
  "errorCode": "E4001",
  "errorType": "error",
  "severity": "error",
  "location": {
    "file": "main.silica",
    "line": 1,
    "column": 1,
    "offset": 0
  },
  "specification": {
    "section": "§19.3.3"
  },
  "conflict": {
    "symbol": "add/2",
    "conflicting_modules": ["math", "string", "collections"],
    "import_order": ["math", "string", "collections"]
  }
}
-->
```

**Multiple Conflicts:**

If multiple name conflicts exist, all are reported:

```silica
// math.silica
export add/2, subtract/2;

// string.silica
export add/2, subtract/2;  // Both add/2 and subtract/2 conflict

// main.silica
use math, string;  // Error: 2 conflicts reported together
```

**Error Message for Multiple Conflicts:**

```
❌ Compilation error: NameConflict error at main.silica:1:1 [E4001]

Multiple name conflicts detected:
1. Function 'add/2' is exported by: math, string
2. Function 'subtract/2' is exported by: math, string

See specification: spec:§19.3.3
```

**Conflict Resolution Strategies:**

When name conflicts occur, developers can resolve them using the following strategies:

**Strategy 1: Module Restructuring**

Restructure modules to avoid conflicts by creating more specific module names:

**Before (Conflict):**
```silica
// math.silica
export add/2, multiply/2;

// string.silica
export add/2;  // Conflict: add/2 already exported by math

// main.silica
use math, string;  // Error: add/2 conflict
```

**After (Resolved):**
```silica
// math_utils.silica
export add/2, multiply/2;

// string_utils.silica
export concat/2;  // Renamed from add/2 to concat/2

// main.silica
use math_utils, string_utils;  // No conflict
```

**Strategy 2: Function Renaming**

Rename conflicting functions in one of the modules:

**Before (Conflict):**
```silica
// collections.silica
export add/2;  // Add element to collection

// math.silica
export add/2;  // Add two numbers

// main.silica
use collections, math;  // Error: add/2 conflict
```

**After (Resolved):**
```silica
// collections.silica
export add_element/2;  // Renamed to avoid conflict

// math.silica
export add/2;  // Keep original name

// main.silica
use collections, math;  // No conflict: add_element/2 vs add/2
```

**Strategy 3: Module Hierarchy (Future Consideration)**

While Silica doesn't currently support module hierarchies, future versions may support:

```silica
// Proposed future syntax (not currently supported)
use math::arithmetic;
use math::geometry;
use string::operations;
```

**Strategy 4: Selective Import Pattern**

Create wrapper modules that selectively re-export functions:

**Before (Conflict):**
```silica
// io.silica
export read/1, write/2;

// network.silica
export read/1, send/2;  // Conflict: read/1

// main.silica
use io, network;  // Error: read/1 conflict
```

**After (Resolved):**
```silica
// io.silica
export read_file/1, write_file/2;

// network.silica
export read_socket/1, send/2;

// main.silica
use io, network;  // No conflict: read_file/1 vs read_socket/1
```

**Naming Conventions to Avoid Conflicts:**

1. **Use Descriptive Names**: Prefer `add_numbers/2` over `add/2` when context is unclear
2. **Use Module Prefixes**: Consider prefixing functions with module context (e.g., `math_add/2`, `string_add/2`)
3. **Use Namespace-Style Names**: Use underscores to create logical namespaces (e.g., `list_add/2`, `set_add/2`)

**Common Conflict Patterns:**

**Pattern 1: Generic Function Names**

Avoid overly generic names that are likely to conflict:

```silica
// Bad: Too generic
export add/2, get/1, set/2, create/1

// Good: More specific
export add_element/2, get_value/1, set_value/2, create_instance/1
```

**Pattern 2: Standard Library Names**

Avoid conflicts with standard library function names:

```silica
// Bad: Conflicts with stdlib
export print/1, read/1, write/2

// Good: More specific
export print_debug/1, read_config/1, write_log/2
```

**Pattern 3: Common Operations**

Common operations (add, remove, get, set) are prone to conflicts:

```silica
// Bad: Common names
export add/2, remove/1, get/1

// Good: Context-specific names
export add_to_list/2, remove_from_set/1, get_map_value/1
```

**Conflict Resolution Workflow:**

1. **Identify Conflict**: Compiler reports conflict with module and function names
2. **Analyze Usage**: Determine which module's function is more commonly used
3. **Choose Strategy**: Select appropriate resolution strategy (restructuring, renaming, etc.)
4. **Refactor**: Apply chosen strategy to resolve conflict
5. **Verify**: Recompile to ensure conflict is resolved

**Example: Complete Conflict Resolution**

**Initial State (Conflict):**
```silica
// calculator.silica
export add/2, subtract/2, multiply/2;

// list_ops.silica
export add/2, remove/1;  // Conflict: add/2

// main.silica
use calculator, list_ops;  // Error: add/2 conflict
```

**Resolution:**
```silica
// calculator.silica (unchanged)
export add/2, subtract/2, multiply/2;

// list_ops.silica (renamed function)
export append/2, remove/1;  // Renamed add/2 to append/2

// main.silica (no conflict)
use calculator, list_ops;

fn example() -> int64 {
    do
        // Use calculator.add/2
        sum: int64 <- add(3, 4);
        
        // Use list_ops.append/2 (renamed from add/2)
        // (list operations would use append/2)
        sum
    end
}
```

**Best Practices:**

1. **Plan Module Structure**: Design module structure to minimize conflicts
2. **Use Descriptive Names**: Choose function names that clearly indicate purpose
3. **Document Naming Conventions**: Establish team naming conventions to avoid conflicts
4. **Review Dependencies**: Review module dependencies before finalizing function names
5. **Refactor Early**: Resolve conflicts as soon as they're detected

#### 19.3.4 Module System Design Principles
Silica's module system is designed to be:
- **Simple**: No complex hierarchical namespaces or selective imports
- **Explicit**: All exports and imports are clearly declared
- **Safe**: Name conflicts are caught at compile time
- **Scalable**: Separate compilation with proper dependency tracking

### 19.4 Module Dependencies

#### 19.4.1 Dependency Resolution
Modules can depend on other modules through imports:

```
// math_utils.silica (module name: math_utils)
export add/2, multiply/2;

fn add(x: int, y: int) -> int { x + y }
fn multiply(x: int, y: int) -> int { x * y }

// main.silica (module name: main)
use math_utils;

fn main() -> int {
    do
        sum:int <- add(3, 4);        // uses function from math_utils
        product:int <- multiply(sum, 2);  // uses another function from math_utils
        product
    end
}
```

#### 17.4.2 Compilation Process
The compiler handles multi-module programs as follows:

1. **Module Discovery**: Scan all source files and extract module names from filenames
2. **Import Resolution**: For each `use` declaration, locate the corresponding `.silica` file in search paths
3. **Dependency Analysis**: Build a dependency graph from import relationships
4. **Type Checking**: Check all modules together to resolve cross-module references
5. **Code Generation**: Generate LLVM IR for all modules with proper function linkages

#### 19.4.3 Example Project Structure
```
my_project/
├── silica-comp -I modules -I stdlib main.silica
├── main.silica           // Entry point module
├── modules/
│   ├── math_utils.silica // Math utilities
│   └── collections.silica // Data structures
└── stdlib/
    ├── io.silica         // Input/output
    └── string.silica     // String operations
```

#### 19.4.4 Compilation Order
Modules are compiled in dependency order:
1. Parse all module files
2. Build dependency graph from import declarations
3. Compile modules with no dependencies first
4. Compile dependent modules after their dependencies

#### 19.4.5 Module Dependency Resolution Algorithm

The compiler uses a topological sorting algorithm to determine module compilation order:

**Algorithm:**
1. **Build Dependency Graph**: For each module, create edges from the module to all modules it imports (`use` declarations)
2. **Topological Sort**: Order modules such that dependencies come before dependents
3. **Cycle Detection**: If a cycle is detected, report an error (circular dependencies are not allowed)
4. **Compilation Order**: Compile modules in topological order (leaves first, roots last)

**Example:**
```
Module dependencies:
  main.silica → math.silica
  main.silica → io.silica
  math.silica → utils.silica
  io.silica → utils.silica

Dependency graph:
  utils.silica (no dependencies)
    ↓
  math.silica, io.silica (depend on utils)
    ↓
  main.silica (depends on math and io)

Compilation order:
  1. utils.silica
  2. math.silica, io.silica (can be compiled in parallel)
  3. main.silica
```

**Implementation Details:**
- The algorithm uses depth-first search (DFS) with cycle detection
- Modules with no dependencies are compiled first
- Parallel compilation is possible for modules at the same level in the dependency tree
- All modules are compiled together in a single compilation unit for cross-module optimization

**Parallel Compilation Strategy:**

The compiler supports parallel compilation of independent modules while maintaining correctness through phase-based execution. Parallelization is constrained by data dependencies between compilation phases.

**Compilation Phases:**

The compilation process consists of three sequential phases:

1. **Parse Phase**: Lexical analysis and syntax parsing
2. **Type Check Phase**: Type checking and effect checking (requires all module ASTs)
3. **Codegen Phase**: LLVM IR generation and optimization

**Phase-by-Phase Parallelization Rules:**

**Phase 1: Parse Phase (Fully Parallelizable)**

- **Parallelization**: All modules can be parsed in parallel
- **Dependencies**: No dependencies between modules for parsing
- **Constraints**: Each module file can be parsed independently
- **Implementation**: Parse all modules concurrently using thread pool

```pseudocode
// Parse phase: fully parallel
for each module_file in parallel:
    ast = parse(module_file)
    module_asts[module_name] = ast
```

**Phase 2: Type Check Phase (Sequential, Cross-Module Dependencies)**

- **Parallelization**: Cannot be parallelized - requires all module ASTs
- **Dependencies**: Type checking requires:
  - All module ASTs (from parse phase)
  - Cross-module type information (imports, exports)
  - Trait implementations across modules
  - Effect declarations across modules
- **Constraints**: Must process all modules together for cross-module type checking
- **Implementation**: Single-threaded type checking with access to all module ASTs

```pseudocode
// Type check phase: sequential (cross-module dependencies)
type_check_all_modules(module_asts, dependency_graph):
    // Build global type environment from all modules
    global_env = build_global_type_environment(module_asts)
    
    // Type check each module (can access all module types)
    for each module in topological_order(dependency_graph):
        type_check_module(module, global_env)
    
    // Verify cross-module type consistency
    verify_cross_module_types(global_env)
```

**Phase 3: Codegen Phase (Parallelizable with Constraints)**

- **Parallelization**: Can be parallelized for modules at the same dependency level
- **Dependencies**: Codegen requires:
  - Type-checked ASTs from all modules
  - Cross-module function signatures (for calls)
  - Trait method dispatch information
- **Constraints**: 
  - Modules at the same dependency level can be codegen'd in parallel
  - Modules with dependencies must wait for dependent modules to complete codegen
- **Implementation**: Parallel codegen for modules at same level, sequential for dependencies

```pseudocode
// Codegen phase: parallel with dependency constraints
codegen_all_modules(type_checked_asts, dependency_graph):
    // Process modules level by level
    for each level in dependency_levels(dependency_graph):
        // Parallel codegen for modules at this level
        for each module in level in parallel:
            llvm_module = codegen_module(module, type_checked_asts[module])
            llvm_modules[module] = llvm_module
        
        // Wait for all modules at this level to complete
        synchronize()
    
    // Link all LLVM modules together
    final_module = link_llvm_modules(llvm_modules)
```

**Parallel Compilation Timeline Example:**

Given the dependency graph:
```
utils.silica (no dependencies)
  ↓
math.silica, io.silica (depend on utils)
  ↓
main.silica (depends on math and io)
```

**Timeline:**

```
Time →
Parse Phase (parallel):
  [utils] [math] [io] [main]  (all parsed concurrently)
  └───────────────┘
  
Type Check Phase (sequential):
  [utils] → [math, io] → [main]  (sequential, uses all ASTs)
  └───────────────┘
  
Codegen Phase (parallel by level):
  Level 0: [utils]              (parallel)
  Level 1: [math] [io]          (parallel)
  Level 2: [main]                (sequential)
```

**Parallelization Constraints:**

1. **Parse Phase**: No constraints - fully parallel
2. **Type Check Phase**: Must be sequential - cross-module type checking requires all module information
3. **Codegen Phase**: Parallel within dependency levels, sequential across levels

**Performance Characteristics:**

- **Parse Phase**: Linear speedup with number of cores (up to number of modules)
- **Type Check Phase**: No parallelization benefit (sequential requirement)
- **Codegen Phase**: Partial parallelization (modules at same level)

**Example: 4-core system with 8 modules:**

- **Parse Phase**: 8 modules parsed in parallel (4 cores) → ~2x speedup
- **Type Check Phase**: Sequential → no speedup
- **Codegen Phase**: Modules at same level codegen'd in parallel → ~2-4x speedup (depends on dependency structure)

**Cross-Module Optimization:**

After codegen, all LLVM modules are linked together for cross-module optimization:

```pseudocode
// Cross-module optimization (sequential)
optimized_module = link_and_optimize(llvm_modules)
```

This phase:
- Links all module LLVM IR into single module
- Performs cross-module optimizations (inlining, dead code elimination)
- Generates final optimized LLVM IR

**Thread Safety:**

- **Parse Phase**: Thread-safe (each thread parses different file)
- **Type Check Phase**: Single-threaded (no thread safety concerns)
- **Codegen Phase**: Thread-safe (each thread generates different module)
- **Shared Data**: Module ASTs and type environments are read-only after construction

#### 19.4.6 Cyclic Dependencies
Cyclic module dependencies are not allowed:

```
// a.silica
use b;        // A depends on B

// b.silica
use a;        // B depends on A (creates cycle)
```
This results in a compilation error.

### 19.5 Module Search Paths

#### 19.5.1 Search Path Configuration
Module files are located using configurable search paths:

```
silica-comp --search-path ./modules --search-path ./stdlib main.silica
```

- Search paths are specified with `--search-path` or `-I` flags
- Multiple paths can be specified
- Paths are searched in the order given
- Default search path is the current directory (`.`)

#### 19.5.2 Module Resolution Algorithm
When resolving a module import:
1. Extract module name from `use module_name;` declaration
2. For each search path in order:
   - Check if `path/module_name.silica` exists
   - If found, load and parse the module
3. If not found in any search path, report compilation error

#### 19.5.3 Search Path Examples
```
Project structure:
project/
├── main.silica
├── modules/
│   ├── math.silica
│   └── io.silica
└── stdlib/
    └── collections.silica

Compilation:
cd project
silica-comp -I modules -I stdlib main.silica
```

### 19.6 Module Validation and Errors

#### 19.6.1 Module Resolution Errors
- **Module Not Found**: When a `use module_name;` declaration cannot locate `module_name.silica` in any search path
- **Invalid Module Name**: Module names must be valid identifiers
- **Circular Dependencies**: Import cycles between modules

#### 19.6.2 Export Validation Errors
- **Undefined Export**: Exporting a function that doesn't exist in the module
- **Wrong Arity**: Export arity doesn't match the actual function definition
- **Duplicate Exports**: Same function exported multiple times

#### 19.6.3 Import Validation Errors
- **Name Conflicts**: Multiple imported modules export the same function name
- **Invalid Module Reference**: Importing a module that fails to parse or type-check

#### 19.6.4 Error Examples
```
// Error: module 'nonexistent' not found in search paths
use nonexistent;

// Error: function 'divide' not defined in this module
export divide/2;

// Error: both 'math' and 'advanced_math' export 'add'
use math, advanced_math;  // if both export add/2
```

## 20. Standard Library

### 20.1 Core Types

#### 20.1.1 Option Type
Represents optional values using concrete variant types:

```
// Concrete option types for common cases
type OptionInt = Some(int) | None
type OptionString = Some(string) | None
type OptionBoolean = Some(boolean) | None
// Additional concrete types can be defined as needed

// Common Option trait for shared operations
trait OptionLike {
    fn is_some(self: Self) -> boolean;
    fn is_none(self: Self) -> boolean;
}

// Implement OptionLike for concrete option types
impl OptionLike for OptionInt {
    fn is_some(self: OptionInt) -> boolean {
        case self of
            Some(_) -> true
            None -> false
        end
    }
    
    fn is_none(self: OptionInt) -> boolean {
        case self of
            Some(_) -> false
            None -> true
        end
    }
}

// Type-specific operations
fn unwrap_int(opt: OptionInt) -> int {
    case opt of
        Some(value) -> value
        None -> panic("unwrap on None")
    end
}

fn find_index_int(list: ListInt, item: int) -> OptionInt {
    // returns Some(index) or None
    // Implementation...
}
```

**Note**: Silica uses concrete option types rather than generic types. For each element type, a corresponding option type is defined (e.g., `OptionInt` for `int`, `OptionString` for `string`). The `OptionLike` trait provides common operations across all option types.

#### 20.1.2 Result Type
Represents computation results or errors using concrete variant types:

```
// Concrete result types for common cases
type ResultIntString = Ok(int) | Error(string)
type ResultStringInt = Ok(string) | Error(int)
type ResultIntInt = Ok(int) | Error(int)
// Additional concrete types can be defined as needed

// Common Result trait for shared operations
trait ResultLike {
    fn is_ok(self: Self) -> boolean;
    fn is_error(self: Self) -> boolean;
}

// Implement ResultLike for concrete result types
impl ResultLike for ResultIntString {
    fn is_ok(self: ResultIntString) -> boolean {
        case self of
            Ok(_) -> true
            Error(_) -> false
        end
    }
    
    fn is_error(self: ResultIntString) -> boolean {
        case self of
            Ok(_) -> false
            Error(_) -> true
        end
    }
}

// Type-specific operations
fn unwrap_ok_int_string(result: ResultIntString) -> int {
    case result of
        Ok(value) -> value
        Error(_) -> panic("unwrap on Error")
    end
}

fn parse_int(s: string) -> ResultIntString {
    // returns Ok(value) or Error("invalid number")
    // Implementation...
}
```

**Note**: Silica uses concrete result types rather than generic types. For each success/error type combination, a corresponding result type is defined (e.g., `ResultIntString` for `int` success and `string` error). The `ResultLike` trait provides common operations across all result types.

#### 20.1.3 List Type
Immutable functional lists using variant types per element type:

```
// Marker trait: any type implementing this can be in a list
trait Listable {
    // Marker trait, no methods needed
}

// Implement Listable for all types that can be in lists
impl Listable for int;
impl Listable for string;
impl Listable for boolean;
// User-defined types can implement Listable trait

// Type descriptor for Listable types (used in constructor)
type ListableType = Int | String | Boolean | CustomType

// Variant types per element type (functional/immutable lists)
type ListInt = Nil | Cons(int, ListInt)
type ListString = Nil | Cons(string, ListString)
type ListBoolean = Nil | Cons(boolean, ListBoolean)
// Additional list types can be defined for other Listable types

// Common List trait for shared operations
trait List {
    fn length(self: Self) -> int;
    fn is_empty(self: Self) -> boolean;
}

// Implement List trait for concrete list types
impl List for ListInt {
    fn length(self: ListInt) -> int {
        case self of
            Nil -> 0
            Cons(_, tail) -> 1 + tail.length()
        end
    }
    
    fn is_empty(self: ListInt) -> boolean {
        case self of
            Nil -> true
            Cons(_, _) -> false
        end
    }
}

// Constructor: takes ListableType enum, compiler infers return type
fn create_list(element_type: ListableType) -> ListInt proc[mem(normal)]  // if element_type == Int
fn create_list(element_type: ListableType) -> ListString proc[mem(normal)]  // if element_type == String
// Compiler infers return type from element_type parameter

// Empty list literal syntax
fn empty_list() -> ListInt {
    Nil  // or use [] literal syntax
}

// Functional cons: adds to front, returns NEW list (immutable)
fn cons(list: ListInt, item: int) -> ListInt proc[mem(normal)] {
    // Create new list with item as head, old list as tail
    // Original list unchanged (immutable)
    Cons(item, list)
}

// Head returns concrete element type (compile-time inference)
fn head(list: ListInt) -> int {
    case list of
        Nil -> panic("head on empty list")
        Cons(head, _) -> head
    end
}

// Tail preserves type and returns new list (immutable)
fn tail(list: ListInt) -> ListInt {
    case list of
        Nil -> panic("tail on empty list")
        Cons(_, tail) -> tail
    end
}
```

**Note**: Silica uses functional/immutable lists. Lists are variant types where each element type has its own list type (e.g., `ListInt` for lists of `int`). The `Listable` trait marks types that can be stored in lists. All list operations preserve immutability - `cons` returns a new list, original lists are never modified. Type inference is compile-time only - mixing types in one list is a compile-time error.

### 20.2 Core Functions

#### 20.2.1 Arithmetic Functions
```
fn abs(x: int) -> int
fn min(a: int, b: int) -> int
fn max(a: int, b: int) -> int
fn pow(base: int, exp: int) -> int
```

#### 20.2.2 String Functions
```
fn concat(s1: string, s2: string) -> string
fn substring(s: string, start: int, len: int) -> string
fn contains(s: string, substr: string) -> boolean
```

#### 20.2.3 List Functions

List functions use concrete types (ListInt, ListString, etc.). Examples shown for ListInt - similar functions exist for other list types.

```
// ListInt functions
fn length_int(list: ListInt) -> int64
fn is_empty_int(list: ListInt) -> boolean
fn nth_int(list: ListInt, index: int64) -> OptionInt
fn append_int(list1: ListInt, list2: ListInt) -> ListInt
fn map_int_int(list: ListInt, f: fn(int64) -> int64) -> ListInt
fn map_int_string(list: ListInt, f: fn(int64) -> string) -> ListString
fn filter_int(list: ListInt, pred: fn(int64) -> boolean) -> ListInt
fn fold_int_int(list: ListInt, init: int64, f: fn(int64, int64) -> int64) -> int64

// Similar functions exist for other list types:
// ListString: length_string, append_string, map_string_string, etc.
// ListBoolean: length_boolean, append_boolean, map_boolean_boolean, etc.
// Each concrete list type has its own set of functions
```

#### 20.2.4 IO Functions
```
fn print(s: string) -> atom proc[device_io]
fn println(s: string) -> atom proc[device_io]
fn read_line() -> string proc[device_io]
```

#### 20.2.5 Debug and Assertion Functions
```
fn debug_print(value: T) -> atom proc[]        // Print any value for debugging
fn debug_println(value: T) -> atom proc[]     // Print any value with newline
fn assert(condition: boolean, message: string)  // Terminate process if condition false
    -> atom proc[]
```

**Assertion Semantics:**
Assertions check for programming errors during development and testing. When an assertion fails:
- The current process/actor terminates with an `AssertionError`
- Supervisors can catch this and decide whether to restart or escalate
- Failed assertions should not occur in production code
- Unlike exceptions, assertions are for catching logic errors, not recoverable runtime conditions
```

### 20.3 Actor Utilities

#### 20.3.1 Actor Registry
```
fn register(name: string, actor: actor_ref<Msg>) -> atom proc[concurrency]
fn lookup(name: string) -> option<actor_ref<Msg>> proc[concurrency]
```

#### 20.3.2 Message Broadcasting
```
fn broadcast(actors: list<actor_ref<Msg>>, message: Msg) -> atom proc[concurrency]
```

#### 20.3.3 Actor Monitoring
```
fn monitor(target: actor_ref, monitor: actor_ref<down_msg>)
    -> atom proc[concurrency]
```

### 20.4 Networking

Silica provides optional networking capabilities through effect-gated modules. Networking is not part of the core language but available as standard library modules that require the `networking` effect.

#### 20.4.1 Core Networking Types
```
type socket_addr = {
    ip: ip_addr,
    port: int
}

type ip_addr = ipv4_addr | ipv6_addr
type ipv4_addr = (int, int, int, int)  // IPv4 tuple
type ipv6_addr = buf(R, normal, int, 16)  // 16-byte IPv6 address

type protocol_type = tcp | udp | raw
type socket_state = closed | listening | connected | error

type net_error =
    ConnectionRefused
  | ConnectionTimeout
  | NetworkUnreachable
  | AddressInUse
  | PermissionDenied
  | BufferOverflow
  | InvalidAddress
```

#### 20.4.2 Socket Module
```
module net.socket {

    pub type socket<T: protocol_type>  // Protocol-specific socket

    pub fn create_socket(protocol: protocol_type)
        -> result<socket<protocol>, net_error> proc[networking]

    pub fn bind_socket(sock: socket<T>, addr: socket_addr)
        -> result<atom, net_error> proc[networking]

    pub fn close_socket(sock: socket<T>)
        -> atom proc[networking]

    pub fn get_socket_addr(sock: socket<T>)
        -> socket_addr proc[networking]

    pub fn set_socket_option<T>(sock: socket<T>, option: socket_option, value: T)
        -> result<atom, net_error> proc[networking]
}
```

#### 20.4.3 TCP Module
```
module net.tcp {

    use module net.socket

    pub type tcp_socket = socket<tcp>
    pub type tcp_connection = {
        socket: tcp_socket,
        local_addr: socket_addr,
        remote_addr: socket_addr,
        state: connection_state
    }

    pub fn connect(sock: tcp_socket, addr: socket_addr)
        -> result<tcp_connection, net_error> proc[networking]

    pub fn listen(sock: tcp_socket, backlog: int)
        -> result<atom, net_error> proc[networking]

    pub fn accept(sock: tcp_socket)
        -> result<tcp_connection, net_error> proc[networking]

    pub fn send(sock: tcp_connection, data: buf(R, normal, byte, size))
        -> result<int, net_error> proc[networking]

    pub fn receive(sock: tcp_connection, buffer: buf(R, normal, byte, max_size))
        -> result<int, net_error> proc[networking]

    pub fn shutdown(sock: tcp_connection, direction: shutdown_direction)
        -> result<atom, net_error> proc[networking]
}
```

#### 20.4.4 UDP Module
```
module net.udp {

    use module net.socket

    pub type udp_socket = socket<udp>
    pub type udp_endpoint = socket_addr

    pub fn send_to(sock: udp_socket, data: buf(R, normal, byte, size), dest: socket_addr)
        -> result<int, net_error> proc[networking]

    pub fn receive_from(sock: udp_socket, buffer: buf(R, normal, byte, max_size))
        -> result<(int, socket_addr), net_error> proc[networking]

    pub fn join_multicast_group(sock: udp_socket, group_addr: ip_addr, interface: ip_addr)
        -> result<atom, net_error> proc[networking]

    pub fn leave_multicast_group(sock: udp_socket, group_addr: ip_addr, interface: ip_addr)
        -> result<atom, net_error> proc[networking]
}
```

#### 20.4.5 Packet Processing Module
```
module net.packet {

    use module arch.sve  // Optional: for SIMD acceleration

    pub type ethernet_frame = {
        dest_mac: mac_addr,
        src_mac: mac_addr,
        ethertype: int,
        payload: buf(R, normal, byte, size)
    }

    pub type ipv4_packet = {
        version: int,
        ihl: int,
        tos: int,
        total_len: int,
        id: int,
        flags: int,
        frag_offset: int,
        ttl: int,
        protocol: int,
        checksum: int,
        src_ip: ipv4_addr,
        dest_ip: ipv4_addr,
        options: buf(R, normal, byte, opt_size),
        payload: buf(R, normal, byte, payload_size)
    }

    pub fn parse_ethernet_frame(data: buf(R, normal, byte, frame_size))
        -> result<ethernet_frame, parse_error> proc[]

    pub fn parse_ipv4_packet(data: buf(R, normal, byte, packet_size))
        -> result<ipv4_packet, parse_error> proc[]

    pub fn calculate_ipv4_checksum(packet: ipv4_packet)
        -> int proc[]

    pub fn validate_packet(packet: ipv4_packet)
        -> result<atom, validation_error> proc[]

    // SIMD-accelerated batch processing (when SVE available)
    pub fn process_packet_batch(packets: buf(R, normal, packet, batch_size))
        -> processed_results proc[]
}
```

#### 20.4.6 Networking Utilities
```
module net.utils {

    pub fn resolve_hostname(hostname: string)
        -> result<ip_addr, resolve_error> proc[networking]

    pub fn get_network_interfaces()
        -> list<network_interface> proc[networking]

    pub fn create_network_buffer(size: int)
        -> buf(R, normal_noncacheable, byte, size) proc[networking, mem(normal_noncacheable)]

    pub fn optimize_buffer_for_nic(buffer: buf(R, normal_noncacheable, T, size), nic_device: device_ref)
        -> buf(R, normal_noncacheable, T, size) proc[networking]
}
```

**Note**: Network buffers use `normal_noncacheable` memory space because network interface cards (NICs) use DMA to access packet buffers. Non-cacheable memory ensures that NIC DMA operations see data immediately without cache coherency overhead. Device memory (`device`) is reserved for future device driver library code that directly accesses NIC control registers via memory-mapped I/O.
}
```

### 20.5 Networking Integration with Chip Features

Silica's networking modules leverage AArch64 chip capabilities for optimal performance:

#### 20.5.1 NUMA-Aware Networking
Network buffers and processing can be NUMA-optimized:

**NUMA Topology Detection:**
The runtime provides functions to discover NUMA topology:
```silica
// Get NUMA node information
get_numa_node_count() -> int
get_numa_node_for_core(core_id: int) -> int
get_numa_node_memory_ranges(numa_node: int) -> list<memory_range>
```

**NUMA-Optimized Allocation:**
```silica
// Place network buffers close to NIC for minimal latency
nic_numa_node: int <- get_nic_numa_node(network_interface)
region: region(R, normal) <- alloc_region_on_numa_node(nic_numa_node, normal)
rx_buffers: buf(R, normal, packet, N) <- alloc_buf(region, buffer_count)

// Allocate actor on same NUMA node as its data
actor_ref: actor_ref <- spawn(initial_state, behavior)
pin_actor_to_numa_node(actor_ref, nic_numa_node)
```

**NUMA-Aware Actor Placement:**
Actors processing NUMA-local data should be placed on cores within the same NUMA node to minimize cross-NUMA memory access latency. The runtime provides NUMA-aware scheduling hints.

#### 20.5.2 CPU Affinity for Network Processing
Network actors can be pinned to optimal cores:
```silica
// Pin network processing to efficiency cores (continuous I/O)
network_actor: actor_ref <- spawn_actor(network_state, packet_processor)
pin_actor_to_efficiency_core(network_actor)

// Pin application logic to performance cores (bursty processing)
app_actor: actor_ref <- spawn_actor(app_state, app_logic)
pin_actor_to_performance_core(app_actor)
```

#### 20.5.3 SIMD-Accelerated Packet Processing
When SVE is available, packet processing is automatically vectorized:
```silica
use module net.packet
use module arch.sve  // Enables SIMD acceleration

// Automatic vectorization for batch packet processing
results <- net.packet.process_packet_batch(packet_batch)
```

#### 20.5.4 Hardware-Assisted Security
Network buffers can use memory tagging for security:
```silica
use module arch.mte

// Tagged network buffers prevent overflow exploits
secure_buffer <- net.utils.create_secure_network_buffer(size)
```

## 21. Architecture-Specific Modules

### 21.0 Feature Detection and Capabilities

Silica provides runtime capability detection for optional AArch64 hardware features. Programs that use optional features must check availability at runtime before use.

#### 21.0.1 Capability Query Functions

The runtime provides functions to query AArch64 hardware capabilities:

```
// Query SVE/SVE2 availability
has_sve() -> boolean
has_sve2() -> boolean

// Query NEON availability
has_neon() -> boolean

// Query Memory Tagging Extensions availability
has_mte() -> boolean

// Query Pointer Authentication availability
has_pac() -> boolean

// Query Apple-specific features
has_amx() -> boolean  // Apple Matrix Engine
```

**Capability Detection Mechanism:**

The runtime detects CPU features using AArch64 system registers:

- **ID_AA64PFR0_EL1** (Processor Feature Register 0): Contains feature bits for SVE, PAC, and other architectural features
- **ID_AA64PFR1_EL1** (Processor Feature Register 1): Contains additional feature bits
- **ID_AA64ZFR0_EL1** (SVE Feature Register 0): Contains SVE-specific feature information
- **ID_AA64MMFR0_EL1** (Memory Model Feature Register 0): Contains MTE feature bits
- **ID_AA64MMFR1_EL1** (Memory Model Feature Register 1): Contains additional memory model features

For detailed information about system register access patterns, privilege requirements, and fallback strategies, see Section 21.0.2 (AArch64 System Register Access).

**Runtime Feature Detection:**

Feature detection occurs during program initialization:

1. **Startup Detection**: Runtime queries system registers at program startup to determine available features
2. **Feature Caching**: Detected features are cached for the lifetime of the program
3. **Privilege Requirements**: Feature detection requires appropriate privilege level (typically EL1 or EL0 with kernel support)
4. **Fallback Mechanisms**: If register access fails, features are assumed unavailable (fail-safe behavior)

**Graceful Degradation:**

Programs using optional features must check availability:

```silica
use arch.sve

fn vector_operation(data: *int32, len: int) -> atom proc[mem(normal)] {
    if has_sve() then
        // Use SVE operations
        pred: Pred <- sve.create_pred_true(len)
        vec: VecInt32 <- sve.load_vector_int32(data, Some(pred))
        // ... SVE operations ...
    else
        // Fallback to scalar operations
        // ... scalar implementation ...
    end
}
```

**Compile-Time vs Runtime Availability:**

- **Compile-Time**: Compiler assumes all optional features may be available and generates code accordingly
- **Runtime**: Programs must check feature availability before using optional features
- **Error Handling**: Attempting to use unavailable features results in runtime errors

**Feature Requirements:**

- **SVE/SVE2**: Optional - programs using SVE must check `has_sve()` or `has_sve2()` before use
- **NEON**: Optional - programs using NEON must check `has_neon()` before use (though NEON is widely available on AArch64)
- **MTE**: Optional - programs using MTE must check `has_mte()` before use
- **PAC**: Optional - programs using PAC must check `has_pac()` before use
- **AMX**: Optional and Apple-specific - programs using AMX must check `has_amx()` before use

**Cross-References:**
- See Section 21.0.2 (AArch64 System Register Access) for system register access patterns and privilege requirements
- See Section 21.0.3 (SVE2-Specific Feature Detection) for SVE2 feature detection details
- See Section 21.1 (SVE) for SVE-specific feature requirements
- See Section 21.2 (NEON) for NEON-specific feature requirements
- See Section 21.3 (MTE) for MTE-specific feature requirements
- See Section 21.4 (PAC) for PAC-specific feature requirements
- See Section 21.5 (Apple Silicon Extensions) for Apple-specific feature requirements

#### 21.0.2 AArch64 System Register Access

AArch64 system registers are accessed at different privilege levels, and Silica's runtime must handle both direct access (when available) and fallback strategies (when direct access requires higher privileges).

**AArch64 Privilege Levels:**

AArch64 defines four exception levels (EL0 through EL3) that control access to system registers:

- **EL0 (User Mode)**: Lowest privilege level, limited system register access
- **EL1 (Kernel Mode)**: Operating system kernel level, full system register access
- **EL2 (Hypervisor Mode)**: Virtualization hypervisor level
- **EL3 (Secure Monitor Mode)**: Highest privilege level, secure monitor

**System Register Access Requirements:**

Different system registers require different privilege levels:

| Register | Required EL | Purpose | Direct Access Available |
|----------|-------------|---------|------------------------|
| `MAIR_EL1` | EL1 | Memory Attribute Indirection | No (kernel configures) |
| `MPIDR_EL1` | EL1 | Multiprocessor Affinity | No (via system calls) |
| `CLIDR_EL1` | EL1 | Cache Level ID | No (via system calls) |
| `CCSIDR_EL1` | EL1 | Cache Size ID | No (via system calls) |
| `ID_AA64PFR0_EL1` | EL1 | Processor Feature Register 0 | No (via system calls) |
| `ID_AA64PFR1_EL1` | EL1 | Processor Feature Register 1 | No (via system calls) |
| `ID_AA64ZFR0_EL1` | EL1 | SVE Feature Register | No (via system calls) |
| `ID_AA64MMFR0_EL1` | EL1 | Memory Model Feature Register 0 | No (via system calls) |
| `ID_AA64MMFR1_EL1` | EL1 | Memory Model Feature Register 1 | No (via system calls) |
| `ZCR_EL1` | EL1 | SVE Control Register | No (via system calls) |
| `MIDR_EL1` | EL1 | Main ID Register | No (via system calls) |

**User-Space Access Patterns:**

Silica runtime runs in user space (EL0) and cannot directly access EL1 registers. Instead, it uses:

1. **Kernel-Provided Interfaces**: 
   - `/proc/cpuinfo` for CPU topology and feature information
   - `/sys/devices/system/cpu/` (sysfs) for detailed CPU and cache information
   - `/sys/devices/system/node/` (sysfs) for NUMA topology

2. **System Calls**:
   - `sched_getaffinity()` for CPU affinity queries
   - `getcpu()` for current CPU identification
   - `pthread_getaffinity_np()` for thread affinity
   - `numa_available()` and `numa_node_of_cpu()` for NUMA queries

3. **Library Functions**:
   - POSIX and Linux-specific APIs for hardware feature detection
   - Architecture-specific libraries (e.g., libnuma for NUMA)

**Fallback Strategies:**

When direct register access is unavailable, the runtime uses fallback mechanisms:

- **Kernel Interface Queries**: Query `/proc/cpuinfo` and sysfs interfaces for hardware information
- **System Call Fallbacks**: Use system calls and library functions to query hardware capabilities
- **Conservative Defaults**: Assume single-core configuration, uniform cache hierarchy, and disabled optional features until proven available

**Register Usage Throughout Specification:**

The following sections reference specific system registers:

- **Section 12.1.1.1** (MAIR_EL1 Memory Attribute Mapping): Uses `MAIR_EL1` for memory attribute configuration
- **Section 15.1.2.1** (AArch64 Runtime Integration): Uses `MPIDR_EL1`, `CLIDR_EL1`, `CCSIDR_EL1` for CPU topology detection
- **Section 21.0.1** (Capability Query Functions): Uses `ID_AA64PFR0_EL1`, `ID_AA64PFR1_EL1`, `ID_AA64ZFR0_EL1`, `ID_AA64MMFR0_EL1`, `ID_AA64MMFR1_EL1` for feature detection
- **Section 21.1.3.1** (SVE Runtime Behavior): Uses `ID_AA64ZFR0_EL1` and `ZCR_EL1` for SVE configuration

**Cross-References:**
- See Section 12.1.1.1 (MAIR_EL1 Memory Attribute Mapping) for memory attribute register usage
- See Section 15.1.2.1 (AArch64 Runtime Integration) for topology register usage
- See Section 21.0.1 (Capability Query Functions) for feature detection register usage
- See Section 26.1.5 (System Register Access Patterns and Privilege Levels) for additional access pattern details

#### 21.0.3 SVE2-Specific Feature Detection

SVE2 extends SVE with additional instructions and capabilities. The runtime provides detailed SVE2 feature detection beyond the basic `has_sve2()` check.

**SVE2 Feature Register Queries:**

SVE2 capabilities are detected via `ID_AA64ZFR0_EL1` (SVE Feature Register 0):

```
// Query SVE2-specific features
has_sve2_bf16() -> boolean        // BFloat16 support
has_sve2_i8mm() -> boolean        // Int8 matrix multiplication
has_sve2_f32mm() -> boolean       // Float32 matrix multiplication
has_sve2_f64mm() -> boolean       // Float64 matrix multiplication
has_sve2_sme() -> boolean         // Scalable Matrix Extension (SME)
```

**SVE2 Feature Detection Mechanism:**

The runtime queries `ID_AA64ZFR0_EL1` register bits:

- **BFloat16 Support** (`ID_AA64ZFR0_EL1.BF16`): Indicates BFloat16 arithmetic support
- **Int8 Matrix Operations** (`ID_AA64ZFR0_EL1.I8MM`): Indicates Int8 matrix multiplication support
- **Float32 Matrix Operations** (`ID_AA64ZFR0_EL1.F32MM`): Indicates Float32 matrix multiplication support
- **Float64 Matrix Operations** (`ID_AA64ZFR0_EL1.F64MM`): Indicates Float64 matrix multiplication support
- **SME Support** (`ID_AA64PFR1_EL1.SME`): Indicates Scalable Matrix Extension support

**Runtime Detection Process:**

1. **SVE2 Base Check**: First checks `has_sve2()` to ensure SVE2 is available
2. **Feature-Specific Checks**: Queries `ID_AA64ZFR0_EL1` for specific SVE2 extensions
3. **Feature Caching**: Detected SVE2 features are cached for program lifetime
4. **Fallback Behavior**: If feature-specific checks fail, assumes feature unavailable

**SVE2 Feature Usage:**

Programs should check for specific SVE2 features before use:

```silica
use arch.sve

fn matrix_operation(data: *int8, len: int) -> atom proc[mem(normal)] {
    if has_sve2() and has_sve2_i8mm() then
        // Use SVE2 Int8 matrix multiplication
        // ... SVE2 I8MM operations ...
    else if has_sve() then
        // Fallback to SVE operations
        // ... SVE operations ...
    else
        // Fallback to scalar operations
        // ... scalar implementation ...
    end
}
```

**SVE vs SVE2 Fallback Strategy:**

When SVE2 features are unavailable:

1. **SVE2 → SVE Fallback**: If SVE2-specific features unavailable, fall back to SVE equivalents
2. **SVE → Scalar Fallback**: If SVE unavailable, fall back to scalar operations
3. **Graceful Degradation**: Programs continue to function with reduced performance

**SVE2 Extension Availability:**

SVE2 extensions have varying availability:

- **BFloat16**: Available on some SVE2 implementations (e.g., Neoverse V2)
- **Int8 Matrix Operations**: Available on some SVE2 implementations
- **Float32/Float64 Matrix Operations**: Available on some SVE2 implementations
- **SME**: Separate extension, requires `has_sve2_sme()` check

**Cross-References:**
- See Section 21.1 (SVE) for SVE base feature requirements
- See Section 21.0.1 (Capability Query Functions) for basic feature detection

### 21.1 SVE (Scalable Vector Extension)

#### 21.1.1 Vector Types
```
module arch.sve {

    // Concrete SVE vector types (no generics)
    pub type VecInt8       // Scalable vector of int8
    pub type VecInt16      // Scalable vector of int16
    pub type VecInt32      // Scalable vector of int32
    pub type VecInt64      // Scalable vector of int64
    pub type VecFloat16    // Scalable vector of float16
    pub type VecFloat32    // Scalable vector of float32
    pub type VecFloat64    // Scalable vector of float64
    pub type VecBoolean       // Scalable boolean vector
    pub type Pred          // predicate mask for conditional operations

    // Marker trait for SVE-supported element types
    trait VecElement {
        // Marker trait, no methods needed
    }
    
    // Implement VecElement for supported types
    impl VecElement for int8;
    impl VecElement for int16;
    impl VecElement for int32;
    impl VecElement for int64;
    impl VecElement for float16;
    impl VecElement for float32;
    impl VecElement for float64;
}
```

**Note**: Silica uses concrete vector types rather than generic types. Each element type has its own vector type (e.g., `VecInt8` for `int8` elements, `VecInt32` for `int32` elements). The `VecElement` trait marks types that can be used as SVE vector elements.

#### 21.1.2 Vector Operations
```
module arch.sve {

    // Concrete load/store operations for each vector type
    pub fn load_vector_int32(ptr: *int32, pred: OptionPred) -> VecInt32
    pub fn store_vector_int32(ptr: *int32, vec: VecInt32, pred: OptionPred) -> atom
    pub fn add_vectors_int32(a: VecInt32, b: VecInt32) -> VecInt32
    pub fn mul_vectors_int32(a: VecInt32, b: VecInt32) -> VecInt32
    
    // Repeat for other types: int8, int16, int64, float16, float32, float64
    pub fn load_vector_int8(ptr: *int8, pred: OptionPred) -> VecInt8
    pub fn load_vector_int64(ptr: *int64, pred: OptionPred) -> VecInt64
    pub fn load_vector_float32(ptr: *float32, pred: OptionPred) -> VecFloat32
    // ... (similar for all element types)

    // Predicate operations
    pub fn create_pred_true(len: int) -> Pred
    pub fn create_pred_from_mask(mask: VecBoolean) -> Pred
    pub fn test_any_true(pred: Pred) -> boolean
    pub fn test_all_true(pred: Pred) -> boolean
}
```

**Note**: All SVE vector operations are pure runtime functions (no effects required). All vector operations use concrete types. For each element type, there are corresponding operations (e.g., `load_vector_int32` for `VecInt32`, `load_vector_float32` for `VecFloat32`). 

The `OptionPred` type is a variant type (not a generic) defined as `OptionPred = Some(Pred) | None`. This allows SVE operations to work with optional predicates - when `Some(pred)` is provided, only elements where the predicate is true are processed; when `None` is provided, all elements are processed.

#### 21.1.2.1 SVE Code Generation Strategy

The compiler generates AArch64 SVE instructions for vector and predicate operations. This section describes the code generation strategy for SVE operations.

**Register Allocation:**

AArch64 SVE provides:
- **Vector Registers**: Z0-Z31 (32 scalable vector registers, 128-2048 bits each)
- **Predicate Registers**: P0-P15 (16 predicate registers, scalable width matching vector length)

**Predicate Register Allocation:**

Predicate operations use predicate registers (P0-P15):

```silica
pred: Pred <- sve.create_pred_true(len)
```

Generated code:
```assembly
// create_pred_true(len) generates:
MOV   X0, len              // Load length to general register
WHILELT P0.B, XZR, X0      // Generate predicate: P0 = (0..len-1) < len
```

**Predicate Operation Code Generation:**

**create_pred_true(len):**
```assembly
// Generates WHILELT instruction
WHILELT P0.B, XZR, X0      // P0 = active elements for length len
```

**create_pred_from_mask(mask: VecBoolean):**
```assembly
// Converts VecBoolean vector to predicate
MOV    Z0, mask            // Load boolean vector
CMPNE  P0.B, Z0/Z, #0      // Generate predicate from non-zero elements
```

**test_any_true(pred: Pred):**
```assembly
// Tests if any predicate bit is set
PTEST  P0, P0.B            // Test predicate
CSET   W0, NE               // Set result if any bit is true
```

**test_all_true(pred: Pred):**
```assembly
// Tests if all predicate bits are set
PTEST  P0, P0.B            // Test predicate
CSET   W0, EQ               // Set result if all bits are true
```

**Vector Register Allocation:**

Vector operations use vector registers (Z0-Z31):

```silica
vec: VecInt32 <- sve.load_vector_int32(ptr, Some(pred))
```

Generated code:
```assembly
// load_vector_int32 with predicate
LD1W   {Z0.S}, P0/Z, [X0]  // Load int32 vector with predicate P0
```

**Register Pressure Considerations:**

The compiler manages register pressure for SVE operations:

1. **Predicate Registers**: P0-P15 (16 registers) - typically sufficient for nested loops
2. **Vector Registers**: Z0-Z31 (32 registers) - compiler allocates based on live ranges
3. **Spilling Strategy**: When register pressure is high, compiler spills to stack using `STR/STP` instructions
4. **Predicate Spilling**: Predicate registers can be spilled to general-purpose registers or stack

**Interaction Between Vector and Predicate Registers:**

Predicate registers control vector operations:

```silica
// Conditional vector operation
vec: VecInt32 <- sve.load_vector_int32(ptr, Some(pred))
```

Generated code:
```assembly
LD1W   {Z0.S}, P0/Z, [X0]  // P0 controls which elements are loaded
                          // Elements where P0 is false are set to zero
```

**Code Generation Examples:**

**Example 1: Simple Vector Load**
```silica
vec: VecInt32 <- sve.load_vector_int32(ptr, None)
```

Generated code:
```assembly
MOV    P0.B, #-1           // All-true predicate (None case)
LD1W   {Z0.S}, P0/Z, [X0]  // Load all elements
```

**Example 2: Predicated Vector Load**
```silica
pred: Pred <- sve.create_pred_true(len)
vec: VecInt32 <- sve.load_vector_int32(ptr, Some(pred))
```

Generated code:
```assembly
WHILELT P0.B, XZR, X1      // Generate predicate for length
LD1W   {Z0.S}, P0/Z, [X0]  // Load with predicate
```

**Example 3: Vector Arithmetic**
```silica
result: VecInt32 <- sve.add_vectors_int32(a, b)
```

Generated code:
```assembly
ADD    Z0.S, Z0.S, Z1.S    // Vector addition (Z0 = a, Z1 = b)
```

**Cross-References:**
- See Section 27.1.1 (Region-Aware Code Generation) for register allocation strategies
- See Section 21.1.3.1 (SVE Runtime Behavior) for vector length runtime behavior

#### 21.1.3 Scalable Width
SVE vectors automatically scale with hardware vector length:

```
use module arch.sve

fn vector_add(a: *int32, b: *int32, len: int) -> atom proc[] {
    pred: Pred <- sve.create_pred_true(len)
    pred_opt: OptionPred <- Some(pred)
    va: VecInt32 <- sve.load_vector_int32(a, pred_opt)
    vb: VecInt32 <- sve.load_vector_int32(b, pred_opt)
    result: VecInt32 <- sve.add_vectors_int32(va, vb)
    sve.store_vector_int32(a, result, pred_opt)  // a = a + b
}
```

**Note**: All operations use concrete types (`VecInt32`, `OptionPred`, etc.). Type inference occurs at compile time.

#### 21.1.3.1 SVE Runtime Behavior

SVE vectors have scalable width determined at runtime based on hardware capabilities. The runtime queries and manages vector length throughout program execution.

**Vector Length Query:**

The runtime provides functions to query SVE vector length:

```
// Query vector length in bytes (VL_B)
get_sve_vector_length_bytes() -> int

// Query vector length in bits (VL_B * 8)
get_sve_vector_length_bits() -> int

// Query number of elements for a specific type
get_sve_elements_int32() -> int  // Returns VL_B / 4 for int32
get_sve_elements_int64() -> int  // Returns VL_B / 8 for int64
get_sve_elements_float32() -> int  // Returns VL_B / 4 for float32
```

**Vector Length Characteristics:**

- **Hardware-Dependent**: Vector length is determined by hardware (typically 128, 256, 512, or 1024 bits)
- **Runtime Constant**: Vector length is constant for the lifetime of the program (queried once at startup)
- **Element Count**: Number of elements per vector depends on element size:
  - `int8`: `VL_B` elements
  - `int16`: `VL_B / 2` elements
  - `int32`: `VL_B / 4` elements
  - `int64`: `VL_B / 8` elements
  - `float32`: `VL_B / 4` elements
  - `float64`: `VL_B / 8` elements

**Predicate Handling:**

SVE predicates (`Pred` type) are scalable masks that match the vector length:

```
// Create predicate for all elements
create_pred_true(len: int) -> Pred

// Create predicate from boolean vector
create_pred_from_mask(mask: VecBoolean) -> Pred

// Test predicate conditions
test_any_true(pred: Pred) -> boolean  // Returns true if any element is active
test_all_true(pred: Pred) -> boolean  // Returns true if all elements are active
```

**Predicate Semantics:**

- **Predicate Width**: Predicates have the same width as vectors (one bit per vector element)
- **Active Elements**: Elements where predicate bit is `1` are processed
- **Inactive Elements**: Elements where predicate bit is `0` are not modified (for stores) or set to zero (for some operations)
- **Predicate Operations**: All SVE operations respect predicates - only active elements are processed

**Scalable Vector Operations:**

All SVE vector operations automatically adapt to hardware vector length:

```
// Load operation processes VL_B / sizeof(T) elements
load_vector_int32(ptr: *int32, pred: OptionPred) -> VecInt32

// Arithmetic operations process all active elements
add_vectors_int32(a: VecInt32, b: VecInt32) -> VecInt32

// Store operation writes only active elements (per predicate)
store_vector_int32(ptr: *int32, vec: VecInt32, pred: OptionPred) -> atom
```

**Interaction with Fixed-Size Buffers:**

When working with fixed-size buffers (`buf(R, Space, T, N)`), SVE operations handle remainder elements:

```
fn process_buffer(buf: buf(R, normal, int32, 100), len: int) -> atom proc[mem(normal)] {
    // Calculate number of full vectors
    elements_per_vector: int <- get_sve_elements_int32();
    full_vectors: int <- len / elements_per_vector;
    remainder: int <- len % elements_per_vector;
    
    // Process full vectors
    i: int <- 0;
    while i < full_vectors {
        // Process one vector
        offset: int <- i * elements_per_vector;
        vec: VecInt32 <- sve.load_vector_int32(&buf[offset], Some(create_pred_true(elements_per_vector)));
        // ... process vector ...
        i <- i + 1;
    }
    
    // Process remainder with predicate
    if remainder > 0 {
        remainder_pred: Pred <- create_pred_true(remainder);
        vec: VecInt32 <- sve.load_vector_int32(&buf[full_vectors * elements_per_vector], Some(remainder_pred));
        // ... process remainder ...
    }
}
```

**Runtime Vector Length Detection:**

At program startup, the runtime:

1. **Hardware Detection**: Checks `ID_AA64ZFR0_EL1` register for SVE support
2. **Vector Length Query**: Reads `ZCR_EL1.LEN` to determine maximum vector length
3. **Vector Length Configuration**: Configures `ZCR_EL1` to enable SVE with desired vector length
4. **Predicate Length**: Sets predicate length to match vector length
5. **Runtime Constants**: Caches vector length values for efficient queries

**SVE Register Specifications:**

**ID_AA64ZFR0_EL1 (AArch64 SVE Feature Register 0):**

The `ID_AA64ZFR0_EL1` register provides information about SVE feature support:

```
ID_AA64ZFR0_EL1 Register Layout:
┌─────────────────────────────────────────────────────────────┐
│ 63:60 │ 59:56 │ 55:52 │ 51:48 │ 47:44 │ 43:40 │ 39:36 │ 35:32 │
│ SVEVer│ F64MM │ F32MM │ I8MM  │ SM4   │ SM3   │ SHA3  │ BF16  │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│ 31:28 │ 27:24 │ 23:20 │ 19:16 │ 15:12 │ 11:8  │ 7:4   │ 3:0   │
│ BitPerm│ BF16  │ AES   │ SVE2  │ SVE   │ Reserved │ Reserved │ Reserved │
└─────────────────────────────────────────────────────────────┘
```

**Key Fields:**

- **SVE [3:0]**: SVE architecture version (0x1 = SVE supported)
- **SVE2 [19:16]**: SVE2 architecture version (0x1 = SVE2 supported)
- **F64MM [59:56]**: F64MM (64-bit floating-point matrix multiplication) support
- **F32MM [55:52]**: F32MM (32-bit floating-point matrix multiplication) support
- **I8MM [51:48]**: I8MM (8-bit integer matrix multiplication) support

**SVE Detection Pseudocode:**

```pseudocode
// Read ID_AA64ZFR0_EL1 register
MRS ID_AA64ZFR0_EL1, ID_AA64ZFR0_EL1

// Check if SVE is supported (bits [3:0] != 0)
if (ID_AA64ZFR0_EL1[3:0] != 0) then
    SVE_SUPPORTED = true
    SVE_VERSION = ID_AA64ZFR0_EL1[3:0]
    
    // Check SVE2 support (bits [19:16] != 0)
    if (ID_AA64ZFR0_EL1[19:16] != 0) then
        SVE2_SUPPORTED = true
        SVE2_VERSION = ID_AA64ZFR0_EL1[19:16]
    end if
else
    SVE_SUPPORTED = false
    // SVE not available, fall back to NEON
end if
```

**ZCR_EL1 (SVE Control Register for EL1):**

The `ZCR_EL1` register controls SVE vector length configuration:

```
ZCR_EL1 Register Layout:
┌─────────────────────────────────────────────────────────────┐
│ 63:4  │ 3:0 │
│ Reserved│ LEN │
└─────────────────────────────────────────────────────────────┘
```

**LEN Field (bits [3:0]):**

The LEN field specifies the maximum vector length:

| LEN Value | Vector Length (bits) | Vector Length (bytes) | Description |
|-----------|---------------------|----------------------|-------------|
| 0b0000 | 128 | 16 | Minimum vector length (always supported) |
| 0b0001 | 256 | 32 | 256-bit vectors |
| 0b0010 | 512 | 64 | 512-bit vectors |
| 0b0011 | 1024 | 128 | 1024-bit vectors |
| 0b0100 | 2048 | 256 | 2048-bit vectors (future) |
| 0b0101-0b1111 | Reserved | - | Reserved for future use |

**Vector Length Configuration:**

The runtime configures SVE vector length as follows:

```pseudocode
// Read current ZCR_EL1 configuration
MRS ZCR_EL1, ZCR_EL1

// Query maximum supported vector length
MAX_LEN = ZCR_EL1[3:0]

// Configure vector length (use maximum supported)
// This enables SVE and sets vector length
ZCR_EL1[3:0] = MAX_LEN

// Write ZCR_EL1 register
MSR ZCR_EL1, ZCR_EL1

// Vector length is now configured
// VL_B (vector length in bytes) = 16 * (2^MAX_LEN)
VL_B = 16 * (1 << MAX_LEN)
```

**Vector Length Query Functions:**

The runtime provides functions to query configured vector length:

```pseudocode
// Query vector length in bytes
function get_sve_vector_length_bytes() -> int:
    // Read ZCR_EL1.LEN
    MRS ZCR_EL1, ZCR_EL1
    LEN = ZCR_EL1[3:0]
    // Return VL_B = 16 * (2^LEN)
    return 16 * (1 << LEN)

// Query vector length in bits
function get_sve_vector_length_bits() -> int:
    return get_sve_vector_length_bytes() * 8

// Query number of elements for int32
function get_sve_elements_int32() -> int:
    VL_B = get_sve_vector_length_bytes()
    // int32 is 4 bytes, so elements = VL_B / 4
    return VL_B / 4
```

**Hardware Constraints:**

- **Minimum Vector Length**: All SVE implementations support at least 128-bit vectors (LEN=0)
- **Maximum Vector Length**: Hardware-dependent, determined by reading ZCR_EL1.LEN
- **Vector Length Granularity**: Vector length must be a multiple of 128 bits
- **Runtime Constant**: Vector length cannot be changed after program startup
- **Per-Core Configuration**: ZCR_EL1 is per-core, but Silica runtime ensures consistent configuration across all cores

**SVE2 Feature Support:**

SVE2 extends SVE with additional features. The runtime detects SVE2 support and exposes SVE2-specific features:

**SVE2 Detection:**

```pseudocode
// Read SVE feature register
MRS ID_AA64ZFR0_EL1, ID_AA64ZFR0_EL1
SVE2_FIELD = ID_AA64ZFR0_EL1[19:16]

// Check SVE2 support
if SVE2_FIELD != 0:
    SVE2_SUPPORTED = true
    SVE2_VERSION = SVE2_FIELD
else:
    SVE2_SUPPORTED = false
end if
```

**SVE2 Features Exposed:**

When SVE2 is supported, Silica exposes the following SVE2-specific features:

- **SVE2 Crypto Extensions**: Hardware-accelerated cryptographic operations (if supported via `ID_AA64ZFR0_EL1` crypto fields)
- **SVE2 Bitwise Operations**: Enhanced bitwise manipulation operations
- **SVE2 Integer Operations**: Additional integer arithmetic operations
- **SVE2 Permutation Operations**: Advanced data permutation and rearrangement operations

**Hardware Variation:**

Different AArch64 implementations may have varying SVE/SVE2 support:

- **SVE Only**: Some implementations support SVE but not SVE2
- **SVE2 Full**: Some implementations support full SVE2 feature set
- **SVE2 Partial**: Some implementations support SVE2 with limited feature subsets
- **Vector Length Variation**: Maximum vector length varies (128, 256, 512, or 1024 bits)

**Fallback Strategy:**

If SVE is not available:
- The runtime falls back to NEON (fixed-width SIMD) operations
- SVE vector types are emulated using NEON 128-bit vectors
- Scalable operations are implemented using fixed-width NEON operations with loop unrolling

**Vector Length Constraints:**

- **Minimum**: 128 bits (16 bytes) - all SVE implementations support at least 128-bit vectors
- **Maximum**: Hardware-dependent (typically 256, 512, or 1024 bits)
- **Granularity**: Vector length must be a multiple of 128 bits
- **Runtime Limit**: Program cannot change vector length after startup

**Vector Length Change Behavior:**

SVE vector length is determined at program startup and remains constant throughout program execution. However, programs must handle scenarios where vector length may vary between different program runs or different hardware platforms.

**Vector Length Stability:**

Once a program starts, the vector length is fixed:

- **Startup Configuration**: Vector length is configured once at program startup via `ZCR_EL1.LEN`
- **Runtime Constant**: Vector length does not change during program execution
- **Per-Program**: Each program instance has its own vector length configuration
- **Cross-Core Consistency**: All cores in a program use the same vector length (runtime ensures consistency)

**Vector Length Variation Between Runs:**

Different program runs may have different vector lengths:

- **Hardware Variation**: Different hardware platforms may support different maximum vector lengths
- **OS Configuration**: Operating system may configure different vector lengths for different programs
- **Runtime Detection**: Program must query vector length at runtime, not assume a specific value
- **Adaptive Code**: Programs should be written to adapt to any supported vector length

**Handling Variable Vector Lengths:**

Programs must be written to handle variable vector lengths gracefully:

```silica
use module arch.sve

fn adaptive_vector_processing(data: *int32, len: int) -> atom proc[mem(normal)] {
    // Query vector length at runtime (not compile-time constant)
    elements_per_vector: int <- get_sve_elements_int32();
    
    // Calculate number of full vectors
    full_vectors: int <- len / elements_per_vector;
    remainder: int <- len % elements_per_vector;
    
    // Process full vectors (adapts to any vector length)
    i: int <- 0;
    while i < full_vectors {
        offset: int <- i * elements_per_vector;
        // Vector operation processes exactly elements_per_vector elements
        vec: VecInt32 <- sve.load_vector_int32(&data[offset], Some(create_pred_true(elements_per_vector)));
        // ... process vector ...
        i <- i + 1;
    }
    
    // Process remainder (handles any remainder size)
    if remainder > 0 {
        remainder_pred: Pred <- create_pred_true(remainder);
        vec: VecInt32 <- sve.load_vector_int32(&data[full_vectors * elements_per_vector], Some(remainder_pred));
        // ... process remainder ...
    }
}
```

**Vector Length Query Performance:**

Querying vector length has minimal overhead:

- **Cached Value**: Runtime caches vector length at startup, queries are O(1) lookups
- **No Register Access**: Queries do not require reading `ZCR_EL1` register (cached value used)
- **Fast Lookup**: Vector length queries typically take < 1 cycle (cached value access)

**Vector Length Impact on Program Behavior:**

Vector length affects program behavior in predictable ways:

- **Element Count**: Number of elements processed per vector operation scales with vector length
- **Remainder Handling**: Remainder size varies with vector length (smaller remainders for larger vectors)
- **Performance**: Larger vector lengths provide better performance for data-parallel operations
- **Memory Alignment**: Programs should align data to vector length boundaries for optimal performance

**Vector Length Change Detection:**

Programs can detect if vector length changes between runs:

```silica
use module arch.sve

// Store expected vector length (from previous run or configuration)
expected_elements: int <- 16;  // Expected elements per vector

fn check_vector_length() -> boolean proc[] {
    actual_elements: int <- get_sve_elements_int32();
    
    // Detect vector length change
    if actual_elements != expected_elements {
        // Vector length changed - program must adapt
        return false;
    else
        // Vector length matches expected
        return true;
    end
}
```

**Vector Length Adaptation Strategies:**

Programs should use adaptive strategies to handle variable vector lengths:

- **Runtime Query**: Always query vector length at runtime, never assume a specific value
- **Dynamic Calculation**: Calculate element counts and remainders dynamically based on queried vector length
- **Predicate-Based Remainder**: Use predicates to handle remainder elements regardless of remainder size
- **Cache-Aware Alignment**: Align data to vector length boundaries for optimal cache performance

**Vector Length Performance Characteristics:**

| Vector Length | Elements (int32) | Performance Impact | Cache Alignment | Remainder Handling |
|--------------|-----------------|-------------------|----------------|-------------------|
| 128 bits | 4 elements | Baseline | 16-byte aligned | Common (small vectors) |
| 256 bits | 8 elements | ~2x improvement | 32-byte aligned | Less common |
| 512 bits | 16 elements | ~4x improvement | 64-byte aligned (cache line) | Rare |
| 1024 bits | 32 elements | ~8x improvement | 128-byte aligned | Very rare |

**Performance Characteristics:**

- **Vectorization Efficiency**: Larger vector lengths provide better performance for data-parallel operations
- **Predicate Overhead**: Predicate operations have minimal overhead (hardware-supported)
- **Remainder Handling**: Processing remainder elements requires additional predicate operations
- **Cache Effects**: Vector operations benefit from cache line alignment (64-byte cache lines)
- **Vector Length Scaling**: Performance scales approximately linearly with vector length for data-parallel workloads

**Example: Vector Length Query**

```silica
use module arch.sve

fn demonstrate_vector_length() -> atom proc[] {
    // Query vector length (runtime values, not compile-time constants)
    vl_bytes: int <- get_sve_vector_length_bytes();
    vl_bits: int <- get_sve_vector_length_bits();
    elements_int32: int <- get_sve_elements_int32();
    
    // Use vector length for processing
    // elements_int32 elements are processed per vector operation
    // Program adapts to any supported vector length
}
```

**Cross-References:**
- See Section 21.0.1 (Capability Query Functions) for SVE feature detection
- See Section 21.1.1 (Vector Types) for SVE vector type definitions
- See Section 21.1.2 (Vector Operations) for SVE operation semantics
- See Section 12.1.1.1 (AArch64 Memory Attribute Mapping) for memory alignment requirements

### 21.2 NEON (Fixed-Width SIMD)

#### 21.2.1 Fixed-Width Vectors
```
module arch.neon {

    // Concrete NEON vector types (no generics)
    pub type Vec128Int8    // 128-bit vector of int8
    pub type Vec128Int16   // 128-bit vector of int16
    pub type Vec128Int32   // 128-bit vector of int32
    pub type Vec128Int64   // 128-bit vector of int64
    pub type Vec128Float32 // 128-bit vector of float32
    pub type Vec128Boolean    // 128-bit boolean vector
    pub type Vec64Int8     // 64-bit vector of int8 (limited use)
    pub type Vec64Int16    // 64-bit vector of int16 (limited use)
    pub type Vec64Int32    // 64-bit vector of int32 (limited use)
    pub type Vec64Float32  // 64-bit vector of float32 (limited use)

    // Marker trait for NEON-supported element types
    trait Vec128Element {
        // Marker trait, no methods needed
    }
    
    // Implement Vec128Element for supported types
    impl Vec128Element for int8;
    impl Vec128Element for int16;
    impl Vec128Element for int32;
    impl Vec128Element for int64;
    impl Vec128Element for float32;
}
```

**Note**: Silica uses concrete NEON vector types rather than generic types. Each element type has its own vector type (e.g., `Vec128Int8` for 128-bit vectors of `int8` elements). The `Vec128Element` trait marks types that can be used as NEON vector elements.

#### 21.2.2 NEON Operations
```
module arch.neon {

    // Concrete operations for each vector type
    pub fn load_128_int32(ptr: *int32) -> Vec128Int32
    pub fn store_128_int32(ptr: *int32, vec: Vec128Int32) -> atom
    pub fn add_128_int32(a: Vec128Int32, b: Vec128Int32) -> Vec128Int32
    pub fn mul_128_int32(a: Vec128Int32, b: Vec128Int32) -> Vec128Int32
    
    // Repeat for other types: int8, int16, int64, float32
    pub fn load_128_int8(ptr: *int8) -> Vec128Int8
    pub fn load_128_float32(ptr: *float32) -> Vec128Float32
    // ... (similar for all element types)

    // Lane access (type-specific)
    pub fn extract_lane_128_int32(vec: Vec128Int32, lane: int) -> int32
    pub fn insert_lane_128_int32(vec: Vec128Int32, lane: int, value: int32) -> Vec128Int32
    // ... (similar for all element types)
}
```

**Note**: All NEON operations are pure runtime functions (no effects required). All NEON operations use concrete types. For each element type, there are corresponding operations (e.g., `load_128_int32` for `Vec128Int32`, `load_128_float32` for `Vec128Float32`).

### 21.3 Memory Tagging Extensions (MTE)

#### 21.3.1 Tagged Types and Trait
Memory Tagging Extensions (MTE) provide hardware-accelerated memory safety through tagged pointers. Types that can be used with MTE must implement the `tagged` trait.

```
trait tagged {
    // Marker trait for types that can be used with MTE
}

// Example: Implement tagged trait for a struct
struct NodeData {
    value: int64,
    next: int64
}

impl tagged for NodeData;
```

#### 21.3.2 Tagged Pointer Operations
Tagged pointer operations work with any type that implements the `tagged` trait. Since Silica does not support generics, separate functions are provided for each concrete type that implements the `tagged` trait:

```
// Allocate tagged pointer (type must implement tagged trait)
// For NodeData type (example):
alloc_tagged_nodedata(region: region(R, normal), size: int) -> ref(R, normal, NodeData) proc[mem(normal)]

// Free tagged pointer
free_tagged_nodedata(ptr: ref(R, normal, NodeData)) -> atom proc[mem(normal)]

// Tag operations
set_tag_nodedata(ptr: ref(R, normal, NodeData), tag: int) -> ref(R, normal, NodeData)
get_tag_nodedata(ptr: ref(R, normal, NodeData)) -> int
check_tag_nodedata(ptr: ref(R, normal, NodeData)) -> boolean
```

**Note**: For each type that implements the `tagged` trait, the compiler generates type-specific versions of these functions. The function names follow the pattern `alloc_tagged_<typename>`, `free_tagged_<typename>`, etc.

**Note**: The `tagged` trait can only be implemented for named types (structs, type aliases), not for primitive types. This ensures type safety and prevents misuse of MTE features.

#### 21.3.3 MTE Runtime Integration

Memory Tagging Extensions (MTE) are always available in Silica. The runtime automatically checks MTE hardware support at program startup and enables MTE features throughout program execution.

**Runtime Initialization:**

At program startup, the runtime performs the following MTE initialization:

1. **Hardware Detection**: Check if MTE is supported via `ID_AA64PFR1_EL1.MTE` register
2. **MTE Enablement**: If supported, enable MTE via `TCO` (Tag Check Override) bit in `TCR_EL1` and configure tag allocation
3. **Tag Granularity**: Configure tag granularity (16-byte granules) via `TCR_EL1.TCMA0` and `TCR_EL1.TCMA1`
4. **Tag Allocation**: Initialize tag allocation via `GCR_EL1` (Tag Control Register)

**Hardware Variation and Limitations:**

Different AArch64 implementations may have varying MTE support:

- **Full MTE Support**: Some implementations support full 4-bit tagging (tags 0-15)
- **Limited MTE Support**: Some implementations may only support top-byte tagging (using only the top byte of the pointer, not all 4 tag bits)
- **Tag Storage**: Tag storage capacity may vary between implementations
- **Performance Characteristics**: MTE performance overhead varies by implementation (typically 1-5% for tag checking)

**Hardware Detection and Fallback:**

The runtime detects MTE capabilities at startup:

```pseudocode
// Read MTE feature register
MRS ID_AA64PFR1_EL1, ID_AA64PFR1_EL1
MTE_FIELD = ID_AA64PFR1_EL1[11:8]

// Check MTE support level
if MTE_FIELD == 0b0001:
    // MTE supported with full features
    MTE_SUPPORTED = true
    MTE_FULL_FEATURES = true
else if MTE_FIELD == 0b0010:
    // MTE supported with limited features (top-byte only)
    MTE_SUPPORTED = true
    MTE_FULL_FEATURES = false
else:
    // MTE not supported
    MTE_SUPPORTED = false
    // Fall back to software-based memory safety
end if
```

**Fallback Strategy:**

If MTE is not available or has limited support:
- The runtime falls back to software-based bounds checking
- Tagged pointer operations are emulated using software checks
- Memory safety is maintained through compiler-generated bounds checks

**Tag Allocation Semantics:**

When allocating tagged memory:

```
alloc_tagged_nodedata(region: region(R, normal), size: int) -> ref(R, normal, NodeData) proc[mem(normal)]
```

The runtime:
1. Allocates memory aligned to 16-byte boundaries (MTE tag granularity)
2. Generates a random 4-bit tag value (0-15) for the allocation
3. Stores the tag in the top 4 bits of the pointer (bits 56-59 on AArch64)
4. Initializes memory tags in the tag storage (separate from data memory)
5. Returns a tagged pointer with the allocation tag embedded

**Tag Checking Semantics:**

MTE hardware automatically validates tags on memory access. Tag checking occurs synchronously with memory operations and provides hardware-accelerated memory safety.

**Load Operations:**

When loading from a tagged pointer, hardware performs the following validation:

1. **Extract Pointer Tag**: Hardware extracts tag from pointer (bits 56-59)
2. **Read Memory Tag**: Hardware reads tag from tag storage (4 bits per 16-byte granule)
3. **Compare Tags**: Hardware compares pointer tag with memory tag
4. **Tag Match**: If tags match, load proceeds normally
5. **Tag Mismatch**: If tags mismatch, hardware generates tag fault

**Tag Checking Process:**

```pseudocode
function hardware_tag_check_load(pointer, address):
    // Extract pointer tag (bits 56-59)
    pointer_tag = (pointer >> 56) & 0x0F
    
    // Calculate tag storage address (16-byte aligned)
    tag_storage_addr = address & ~0x0F  // Align to 16-byte boundary
    tag_index = (address >> 4) & 0xFFFFFFFF
    
    // Read memory tag from tag storage
    memory_tag = read_tag_storage(tag_storage_addr, tag_index)
    
    // Compare tags
    if pointer_tag != memory_tag:
        // Generate tag fault
        raise_tag_fault(address, pointer_tag, memory_tag)
    else:
        // Tags match, proceed with load
        return load_data(address)
    end if
end function
```

**Store Operations:**

When storing to a tagged pointer, hardware validates tags before allowing the write:

1. **Tag Validation**: Same process as load operations
2. **Write Proceeds**: If tags match, store proceeds normally
3. **Tag Fault**: If tags mismatch, store is prevented and fault is generated

**Tag Mismatch Detection:**

MTE hardware detects tag mismatches with minimal overhead:

- **Synchronous Detection**: Tag checking occurs during memory access (no separate check step)
- **Hardware Acceleration**: Tag comparison is performed by hardware (no software overhead)
- **Granularity**: Tag checking is per-16-byte granule (not per-byte)

**Tag Fault Generation:**

When a tag mismatch is detected:

1. **Fault Exception**: Hardware generates tag fault exception
2. **Signal Delivery**: OS delivers `SIGSEGV` signal with `SEGV_MTESERR` code
3. **Fault Information**: Fault includes:
   - Fault address (memory address accessed)
   - Pointer tag (tag from pointer)
   - Memory tag (tag from tag storage)
   - Access type (load or store)

**Tag Fault Handling:**

```pseudocode
function handle_tag_fault(fault_info):
    // Extract fault information
    fault_address = fault_info.address
    pointer_tag = fault_info.pointer_tag
    memory_tag = fault_info.memory_tag
    access_type = fault_info.access_type
    
    // Log fault information
    log_tag_fault(fault_address, pointer_tag, memory_tag, access_type)
    
    // Default behavior: terminate program
    // (Runtime can provide custom fault handlers)
    terminate_program("MTE tag mismatch detected")
end function
```

#### 21.3.3.1 Tag Storage Capacity and Limits

MTE tag storage has capacity limits that affect memory allocation and runtime behavior. This section documents tag storage capacity, exhaustion handling, and error reporting.

**Tag Storage Capacity:**

MTE tag storage capacity is hardware-dependent:

- **Tag Granularity**: 4 bits per 16-byte memory granule
- **Tag Storage Overhead**: ~3.125% additional memory (4 bits per 128 bits of data)
- **Tag Storage Location**: Separate from data memory, uses dedicated tag storage
- **Capacity Limits**: Tag storage capacity varies by hardware implementation

**Tag Storage Architecture:**

Tag storage is organized as:

1. **Granule-Based**: Each 16-byte memory granule has a 4-bit tag
2. **Separate Storage**: Tag storage is separate from data memory
3. **Cache Hierarchy**: Tag storage participates in cache hierarchy
4. **Hardware-Managed**: Tag storage is managed by hardware, not directly accessible to software

**Tag Storage Capacity Limits:**

Different AArch64 implementations have varying tag storage capacity:

- **Full Tag Storage**: Some implementations provide tag storage for all memory
- **Limited Tag Storage**: Some implementations provide tag storage for a subset of memory
- **Tag Storage Pool**: Tag storage may be allocated from a fixed-size pool
- **Dynamic Allocation**: Tag storage may be dynamically allocated as needed

**Tag Storage Exhaustion:**

When tag storage is exhausted:

1. **Allocation Failure**: `alloc_tagged_*()` functions fail when tag storage is unavailable
2. **Error Reporting**: Tag storage exhaustion is reported via error return values
3. **Fallback Behavior**: Runtime may fall back to non-tagged allocation if tag storage unavailable
4. **Recovery**: Tag storage becomes available when tagged memory is freed

**Tag Storage Exhaustion Handling:**

The runtime handles tag storage exhaustion:

```silica
// Attempt tagged allocation
result: result<ref(R, normal, NodeData), alloc_error> <- 
    alloc_tagged_nodedata(region, size);

case result of {
    {ok, ptr} -> // Tagged allocation succeeded
        // Use tagged pointer
    {fail, error: alloc_error} -> // Allocation failed
        case error of {
            TagStorageExhausted -> // Tag storage is full
                // Fallback to non-tagged allocation or report error
            _ -> // Other allocation errors
                // Handle other errors
        }
}
```

**Tag Storage Usage Tracking:**

The runtime tracks tag storage usage:

- **Usage Monitoring**: Runtime monitors tag storage usage throughout program execution
- **Usage Reporting**: Runtime may provide tag storage usage statistics
- **Warning Thresholds**: Runtime may warn when tag storage usage exceeds thresholds
- **Exhaustion Prediction**: Runtime may predict tag storage exhaustion before it occurs

**Tag Storage Capacity Queries:**

Programs can query tag storage capacity:

```
// Query tag storage capacity (if supported by runtime)
get_tag_storage_capacity() -> OptionInt64  // Returns Some(capacity) or None
get_tag_storage_usage() -> OptionInt64     // Returns Some(usage) or None
get_tag_storage_available() -> OptionInt64  // Returns Some(available) or None
```

**Note**: Tag storage capacity queries are optional and may not be available on all implementations. Programs should handle `None` return values gracefully.

**Tag Storage Exhaustion Prevention:**

To prevent tag storage exhaustion:

1. **Free Tagged Memory**: Free tagged memory when no longer needed using `free_tagged_*()`
2. **Limit Tagged Allocations**: Use tagged allocations only when MTE safety is required
3. **Monitor Usage**: Monitor tag storage usage and adjust allocation strategies
4. **Batch Operations**: Batch tagged allocations and frees to reduce fragmentation

**Tag Storage Fragmentation:**

Tag storage may become fragmented:

- **Fragmentation Impact**: Fragmented tag storage may reduce effective capacity
- **Fragmentation Mitigation**: Runtime may defragment tag storage periodically
- **Allocation Strategies**: Contiguous allocations reduce fragmentation

**Error Reporting for Tag Storage Exhaustion:**

When tag storage is exhausted:

```
❌ Runtime error: TagStorageExhausted at example.silica:15:10

Tag storage capacity exhausted
Tag storage usage: 95% of capacity
Consider freeing tagged memory or reducing tagged allocations
See specification: spec:§21.3.3.1
```

**Cross-References:**
- See Section 21.3.3 (MTE Runtime Integration) for MTE initialization and tag checking
- See Section 21.3.2 (Tagged Pointer Operations) for tagged allocation functions

**Performance Implications:**

MTE tag checking has minimal performance impact:

- **Tag Check Overhead**: < 1% for most workloads
  - Hardware-accelerated tag comparison
  - No software overhead for tag checking
  - Tag storage uses separate memory (doesn't compete with data cache)
- **Tag Storage Overhead**: ~3.125% additional memory
  - 4 bits per 16-byte granule
  - Tag storage is separate from data memory
  - Tag storage participates in cache hierarchy
- **Cache Effects**: Tag checking uses same cache hierarchy as data
  - Tag storage cache hits: minimal overhead
  - Tag storage cache misses: additional memory access (~50-100 nanoseconds)

**Tag Checking Performance Characteristics:**

| Operation | Overhead | Description |
|-----------|----------|-------------|
| **Tag Check (cache hit)** | < 1 cycle | Tag comparison in hardware |
| **Tag Check (cache miss)** | +50-100ns | Tag storage memory access |
| **Tag Fault** | ~1-10μs | Fault handling overhead |
| **Tag Allocation** | ~10-50ns | Tag generation and storage |

**Tag Propagation Rules:**

Tags propagate through reference operations:

1. **Reference Creation**: New references inherit tags from source
2. **Reference Copying**: Copied references maintain tags
3. **Reference Passing**: Tags preserved when passing references between functions
4. **Region Operations**: Tags preserved across region operations

**Tag Isolation:**

MTE provides hardware-enforced isolation between regions:

- **Region Tagging**: Each region can have distinct tag values
- **Cross-Region Access**: Accessing one region's memory with another region's tag causes fault
- **Isolation Guarantee**: Tags prevent accidental cross-region access

**Error Handling for Tag Violations:**

Tag violations indicate memory safety issues:

- **Use-After-Free**: Tag mismatch when accessing freed memory
- **Buffer Overflow**: Tag mismatch when accessing out-of-bounds memory
- **Double-Free**: Tag mismatch when freeing already-freed memory
- **Memory Corruption**: Tag mismatch when accessing corrupted memory

**Runtime Tag Management:**

The runtime manages tags automatically:

```pseudocode
function allocate_tagged_memory(region, size):
    // Allocate memory (16-byte aligned)
    address = allocate_aligned_memory(size, 16)
    
    // Generate random tag (4-bit value: 0-15)
    tag = generate_random_tag()
    
    // Store tag in tag storage
    store_tag_in_storage(address, tag)
    
    // Embed tag in pointer (bits 56-59)
    tagged_pointer = address | (tag << 56)
    
    return tagged_pointer
end function

function free_tagged_memory(tagged_pointer):
    // Extract address (clear tag bits)
    address = tagged_pointer & 0x00FFFFFFFFFFFFFF
    
    // Clear tag in tag storage (set to invalid tag)
    clear_tag_in_storage(address)
    
    // Free memory
    free_memory(address)
end function
```

**Tag Checking Behavior Summary:**

| Aspect | Behavior | Performance Impact |
|--------|----------|-------------------|
| **Tag Check Timing** | Synchronous with memory access | < 1% overhead |
| **Tag Storage** | Separate from data memory | ~3.125% memory overhead |
| **Tag Fault Latency** | Immediate detection | ~1-10μs fault handling |
| **Cache Effects** | Tag storage uses cache hierarchy | Minimal (cache hits) |
| **Hardware Acceleration** | Tag comparison in hardware | No software overhead |

**Tag Management:**

Tags are managed automatically by the runtime:

- **Tag Allocation**: Tags are allocated when memory is allocated via `alloc_tagged_*` functions
- **Tag Deallocation**: Tags are cleared when memory is freed via `free_tagged_*` functions
- **Tag Preservation**: Tags are preserved across region operations (tags move with data when regions are copied)
- **Tag Isolation**: Tags are isolated per region - tags from one region cannot be used to access another region's memory

#### 21.3.4 MTE Tag Allocation Strategies and Management

The runtime implements sophisticated tag allocation strategies to maximize security while minimizing tag space exhaustion and performance overhead.

**Tag Allocation Policies:**

The runtime supports multiple tag allocation policies, each optimized for different use cases:

- **Per-Region Tagging**: Each region receives a unique tag value, providing strong isolation between regions
  - **Security**: Maximum isolation - cross-region access is prevented by tag mismatch
  - **Tag Space Usage**: Efficient tag usage - one tag per region regardless of region size
  - **Use Case**: General-purpose memory allocation with strong isolation guarantees
  
- **Per-Allocation Tagging**: Each allocation within a region receives a unique tag value
  - **Security**: Fine-grained isolation - each allocation is protected from other allocations
  - **Tag Space Usage**: Higher tag usage - one tag per allocation
  - **Use Case**: High-security scenarios requiring per-allocation protection
  
- **Per-Type Tagging**: All allocations of the same type receive the same tag value
  - **Security**: Type-based isolation - allocations of different types are isolated from each other
  - **Tag Space Usage**: Efficient tag usage - one tag per type
  - **Use Case**: Type-safe memory management with reduced tag space usage

**Tag Generation Strategies:**

Tag values are generated using different strategies based on security requirements:

- **Random Tag Generation**: Tags are generated using cryptographically secure random number generation
  - **Security**: Maximum security - unpredictable tag values prevent tag guessing attacks
  - **Performance**: Minimal overhead - random generation is hardware-accelerated on many AArch64 implementations
  - **Use Case**: High-security scenarios requiring maximum protection
  
- **Sequential Tag Generation**: Tags are generated sequentially, wrapping around when tag space is exhausted
  - **Security**: Moderate security - predictable tag values but still provide isolation
  - **Performance**: Minimal overhead - simple counter increment
  - **Use Case**: Performance-critical scenarios with moderate security requirements
  
- **Hash-Based Tag Generation**: Tags are generated by hashing allocation metadata (address, size, type)
  - **Security**: Good security - deterministic but hard to predict without allocation metadata
  - **Performance**: Moderate overhead - hash computation required
  - **Use Case**: Balanced security and performance requirements

**Tag Checking Overhead and Optimization:**

Tag checking overhead is minimized through hardware acceleration and optimization strategies:

- **Hardware Tag Checking**: MTE hardware performs tag checking synchronously with memory access, adding minimal overhead (< 1% for most workloads)
- **Tag Storage Caching**: Tag storage participates in the cache hierarchy, minimizing tag storage access latency
- **Tag Check Elimination**: The compiler eliminates redundant tag checks when tag values are statically known
- **Tag Check Hoisting**: Tag checks are hoisted outside loops when possible, reducing per-iteration overhead

**Tag Propagation Rules:**

Tags propagate through reference operations according to specific rules:

- **Reference Creation**: When a new reference is created from an existing reference, the new reference inherits the tag from the source reference
- **Reference Copying**: When a reference is copied (e.g., passed as function parameter), the copied reference maintains the same tag as the original
- **Reference Passing**: When references are passed between functions or actors, tags are preserved to maintain isolation guarantees
- **Region Operations**: When regions are copied or moved, tags are preserved to maintain isolation across region operations
- **Tag Modification**: Tags can be explicitly modified using `set_tag_*` functions, but this breaks isolation guarantees and should be used with caution

**Tag Space Exhaustion Handling:**

The runtime handles tag space exhaustion gracefully with sophisticated tag management strategies:

- **Tag Space Limits**: MTE provides 4-bit tags (values 0-15), providing 16 distinct tag values
- **Tag Reuse**: When tag space is exhausted, the runtime reuses tags from deallocated memory
- **Tag Validation**: Before reusing a tag, the runtime validates that all memory with that tag has been deallocated
- **Tag Space Monitoring**: The runtime monitors tag space usage and warns when tag space is approaching exhaustion
- **Fallback Strategies**: When tag space is exhausted, the runtime can fall back to software-based bounds checking

**Tag Allocation Failure Scenarios:**

The runtime handles various tag allocation failure scenarios:

**Scenario 1: Tag Space Exhaustion**

When all 16 tag values are in use and a new allocation requires a tag:

```pseudocode
function allocate_with_tag_exhaustion(region, size):
    // Attempt to allocate tag
    available_tag = find_available_tag()
    
    if available_tag == NO_TAG_AVAILABLE:
        // Tag space exhausted
        // Strategy 1: Wait for tag reuse (if memory is being freed)
        // Strategy 2: Reuse tag from deallocated memory (after validation)
        // Strategy 3: Fall back to software bounds checking
        
        // Try to reuse tag from deallocated memory
        reused_tag = find_reusable_tag()
        
        if reused_tag != NO_TAG_AVAILABLE:
            // Validate tag is safe to reuse
            if validate_tag_reuse(reused_tag):
                // Reuse tag
                tag = reused_tag
            else:
                // Tag still in use, cannot reuse
                // Fall back to software bounds checking
                return allocate_with_software_bounds_checking(region, size)
            end if
        else:
            // No tags available for reuse
            // Fall back to software bounds checking
            return allocate_with_software_bounds_checking(region, size)
        end if
    else:
        // Tag available, use it
        tag = available_tag
    end if
    
    // Allocate memory with tag
    return allocate_tagged_memory(region, size, tag)
end function
```

**Scenario 2: Tag Reuse Validation Failure**

When attempting to reuse a tag, validation may fail:

```pseudocode
function validate_tag_reuse(tag):
    // Check if any memory with this tag is still allocated
    allocated_memory = find_memory_with_tag(tag)
    
    if allocated_memory != EMPTY:
        // Tag still in use, cannot reuse
        return false
    else:
        // Tag is safe to reuse
        return true
    end if
end function
```

**Scenario 3: Tag Allocation Performance Degradation**

When tag space is nearly exhausted, tag allocation performance may degrade:

- **Tag Search Overhead**: Finding available tags requires scanning tag space (O(n) where n is number of tags)
- **Tag Validation Overhead**: Validating tag reuse requires checking allocated memory (O(m) where m is number of allocations)
- **Performance Impact**: Tag allocation time increases from ~10-50ns to ~100-500ns when tag space is exhausted

**Tag Allocation Failure Recovery Strategies:**

The runtime implements multiple recovery strategies:

**Strategy 1: Tag Reuse with Validation**

When tag space is exhausted, the runtime attempts to reuse tags from deallocated memory:

```pseudocode
function recover_via_tag_reuse():
    // Find tags from deallocated memory
    deallocated_tags = find_deallocated_tags()
    
    for each tag in deallocated_tags:
        // Validate tag is safe to reuse
        if validate_tag_reuse(tag):
            // Tag is safe to reuse
            return tag
        end if
    end for
    
    // No reusable tags found
    return NO_TAG_AVAILABLE
end function
```

**Strategy 2: Software-Based Bounds Checking Fallback**

When tag reuse is not possible, the runtime falls back to software-based bounds checking:

```pseudocode
function allocate_with_software_bounds_checking(region, size):
    // Allocate memory without MTE tagging
    address = allocate_memory(region, size)
    
    // Use software bounds checking instead of hardware tagging
    // Compiler generates bounds checks for all memory accesses
    // Runtime tracks allocation bounds in software metadata
    
    // Store bounds in software metadata
    store_allocation_bounds(address, size)
    
    // Return untagged pointer (bounds checked in software)
    return address
end function
```

**Strategy 3: Tag Space Monitoring and Prevention**

The runtime monitors tag space usage to prevent exhaustion:

```pseudocode
function monitor_tag_space():
    // Track tag usage
    used_tags = count_used_tags()
    total_tags = 16  // MTE provides 16 tag values
    
    // Calculate usage percentage
    usage_percentage = (used_tags / total_tags) * 100
    
    // Warn when approaching exhaustion
    if usage_percentage > 75:
        log_warning("Tag space usage high: " + usage_percentage + "%")
    end if
    
    if usage_percentage > 90:
        log_error("Tag space nearly exhausted: " + usage_percentage + "%")
        // Suggest tag reuse or software fallback
    end if
end function
```

**Tag Allocation Failure Behavioral Guarantees:**

The runtime provides the following behavioral guarantees for tag allocation failures:

1. **Correctness**: Memory allocation always succeeds, even when tag allocation fails (falls back to software bounds checking)
2. **Memory Safety**: Memory safety is maintained regardless of tag allocation success or failure
3. **Performance Degradation**: Tag allocation failures may cause performance degradation (software bounds checking is slower than hardware tagging)
4. **Transparency**: Tag allocation failures are transparent to user code (no exceptions raised, graceful fallback)

**Tag Allocation Failure Handling Examples:**

**Example 1: Tag Exhaustion with Reuse**

```silica
fn allocate_with_tag_reuse() -> ref(R, normal, NodeData) proc[mem(normal)] {
    do
        r: region(R, normal) <- alloc_region(normal);
        
        // Attempt allocation (may trigger tag reuse if tag space exhausted)
        node: ref(R, normal, NodeData) <- alloc_tagged_nodedata(r, 1);
        // Runtime handles tag exhaustion internally:
        // 1. Attempts to reuse tags from deallocated memory
        // 2. Validates tag reuse safety
        // 3. Falls back to software bounds checking if reuse fails
        
        node
    end
}
```

**Example 2: Tag Exhaustion Detection**

```silica
fn check_tag_space_usage() -> boolean proc[] {
    // Runtime monitors tag space usage internally
    // Programs can query tag space status (if runtime provides API)
    // For now, tag exhaustion is handled transparently by runtime
    
    // Allocation always succeeds (may use software fallback)
    return true
}
```

**Tag Allocation Failure Performance Characteristics:**

| Failure Scenario | Allocation Time | Memory Safety | Performance Impact |
|-----------------|----------------|---------------|-------------------|
| Normal allocation | ~10-50ns | Hardware tagging | Minimal overhead |
| Tag exhaustion with reuse | ~100-300ns | Hardware tagging (reused tag) | Moderate overhead (tag search) |
| Tag exhaustion without reuse | ~500-1000ns | Software bounds checking | Higher overhead (software checks) |
| Tag space monitoring | ~1-5ns | Hardware tagging | Minimal overhead (monitoring) |

**Tag Allocation Failure Best Practices:**

Developers should be aware of tag allocation failure scenarios:

- **Tag Space Management**: Be aware that MTE provides limited tag space (16 values)
- **Tag Reuse**: Runtime automatically reuses tags from deallocated memory when possible
- **Software Fallback**: Runtime falls back to software bounds checking when tag reuse is not possible
- **Performance Impact**: Tag exhaustion may cause performance degradation (software bounds checking)
- **Transparency**: Tag allocation failures are handled transparently - no code changes required

**Cross-References:**
- See Section 12.1.1.1 (AArch64 Memory Attribute Mapping) for memory space configuration and tag support
- See Section 12.1.1.2 (Memory Space Runtime Guarantees) for tag checking guarantees in different memory spaces
- See Section 12.2 (Reference Semantics) for tag propagation through reference operations
- See Section 12.4.1 (Region Isolation) for tag-based region isolation guarantees
- See Section 21.3.4 (MTE Tag Allocation Strategies and Management) for tag allocation strategies

**Tag Allocation for Different Memory Spaces:**

Different memory spaces use different tag allocation strategies:

- **normal**: Uses per-region tagging by default, providing strong isolation between regions
- **normal_writethrough**: Uses per-region tagging, tags checked on write-through operations
- **normal_noncacheable**: Uses per-allocation tagging for maximum isolation of non-cacheable memory
- **atomic**: Uses per-region tagging, tags checked before atomic operations
- **device**: Does not use MTE tagging (device memory does not support MTE)

**Tag Allocation Performance Characteristics:**

Tag allocation performance varies based on allocation policy:

- **Per-Region Tagging**: O(1) tag allocation, typically 10-20 nanoseconds per region allocation
- **Per-Allocation Tagging**: O(1) tag allocation, typically 10-30 nanoseconds per allocation
- **Per-Type Tagging**: O(1) tag lookup, typically 5-10 nanoseconds per allocation
- **Tag Space Exhaustion Handling**: O(n) tag validation, typically 100-500 nanoseconds when tag space is exhausted

**Tag Management Best Practices:**

The runtime provides guidance for effective tag management:

- **Tag Policy Selection**: Choose tag allocation policy based on security requirements and tag space constraints
- **Tag Space Monitoring**: Monitor tag space usage to avoid exhaustion in long-running programs
- **Tag Reuse**: Allow tag reuse when security requirements permit to maximize tag space utilization
- **Tag Isolation**: Use per-region or per-allocation tagging when strong isolation is required
- **Tag Performance**: Use per-type tagging when performance is critical and security requirements are moderate

**Cross-References:**
- See Section 12.1.1.1 (AArch64 Memory Attribute Mapping) for memory space configuration and tag support
- See Section 12.1.1.2 (Memory Space Runtime Guarantees) for tag checking guarantees in different memory spaces
- See Section 12.2 (Reference Semantics) for tag propagation through reference operations
- See Section 12.4.1 (Region Isolation) for tag-based region isolation guarantees

**Interaction with Region-Based Memory:**

MTE integrates seamlessly with Silica's region-based memory model:

1. **Region Tagging**: Each region can optionally use MTE tagging (determined by the memory space type)
2. **Reference Tagging**: References allocated in tagged regions automatically receive tags
3. **Tag Propagation**: Tags propagate through reference operations (`read_ref`, `write_ref`)
4. **Region Isolation**: MTE tags provide additional hardware-enforced isolation between regions

**Tag Operations:**

```
set_tag_nodedata(ptr: ref(R, normal, NodeData), tag: int) -> ref(R, normal, NodeData)
```
Sets the tag in the pointer (bits 56-59). The tag value must be in range 0-15. This operation does not modify memory tags, only the pointer tag.

```
get_tag_nodedata(ptr: ref(R, normal, NodeData)) -> int
```
Extracts the tag value from the pointer (bits 56-59).

```
check_tag_nodedata(ptr: ref(R, normal, NodeData)) -> boolean
```
Checks if the pointer tag matches the memory tag at the pointer's address. Returns `true` if tags match, `false` if mismatch (without generating a fault). This is a software check that does not trigger hardware tag faults.

**Hardware Tag Fault Handling:**

When MTE detects a tag mismatch:
1. Hardware generates a tag fault exception
2. Runtime receives `SIGSEGV` with `SEGV_MTESERR` code
3. Runtime can optionally provide fault handlers, but by default terminates the program
4. Tag faults indicate memory safety violations (use-after-free, buffer overflow, etc.)

**Performance Characteristics:**

- **Tag Checking Overhead**: Hardware tag checking adds minimal overhead (< 1% for most workloads)
- **Tag Storage**: MTE uses separate tag storage (4 bits per 16-byte granule), requiring ~3.125% additional memory
- **Tag Allocation**: Tag allocation is automatic and transparent to the programmer
- **Cache Effects**: Tag checking uses the same cache hierarchy as data, with minimal impact on cache performance

**MTE and Memory Spaces:**

MTE tagging is available for all memory spaces:
- `normal`: Tagged by default for memory safety
- `normal_writethrough`: Tagged, tags checked on write-through operations
- `normal_noncacheable`: Tagged, tags checked even for non-cacheable accesses
- `atomic`: Tagged, tags checked before atomic operations
- `device`: Not tagged (device memory does not support MTE)

**Example Usage:**

```silica
struct NodeData {
    value: int64,
    next: int64
}

impl tagged for NodeData;

fn example() -> atom {
    do
        r: region(R, normal) <- alloc_region(normal);
        // Allocate tagged memory
        node: ref(R, normal, NodeData) <- alloc_tagged_nodedata(r, 1);
        
        // Tag is automatically set and checked by hardware
        data: NodeData <- read_ref(node);  // Hardware validates tag
        
        // Get tag value
        tag: int <- get_tag_nodedata(node);
        
        // Set new tag (pointer tag only, not memory tag)
        new_ptr: ref(R, normal, NodeData) <- set_tag_nodedata(node, 5);
        
        // Check tag match
        matches: boolean <- check_tag_nodedata(new_ptr);
        
        // Free tagged memory (clears tags)
        free_tagged_nodedata(node);
    end
}
```

### 21.4 Pointer Authentication (PAC)

#### 21.4.1 Authenticated Types and Trait
Pointer Authentication Codes (PAC) provide cryptographic signing of pointers for security. Types that can be used with PAC must implement the `authenticated` trait.

```
trait authenticated {
    // Marker trait for types that can be used with PAC
}

// Example: Implement authenticated trait for a struct
struct SecureData {
    value: int64,
    metadata: string
}

impl authenticated for SecureData;
```

#### 21.4.2 Authenticated Pointer Operations
Authenticated pointer operations work with any type that implements the `authenticated` trait. Since Silica does not support generics, separate functions are provided for each concrete type that implements the `authenticated` trait:

```
// Sign pointer with context (type must implement authenticated trait)
// For SecureData type (example):
sign_ptr_securedata(ptr: ref(R, Space, SecureData), context: int) -> ref(R, Space, SecureData)

// Authenticate pointer - hardware validates signature
auth_ptr_securedata(ptr: ref(R, Space, SecureData), context: int) -> ref(R, Space, SecureData) proc[mem(Space)]

// Check if authentication would fail (without dereferencing)
auth_fail_securedata(ptr: ref(R, Space, SecureData), context: int) -> boolean
```

**Note**: For each type that implements the `authenticated` trait, the compiler generates type-specific versions of these functions. The function names follow the pattern `sign_ptr_<typename>`, `auth_ptr_<typename>`, `auth_fail_<typename>`, etc.

**Note**: The `authenticated` trait can only be implemented for named types (structs, type aliases), not for primitive types. This ensures type safety and prevents misuse of PAC features.

#### 21.4.3 PAC Runtime Integration

Pointer Authentication Codes (PAC) provide cryptographic signing of pointers for security. The runtime manages PAC keys and authentication throughout program execution.

**Key Management:**

PAC uses cryptographic keys stored in AArch64 system registers:

- **APIAKey** (Instruction Key A): Used for instruction pointer authentication
- **APIBKey** (Instruction Key B): Used for instruction pointer authentication (alternative)
- **APDAKey** (Data Key A): Used for data pointer authentication
- **APDBKey** (Data Key B): Used for data pointer authentication (alternative)
- **APGAKey** (Generic Key A): Used for generic pointer authentication

The runtime:
1. **Key Initialization**: Keys are initialized at program startup (typically random values)
2. **Key Storage**: Keys are stored in `APIAKey_EL1`, `APIBKey_EL1`, `APDAKey_EL1`, `APDBKey_EL1`, `APGAKey_EL1` registers
3. **Key Protection**: Keys are protected by privilege level (EL1 for user-space, EL2/EL3 for kernel/hypervisor)
4. **Key Rotation**: Keys can be rotated for enhanced security (requires re-signing all pointers)

#### 21.4.4 PAC Key Selection Strategies

The runtime implements sophisticated key selection strategies to maximize security while maintaining performance and compatibility.

**Key Selection Policies:**

The runtime supports multiple key selection policies for different use cases:

- **Instruction Pointer Keys (APIAKey, APIBKey)**: Used for authenticating instruction pointers (function return addresses, indirect jumps)
  - **APIAKey**: Primary instruction pointer key, used for most instruction pointer authentication
  - **APIBKey**: Alternative instruction pointer key, used for additional security layers or key rotation
  - **Use Case**: Protecting return addresses and indirect control flow transfers
  
- **Data Pointer Keys (APDAKey, APDBKey)**: Used for authenticating data pointers (references, function pointers)
  - **APDAKey**: Primary data pointer key, used for most data pointer authentication
  - **APDBKey**: Alternative data pointer key, used for additional security layers or key rotation
  - **Use Case**: Protecting data references and function pointers passed as data
  
- **Generic Key (APGAKey)**: Used for generic pointer authentication when specific key type is not required
  - **Use Case**: Generic pointer authentication for compatibility or when key type is not critical

**Key Initialization Strategies:**

Keys are initialized using different strategies based on security requirements:

- **Random Key Generation**: Keys are generated using cryptographically secure random number generation
  - **Security**: Maximum security - unpredictable keys prevent key guessing attacks
  - **Performance**: Minimal overhead - random generation is hardware-accelerated on many AArch64 implementations
  - **Use Case**: High-security scenarios requiring maximum protection
  
- **Derived Key Generation**: Keys are derived from a master key using key derivation functions
  - **Security**: Good security - keys are unpredictable but reproducible from master key
  - **Performance**: Moderate overhead - key derivation requires computation
  - **Use Case**: Scenarios requiring key recovery or key synchronization across processes
  
- **Fixed Key Generation**: Keys are set to fixed values (for testing or compatibility)
  - **Security**: Low security - predictable keys provide minimal protection
  - **Performance**: Minimal overhead - no key generation required
  - **Use Case**: Testing, debugging, or compatibility scenarios

**Key Selection for Different Pointer Types:**

Different pointer types use different keys based on their usage patterns:

- **Function Return Addresses**: Use APIAKey (instruction pointer key) for authenticating return addresses
- **Indirect Function Calls**: Use APIAKey for authenticating function pointers used in indirect calls
- **Data References**: Use APDAKey (data pointer key) for authenticating data references
- **Function Pointers as Data**: Use APDAKey for authenticating function pointers stored as data
- **Generic Pointers**: Use APGAKey for generic pointer authentication when key type is not critical

**Key Rotation Strategies:**

Keys can be rotated for enhanced security:

- **Periodic Rotation**: Keys are rotated periodically (e.g., every hour, every day) to limit exposure window
- **Event-Based Rotation**: Keys are rotated based on security events (e.g., suspected compromise, privilege escalation)
- **Gradual Rotation**: Keys are rotated gradually using alternative keys (APIBKey, APDBKey) to minimize disruption
- **Rotation Overhead**: Key rotation requires re-signing all pointers, which can be expensive for large programs

**Key Selection Performance Implications:**

Key selection affects performance characteristics:

- **Key Access**: Key access from system registers is fast (~1-2 cycles) but requires privilege level access
- **Key Switching**: Switching between keys (e.g., APIAKey vs APIBKey) requires minimal overhead
- **Key Derivation**: Key derivation adds overhead (~100-500 cycles) but is only performed during initialization
- **Key Storage**: Keys are stored in system registers, providing fast access without memory overhead

#### 21.4.5 PAC Authentication Failure Handling

The runtime provides comprehensive handling of PAC authentication failures, balancing security requirements with program recovery capabilities.

**Authentication Failure Detection:**

Authentication failures are detected by AArch64 hardware during pointer authentication:

- **Hardware Detection**: AArch64 hardware automatically detects authentication failures during `AUTDA`, `AUTDB`, `AUTIA`, `AUTIB` instruction execution
- **Failure Indication**: Authentication failures are indicated by hardware exception generation, not by return value
- **Failure Timing**: Authentication failures are detected synchronously with authentication instruction execution
- **Failure Information**: Hardware provides failure information including fault address and authentication error code

**Authentication Failure Causes:**

Authentication failures can occur due to various causes:

- **Pointer Corruption**: Pointer value has been modified, invalidating the embedded PAC
- **Context Mismatch**: Context value used for authentication differs from context value used for signing
- **Key Mismatch**: Key used for authentication differs from key used for signing (e.g., after key rotation)
- **Memory Corruption**: Memory containing pointer has been corrupted, modifying the pointer value
- **Attack Attempt**: Malicious modification of pointer or context to bypass authentication (ROP/JOP attack)

**Authentication Failure Handling Strategies:**

The runtime supports multiple strategies for handling authentication failures:

- **Termination Strategy**: Program terminates immediately on authentication failure (default behavior)
  - **Security**: Maximum security - prevents any execution after authentication failure
  - **Recovery**: No recovery - program cannot continue after authentication failure
  - **Use Case**: High-security scenarios where authentication failures indicate security violations
  
- **Fault Handler Strategy**: Custom fault handlers are invoked on authentication failure
  - **Security**: Moderate security - allows program to handle authentication failures
  - **Recovery**: Limited recovery - program can attempt recovery but should be cautious
  - **Use Case**: Scenarios requiring graceful error handling or logging
  
- **Logging Strategy**: Authentication failures are logged but program continues (not recommended)
  - **Security**: Low security - allows execution after authentication failure
  - **Recovery**: Full recovery - program continues execution after logging failure
  - **Use Case**: Debugging or testing scenarios (not recommended for production)

**Authentication Failure Recovery:**

Recovery from authentication failures is limited due to security implications:

- **Pointer Validation**: Failed pointers cannot be validated - authentication failure indicates pointer is invalid
- **Context Correction**: Context mismatches can be corrected by using correct context value
- **Key Synchronization**: Key mismatches can be resolved by synchronizing keys (requires re-signing pointers)
- **Attack Mitigation**: Authentication failures indicating attacks should trigger security responses (termination, logging, alerting)

**Authentication Failure Performance Impact:**

Authentication failures have significant performance impact:

- **Fault Handling Overhead**: Authentication failure handling adds ~1-10 microseconds overhead per failure
- **Exception Processing**: Hardware exception processing adds ~100-500 nanoseconds overhead
- **Security Response**: Security responses (termination, logging) add additional overhead
- **Recovery Attempts**: Recovery attempts add overhead but may not succeed

**Authentication Failure Best Practices:**

The runtime provides guidance for effective authentication failure handling:

- **Default Termination**: Use default termination strategy unless specific recovery requirements exist
- **Fault Handler Design**: Design fault handlers carefully - authentication failures typically indicate security violations
- **Context Management**: Ensure context values are consistent between signing and authentication
- **Key Management**: Manage keys carefully - key mismatches cause authentication failures
- **Security Monitoring**: Monitor authentication failures - high failure rates may indicate attacks

**Cross-References:**
- See Section 21.4.4 (PAC Key Selection Strategies) for key selection and management
- See Section 12.2 (Reference Semantics) for pointer authentication in reference operations
- See Section 15.2.1 (Behavior Function Signature) for function pointer authentication
- See Section 25.1.1 (Runtime Errors) for authentication failure error handling

**Authentication Code Generation:**

When signing a pointer:

```
sign_ptr_securedata(ptr: ref(R, Space, SecureData), context: int) -> ref(R, Space, SecureData)
```

The runtime:
1. **Extracts Pointer Value**: Gets the 64-bit pointer value
2. **Computes PAC**: Uses QARMA64 cryptographic algorithm with:
   - Pointer value (64 bits)
   - Context value (provided as parameter, typically function address or constant)
   - Appropriate key (APDAKey for data pointers)
3. **Embeds PAC**: Inserts PAC into top bits of pointer:
   - Bits 63-56: PAC value (8 bits, truncated from 64-bit PAC)
   - Bits 55-48: Reserved (must be sign extension of bit 55)
   - Bits 47-0: Original pointer value
4. **Returns Signed Pointer**: Returns pointer with embedded PAC

**Authentication Validation:**

When authenticating a pointer:

```
auth_ptr_securedata(ptr: ref(R, Space, SecureData), context: int) -> ref(R, Space, SecureData) proc[mem(Space)]
```

The runtime:
1. **Extracts PAC**: Extracts PAC from pointer bits 63-56
2. **Recomputes PAC**: Computes expected PAC using same algorithm and context
3. **Compares PACs**: Compares extracted PAC with recomputed PAC
4. **Hardware Validation**: Uses AArch64 `AUTDA` (Authenticate Data Address) instruction:
   - If PAC matches: Pointer is authenticated, original pointer value is restored
   - If PAC mismatches: Hardware generates authentication fault (`SIGSEGV` with authentication error)
5. **Returns Authenticated Pointer**: Returns pointer with PAC removed (original pointer value)

**Authentication Failure Handling:**

When authentication fails:
1. **Hardware Fault**: AArch64 hardware generates authentication fault exception
2. **Runtime Handling**: Runtime receives `SIGSEGV` with authentication error code
3. **Security Violation**: Authentication failure indicates:
   - Pointer corruption
   - Memory corruption attack
   - Return-oriented programming (ROP) attack attempt
4. **Default Behavior**: By default, program terminates on authentication failure
5. **Optional Handlers**: Runtime can provide fault handlers, but authentication failures typically indicate security violations

**Context Values:**

Context values provide additional security by binding pointers to specific contexts:

- **Function Address**: Use function's address as context to bind pointers to specific functions
- **Constant Values**: Use constant values to distinguish different pointer uses
- **Call Site**: Use call site address to bind pointers to specific call locations

**Example Context Usage:**

```silica
// Sign pointer with function address as context
fn create_secure_ptr(data: SecureData) -> ref(R, normal, SecureData) proc[mem(normal)] {
    ptr: ref(R, normal, SecureData) <- alloc_ref(region, data);
    // Sign with function address as context
    fn_addr: int <- get_function_address(create_secure_ptr);
    sign_ptr_securedata(ptr, fn_addr)
}

// Authenticate pointer with same context
fn use_secure_ptr(signed_ptr: ref(R, normal, SecureData)) -> SecureData proc[mem(normal)] {
    fn_addr: int <- get_function_address(use_secure_ptr);
    // Authenticate before use
    authenticated_ptr: ref(R, normal, SecureData) <- auth_ptr_securedata(signed_ptr, fn_addr);
    read_ref(authenticated_ptr)
}
```

**PAC and Region-Based Memory:**

PAC integrates with Silica's region-based memory model:

1. **Region Isolation**: PAC provides additional security for region isolation
2. **Reference Authentication**: References can be authenticated before use
3. **Cross-Region Protection**: PAC prevents pointers from one region being used to access another region
4. **Lifetime Security**: PAC can encode lifetime information in the context value

**Performance Characteristics:**

- **Signing Overhead**: Pointer signing adds minimal overhead (~10-20 cycles per sign operation)
- **Authentication Overhead**: Pointer authentication adds minimal overhead (~10-20 cycles per auth operation)
- **Hardware Acceleration**: PAC uses dedicated AArch64 cryptographic instructions (`PACDA`, `AUTDA`)
- **Cache Effects**: PAC operations have minimal impact on cache performance

**Security Guarantees:**

- **Cryptographic Security**: PAC uses QARMA64 cryptographic algorithm (64-bit block cipher)
- **Tamper Detection**: Any modification to pointer or context invalidates authentication
- **Attack Prevention**: PAC prevents:
  - Return-oriented programming (ROP) attacks
  - Jump-oriented programming (JOP) attacks
  - Pointer corruption attacks
  - Memory corruption exploits

**Example: Secure Pointer Usage**

```silica
struct SecureData {
    value: int64,
    metadata: string
}

impl authenticated for SecureData;

fn secure_operation(data: SecureData) -> atom proc[mem(normal)] {
    do
        // Allocate secure data
        ptr: ref(R, normal, SecureData) <- alloc_ref(region, data);
        
        // Sign pointer with context
        context: int <- 0x1234;  // Context value
        signed_ptr: ref(R, normal, SecureData) <- sign_ptr_securedata(ptr, context);
        
        // Later: authenticate before use
        authenticated_ptr: ref(R, normal, SecureData) <- auth_ptr_securedata(signed_ptr, context);
        
        // Use authenticated pointer
        data: SecureData <- read_ref(authenticated_ptr);
    end
}
```

### 21.5 Apple Silicon Extensions

#### 21.5.1 AMX (Apple Matrix Engine)
The Apple Matrix Engine (AMX) provides hardware-accelerated matrix operations. Types that can be used with AMX must implement the `apple_matrix` trait.

```
module arch.apple.amx {

    trait apple_matrix {
        // Marker trait for types that can be used with AMX
    }

    // Example: Implement apple_matrix trait for a struct
    struct MatrixData {
        values: buf(R, normal, float32, 16)
    }

    impl apple_matrix for MatrixData;

    // AMX matrix operations (type must implement apple_matrix trait)
    // For each type that implements the apple_matrix trait, the compiler generates
    // type-specific versions of these functions. The function names follow the pattern
    // load_matrix_<typename>, store_matrix_<typename>, matmul_<typename>, etc.
    --
    // Example for MatrixData type:
    pub fn load_matrix_matrixdata(data: ref(R, Space, MatrixData), rows: int, cols: int) -> ref(R, Space, MatrixData)
    pub fn store_matrix_matrixdata(matrix: ref(R, Space, MatrixData), data: ref(R, Space, MatrixData)) -> atom
    pub fn matmul_matrixdata(a: ref(R, Space, MatrixData), b: ref(R, Space, MatrixData)) -> ref(R, Space, MatrixData)
    --
    // Similar functions are generated for all types that implement apple_matrix
}
```

**Note**: The `apple_matrix` trait can only be implemented for named types (structs, type aliases), not for primitive types. This ensures type safety and prevents misuse of AMX features.

**Note**: For each type that implements the `apple_matrix` trait, the compiler generates type-specific versions of these functions. The function names follow the pattern `load_matrix_<typename>`, `store_matrix_<typename>`, `matmul_<typename>`, etc. This approach maintains consistency with Silica's no-generics design principle, matching the pattern used for SVE, NEON, MTE, and PAC operations.

## 22. Built-in Functions

This section documents all built-in functions and primitives available in Silica. These are always available without requiring imports.

### 22.1 Memory Management Primitives

**Lifetime Factory:**
```
fresh_lifetime() -> lifetime
```
Returns a unique lifetime value. Each call produces a distinct lifetime. Use when allocating multiple regions in the same scope to avoid lifetime collisions. See §4.4.1, §12.1.4.

**Region Allocation:**
```
alloc_region(space: memory_space) -> region(L, space) proc[mem(space)]
alloc_ref(region, initial_value) -> ref(L, space, T) proc[mem(space)]
alloc_buf(region, capacity) -> buf(L, space, T, capacity) proc[mem(space)]
alloc_atomic(region, initial_value) -> atomic_ref(L, space, T) proc[mem(space), atomic]
alloc_region_on_numa_node(numa_node: int, space: memory_space) -> region(L, space) proc[mem(space)]
```
The lifetime L is taken from the binding's explicit type annotation (e.g. `region(L1, Space)`). L is typically obtained from `fresh_lifetime()`. Region allocation is permitted only within sequence blocks.

**Memory Space Types:**
- `normal` or `normal_writeback` - Write-back cacheable memory (default)
- `normal_writethrough` - Write-through cacheable memory
- `normal_noncacheable` - Non-cacheable memory
- `atomic` - Atomic memory for concurrent access
- `device` - Device memory (reserved for driver library)

**NUMA-Aware Allocation:**
The `alloc_region_on_numa_node()` function allocates a memory region on a specific NUMA node, optimizing memory access latency for NUMA-aware applications.

**Example Usage:**
```silica
sequence proc[mem(normal)]
    L1: lifetime <- fresh_lifetime();
    L2: lifetime <- fresh_lifetime();
    L3: lifetime <- fresh_lifetime();
    L4: lifetime <- fresh_lifetime();
    // Standard write-back cacheable memory (most common)
    region1: region(L1, normal) <- alloc_region(normal);
    // Write-through for immediate visibility
    region2: region(L2, normal_writethrough) <- alloc_region(normal_writethrough);
    // Non-cacheable for DMA buffers
    dma_region: region(L3, normal_noncacheable) <- alloc_region(normal_noncacheable);
    dma_buffer: buf(L3, normal_noncacheable, byte, 4096) <- alloc_buf(dma_region, 4096);
    // Atomic memory for shared counters
    atomic_region: region(L4, atomic) <- alloc_region(atomic);
    counter: atomic_ref(L4, atomic, int64) <- alloc_atomic(atomic_region, 0);
produces pure () end
```

### 22.2 Reference Operations
```
read_ref(reference) -> T proc[mem(space)]
write_ref(reference, value) -> atom proc[mem(space)]
```

### 22.3 Buffer Operations
```
read_buf(buffer, index) -> T proc[mem(space)]
write_buf(buffer, index, value) -> atom proc[mem(space)]
buffer_length(buffer) -> int
buffer_capacity(buffer) -> int
```

### 22.4 Actor Operations
```
spawn(initial_state, behavior [, core_affinity]) -> actor_ref proc[concurrency]
send(actor, message) -> atom proc[concurrency]
cast(actor, message) -> boolean proc[concurrency]
recv([actor]) -> Msg proc[mailbox, concurrency]          // Runtime internal
self() -> actor_ref proc[mailbox, concurrency]
```

**Note**: `recv()` is a runtime internal function and cannot be called directly from user code.

**Core Affinity Parameter:**
The `spawn()` function accepts an optional third parameter for CPU core affinity:
- `performance_cores` - Place actor on high-performance cores
- `efficiency_cores` - Place actor on low-power efficiency cores
- `core_id(n)` - Place actor on specific core ID
- `core_set([id1, id2, ...])` - Place actor on any core in the set

### 22.5 Print Functions

All print functions require the `device_io` effect.

#### 22.5.1 String Printing
```
print(value: string) -> atom proc[device_io]
println(value: string) -> atom proc[device_io]
```

#### 22.5.2 Numeric Printing
```
print_int8(value: int8) -> atom proc[device_io]
print_int16(value: int16) -> atom proc[device_io]
print_int32(value: int32) -> atom proc[device_io]
print_int64(value: int64) -> atom proc[device_io]
```

#### 22.5.3 Floating-Point Printing
```
print_float16(value: float16) -> atom proc[device_io]
print_float32(value: float32) -> atom proc[device_io]
print_float64(value: float64) -> atom proc[device_io]
```

#### 22.5.4 Other Type Printing
```
print_boolean(value: boolean) -> atom proc[device_io]
print_char(value: char) -> atom proc[device_io]
```

### 22.6 File I/O Functions

All file I/O functions require the `device_io` effect.

#### 22.6.1 File Reading
```
read_file(path: string) -> string proc[device_io]
read_lines(path: string) -> list<string> proc[device_io]
```

#### 22.6.2 File Writing
```
write_file(path: string, content: string) -> atom proc[device_io]
append_file(path: string, content: string) -> atom proc[device_io]
```

#### 22.6.3 File Operations
```
file_exists(path: string) -> boolean proc[device_io]
delete_file(path: string) -> atom proc[device_io]
get_file_size(path: string) -> int64 proc[device_io]
```

#### 22.6.4 Directory Operations
```
create_directory(path: string) -> atom proc[device_io]
remove_directory(path: string) -> atom proc[device_io]
list_directory(path: string) -> list<string> proc[device_io]
```

### 22.7 String Operations

String operations are pure functions (no effects required).

#### 22.7.1 String Length
```
length_bytes(s: string) -> int64            // byte length (UTF-8 encoded size)
length_chars(s: string) -> int64           // character count (Unicode scalar values)
```

These are the only user-available functions for finding the length of strings.

#### 22.7.2 String Manipulation
```
concat(a: string, b: string) -> string
substring(s: string, start: int64, end: int64) -> string   // character-based indices
substring_until_char(s: string, start: int64, char: char) -> string
```

#### 22.7.3 String Predicates
```
starts_with(s: string, prefix: string) -> boolean
ends_with(s: string, suffix: string) -> boolean
contains(s: string, substr: string) -> boolean
```

### 22.8 Process Execution

```
exec_command(command: string, args: list<string>) -> string proc[device_io]
```

Execute a system command and return its output.

### 22.9 System Information

```
get_cpu_topology_info() -> string proc[device_io]
```

Get CPU topology information as a JSON string.

### 22.10 CPU Affinity Operations

**CPU Affinity Types:**
```
type numa_info = {
    id: int,
    cores: list<int>,
    memory_ranges: list<memory_range>
}

type memory_range = {
    start_address: int,
    size: int,
    latency: int  // relative latency to this NUMA node
}

type cache_info = {
    levels: list<cache_level>
}

type cache_level = {
    level: int,  // L1, L2, L3
    size_kb: int,
    line_size: int,
    associativity: int,
    shared_cores: list<int>
}

type cpu_topology = {
    cores: list<core_info>,
    numa_nodes: list<numa_info>,
    cache_hierarchy: cache_info
}

type core_info = {
    id: int,
    core_type: core_type,  // efficiency | performance
    capabilities: list<string>,
    frequency_mhz: int
}

type core_type = efficiency | performance

type affinity_error =
    InvalidCoreId
  | CoreUnavailable
  | PermissionDenied
  | ResourceExhausted
```

**CPU Affinity Functions:**
All CPU affinity functions are pure runtime functions (no effects required):

```
// CPU topology and status discovery
get_cpu_topology() -> cpu_topology
get_efficiency_cores() -> list<int>
get_performance_cores() -> list<int>
get_core_capabilities(core_id: int) -> core_info

// Actor pinning operations
// Returns (int64, affinity_error) tuple: (0, error) on failure, (1, error) on success
// error is set to appropriate value or empty on success
pin_actor_to_core(actor: actor_ref, core_id: int) -> (int64, affinity_error)
pin_actor_to_efficiency_core(actor: actor_ref) -> (int64, affinity_error)
pin_actor_to_performance_core(actor: actor_ref) -> (int64, affinity_error)
pin_actor_realtime(actor: actor_ref, priority: int) -> (int64, affinity_error)
unpin_actor(actor: actor_ref) -> atom

// Actor removal
// Removes an actor from the system, unpinning it and allowing cleanup
// All actors are pinned until remove() is called
remove_actor(actor: actor_ref) -> (int64, affinity_error)

// Advanced scheduling hints
set_actor_priority(actor: actor_ref, priority: priority_level) -> atom
```

**Error Handling:**
Actor pinning functions return a tuple `(int64, affinity_error)`:
- On success: `(1, affinity_error)` where `affinity_error` is empty/unused
- On failure: `(0, affinity_error)` where `affinity_error` indicates the failure reason

**Actor Pinning Behavior:**
All actors are pinned to their initial core assignment until `remove_actor()` is called. Once an actor is pinned:
- It remains on the assigned core(s) until explicitly unpinned or removed
- The runtime respects the pinning constraint for scheduling decisions
- Thermal management and power optimization may still migrate the actor if necessary, but will attempt to return it to the pinned core when conditions allow

Example usage:
```silica
// Pin actor to specific core
(result: int64, error: affinity_error) <- pin_actor_to_core(actor_ref, 2);
case result of {
    1 -> // Success, actor pinned
    0 -> // Failure, check error for reason
}

// Remove actor (unpins and allows cleanup)
(remove_result: int64, remove_error: affinity_error) <- remove_actor(actor_ref);
case remove_result of {
    1 -> // Success, actor removed
    0 -> // Failure, check error for reason
}
```

### 22.11 SVE Vector Length Queries

SVE vector length query functions are built-in runtime functions (no effects required):

```
// Get SVE vector length in bytes (hardware vector length)
get_sve_vector_length() -> int
```

**Note**: This function queries the runtime SVE vector length in bytes. To determine how many elements of a specific type fit in one SVE vector, divide the vector length by the size of the element type using `size_of()`.

Example usage:
```silica
vector_bytes: int <- get_sve_vector_length();
int32_size: int <- size_of<int32>();
int32_elements: int <- vector_bytes / int32_size;
// int32_elements indicates how many int32 values fit in one SVE vector
```

### 22.12 Atomic Operations
```
atomic_load(ref, order) -> T proc[mem(space), atomic]
atomic_store(ref, value, order) -> atom proc[mem(space), atomic]
atomic_fetch_add(ref, delta, order) -> T proc[mem(space), atomic]
atomic_compare_exchange(ref, expected, new_val, order)
    -> {ok, T} | {fail, T} proc[mem(space), atomic]
```

### 22.13 Type Operations
```
size_of<T>() -> int                    // size in bytes
align_of<T>() -> int                   // alignment requirement
type_name<T>() -> string               // type name as string
```

**Note**: The `size_of<T>()`, `align_of<T>()`, and `type_name<T>()` functions use generic-like syntax for documentation purposes, but Silica requires concrete type arguments at compile time. For example, `size_of<int64>()` or `size_of<Point>()` where `Point` is a concrete struct type.

### 22.14 Runtime Operations
```
current_time() -> int proc[]           // milliseconds since epoch
random_int(min: int, max: int) -> int proc[]
hash<T>(value: T) -> int               // stable hash function
```

**Note**: The `hash<T>()` function uses generic-like syntax for documentation, but requires concrete type arguments at compile time.

### 22.15 String Operations
```
length_bytes(s: string) -> int64            // byte length; one of two user-available string length functions
length_chars(s: string) -> int64           // character count; one of two user-available string length functions
string_concat(s1: string, s2: string) -> string
string_slice(s: string, start: int, end: int) -> string
string_to_int(s: string) -> option<int>
int_to_string(n: int) -> string
```

### 22.16 Control Flow
```
panic(message: string) -> ! proc[]          // terminate with error
assert(condition: boolean, message: string) -> atom proc[]
unreachable() -> ! proc[]                   // mark unreachable code
```

## 23. Runtime System

### 23.1 Execution Environment

#### 23.1.1 Process Scheduler
The runtime provides a scheduler for process execution:

- **Fair Scheduling**: Processes are scheduled fairly across available cores
- **Preemptive**: Long-running processes can be preempted
- **Priority Support**: Optional priority hints for process scheduling
- **Load Balancing**: Automatic distribution across CPU cores

#### 23.1.2 Actor Runtime
Actors are managed by the runtime:

- **Mailbox Management**: Each actor has a dedicated message queue
- **Message Delivery**: Asynchronous message delivery with ordering guarantees
- **Failure Isolation**: Actor failures don't affect the runtime or other actors
- **Resource Limits**: Optional memory and message queue limits per actor

#### 23.1.3 CPU Scheduling and Affinity
The runtime provides intelligent CPU scheduling with optional affinity controls:

- **NUMA-Aware Scheduling**: Automatic scheduling considers memory locality to minimize cross-NUMA communication latency
- **Optional CPU Pinning**: Developers can optionally pin actors to specific cores or core types when needed
- **Core Type Awareness**: Distinguishes between efficiency cores (power-optimized) and performance cores (speed-optimized)
- **Load Balancing**: Automatic distribution across available cores with affinity constraints
- **Thermal Management**: Runtime monitors thermal conditions and migrates actors to prevent overheating while respecting affinity settings
- **Real-Time Scheduling**: Optional real-time priority scheduling for latency-critical actors with CPU affinity guarantees
- **Power Management**: Automatic migration between core types based on battery level and power constraints

#### 23.1.4 Memory Manager
Region-based memory management:

- **Region Tracking**: Runtime tracks region lifetimes and ownership
- **Garbage-Free**: No garbage collection - explicit region deallocation
- **Safety Checks**: Bounds checking and region isolation enforcement
- **Optimization**: Region coalescing and memory layout optimization

### 23.2 Capability System

#### 23.2.1 Effect Capabilities
Runtime enforces effect capabilities:

- **Capability Tokens**: Processes carry capability tokens for allowed effects
- **Stack Inspection**: Runtime checks capabilities on effectful operations
- **Dynamic Checking**: Effect violations caught at runtime with clear errors
- **Performance**: Capability checks optimized for common cases

#### 21.2.2 Memory Capabilities
Memory access requires appropriate capabilities:

```
Memory Spaces:
- normal / normal_writeback: General-purpose memory allocation (write-back cacheable)
- normal_writethrough: Write-through cacheable memory (immediate visibility)
- normal_noncacheable: Non-cacheable memory (for DMA buffers, shared memory)
- atomic: Memory for atomic operations
- device: Memory-mapped device access (reserved for future driver library)
```

**Memory Space Characteristics:**

**Normal Memory (write-back):**
- Cached with write-back policy
- Best performance for most use cases
- Writes cached, flushed on eviction
- Use for: Application data, actor state, general buffers

**Normal Memory (write-through):**
- Cached with write-through policy
- Writes update cache and memory immediately
- Slightly slower but ensures immediate visibility
- Use for: Data shared with DMA devices, when coherency is critical

**Normal Memory (non-cacheable):**
- No caching, direct memory access
- Bypasses cache entirely
- Use for: DMA buffers, network packet buffers, shared memory with devices

**Atomic Memory:**
- Supports hardware atomic operations
- Required for atomic references
- Use for: Shared counters, lock-free data structures

**Device Memory:**
- Memory-mapped I/O space
- Reserved for future device driver library
- Application code should not use this directly

#### 23.2.3 Concurrency Capabilities
Concurrency operations require capabilities:

- `concurrency`: Actor spawning and management
- `mailbox`: Message send/receive operations
- `atomic`: Atomic memory operations
- `cpu_affinity`: CPU pinning and affinity controls
- `networking`: Network device access and communication

### 23.3 Error Handling and Recovery

#### 23.3.1 Runtime Errors
Runtime catches and reports errors:

- **Effect Violations**: Missing capability for operation
- **Memory Errors**: Bounds violations, invalid references
- **Type Errors**: Pattern match failures, invalid operations
- **Resource Exhaustion**: Out of memory, too many actors

#### 23.3.2 Error Propagation
Errors propagate through the process system:

```
fn safe_divide(x: int, y: int) -> result<int, string> proc[] {
    if y == 0 {
        return Error("division by zero")
    }
    return Ok(x / y)
}

// Usage
do
    result: Result<int, string> <- safe_divide(10, 0)
    case result of
        Ok(value) -> print(value)
        Error(msg) -> print("Error: " + msg)
    end
end
```

## 24. Implementation Requirements

### 24.1 Compiler Obligations

#### 24.1.1 Type Safety
The compiler must ensure:

- **Type Soundness**: Well-typed programs don't go wrong
- **Effect Tracking**: All effects properly tracked and enforced
- **Memory Safety**: No dangling pointers or use-after-free
- **Concurrency Safety**: No data races or invalid message sends

#### 22.1.2 Optimization Requirements
The compiler should perform:

- **Effect Checking**: Verify all effects are explicitly declared
- **Region Optimization**: Minimize region allocation overhead
- **Actor Optimization**: Optimize message passing and actor scheduling
- **Vectorization**: Automatic vectorization for suitable loops (when safe)

#### 24.1.3 Code Generation
Generated code must:

- **Preserve Semantics**: Maintain operational semantics of the specification
- **Runtime Integration**: Properly interface with the Silica runtime
- **Platform Specific**: Generate optimal AArch64 code
- **Debuggable**: Support debugging with source-level information

### 24.2 Runtime Requirements

#### 24.2.1 Memory Management
The runtime must provide:

- **Region Allocation**: Safe region creation and deallocation
- **Reference Tracking**: Valid reference checking
- **Bounds Checking**: Array and buffer bounds validation
- **Atomic Operations**: Hardware-accelerated atomic primitives

#### 24.2.2 Concurrency Management
The runtime must support:

- **Actor Scheduling**: Fair and efficient actor execution
- **Message Delivery**: Reliable, ordered message passing
- **Synchronization**: Proper happens-before relationships
- **Failure Handling**: Graceful actor failure and cleanup

#### 24.2.3 Effect Enforcement
The runtime must enforce:

- **Capability Checking**: Effect capability validation
- **Isolation**: Actor and process isolation
- **Resource Limits**: Prevent resource exhaustion attacks
- **Performance**: Efficient capability checking

### 24.3 Conformance Testing

#### 24.3.1 Language Conformance
Implementations must pass:

- **Type Checking Tests**: All examples in specification type-check
- **Execution Tests**: Programs execute with expected results
- **Safety Tests**: No memory corruption or race conditions
- **Performance Tests**: Reasonable performance characteristics

#### 22.3.2 Runtime Conformance
Runtime implementations must:

- **Preserve Semantics**: Match operational semantics
- **Handle Errors**: Proper error reporting and recovery
- **Scale**: Support reasonable numbers of actors and processes
- **Stability**: No crashes under normal operation

## 25. Error Handling

### 25.1 Error Types

#### 25.1.1 Runtime Errors
```
type runtime_error =
    EffectViolation(string)        // missing capability
  | MemoryError(string)           // memory corruption
  | BoundsError(string)           // array bounds violation
  | TypeError(string)             // type mismatch at runtime
  | ActorError(string)            // actor failure
```

#### 25.1.2 Compilation Errors
```
type compile_error =
    TypeError(string)             // static type error
  | EffectError(string)           // effect mismatch
  | SyntaxError(string)           // parse error
  | ModuleError(string)           // module resolution error
```

### 25.2 Error Propagation

#### 25.2.1 Result-Based Error Handling
Functions return results to indicate success or failure:

```
fn parse_number(s: string) -> result<int, string> {
    // attempt parsing, return Ok(value) or Error(message)
}

fn safe_operation() -> result<atom, runtime_error> proc[] {
    // operations that might fail at runtime
}
```

#### 25.2.2 Panic Mechanism
For unrecoverable errors:

```
fn panic(message: string) -> ! {
    // terminates the current process with error message
    // '!' indicates this function never returns normally
}
```

#### 25.2.3 Actor Failure
Actors can fail and notify monitors:

```
type down_message = Down(actor_ref, exit_reason)

fn failing_actor(msg: unit, state: unit) -> atom proc[mailbox] {
    case msg of
        () -> panic("intentional failure")
    end
}
```

### 25.3 Error Recovery

#### 25.3.1 Supervision
Actors can supervise other actors:

```
fn supervisor(child_failure: down_message, state: supervisor_state)
 -> supervisor_state proc[mailbox] {

    case child_failure of
        Down(child_ref, reason) ->
            new_child: actor_ref <- spawn_actor(initial_state, child_behavior)
            // restart failed child
            return updated_state
    end
}
```

#### 25.3.2 Try-Catch Style
Monadic error handling:

```
do
    x: Result<T, Error> <- operation_that_might_fail()
    case x of
        Ok(result) -> continue_with(result)
        Error(err) -> handle_error(err)
    end
end
```

## 26. Platform Integration

### 26.1 AArch64 Architecture Support

#### 26.1.0 Hardware Architecture Integration

Silica's design fundamentally aligns with modern AArch64 chip architectures, providing optimizations that traditional languages cannot achieve.

**Cache Hierarchy Utilization:**
Silica's region-based memory model is designed to work optimally with modern CPU cache hierarchies (L1/L2/L3). Regions are allocated and managed to minimize cache thrashing and maximize cache locality:

- **Region Placement**: Runtime allocates regions to optimize for specific cache levels based on access patterns
- **Cache-Aware Scheduling**: Actor scheduling considers cache affinity to reduce cross-cache communication
- **Memory Layout**: Region-based allocation enables optimal memory layout for cache line utilization

**Cache Hierarchy Mapping:**
The compiler and runtime work together to map regions to cache levels:

1. **L1 Cache (Per-Core)**: Frequently accessed, small regions are placed to maximize L1 cache hits
2. **L2 Cache (Per-Core or Shared)**: Medium-sized regions with moderate access frequency target L2 cache
3. **L3 Cache (Shared)**: Large regions or infrequently accessed data are placed to utilize L3 cache
4. **Main Memory**: Very large regions or cold data are placed in main memory with cache line alignment

**Cache-Aware Region Allocation:**
```silica
// Runtime queries cache hierarchy
cache_info: cache_info <- get_cache_hierarchy()
// Returns cache levels with sizes, line sizes, and associativity

// Regions are automatically aligned to cache line boundaries
region: region(R, normal) <- alloc_region(normal)
// Compiler ensures region data structures align to cache lines (typically 64 bytes on AArch64)
```

**Cache Line Optimization:**
- Struct fields are arranged to minimize cache line false sharing
- Buffer allocations are aligned to cache line boundaries
- Region metadata is placed to avoid cache conflicts with region data

**Asymmetric Multiprocessing Support:**
AArch64's big.LITTLE and similar asymmetric designs are leveraged through Silica's actor model:

- **Core Type Awareness**: Runtime distinguishes between performance cores (high-speed) and efficiency cores (power-optimized)
- **Intelligent Task Placement**: Actors are automatically scheduled on appropriate core types based on workload characteristics
- **Dynamic Migration**: Runtime can migrate actors between core types based on system load and thermal conditions

**Memory Coherence and Interconnects:**
Silica's message-passing concurrency aligns with AArch64's cache-coherent interconnects:

- **Hardware-Assisted Messaging**: Actor communication leverages hardware message passing primitives where available
- **Coherence Protocol Optimization**: Region ownership reduces unnecessary coherence traffic
- **NUMA Optimization**: Cross-NUMA communication is minimized through intelligent actor placement

#### 26.1.1 Performance Guarantees

Silica provides formal performance guarantees that surpass traditional systems languages on AArch64 platforms.

**Memory Performance:**
- **Zero GC Overhead**: Region-based memory management eliminates garbage collection pauses
- **Hardware-Accelerated Safety**: MTE and PAC provide memory safety without software overhead
- **Optimal Cache Utilization**: Region-based allocation maximizes cache hit rates compared to stack-based approaches

**Concurrency Performance:**
- **Lock-Free Actor Communication**: Message passing eliminates locking overhead
- **NUMA-Aware Scheduling**: Automatic optimization for multi-socket systems
- **Hardware-Assisted Atomics**: Direct mapping to AArch64 atomic instructions

**Vector Performance:**
- **Native SVE Support**: Automatic scaling with hardware vector length
- **Hardware SIMD Acceleration**: Direct utilization of NEON and SVE instructions
- **Compiler Vectorization**: Automatic generation of vectorized code for suitable operations

**Comparative Performance Claims:**
Silica is designed to achieve C-level performance while providing memory safety and concurrency guarantees that C cannot offer:

- **Memory Safety Overhead**: ≤5% compared to unsafe C code
- **Concurrency Overhead**: ≤10% compared to hand-optimized thread-based C code
- **Vector Performance**: Equivalent to or better than C with hand-tuned SIMD intrinsics
- **Real-World Performance**: Silica applications typically outperform equivalent safe Rust code by 15-25% on AArch64

**Performance Benchmarks:**
Implementations must provide benchmarks demonstrating:
- Memory allocation/deallocation performance vs. C malloc/free
- Actor message passing throughput vs. thread-based communication
- Vector operation performance vs. C SIMD intrinsics
- Overall application performance vs. equivalent C and Rust implementations

#### 26.1.2 Instruction Selection
The compiler generates optimal AArch64 code:

- **Load/Store**: Efficient memory access patterns
- **Atomic Operations**: Direct mapping to LDXR/STXR instructions
- **Vector Instructions**: SVE/NEON code generation
- **Branch Prediction**: Optimal branch patterns

#### 26.1.3 Memory Model Mapping
Direct mapping to AArch64 memory model:

```
Silica Ordering    AArch64 Instruction
relaxed            LDR/STR
acquire            LDAR
release            STLR
acq_rel            LDAXR/STLXR + barriers
seq_cst            DMB + LDAR/STLR
```

#### 26.1.4 Hardware Features
Utilization of AArch64 hardware features:

- **Large Address Space**: 64-bit addressing
- **Memory Tagging**: Hardware-assisted bounds checking (MTE)
- **Pointer Authentication**: Code pointer protection (PAC)
- **Scalable Vectors**: SVE for data parallelism

#### 26.1.5 System Register Access Patterns and Privilege Levels

For comprehensive information about AArch64 system register access patterns, privilege levels, and fallback strategies, see Section 21.0.2 (AArch64 System Register Access).

**Register Access Examples Throughout Specification:**

The following sections document register access patterns:

- **Section 12.1.1.1 (AArch64 Memory Attribute Mapping)**: Documents `MAIR_EL1` configuration via kernel interfaces
- **Section 15.1.2.1 (AArch64 Runtime Integration)**: Documents `MPIDR_EL1`, `CLIDR_EL1`, `CCSIDR_EL1` access via system calls and kernel interfaces
- **Section 21.1.3.1 (SVE Runtime Behavior)**: Documents `ID_AA64ZFR0_EL1` and `ZCR_EL1` access via feature detection APIs
- **Section 26.1.0 (Hardware Architecture Integration)**: Documents cache hierarchy detection via kernel interfaces

**Error Handling:**

When register access fails:

1. **Non-Critical Information**: Runtime logs a warning and uses conservative defaults
   - Example: Cache size detection fails → assume standard cache sizes
   
2. **Critical Information**: Runtime reports an error and may abort initialization
   - Example: CPU topology detection fails → cannot schedule actors correctly

3. **Graceful Degradation**: Runtime disables features that require unavailable information
   - Example: SVE feature detection fails → disable SVE vectorization

**Cross-References:**
- See Section 12.1.1.1 (AArch64 Memory Attribute Mapping) for `MAIR_EL1` access patterns
- See Section 15.1.2.1 (AArch64 Runtime Integration) for CPU topology register access
- See Section 21.1.3.1 (SVE Runtime Behavior) for SVE feature register access
- See Section 26.1.0 (Hardware Architecture Integration) for cache hierarchy detection

### 26.2 Operating System Integration

#### 26.2.1 Thread Management
Runtime integrates with OS threading:

- **Native Threads**: Uses OS threads for actor scheduling
- **CPU Affinity**: Optional thread pinning to CPU cores
- **Priority Mapping**: Maps Silica priorities to OS priorities
- **Signal Handling**: Proper signal handling for runtime control

#### 26.2.2 Memory Management
Integration with OS memory facilities:

- **Virtual Memory**: Uses OS virtual memory for regions
- **Huge Pages**: Support for large page sizes
- **Memory Locking**: Optional memory locking for real-time use
- **NUMA Awareness**: NUMA-aware memory allocation

### 26.3 Foreign Function Interface

#### 26.3.1 C Interoperability (Future)
While Silica doesn't target C interop by design, future extensions might include:

- **Safe Wrappers**: Type-safe C function wrappers
- **Memory Layout**: Compatible data layout with C
- **Calling Convention**: AArch64 calling convention compliance
- **Error Propagation**: C error code to Silica result conversion

#### 26.3.2 Runtime Linking
Dynamic loading of Silica modules:

- **Module Loading**: Runtime module loading and linking
- **Version Compatibility**: Module version checking
- **Dependency Resolution**: Automatic dependency loading
- **Security**: Module signature verification

### 26.4 Performance Characteristics

#### 26.4.1 Memory Efficiency
- **Zero GC Overhead**: No garbage collection pauses
- **Compact Representations**: Efficient type representations
- **Cache-Friendly**: Optimized data layout for cache performance
- **Memory Reuse**: Region-based memory reuse patterns

#### 26.4.2 Concurrency Performance
- **Lock-Free Operations**: Where possible, lock-free algorithms
- **Message Batchinge**: Optimized message passing
- **Actor Locality**: NUMA-aware actor scheduling
- **Vector Acceleration**: Hardware vector utilization

#### 26.4.3 Startup and Compilation
- **Fast Compilation**: Incremental compilation support
- **Small Binaries**: Efficient code generation
- **Quick Startup**: Minimal runtime initialization
- **Cross-Compilation**: Support for cross-compiling to AArch64

#### 26.4.4 Performance Characteristics Summary

This section provides a quick reference for performance characteristics documented throughout the specification. For detailed explanations, see the referenced sections.

**Memory Space Latency Guarantees:**

| Memory Space | Write Visibility Latency | Cache Coherency | DMA Visibility Latency | Read Latency (Cache Hit) | Read Latency (Cache Miss) |
|--------------|-------------------------|-----------------|------------------------|-------------------------|---------------------------|
| `normal` | 10-100ns (cache-to-cache) | Yes (MESI/MOESI) | 100-500ns (with flush) | 1-5ns (L1), 5-15ns (L2), 15-50ns (L3) | 50-300ns |
| `normal_writethrough` | 50-200ns (memory write) | Yes (Shared/Exclusive) | 50-200ns (immediate) | 1-5ns (L1), 5-15ns (L2) | 50-200ns |
| `normal_noncacheable` | 100-300ns (memory access) | N/A (no cache) | 100-300ns (immediate) | N/A (no cache) | 100-300ns |
| `atomic` | 100-200ns (acquire/release) | Yes (MESI/MOESI) | 150-300ns (coherency) | 1-5ns (L1), 5-15ns (L2) | 50-300ns |
| `device` | Device-dependent | N/A | Direct | Device-dependent | Device-dependent |

**Cross-Reference**: See Section 12.1.1.2 (Memory Space Runtime Guarantees) for detailed explanations.

**Atomic Operation Performance:**

| Operation Type | Ordering | Typical Latency | Contention Impact | Hardware Instruction |
|----------------|----------|-----------------|-------------------|---------------------|
| Load | relaxed | 1-5ns (L1 cache) | Minimal | `LDR` |
| Load | acquire | 5-10ns (L1 cache) | Minimal | `LDAR` |
| Store | relaxed | 1-5ns (L1 cache) | Minimal | `STR` |
| Store | release | 5-10ns (L1 cache) | Minimal | `STLR` |
| RMW | relaxed | 50-100ns | Low: 50-100ns<br>High: 100-500ns | `LDXR`/`STXR` loop |
| RMW | acq_rel | 50-200ns | Low: 50-150ns<br>High: 150-500ns | `LDAXR`/`STLXR` + barriers |
| RMW | seq_cst | 150-300ns | Low: 150-250ns<br>High: 250-600ns | `DMB` + `LDAXR`/`STLXR` + `DMB` |

**Cross-Reference**: See Section 17.2 (Memory Ordering Semantics) for detailed ordering guarantees.

**Actor Message Passing Performance:**

| Operation | Typical Latency | Throughput | Notes |
|-----------|-----------------|-------------|-------|
| `send()` message enqueue | 10-50ns | 10-50M messages/sec | Unbounded mailbox, O(1) enqueue |
| `cast()` message enqueue | 10-50ns | 10-50M messages/sec | Same as `send()`, returns success status |
| Message delivery (same core) | 50-200ns | 5-20M messages/sec | Includes actor scheduling overhead |
| Message delivery (cross-core) | 100-500ns | 2-10M messages/sec | Includes cache coherency and scheduling |
| Message delivery (cross-NUMA) | 200-1000ns | 1-5M messages/sec | Includes NUMA access latency |

**Cross-Reference**: See Section 16.1 (Message Send Semantics) and Section 15.1.2.1 (AArch64 Runtime Integration) for detailed message passing semantics.

**Pattern Matching Performance:**

| Pattern Type | Typical Latency | Optimization Strategy |
|--------------|-----------------|----------------------|
| Literal match | 1-2 cycles | Direct branch (`B.cond`) |
| Integer range | 2-5 cycles | Conditional compare (`CCMP`) |
| Enum/variant | 2-10 cycles | Jump table (dense) or decision tree (sparse) |
| Record pattern | 5-20 cycles | Load-pair (`LDP`) + field comparison |
| Tuple pattern | 3-15 cycles | Load-pair (`LDP`) + element comparison |
| Guard evaluation | 1-50 cycles | Conditional instructions (`CSET`, `CCMP`) |
| Nested pattern | 10-100 cycles | Decision tree with register allocation |

**Cross-Reference**: See Section 3.6.5 (AArch64-Specific Pattern Matching Optimizations) for detailed code generation strategies.

**Vector Operation Performance:**

| Operation Type | SVE Latency | NEON Latency | Throughput | Notes |
|----------------|-------------|--------------|------------|-------|
| Load vector | 10-50ns | 5-20ns | 10-50M ops/sec | Depends on vector length |
| Store vector | 10-50ns | 5-20ns | 10-50M ops/sec | Depends on vector length |
| Add vectors | 5-20ns | 3-10ns | 50-200M ops/sec | Per vector operation |
| Multiply vectors | 10-30ns | 5-15ns | 30-100M ops/sec | Per vector operation |
| Fused multiply-add | 15-40ns | 8-20ns | 25-80M ops/sec | Per vector operation |

**Cross-Reference**: See Section 21.1 (SVE - Scalable Vector Extensions) and Section 21.2 (NEON - Fixed-Width SIMD) for detailed vector operation semantics.

**Region Allocation Performance:**

| Operation | Typical Latency | Notes |
|-----------|-----------------|-------|
| Region allocation (`normal`) | 100-500ns | Includes memory mapping and MTE tag allocation |
| Region allocation (`atomic`) | 100-500ns | Same as `normal` with atomic guarantees |
| Reference allocation | 10-50ns | Within existing region |
| Buffer allocation | 20-100ns | Depends on buffer size |
| Region deallocation | 50-200ns | Includes tag cleanup |

**Cross-Reference**: See Section 12.1 (Region-Based Memory Management) for detailed allocation semantics.

**Performance Notes:**

- **Latency Ranges**: All latency ranges represent typical/average latencies under normal system load. Actual latencies may vary based on:
  - System load and contention
  - Cache state and pressure
  - NUMA topology and distance
  - Hardware-specific optimizations (e.g., Apple Silicon M-series typically 2-3x faster)
  
- **Best-Case Scenarios**: Latencies may be faster than lower bounds in optimal conditions (cache hits, low contention, optimal alignment).

- **Worst-Case Scenarios**: Latencies may be slower than upper bounds under adverse conditions (cache pressure, high contention, cross-NUMA access).

**Cross-References:**
- See Section 12.1.1.2 (Memory Space Runtime Guarantees) for memory space latency details
- See Section 17.2 (Memory Ordering Semantics) for atomic operation details
- See Section 16.1 (Message Send Semantics) for message passing details
- See Section 3.6.5 (AArch64-Specific Pattern Matching Optimizations) for pattern matching details
- See Section 21.1 (SVE) and Section 21.2 (NEON) for vector operation details

## 27. Compilation and Linking

### 27.1 Chip-Centric Compilation Strategy

Silica's compilation process is fundamentally different from traditional compilers, designed specifically for AArch64 chip architectures rather than retrofitted from x86-era tools.

#### 27.1.1 Region-Aware Code Generation

**Memory Layout Optimization:**
The compiler analyzes region lifetimes and access patterns to optimize memory placement:
- **NUMA-Optimal Allocation**: Places frequently accessed data in the same NUMA node as the actor
- **Cache-Aware Layout**: Structures data to maximize cache line utilization
- **TLB Optimization**: Minimizes page table walks through intelligent address space layout

**Region-Based Register Allocation:**
Traditional register allocation is stack-centric. Silica's approach:
- **Region Lifetime Tracking**: Registers are allocated based on region lifetimes, not function scopes
- **Cross-Actor Optimization**: Register allocation considers actor message patterns
- **Hardware Register Utilization**: Leverages AArch64's larger register set (32 general-purpose registers)

**AArch64 Register Allocation Strategy:**

The compiler implements a sophisticated register allocation strategy specifically designed for AArch64's register architecture, which provides 32 general-purpose registers (X0-X31) plus additional floating-point and SIMD registers.

**Register Set Overview:**

AArch64 provides the following register sets for register allocation:
- **General-Purpose Registers (X0-X31)**: 32 64-bit registers used for integer values, pointers, and addresses
- **Floating-Point Registers (V0-V31)**: 32 128-bit SIMD/floating-point registers used for floating-point values and vector operations
- **Special Registers**: XZR (zero register), SP (stack pointer), and platform-specific registers

**Register Allocation Policies:**

The compiler uses different allocation policies for different register classes:

- **Callee-Saved Registers (X19-X28)**: Preserved across function calls, allocated to values with long lifetimes spanning multiple function calls
- **Caller-Saved Registers (X0-X7, X9-X15)**: Not preserved across function calls, allocated to temporary values within function scopes
- **Argument Registers (X0-X7)**: Used for function arguments and return values, allocated first for function parameters
- **Scratch Registers (X16-X17, X29-X30)**: Platform-specific uses (X16-X17 for linker, X29-X30 for frame pointer and link register), avoided for general allocation

**Region Lifetime-Based Allocation:**

Register allocation considers region lifetimes rather than lexical scopes:

- **Region Creation**: When a region is created, registers are allocated for region metadata and initial references
- **Reference Lifetime**: References within a region are allocated to registers based on their usage frequency and lifetime
- **Region Deallocation**: Registers allocated to region data are freed when the region is deallocated, not when lexical scope ends
- **Cross-Region Optimization**: Registers are reused across different regions when lifetimes don't overlap

**Actor Message Passing Optimization:**

Register allocation considers actor message passing patterns:

- **Message Arguments**: Message arguments are allocated to argument registers (X0-X7) when possible, reducing memory traffic
- **Message Return Values**: Return values from actor behavior functions use return registers (X0-X7) for efficient value passing
- **Message Buffering**: Frequently accessed message fields are kept in registers across message processing
- **Cross-Actor Register Reuse**: Registers are reused across different actors when actor execution doesn't overlap

**Register Pressure Management:**

The compiler manages register pressure to minimize register spills:

- **Spill Threshold**: When register pressure exceeds available registers, values are spilled to stack memory
- **Spill Selection**: Values with lowest usage frequency and longest time until next use are selected for spilling
- **Spill Optimization**: Spilled values are reloaded only when needed, using AArch64's efficient load/store instructions
- **Register Renaming**: The compiler leverages AArch64's register renaming capabilities to reduce false dependencies

**Floating-Point Register Allocation:**

Floating-point and SIMD values use separate register allocation:

- **Floating-Point Registers (V0-V31)**: Allocated to float16, float32, float64, and SIMD vector types
- **Register Pairing**: Floating-point register pairs (V0-V1, V2-V3, etc.) are used for 128-bit SIMD operations
- **Mixed Allocation**: General-purpose and floating-point registers are allocated independently, maximizing register utilization
- **SIMD Optimization**: SIMD operations use consecutive floating-point registers when possible, enabling efficient SIMD instruction generation

**Register Allocation for Pattern Matching:**

Pattern matching code receives special register allocation treatment:

- **Pattern Variables**: Variables bound in patterns are allocated to registers immediately after pattern matching succeeds
- **Guard Evaluation**: Guard expressions use temporary registers that are freed after guard evaluation
- **Pattern Result Values**: Values computed in pattern branches are allocated to registers for efficient value passing
- **Decision Tree Optimization**: Decision tree code uses registers for intermediate pattern matching results

**Register Allocation for Actor State:**

Actor state receives optimized register allocation:

- **State Fields**: Frequently accessed actor state fields are allocated to callee-saved registers (X19-X28)
- **State Updates**: State updates use registers to minimize memory writes, leveraging AArch64's write-back addressing modes
- **State Preservation**: Actor state is preserved in registers across message processing when possible
- **State Migration**: When actors migrate between cores, register-allocated state is efficiently transferred

**Register Allocation Performance:**

Register allocation on AArch64 provides significant performance benefits:

- **Register Utilization**: Typical programs use 20-25 of 32 general-purpose registers, minimizing spills
- **Spill Reduction**: Region-based allocation reduces spills by 30-40% compared to stack-based allocation
- **Memory Traffic Reduction**: Register allocation reduces memory traffic by 20-30% compared to naive allocation
- **Instruction Count Reduction**: Efficient register allocation reduces instruction count by 10-15% through better instruction selection

**Cross-References:**
- See Section 3.6.5 (AArch64-Specific Pattern Matching Optimizations) for register allocation in pattern matching
- See Section 15.1.2.1 (AArch64 Runtime Integration) for actor state register allocation
- See Section 27.1.2 (Effect-Driven Optimization) for effect-aware register allocation
- See Section 21.1.3.1 (SVE Runtime Behavior) for SIMD register allocation

#### 27.1.2 Effect-Driven Optimization

**Capability-Aware Code Generation:**
The compiler uses effect information to generate hardware-optimized code:
- **Memory Barrier Insertion**: Automatic generation of appropriate barriers based on effect annotations
- **Atomic Operation Selection**: Chooses optimal AArch64 atomic instructions based on ordering requirements
- **Cache Coherency Hints**: Uses AArch64 cache maintenance instructions for effect-tracked operations

**Memory Barrier Insertion Based on Effects:**

The compiler automatically inserts memory barriers based on effect annotations. The following examples illustrate barrier insertion for different effect combinations:

**Example 1: Atomic Operations with Acquire Semantics**

```silica
// Source code
fn load_shared_data(atomic_ref: atomic_ref(R, atomic, Data)) -> Data proc[mem(atomic), atomic] {
    atomic_load(atomic_ref, acquire)  // acquire ordering
}
```

**Generated AArch64 Code (Before Optimization):**
```assembly
// Load with acquire semantics
LDAR X0, [X1]  // Load-acquire: X0 = atomic_ref, X1 = address
// LDAR provides acquire semantics automatically
```

**Barrier Insertion**: `LDAR` instruction provides acquire semantics, so no additional barrier is needed. The compiler recognizes that `LDAR` already includes the necessary ordering guarantees.

**Example 2: Sequential Consistency Operations**

```silica
// Source code
fn seq_cst_load(atomic_ref: atomic_ref(R, atomic, int64)) -> int64 proc[mem(atomic), atomic] {
    atomic_load(atomic_ref, seq_cst)  // sequential consistency
}
```

**Generated AArch64 Code:**
```assembly
// Sequential consistency requires full memory barrier
DMB ISH        // Data Memory Barrier, Inner Shareable domain
LDAR X0, [X1]  // Load-acquire after barrier
// DMB ensures all prior operations complete before LDAR
// LDAR ensures all subsequent operations see the load result
```

**Barrier Insertion**: For `seq_cst` ordering, the compiler inserts `DMB ISH` before `LDAR` to ensure global ordering. This provides the strongest ordering guarantee required by sequential consistency.

**Example 3: Release Store Followed by Normal Store**

```silica
// Source code
fn publish_data(data_ref: ref(R, normal, Data), atomic_flag: atomic_ref(R, atomic, boolean)) -> atom proc[mem(normal), mem(atomic), atomic] {
    write_ref(data_ref, new_data);                    // Normal store
    atomic_store(atomic_flag, true, release);        // Release store
}
```

**Generated AArch64 Code:**
```assembly
// Store data
STR X2, [X3]        // Normal store: X2 = new_data, X3 = data_ref address
// Release store ensures all prior stores are visible
STLR X4, [X5]       // Store-release: X4 = true, X5 = atomic_flag address
// STLR provides release semantics automatically
```

**Barrier Insertion**: `STLR` provides release semantics, ensuring all prior stores (including the normal store) are visible before the release store completes. No additional barrier needed.

**Example 4: Cross-Actor Memory Visibility**

```silica
// Source code (Actor A)
fn actor_a_behavior(msg: Message, state: State) -> State proc[mem(normal), mem(atomic), atomic, concurrency] {
    // Prepare shared data
    write_ref(shared_data_ref, prepared_data);
    // Publish with release semantics
    atomic_store(publish_flag, true, release);
    state
}

// Source code (Actor B)
fn actor_b_behavior(msg: Message, state: State) -> State proc[mem(normal), mem(atomic), atomic, concurrency] {
    // Wait for data to be published
    ready: boolean <- atomic_load(publish_flag, acquire);
    if ready {
        data: Data <- read_ref(shared_data_ref);  // Read published data
    }
    state
}
```

**Generated AArch64 Code (Actor A):**
```assembly
// Prepare data
STR X2, [X3]        // Store shared data
// Publish with release
STLR X4, [X5]       // Store-release: publish_flag = true
// STLR ensures all prior stores (including shared_data) are visible
```

**Generated AArch64 Code (Actor B):**
```assembly
// Wait for publication
LDAR X0, [X5]       // Load-acquire: X0 = publish_flag
CBNZ X0, data_ready // Branch if flag is set
// Load-acquire ensures we see all stores before the release store
data_ready:
LDR X1, [X3]        // Load shared data (guaranteed to see Actor A's store)
```

**Barrier Insertion**: The acquire-release pair (`LDAR`/`STLR`) provides the necessary synchronization. The compiler recognizes this pattern and ensures proper ordering without additional barriers.

**Example 5: Multiple Atomic Operations with Different Orderings**

```silica
// Source code
fn complex_atomic_operation(
    counter: atomic_ref(R, atomic, int64),
    flag: atomic_ref(R, atomic, boolean)
) -> int64 proc[mem(atomic), atomic] {
    // Relaxed increment (no ordering needed)
    old_count: int64 <- atomic_fetch_add(counter, 1, relaxed);
    
    // Sequential consistency store (needs full ordering)
    atomic_store(flag, true, seq_cst);
    
    old_count
}
```

**Generated AArch64 Code:**
```assembly
// Relaxed increment (no barrier needed)
LDXR X0, [X1]      // Load-exclusive: X0 = counter value
ADD X0, X0, #1     // Increment
STXR W2, X0, [X1]  // Store-exclusive: W2 = success flag
CBNZ W2, retry     // Retry if store failed
retry:
// Sequential consistency store (needs barrier)
DMB ISH            // Full memory barrier before seq_cst store
STLR X3, [X4]      // Store-release: X3 = true, X4 = flag address
DMB ISH            // Full memory barrier after seq_cst store
```

**Barrier Insertion**: For `seq_cst` operations, the compiler inserts `DMB ISH` barriers before and after the operation to ensure global ordering. The relaxed operation uses `LDXR`/`STXR` without barriers.

**Effect-Based Barrier Selection Algorithm:**

```pseudocode
function insert_barriers_for_effects(operation, effects, ordering):
    barriers = []
    
    // Check if operation requires barriers based on effects
    if "atomic" in effects:
        case ordering:
            relaxed:
                // No barriers needed for relaxed ordering
                barriers = []
            
            acquire:
                // Load-acquire provides acquire semantics
                if operation.type == "load":
                    barriers = []  // LDAR provides acquire
                else:
                    barriers = []  // No barrier needed for acquire load
            
            release:
                // Store-release provides release semantics
                if operation.type == "store":
                    barriers = []  // STLR provides release
                else:
                    barriers = []  // No barrier needed for release store
            
            acq_rel:
                // RMW operations use LDAXR/STLXR which provide acq_rel
                if operation.type == "rmw":
                    barriers = []  // LDAXR/STLXR provide acq_rel
                else:
                    // Need both acquire and release
                    barriers = [DMB_ISH]  // Barrier for cross-operation ordering
            
            seq_cst:
                // Sequential consistency requires full barriers
                barriers = [DMB_ISH_BEFORE, DMB_ISH_AFTER]
        end case
    end if
    
    // Insert barriers in generated code
    insert_barriers(operation, barriers)
end function
```

**Performance Impact of Barrier Insertion:**

Barrier insertion has measurable performance impact:

- **No Barrier** (relaxed ordering): Baseline performance (~1-5ns per operation)
- **Acquire/Release Barriers** (`LDAR`/`STLR`): Minimal overhead (~5-10ns vs. normal load/store)
- **Full Barriers** (`DMB ISH`): Moderate overhead (~10-50ns per barrier)
- **Sequential Consistency** (`DMB` + `LDAR`/`STLR` + `DMB`): Higher overhead (~50-150ns per operation)

The compiler minimizes barrier overhead by:
- Using acquire/release instructions (`LDAR`/`STLR`) instead of barriers when possible
- Eliminating redundant barriers when multiple operations share ordering requirements
- Hoisting barriers outside loops when safe

**Cache Coherency Hints:**

For effect-tracked memory operations, the compiler inserts cache maintenance instructions:

```silica
// Source code
fn flush_cache_line(ptr: ref(R, normal, Data)) -> atom proc[mem(normal)] {
    // Write data that needs to be visible to other cores
    write_ref(ptr, new_data);
    // Compiler inserts cache flush for cross-core visibility
}
```

**Generated AArch64 Code:**
```assembly
STR X0, [X1]        // Store data
DC CIVAC, X1        // Data Cache Clean and Invalidate by Virtual Address to Point of Coherency
// Ensures data is visible to other cores and DMA devices
```

**Speculative Execution Control:**

Modern AArch64 chips have sophisticated speculative execution. Silica controls this through effects:

- **Speculation Barriers**: Automatic insertion for security-sensitive operations
- **Branch Prediction Hints**: Compiler hints based on actor behavior patterns
- **Memory Ordering**: Hardware memory model exploitation for actor communication

**Speculation Barrier Example:**

```silica
// Source code with security-sensitive operation
fn secure_operation(secret: ref(R, normal, Secret)) -> atom proc[mem(normal)] {
    // Access secret data
    secret_data: Secret <- read_ref(secret);
    // Compiler inserts speculation barrier to prevent speculative leaks
    process_secret(secret_data);
}
```

**Generated AArch64 Code:**
```assembly
LDR X0, [X1]        // Load secret data
DSB SY              // Data Synchronization Barrier (full system)
ISB                 // Instruction Synchronization Barrier
// Barriers prevent speculative execution from leaking secret data
BL process_secret   // Call processing function
```

**Cross-References:**
- See Section 17.2 (Memory Ordering Semantics) for detailed ordering guarantees
- See Section 12.1.1.2 (Memory Space Runtime Guarantees) for memory visibility semantics
- See Section 9.4 (Effect System) for effect declaration and checking

### 27.2 AArch64-Specific Optimizations

#### 27.2.1 Asymmetric Core Utilization

**Big.LITTLE Optimization:**
The compiler generates code optimized for heterogeneous core architectures:
- **Performance Core Targeting**: Latency-critical actor code optimized for high-performance cores
- **Efficiency Core Targeting**: Background tasks optimized for power-efficient cores
- **Dynamic Code Paths**: Runtime core migration with recompilation hints

**Custom Instruction Utilization:**
Direct exploitation of AArch64-specific instructions:
- **MTE Integration**: Memory tagging operations built into region allocation
- **PAC Integration**: Pointer authentication in effect-tracked code
- **SVE Conditional Execution**: Vector operations with hardware predication

#### 27.2.2 Hardware-Assisted Concurrency

**Message Passing Optimization:**
Actor communication leverages AArch64 hardware:
- **Cache-Coherent Interconnects**: Optimized message routing across CPU clusters
- **Hardware Lock Elision**: Automatic use of transactional memory where available
- **Atomic Operation Fusion**: Combines multiple atomics into single hardware operations

**Interrupt and Signal Handling:**
Modern chips have advanced interrupt controllers. Silica uses these for:
- **Actor Preemption**: Hardware-assisted actor scheduling
- **Real-Time Guarantees**: Direct hardware timer integration
- **Power Management**: Chip-level sleep state coordination

### 27.3 Linking and Module Resolution

#### 27.3.1 Module Linking Process

**Multi-Module Compilation:**
Silica compiles all modules in a program together:
1. **Dependency Ordering**: Modules are compiled in dependency order (leaves first)
2. **Cross-Module Optimization**: All modules are visible during optimization passes
3. **Unified Binary**: All modules are linked into a single executable

**Symbol Resolution:**
- **Global Symbol Table**: All exported functions are collected into a global namespace
- **Import Resolution**: `use` declarations make exported symbols available locally
- **Link-Time Verification**: Ensures all imports can be satisfied

#### 25.3.2 Hardware-Aware Linking

**Address Space Optimization:**
Traditional linkers focus on symbol resolution. Silica's linker:
- **NUMA-Aware Placement**: Places code and data to minimize cross-NUMA communication
- **Cache Line Alignment**: Aligns functions and data structures for optimal cache utilization
- **TLB Efficiency**: Minimizes virtual-to-physical address translations

**Effect-Based Linking:**
Modules are linked based on their effect profiles:
- **Capability Verification**: Ensures linked modules have compatible effect requirements
- **Optimization Across Boundaries**: Inter-module optimizations based on shared effects
- **Security Isolation**: Hardware-enforced boundaries between modules with different trust levels

#### 27.3.3 Runtime Linking

**Dynamic Module Loading:**
Runtime module loading leverages AArch64 capabilities:
- **Just-In-Time Compilation**: Chip-specific optimizations at load time
- **Relocation Optimization**: Hardware-assisted address relocation
- **Security Validation**: PAC and MTE verification during loading

### 27.4 Performance Profiling and Adaptation

#### 25.4.1 Hardware Performance Counters

**PMU Integration:**
Direct access to AArch64 Performance Monitoring Unit:
- **Cache Hit/Miss Tracking**: Automatic optimization based on cache performance
- **Branch Prediction Analysis**: Dynamic recompilation for mispredicted branches
- **Memory Bandwidth Monitoring**: NUMA optimization based on bandwidth usage

**Adaptive Compilation:**
Runtime performance feedback drives recompilation:
- **Hot Path Identification**: Hardware counters identify frequently executed code
- **Specialized Code Generation**: Recompile hot paths with chip-specific optimizations
- **Actor Behavior Learning**: Adjust scheduling based on observed communication patterns

#### 27.4.2 Thermal and Power Awareness

**Chip Temperature Integration:**
Compiler adapts to thermal conditions:
- **Dynamic Voltage Scaling**: Code generation considers power states
- **Core Migration Planning**: Pre-planned migration paths for thermal events
- **Workload Balancing**: Distribute computation to avoid thermal hotspots

### 27.5 Compilation Phases

#### 27.5.1 Frontend: Language-Centric

1. **Module Resolution**: Locate and parse module dependencies using search paths
2. **Parsing**: UTF-8 aware, AI-assisted error recovery for all modules
3. **Import/Export Validation**: Verify module interfaces and resolve cross-module references
4. **Type Checking**: Effect-aware type system with hardware capability validation across all modules
5. **Region Analysis**: Lifetime and ownership verification across module boundaries
6. **Effect Checking**: Verify all effects are explicitly declared

#### 27.5.2 Middle-End: Architecture-Aware

1. **Region Optimization**: NUMA and cache-aware memory layout
2. **Actor Optimization**: Message passing and scheduling optimization
3. **Effect Lowering**: Translation of high-level effects to hardware primitives
4. **Vectorization**: Automatic SVE code generation

#### 27.5.3 Backend: Chip-Native

1. **Instruction Selection**: AArch64-specific instruction choice
2. **Register Allocation**: Region-lifetime aware register assignment
3. **Code Layout**: Cache and TLB optimized code placement
4. **Link-Time Optimization**: Cross-module hardware-aware optimization

### 27.6 Build System Integration

#### 27.6.1 Hardware-Aware Build Configuration

**Target Detection:**
Automatic detection and optimization for specific AArch64 variants:
- **CPU Feature Detection**: Runtime feature detection with compile-time fallbacks
- **Cache Hierarchy Discovery**: Automatic adaptation to different cache configurations
- **NUMA Topology Mapping**: Build-time optimization based on system topology

**Cross-Compilation Support:**
Native cross-compilation for different AArch64 targets:
- **Feature Matrix Compilation**: Generate code for multiple feature sets
- **Runtime Feature Selection**: Dynamic dispatch based on detected capabilities
- **Binary Compatibility**: Hardware-aware ABI compatibility

---

*Phase 6 Extended: Complete Compilation and Linking*

*Extended Phase 6 deliverables achieved:*
- Runtime system with scheduler, actor management, and memory management
- Implementation requirements for compilers and runtimes
- Error handling with types, propagation, and recovery mechanisms
- Platform integration for AArch64, OS interaction, and performance
- **NEW**: Comprehensive compilation process focused on AArch64 chip capabilities
- **NEW**: Hardware-aware linking and module resolution
- **NEW**: Performance profiling and adaptive compilation
- **NEW**: Debug and assertion facilities

*Phase 7: Module System Implementation*

*Module system deliverables completed:*
- Filename-based module naming (no explicit module declarations)
- Export syntax: `export func/arity, func/arity;`
- Import syntax: `use module1, module2;`
- Configurable search paths via `--search-path`/`-I` command line options
- Cross-module type checking and symbol resolution
- Multi-module compilation with dependency ordering
- Module validation with comprehensive error reporting

## 28. Compiler Infrastructure

### 28.1 Incremental Compilation

#### 28.1.1 Dependency Tracking
Only recompiling changed modules and their dependents:

```
main.silica ──┐
              ├── math.silica (changed) ──┐
              │                           ├── utils.silica (recompile)
              └── io.silica ────┘
```

#### 28.1.2 Module Caching
Persistent caching of compiled modules:

```silica
// Module cache structure
.cache/
├── math.silica.bc       // Compiled bytecode
├── math.silica.deps     // Dependency information
└── math.silica.types    // Type information
```

**Cache Structure Details:**

The module cache stores compiled artifacts to enable incremental compilation:

- **Bytecode Files** (`.bc`): Compiled module bytecode ready for linking
- **Dependency Files** (`.deps`): Module dependency graph and interface information
- **Type Files** (`.types`): Type information for type checking dependent modules

**Cache Invalidation Strategy:**

The compiler invalidates cached modules when:

1. **Source File Changes**: Source file modification time is newer than cache timestamp
2. **Dependency Changes**: Any dependency module has been recompiled
3. **Interface Changes**: Exported interface of a dependency module has changed
4. **Compiler Version Mismatch**: Cache was created with a different compiler version
5. **Manual Invalidation**: User explicitly requests cache clearing

**Cache Persistence and Recovery:**

- **Persistent Storage**: Cache persists across compiler invocations in `.cache/` directory
- **Cache Recovery**: Compiler checks cache validity before recompiling
- **Cache Corruption Handling**: Corrupted cache files are automatically regenerated
- **Cache Size Management**: Old cache entries are evicted when cache size exceeds limits

### 28.2 Multiple-Pass Architecture

The Silica compiler uses a **multiple-pass** architecture rather than a single-pass descent-based design. This choice affects parsing, type checking, and optimization.

#### 28.2.1 Rationale

- **Separation of concerns**: Each pass has a single responsibility. Parsing produces structure; type checking validates semantics; optimization improves performance. Keeping these phases distinct simplifies reasoning, testing, and incremental development.
- **Cross-module analysis**: Type checking and effect checking require visibility into all declarations before validating bodies. A two-pass approach (add declarations, then check bodies) enables forward references and cross-module resolution without ad-hoc workarounds.
- **Optimization flexibility**: SIR (Silica Intermediate Representation) optimization applies a fixed sequence of passes (constant folding, constant propagation, CSE, dead code elimination, inlining, etc.). Multiple passes allow each optimization to run to a fixed point and to depend on the results of earlier passes.

#### 28.2.2 Parser: Constraint Propagation

The parser does **not** use conventional recursive descent. It uses **constraint propagation**:

- Each token starts with a set of possible roles; grammar rules are expressed as constraints between adjacent tokens.
- Parsing iterates until each token has exactly one role (or a contradiction is found).
- Structure emerges from the final role sequence; there is no explicit recursive descent.

This design supports incremental capability addition and strong modularity. See [parser_design.md](parser_design.md) for details.

#### 28.2.3 Type Checker: Two-Pass

The type checker runs in two passes:

1. **Declaration pass**: Add all function and effect declarations to the symbol table.
2. **Body pass**: Check all function bodies against the populated context.

This ordering allows functions to reference each other and enables cross-module type resolution before any body is validated.

#### 28.2.4 Pipeline Overview

The full compilation pipeline consists of sequential phases:

1. **Lexer** → token stream
2. **Parser** → AST (via constraint propagation)
3. **Type checker** → type-checked AST (two-pass)
4. **Effect checker** → effect-validated AST
5. **SIR generator** → SIR
6. **Optimization** → optimized SIR (multiple passes: constant folding, propagation, CSE, DCE, inlining, etc.)
7. **Code generation** → assembly

Each phase consumes the output of the previous phase. Parallelization is possible within phases (e.g., parsing multiple modules concurrently) but not across phases. See [silica-compiler-creation-order.md](../silica-compiler-creation-order.md) for the complete data flow.

## 29. IDE & Developer Experience

### 29.1 Language Server

#### 29.1.1 Syntax Highlighting
Editor integration for Silica syntax highlighting:

```silica
keywords: fn, if, case, actor, effect, type
types: int, boolean, string, actor_ref
effects: [mem(normal)], [concurrency]
```

#### 29.1.2 Go to Definition
Navigate to symbol definitions across modules:

```
fn main() {
    result: int <- add(1, 2)  // Ctrl+click on 'add' jumps to math.silica
}
```

#### 29.1.3 Hover Information
Display type and documentation information:

```silica
fn add(x: int, y: int) -> int  // Hover shows signature
```

#### 29.1.4 Auto-completion
Context-aware code completion:

```silica
use math_
      // Suggests: math_utils
```

### 29.2 Debugging Support

#### 29.2.1 Source-Level Debugging
Step through Silica code with source line mapping:

```silica
fn factorial(n: int) -> int {
    if n <= 1 {           // Breakpoint here
        return 1
    }
    return n * factorial(n - 1)
}
```

#### 29.2.2 Actor State Inspection
Examine actor internal state during debugging:

```silica
actor counter {
    state: int = 0

    increment() -> atom {
        state = state + 1  // Inspect 'state' variable
    }
}
```

#### 29.2.3 Message Tracing
Trace message passing between actors:

```
Actor A sends: increment()
  ↓
Actor B receives: increment()
  ↓
Actor B state: 0 → 1
```

## 30. Advanced Type System

### 30.1 Traits

#### 30.1.1 Trait Definitions
Traits define interfaces that types can implement:

```silica
trait Display {
    fn to_string(self) -> string
}

trait Comparable {
    fn equals(self, other) -> boolean
    fn less_than(self, other) -> boolean
}
```

#### 30.1.2 Multiple Trait Implementation
Types can implement any number of independent traits. Traits do not inherit from or include other traits. Instead, types compose behavior by implementing multiple traits directly.

**Independent Traits:**
```silica
trait Printable {
    fn to_string(self) -> string
}

trait Debug {
    fn debug_string(self) -> string
}

trait Serializable {
    fn serialize(self) -> bytes
}

trait Comparable {
    fn equals(self, other) -> boolean
}
```

**Implementing Multiple Traits on a Type:**
```silica
struct Widget {
    name: string,
    version: int
}

impl Printable for Widget {
    fn to_string(self) = format("{} v{}", self.name, self.version)
}

impl Debug for Widget {
    fn debug_string(self) = format("Widget {{ name: {}, version: {} }}", self.name, self.version)
}

impl Serializable for Widget {
    fn serialize(self) = encode(self.name, self.version)
}

impl Comparable for Widget {
    fn equals(self, other) = self.name == other.name && self.version == other.version
}
```

**Important Notes:**
- Each trait is independent and self-contained
- Types implement each trait separately via `impl`
- A type can implement as many traits as needed
- Trait implementations are independent of each other

**Method Name Disambiguation:**

When a type implements multiple traits that define methods with the same name, use qualified method names (`traitname.functionname`) to disambiguate:

```silica
trait Reader {
    fn read(self: Self) -> int64;
}

trait Writer {
    fn read(self: Self) -> int64;  // Same method name as Reader
}

// A type implementing both traits
struct MyType { data: int64 }

impl Reader for MyType {
    fn read(self: MyType) -> int64 { self.data }
}

impl Writer for MyType {
    fn read(self: MyType) -> int64 { self.data + 1 }
}

fn process(value: MyType) -> int64 {
    // Use qualified names to disambiguate
    reader_result: int64 <- Reader.read(value);
    writer_result: int64 <- Writer.read(value);
    reader_result + writer_result
}
```

**Disambiguation Syntax:**

```
qualified_method_call ::= trait_name "." method_name "(" [arguments] ")"
```

**Disambiguation Rules:**

1. **Unique Method Names**: If method names are unique across all implemented traits, no disambiguation needed
2. **Ambiguous Method Names**: If multiple traits define the same method name, must use qualified name
3. **Trait Method Access**: Use `TraitName.method_name()` to call methods from specific traits
4. **Self Parameter**: Qualified method calls still pass `self` as the first parameter

#### 30.1.3 Disambiguation Requirements

This section clarifies when trait method disambiguation is required versus optional, and provides guidance for avoiding ambiguity.

**When Disambiguation is Required:**

Disambiguation is **required** when:

1. **Multiple Traits Define Same Method**: Two or more implemented traits define methods with identical names and signatures
2. **Explicit Trait Selection**: The programmer wants to explicitly call a method from a specific trait

**When Disambiguation is Optional:**

Disambiguation is **optional** when:

1. **Unique Method Names**: Method names are unique across all implemented traits
2. **No Ambiguity**: The compiler can unambiguously determine which method to call

**Disambiguation Decision Tree:**

```
Is method name unique across all implemented traits?
├─ Yes → No disambiguation needed
└─ No → Is method defined in multiple traits?
    ├─ Yes → Disambiguation required
    └─ No → No disambiguation needed
```

**Examples: Unambiguous Cases (No Disambiguation Needed)**

```silica
trait Reader {
    fn read(self: Self) -> int64;
}

trait Writer {
    fn write(self: Self, data: int64) -> int64;  // Different name
}

struct MyType { data: int64 }

impl Reader for MyType {
    fn read(self: MyType) -> int64 { self.data }
}

impl Writer for MyType {
    fn write(self: MyType, data: int64) -> int64 { data }
}

// No disambiguation needed: method names are unique
fn process(value: MyType) -> int64 {
    value2: int64 <- value.read();        // Unambiguous: only Reader.read()
    value.write(value2)                   // Unambiguous: only Writer.write()
}
```

**Examples: Ambiguous Cases (Disambiguation Required)**

```silica
trait Logger {
    fn log(self: Self, msg: string) -> atom;
}

trait Debugger {
    fn log(self: Self, msg: string) -> atom;  // Same name and signature
}

struct MyTool { name: string }

impl Logger for MyTool {
    fn log(self: MyTool, msg: string) -> atom { print_string(msg) }
}

impl Debugger for MyTool {
    fn log(self: MyTool, msg: string) -> atom { print_string("DEBUG: " + msg) }
}

// Disambiguation required: both Logger and Debugger define log()
fn run(tool: MyTool) -> atom {
    Logger.log(tool, "info");      // Required: specify Logger.log()
    Debugger.log(tool, "debug");   // Required: specify Debugger.log()
}
```

**Best Practices for Trait Design:**

To avoid disambiguation requirements:

1. **Use Descriptive Method Names**: Prefer `read_from_file()` over `read()` when multiple traits might define `read()`
2. **Namespace Methods**: Use trait-specific prefixes: `logger_log()`, `debug_log()`
3. **Avoid Common Names**: Avoid generic method names like `get()`, `set()`, `process()` in multiple traits
4. **Document Ambiguity**: If ambiguity is intentional, document which trait's method should be used

**Example: Well-Designed Traits (No Ambiguity)**

```silica
trait FileReader {
    fn read_file(self: Self, path: string) -> string;
}

trait NetworkReader {
    fn read_socket(self: Self, sock: socket) -> string;
}

struct MyReader { base_path: string }

impl FileReader for MyReader {
    fn read_file(self: MyReader, path: string) -> string { ... }
}

impl NetworkReader for MyReader {
    fn read_socket(self: MyReader, sock: socket) -> string { ... }
}

// No disambiguation needed: method names are distinct
fn read_all(reader: MyReader) -> string {
    file_data: string <- reader.read_file("data.txt");      // Unambiguous
    socket_data: string <- reader.read_socket(sock);        // Unambiguous
    file_data + socket_data
}
```

**Compiler Error for Missing Disambiguation:**

When disambiguation is required but not provided, the compiler reports:

```
❌ Compilation error: AmbiguousMethodError at example.silica:10:5 [E4000]

Ambiguous method call: log()
Multiple traits define log(): Logger, Debugger
Use qualified name: Logger.log() or Debugger.log()
See specification: spec:§30.1.3
```

**Cross-References:**
- See Section 30.1.2 (Multiple Trait Implementation) for trait composition semantics
- See Section 3.3.2 (Function Calls) for qualified method call syntax

#### 30.1.4 Implementation Requirements
Types implement traits with concrete methods:

```silica
impl Display for int {
    fn to_string(self) = int_to_string(self)
}

impl Comparable for int {
    fn equals(self, other) = self == other
    fn compare(self, other) =
        if self < other { Less }
        else if self > other { Greater }
        else { Equal }
}
```

#### 30.1.5 Actor System Traits
Silica provides two marker traits for the actor system:

**ActorState Trait:**
```silica
trait ActorState {
    // No methods required - marker trait for type safety
}
```

Types used as actor initial state in `spawn(initial_state, ...)` must implement `ActorState`. Only named types (structs, type aliases) implement this trait - no blanket implementations for primitive types.

**ActorMessage Trait:**
```silica
trait ActorMessage {
    // No methods required - marker trait for type safety
}
```

Types used as messages in `send()` or `cast()` must implement `ActorMessage`. Only named types (structs, type aliases) implement this trait - no blanket implementations for primitive types.

**Trait-as-Type:**
When a trait is used directly as a type (e.g., `ActorMessage`), it represents any concrete type implementing that trait. This is resolved at compile time through trait implementation checking.

**Example:**
```silica
type Request = {data: int, reply_to: actor_ref};
type Response = {result: int};
impl ActorMessage for Request;
impl ActorMessage for Response;

// ActorMessage can be used as a type
cast(actor_ref, message: ActorMessage) -> boolean proc[concurrency]
```

#### 30.1.6 Trait Bounds
Functions can require trait implementations:

```silica
fn print_value(x) -> atom where Display {
    print(to_string(x))
}
```

#### 30.1.7 Implementing Multiple Traits
Each trait is implemented independently for a type. A type can implement as many traits as needed:

```silica
trait Printable {
    fn to_string(self) -> string
}

trait Debug {
    fn debug_string(self) -> string
}

impl Printable for int {
    fn to_string(self) = int_to_string(self)
}

impl Debug for int {
    fn debug_string(self) = format("int: {}", self)
}

impl Printable for boolean {
    fn to_string(self) = if self { "true" } else { "false" }
}
```

### 30.2 Trait Composition

#### 30.2.1 Multiple Trait Implementation
Types can implement multiple traits:

```silica
struct Person {
    name: string,
    age: int
}

impl Display for Person {
    fn to_string(self) = format("Person({}, {})", self.name, self.age)
}

impl Comparable for Person {
    fn equals(self, other) = self.name == other.name && self.age == other.age
    fn less_than(self, other) = self.age < other.age
}
```

#### 30.2.2 Complex Trait Relationships
Types can implement traits that require coordination between multiple traits:

```silica
trait Display {
    fn to_string(self) -> string
}

trait Debug {
    fn debug_string(self) -> string
}

// A type that implements both traits
struct Point {
    x: int,
    y: int
}

impl Display for Point {
    fn to_string(self) = format("({}, {})", self.x, self.y)
}

impl Debug for Point {
    fn debug_string(self) = format("Point {{ x: {}, y: {} }}", self.x, self.y)
}
```

---

*Phase 9: Advanced Language Features*

*Advanced features deliverables completed:*
- Advanced pattern matching with records and variants
- Structured exception handling
- Advanced effect system with composition
- Compiler optimizations and incremental compilation
- IDE support with language server and debugging
- Advanced type system with traits

*Specification Complete: All Major Features Specified*

The Silica programming language specification now includes comprehensive coverage of:
- Core language features (Phases 1-7)
- Advanced language features and type system (Phase 8)
- Compiler infrastructure and tooling (Phase 9)

This specification serves as the definitive reference for Silica implementation and usage.
