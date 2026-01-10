# The Silica Programming Language Specification

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

### 1.3 Target Platform
Silica targets AArch64 (64-bit ARM) architectures with optional support for:
- Scalable Vector Extensions (SVE/SVE2)
- NEON vector instructions
- Memory Tagging Extensions (MTE)
- Pointer Authentication (PAC)

### 1.4 Compiler Interface

#### 1.4.1 Command Line Usage
```
silica-comp [options] <input.silica> [output.bc]
```

#### 1.4.2 Optimization Options
- `--opt <level>`, `-O <level>`: Set optimization level
  - `none` (default): No optimizations
  - `less`: Basic optimizations
  - `default`: Standard optimizations
  - `aggressive`: Maximum optimizations

#### 1.4.3 Module Search Path Options
- `--search-path <path>`, `-I <path>`: Add directory to module search paths
  - Can be specified multiple times
  - Default: current directory (`.`)

#### 1.4.4 Examples
```bash
# Basic compilation
silica-comp main.silica

# With optimization
silica-comp --opt default main.silica output.bc

# With custom module paths
silica-comp -I ./modules -I ./stdlib main.silica

# Full command
silica-comp --opt default -I modules -I stdlib main.silica app.bc
```

## 2. Lexical Structure

### 2.1 Character Set
Silica source code is UTF-8 encoded. The language uses ASCII characters for keywords, operators, and punctuation. Unicode is allowed in string literals, character literals, and comments.

### 2.2 Tokens

#### 2.2.1 Keywords
The following identifiers are reserved keywords:

```
actor     atomic    bool      buf       case      char
concurrency       device_io do        effect    end
false     fn        int       mailbox   mem
proc      recv      ref       region    return    self
send      spawn     true      type      unit      use
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
Line comments start with `--` and continue to the end of the line:
```
line_comment ::= "--" {any_character_except_newline}
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
              | effect_declaration
              | import_declaration
              | export_declaration
```

### 3.3 Expressions
```
expression ::= literal
             | identifier
             | "(" expression ")"
             | expression binary_operator expression
             | unary_operator expression
             | function_call
             | case_expression
             | do_expression
```

#### 3.3.1 Literals
```
literal ::= integer_literal
          | boolean_literal
          | character_literal
          | string_literal
          | unit_literal
```

#### 3.3.2 Function Calls
```
function_call ::= expression "(" [argument_list] ")"
argument_list ::= expression {"," expression}
```

#### 3.3.3 Case Expressions
```
case_expression ::= "case" expression "of" "{" {case_branch} "}"
case_branch    ::= pattern "->" expression ";"
```

#### 3.3.4 Do Expressions
```
do_expression ::= "do" {statement} "end"
statement     ::= pattern "<-" expression ";"
                | expression ";"
```

### 3.4 Declarations

#### 3.4.1 Function Declarations
```
function_declaration ::= "fn" identifier parameter_list [":" type] ["proc" "[" effect_list "]"] "{" statement_list "}"

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

### 3.5 Patterns
```
pattern ::= literal_pattern
          | identifier_pattern
          | wildcard_pattern
          | tuple_pattern
          | record_pattern
          | variant_pattern

literal_pattern  ::= literal
identifier_pattern ::= identifier ":" type
wildcard_pattern  ::= "_"
tuple_pattern     ::= "(" typed_pattern {"," typed_pattern} ")"
typed_pattern     ::= identifier ":" type
record_pattern    ::= "{" identifier ":" pattern {"," identifier ":" pattern} "}"
variant_pattern   ::= identifier [pattern]
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
ρ ⊢ _ ⇓ v ⇒ ρ           (match any value, no binding)
```

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
4. **Wildcard Patterns**: `_` covers all remaining cases

**Non-Exhaustive Match Detection:**
If a match is not exhaustive, the compiler reports an error with:
- The uncovered cases
- Suggestions for additional patterns to add

#### 3.6.4 Pattern Compilation Strategy

**Decision Tree Compilation:**
Patterns are compiled into an efficient decision tree:
1. **Constructor Splitting**: First test variant constructors
2. **Field Extraction**: Extract tuple/record fields
3. **Value Testing**: Test literal values
4. **Binding Assignment**: Create environment bindings

**Optimization Techniques:**
- **Common Subexpression Elimination**: Share pattern tests across branches
- **Guard Hoisting**: Move expensive tests earlier in the tree
- **Redundancy Elimination**: Remove unreachable pattern branches
- **Backtracking Minimization**: Prefer deterministic patterns over backtracking

**Performance Characteristics:**
- **O(1)** for simple literal matches
- **O(depth)** for nested pattern matching
- **Optimal Branching**: Decision tree minimizes comparisons

### 3.6 Types
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

### 3.7 Effects
```
effect ::= effect_identifier [type_arguments]
```

### 3.8 Operator Precedence and Associativity

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
The `bool` type represents boolean values.

```
type bool = true | false
```

#### 4.1.3 Integer Type
The `int` type represents signed 64-bit integers.

```
type int
```

Supported range: -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807

#### 4.1.4 Character Type
The `char` type represents Unicode scalar values.

```
type char
```

#### 4.1.5 Any Type
The `any` type is a special type that can represent values of any other type. It is used for type matching and dynamic typing scenarios where the exact type is not known at compile time or needs to be determined at runtime.

```
type any
```

The `any` type supports:
- Assignment from any other type
- Type matching with any other type
- Runtime type introspection capabilities
- Explicit casting to concrete types

### 4.2 Compound Types

#### 4.2.1 Function Types
Function types have the form `(ParamTypes...) -> ReturnType`.

Examples:
```
(int, int) -> int                    -- binary function
() -> unit                           -- nullary function returning unit
(string) -> proc[mem(normal)] int    -- function returning a process
```

#### 4.2.2 Tuple Types
Tuple types have the form `(Type1, Type2, ..., TypeN)`.

Examples:
```
(int, bool)                          -- pair of int and bool
(char, char, char)                   -- triple of characters
()                                   -- unit (empty tuple)
```

#### 4.2.3 Record Types
Record types have the form `{field1: Type1, field2: Type2, ..., fieldN: TypeN}`.

Example:
```
{ name: string, age: int, active: bool }
```

#### 4.2.4 Variant Types
Variant types represent sum types with the form `Constructor1 [Type1] | Constructor2 [Type2] | ...`.

Examples:
```
type status = Ok | Error
```

### 4.3 Process Types
Process types represent monadic computations and have the form `proc[Effects] ResultType`.

Examples:
```
proc[] int                           -- pure computation returning int
proc[mem(normal)] ref(region, int)   -- computation allocating memory
proc[concurrency] actor_ref(msg)     -- computation spawning an actor
```

### 4.4 Region and Memory Types

#### 4.4.1 Region Types
Region types represent memory regions: `region(R, Space)` where R is a region identifier and Space is a memory space.

```
region(normal)                       -- normal memory region
region(atomic)                       -- atomic memory region
```

#### 4.4.2 Reference Types
Reference types represent pointers to memory: `ref(R, Space, T)`.

```
ref(R, normal, int)                  -- reference to int in region R
```

#### 4.4.3 Buffer Types
Buffer types represent contiguous arrays: `buf(R, Space, T, N)`.

```
buf(R, normal, int, 1024)            -- buffer of 1024 ints
```

#### 4.4.4 Atomic Types
Atomic reference types: `atomic_ref(R, Space, T)`.

```
atomic_ref(R, normal, int)           -- atomic reference to int
```

### 4.5 Actor Types

#### 4.5.1 Actor Reference Types
Actor references are a primitive type (like `int` or `bool`):

```
actor_ref                            -- actor reference (primitive type)
```

The `actor_ref` type is not parameterized by message type. It is a primitive type that represents a reference to an actor, created by the `spawn()` function.

## 5. Language Features

### 5.1 Advanced Pattern Matching

#### 5.1.1 Record Patterns
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

#### 5.1.2 Variant Patterns
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

### 5.2 Exception Handling

#### 5.2.1 Exception Types
Silica provides structured exception handling:

```silica
type exception =
    DivisionByZero
  | InvalidArgument(string)
  | FileNotFound(string)
```

#### 5.2.2 Throwing Exceptions
Safe division using case expressions:

```silica
fn safe_divide(x: int, y: int) -> int {
    case y == 0 of {
        true -> 0  // Return 0 for division by zero
        false -> x / y
    }
}
```

#### 5.2.3 Result Handling
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

### 5.3 Advanced Effects

#### 5.3.1 Effect Composition
Effects can be combined:

```silica
effect io_and_mem = [device_io, mem(normal)]

fn combined_operation() : proc[io_and_mem] int {
    do
        // Operations that require both I/O and memory effects
        42
    end
}
```

#### 5.3.2 Effect Inheritance
Effects can extend other effects:

```silica
effect basic_io = [device_io]
effect network_io extends basic_io = [networking]
effect file_io extends basic_io = [mem(normal)]
```

## 6. Basic Expressions

### 5.1 Literals
Literal expressions evaluate to their corresponding values:

```
42          -- evaluates to integer 42
true        -- evaluates to boolean true
'a'         -- evaluates to character 'a'
"hello"     -- evaluates to string "hello"
()          -- evaluates to unit value
```

### 5.2 Identifiers
Identifier expressions evaluate to the value bound to that identifier in the current scope.

```
x           -- evaluates to the value of variable x
my_function -- evaluates to the function bound to my_function
```

### 5.3 Arithmetic Expressions
Arithmetic operators work on integers:

```
x + y       -- integer addition
a - b       -- integer subtraction
m * n       -- integer multiplication
p / q       -- integer division (truncates toward zero)
r % s       -- integer modulo
```

### 5.4 Comparison Expressions
Comparison operators return boolean values:

```
x == y      -- equality
a != b      -- inequality
p < q       -- less than
r <= s      -- less than or equal
m > n       -- greater than
u >= v      -- greater than or equal
```

### 5.5 Logical Expressions
Logical operators work on booleans:

```
not p       -- logical negation
p and q     -- logical conjunction
r or s      -- logical disjunction
```

### 5.6 Function Application
Function application has the form `function(arg1, arg2, ..., argN)`:

```
add(3, 4)           -- applies add function to 3 and 4
length("hello")     -- applies length function to string
f()                 -- applies nullary function
```

### 5.7 Grouping
Parentheses can be used to group expressions and override precedence:

```
(2 + 3) * 4         -- evaluates to 20, not 14
not (p and q)       -- equivalent to (not p) or (not q)
```

## 7. Type System

### 6.1 Type Constructors
Type constructors define how to build complex types from simpler ones.

Built-in type constructors:
- `ref<R, S, T>` - reference in region R, space S, to type T
- `buf<R, S, T, N>` - buffer in region R, space S, of N elements of type T

User-defined types are declared with concrete types:

```
type int_stack = { elements: list<int>, size: int }
type string_map = { data: list<pair<string, string>>, size: int }
```

### 6.2 Type Equivalence and Subtyping

#### 6.2.1 Structural Equivalence
Types are equivalent if they have the same structure:

```
int ≡ int                                   -- primitive types
(int, bool) ≡ (int, bool)                   -- tuple types
{a: int, b: bool} ≡ {a: int, b: bool}       -- record types
```

#### 6.2.2 Nominal Equivalence for User Types
User-defined types are equivalent only if they have the same name:

```
type my_int = int
type your_int = int

my_int ≢ your_int    -- different names, not equivalent
my_int ≢ int         -- user type vs primitive
```

#### 6.2.3 Subtyping Rules
Silica has no subtyping - all types must match exactly.

```
list<T> <: list<U>    if T <: U    -- covariant
ref<R, S, T> ≢ ref<R, S, U>       -- invariant (unless T ≡ U)
```

### 6.3 Type Inference

#### 6.3.1 Hindley-Milner Style Inference
Silica uses Hindley-Milner style type inference extended for effects.

For expressions without explicit type annotations, the compiler infers the most general type.

#### 6.3.2 Inference Algorithm
1. Check explicit type annotations first
2. Infer types from literal values
3. Propagate types through expressions
4. Verify effect annotations match operations

#### 6.3.3 Type Annotations
Explicit type annotations can be provided to guide inference or document intent:

```
fn add(x: int, y: int) -> int { x + y }        -- explicit types

fn example() -> int {
    do
        x:int <- 42;                           -- variable binding with type
        x
    end
}
```

## 8. Effect System

### 7.1 Effect Types

#### 7.1.1 Built-in Effects
Silica defines several built-in effects that track different kinds of side effects:

- `mem(Space)` - Memory allocation/deallocation in space `Space`
- `mailbox(Msg)` - Message passing with message type `Msg`
- `concurrency` - Actor spawning and scheduling
- `atomic` - Atomic memory operations
- `device_io` - Device input/output operations

#### 7.1.2 Effect Aliases
Effects can be aliased for convenience and abstraction:

```
effect actor_eff = [mailbox<Msg>, concurrency]
effect io_eff = [mem(normal), device_io]
effect atomic_eff = [mem(atomic), atomic]
```

#### 7.1.3 User-Defined Effects
New effects can be declared for domain-specific side effects:

```
effect logging = []        -- pure effect for logging framework
effect database = [mem(normal), device_io]  -- database operations
```

### 7.2 Effect Composition

#### 7.2.1 Effect Sets
Effects are combined in sets: `proc[effect1, effect2, ...] Result`

```
proc[mem(normal), atomic] int           -- memory + atomic operations
proc[concurrency] actor_ref<msg>        -- actor spawning
proc[] unit                             -- pure computation
```

#### 7.2.2 Built-in Memory Operations
Silica provides built-in memory operations as primitive language constructs that return processes:

```
alloc_region(Space) : proc[mem(Space)] region(R, Space)
alloc_ref(Region, Value) : proc[mem(Space)] ref(R, Space, T)
read_ref(Ref) : proc[mem(Space)] T
write_ref(Ref, Value) : proc[mem(Space)] unit
```

These operations are not function calls but fundamental language primitives for memory management.

#### 7.2.3 Process Composition
Processes compose through monadic binding:

```
do
    x <- alloc_region(normal)     -- proc[mem(normal)] region
    y <- alloc_region(atomic)     -- proc[mem(atomic)] region
    return (x, y)
end

-- Result type: proc[mem(normal), mem(atomic)] (region, region)
```

#### 7.2.3 Effect Subeffecting
Effects form a subeffecting lattice:

- `mem(normal) <: mem(atomic)` - atomic space includes normal operations
- `[] <: E` - pure computations can be used where effects are expected
- `[e1] <: [e1, e2]` - subset relation for effect sets

### 7.3 Effect Tracking Rules

#### 7.3.1 Function Effects
Functions track effects of their body:

```
fn allocate_int(region: region(R, normal))
    : proc[mem(normal)] ref(R, normal, int) {
    alloc_ref(region, 0)  // built-in memory allocation primitive
}
```

#### 7.3.2 Effect Composition
Effect variables allow abstracting over unknown effects:

```
fn with_logging<E>(action: proc[E] int) : proc[E, logging] int {
    log("Starting action")
    result <- action
    log("Action complete")
    return result
}
```

### 7.4 Effect Safety

#### 7.4.1 Effect Checking
The type checker ensures:
- All effects in a process body are declared in its type
- Effect variables are properly instantiated
- Effect subeffecting is respected

#### 7.4.2 Runtime Effect Enforcement
At runtime, effect violations are caught:
- Attempting `mem` operations without `mem` capability
- Accessing mailbox without `mailbox` capability
- Atomic operations without `atomic` capability

## 9. Type Checking

### 8.1 Type Checking Rules

#### 8.1.1 Expression Typing
Every expression has a type and effect:

```
Γ ⊢ e : τ ! ε

Where:
- Γ is the type environment (variable bindings)
- e is the expression
- τ is the result type
- ε is the effect set
```

#### 8.1.2 Literal Typing
```
Γ ⊢ n : int ! []          where n is an integer literal
Γ ⊢ true : bool ! []
Γ ⊢ false : bool ! []
Γ ⊢ 'c' : char ! []
Γ ⊢ "s" : string ! []
Γ ⊢ () : unit ! []
```

#### 8.1.3 Variable Typing
```
Γ ⊢ x : Γ(x) ! []         if x ∈ dom(Γ)
```

#### 8.1.4 Function Application
```
Γ ⊢ f : (τ₁, τ₂, ..., τₙ) → τ ! ε
Γ ⊢ e₁ : τ₁ ! ε₁
...
Γ ⊢ eₙ : τₙ ! εₙ
─────────────────────────────────────────
Γ ⊢ f(e₁, ..., eₙ) : τ ! ε ∪ ε₁ ∪ ... ∪ εₙ
```

#### 8.1.5 Process Creation
```
Γ ⊢ e : τ ! ε
─────────────────
Γ ⊢ proc { e } : proc[ε] τ ! []
```

### 8.2 Declaration Type Checking

#### 8.2.1 Function Declaration
```
Γ ⊢ body : τ_body ! ε_body
τ_body ≡ τ_return
parameters define new bindings in Γ
─────────────────
Γ ⊢ fn f(params): τ_return { body } : () ! []
```

#### 8.2.2 Type Declaration
```
Type declaration is well-formed
─────────────────
Γ ⊢ type T<α₁, ..., αₙ> = τ : () ! []
```

#### 8.2.3 Effect Declaration
```
Effect declaration is well-formed
─────────────────
Γ ⊢ effect E<α₁, ..., αₙ> = [effects] : () ! []
```

### 8.3 Type Errors

#### 8.3.1 Type Mismatch
```
Expected type τ_expected, but got τ_actual
```

#### 8.3.2 Effect Mismatch
```
Process requires effects [ε_required] but declares [ε_declared]
```

#### 8.3.3 Unbound Variable
```
Variable x is not in scope
```

#### 8.3.4 Arity Mismatch
```
Function expects n arguments, but got m
```

### 8.4 Effect Checking Examples

```
fn pure_add(x: int, y: int) -> int {
    x + y        -- Type: int ! []
}
-- Function type: (int, int) -> int ! []

fn allocate_pair(r: region(R, normal))
    : proc[mem(normal)] (ref(R, normal, int), ref(R, normal, int)) {
    x: ref(R, normal, int) <- alloc_ref(r, 1)
    y: ref(R, normal, int) <- alloc_ref(r, 2)
    return (x, y)
}
-- Effects properly declared: mem(normal)

fn bad_alloc(r: region(R, normal)) : proc[] ref(R, normal, int) {
    alloc_ref(r, 42)    -- ERROR: requires mem(normal) but declares []
}
```

## 10. Process Semantics and Execution

### 9.1 Process Monad Structure

#### 9.1.1 Process Type
A process `proc[ε] τ` represents a computation that:
- Produces a value of type `τ`
- May perform effects in the set `ε`
- Is executed sequentially within its context

#### 9.1.2 Monadic Operations
Processes support monadic binding (`<-`) and return:

```
return v     -- lift pure value into process monad
p <- m; q    -- bind: execute m, bind result to p, then execute q
```

#### 9.1.3 Sequential Execution
Process execution is strictly sequential:

```
do
    x <- computation1    -- executes first
    y <- computation2    -- executes after computation1 completes
    return (x, y)        -- executes last
end
```

### 9.2 Effect Execution Model

#### 9.2.1 Effect Capabilities
Each effect requires runtime capability checking:

- `mem(S)` - Access to memory space S
- `mailbox(M)` - Message queue for type M
- `concurrency` - Actor spawning and scheduling
- `atomic` - Atomic memory operations
- `device_io` - Device access permissions

#### 9.2.2 Effect Tracking
Effects are tracked through the execution stack:

```
Execution Stack:
┌─────────────────────────────────┐
│ Process: proc[mem(normal)] ref  │  -- current process
├─────────────────────────────────┤
│ Process: proc[concurrency] unit │  -- caller
├─────────────────────────────────┤
│ Process: proc[] int            │  -- root
└─────────────────────────────────┘

Active Effects: [mem(normal), concurrency]
```

#### 9.2.3 Effect Safety
Runtime enforces effect capabilities:

```
proc[mem(normal)] ref = alloc_ref(region, value)
-- ✓ Allowed: mem(normal) capability active

proc[] ref = alloc_ref(region, value)
-- ✗ Runtime Error: missing mem(normal) capability
```

### 9.3 Process Lifecycle

#### 9.3.1 Process Creation
Processes are created but not executed until bound:

```
let p = alloc_ref(r, 42)    -- creates process, doesn't execute
x <- p                      -- executes process, binds result
```

#### 9.3.2 Process Execution
Process execution is lazy - triggered by binding:

```
-- Process creation (no execution)
let computation = do
    x <- alloc_ref(region, 1)
    y <- alloc_ref(region, 2)
    return (x, y)
end

-- Process execution (triggered by binding)
result <- computation    -- now executes allocations
```

#### 9.3.3 Process Composition
Processes compose through monadic binding:

```
fn allocate_pair(r: region(R, normal))
    : proc[mem(normal)] (ref(R, normal, int), ref(R, normal, int)) {
    x <- alloc_ref(r, 1)
    y <- alloc_ref(r, 2)
    return (x, y)
}

fn allocate_quad(r: region(R, normal))
    : proc[mem(normal)] (ref(R, normal, int), ref(R, normal, int),
                         ref(R, normal, int), ref(R, normal, int)) {
    (a: ref(R, normal, int), b: ref(R, normal, int)) <- allocate_pair(r)
    (c: ref(R, normal, int), d: ref(R, normal, int)) <- allocate_pair(r)
    return (a, b, c, d)
}
```

## 11. Memory Model

### 10.1 Region-Based Memory Management

#### 10.1.1 Region Allocation
Regions are allocated explicitly and provide memory pools:

```
alloc_region(normal) : proc[mem(normal)] region(R, normal)
alloc_region(atomic) : proc[mem(atomic)] region(R, atomic)
```

#### 10.1.2 Region Lifetime
Regions exist until explicitly deallocated or process termination:

```
r <- alloc_region(normal)    -- region created
refs <- allocate_in_region(r) -- allocate references in region
-- implicit deallocation when r goes out of scope
```

#### 10.1.3 Region Isolation
Regions provide memory isolation:

```
r1 <- alloc_region(normal)
r2 <- alloc_region(normal)
-- r1 and r2 are separate memory pools
-- no aliasing between different regions
```

### 10.2 Reference Semantics

#### 10.2.1 Reference Creation
References are allocated within regions:

```
alloc_ref(region, initial_value) : proc[mem(Space)] ref(R, Space, T)
```

#### 10.2.2 Reference Operations
References support reading and writing:

```
read_ref(reference)  : proc[mem(Space)] T
write_ref(reference, value) : proc[mem(Space)] unit
```

#### 10.2.3 Reference Identity
References are identity-based:

```
r1 <- alloc_ref(region, 42)
r2 <- alloc_ref(region, 42)
r1 ≠ r2    -- different references, even with same value
```

### 10.3 Buffer Semantics

#### 10.3.1 Buffer Types
Buffers represent contiguous memory arrays:

```
buf(R, Space, T, N)    -- buffer of N elements of type T
```

#### 10.3.2 Buffer Operations
Buffers support indexed access:

```
read_buf(buffer, index)  : proc[mem(Space)] T
write_buf(buffer, index, value) : proc[mem(Space)] unit
```

#### 10.3.3 Bounds Checking
Buffer access is bounds-checked:

```
let buf = alloc_buf(region, 10)  -- buffer of size 10
x <- read_buf(buf, 5)           -- ✓ valid index
y <- read_buf(buf, 15)          -- ✗ runtime bounds error
```

### 10.4 Memory Safety Guarantees

#### 10.4.1 Region Isolation
No cross-region references:

```
r1 <- alloc_region(normal)
r2 <- alloc_region(normal)
ref1 <- alloc_ref(r1, 42)

-- Cannot create reference in r2 pointing to r1's memory
-- Type system prevents: ref(R2, normal, ref(R1, normal, int))
```

#### 10.4.2 Lifetime Safety
References cannot outlive their regions:

```
{
    r <- alloc_region(normal)
    ref <- alloc_ref(r, 42)
    -- ref is valid here
}
-- r deallocated here
-- ref is now invalid (use would be memory error)
```

#### 10.4.3 Type Safety
Memory operations preserve types:

```
ref_int <- alloc_ref(r, 42)
ref_str <- alloc_ref(r, "hello")

x <- read_ref(ref_int)    -- x : int
y <- read_ref(ref_str)    -- y : string
```

## 12. Operational Semantics

### 11.1 Evaluation Judgment

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

### 11.2 Expression Evaluation Rules

#### 11.2.1 Literal Evaluation
```
ρ; σ; κ ⊢ n ⇓ n; σ; κ          (integer literal)
ρ; σ; κ ⊢ true ⇓ true; σ; κ     (boolean literal)
ρ; σ; κ ⊢ 'c' ⇓ 'c'; σ; κ       (character literal)
ρ; σ; κ ⊢ "s" ⇓ "s"; σ; κ       (string literal)
ρ; σ; κ ⊢ () ⇓ (); σ; κ         (unit literal)
```

#### 11.2.2 Variable Lookup
```
ρ(x) = v
─────────────────────────────
ρ; σ; κ ⊢ x ⇓ v; σ; κ
```

#### 11.2.3 Arithmetic Operations
```
ρ; σ; κ ⊢ e₁ ⇓ n₁; σ₁; κ₁
ρ; σ₁; κ₁ ⊢ e₂ ⇓ n₂; σ₂; κ₂
─────────────────────────────
ρ; σ; κ ⊢ e₁ + e₂ ⇓ n₁ + n₂; σ₂; κ₂
```

Similar rules for `-`, `*`, `/`, `%`.

#### 11.2.4 Function Application
```
ρ; σ; κ ⊢ f ⇓ <λx.e, ρ'>; σ₁; κ₁
ρ; σ₁; κ₁ ⊢ e_arg ⇓ v_arg; σ₂; κ₂
ρ'[x → v_arg]; σ₂; κ₂ ⊢ e ⇓ v_result; σ₃; κ₃
────────────────────────────────────────────────
ρ; σ; κ ⊢ f(e_arg) ⇓ v_result; σ₃; κ₃
```

#### 11.2.5 Process Creation
```
─────────────────────────────
ρ; σ; κ ⊢ proc { e } ⇓ <proc e ρ>; σ; κ
```

#### 11.2.6 Process Execution (Binding)
```
ρ; σ; κ ⊢ e_proc ⇓ <proc e_body ρ_proc>; σ₁; κ₁
ρ_proc; σ₁; κ₁ ⊢ e_body ⇓ v_result; σ₂; κ₂
────────────────────────────────────────────
ρ; σ; κ ⊢ x <- e_proc; e_cont ⇓ v_cont; σ₃; κ₃

Where ρ[x → v_result]; σ₂; κ₂ ⊢ e_cont ⇓ v_cont; σ₃; κ₃
```

### 11.3 Memory Operation Semantics

#### 11.3.1 Region Allocation
```
κ contains mem(Space)
σ' = σ[r ↦ new_region(Space)]
─────────────────────────────
ρ; σ; κ ⊢ alloc_region(Space) ⇓ r; σ'; κ
```

#### 11.3.2 Reference Allocation
```
κ contains mem(Space)
σ(region) = region_state
σ' = σ[region ↦ region_state[ref ↦ initial_value]]
─────────────────────────────
ρ; σ; κ ⊢ alloc_ref(region, initial_value) ⇓ ref; σ'; κ
```

#### 11.3.3 Reference Read
```
κ contains mem(Space)
σ(region)(ref) = value
─────────────────────────────
ρ; σ; κ ⊢ read_ref(ref) ⇓ value; σ; κ
```

#### 11.3.4 Reference Write
```
κ contains mem(Space)
region_state' = σ(region)[ref → new_value]
σ' = σ[region ↦ region_state']
─────────────────────────────
ρ; σ; κ ⊢ write_ref(ref, new_value) ⇓ (); σ'; κ
```

### 11.4 Control Flow Semantics

#### 11.4.1 Case Expression
```
ρ; σ; κ ⊢ e_scrut ⇓ v; σ₁; κ₁
pattern_match(v, p₁) = bindings₁
ρ₁ = ρ ∪ bindings₁
ρ₁; σ₁; κ₁ ⊢ e₁ ⇓ v_result; σ₂; κ₂
─────────────────────────────────
ρ; σ; κ ⊢ case e_scrut of { p₁ -> e₁; ... } ⇓ v_result; σ₂; κ₂
```

### 11.5 Do Expression Semantics

The `do ... end` expression is syntactic sugar for monadic binding:

```
do
    x <- e1
    y <- e2
    return result
end

≡

x <- e1;
y <- e2;
result
```

## 13. Safety Properties

### 12.1 Memory Safety

#### 12.1.1 No Null Pointers
All references are guaranteed to be valid:

- References are created by explicit allocation
- No implicit null values
- Type system prevents uninitialized references

#### 12.1.2 No Dangling Pointers
Reference lifetimes are bounded by region lifetimes:

```
{
    r <- alloc_region(normal)
    ref <- alloc_ref(r, 42)
    -- ref is valid here
}
-- r and ref are deallocated together
```

#### 12.1.3 No Use-After-Free
Attempting to use a reference after its region is deallocated is a type error:

```
fn bad_lifetime() {
    r <- alloc_region(normal)
    ref <- alloc_ref(r, 42)
    return ref    -- ERROR: ref would outlive region r
}
```

### 12.2 Type Safety

#### 12.2.1 Type Preservation
Well-typed programs don't go wrong:

```
If ⊢ program : τ then either:
- program evaluates to value of type τ, or
- program encounters runtime effect violation
```

#### 12.2.2 Effect Safety
Effect violations are caught at runtime:

```
proc[] int = alloc_ref(r, 42)    -- Type checks!
-- But fails at runtime: missing mem(normal) capability
```

#### 12.2.3 Pattern Match Exhaustiveness
Case expressions must cover all possible values:

```
type option<T> = Some(T) | None

case opt of
    Some(x) -> x
    -- Missing None case: compilation error
```

### 12.3 Concurrency Safety

#### 12.3.1 Actor Isolation
Actors have isolated state and communication:

- No shared mutable state between actors
- All communication through message passing
- Actor failures don't corrupt other actors

#### 12.3.2 Message Ordering
Messages maintain happens-before relationships:

```
send(actor1, msg1)
send(actor1, msg2)
-- actor1 receives msg1 before msg2
```

#### 12.3.3 Atomicity Guarantees
Atomic operations provide strong guarantees:

```
atomic_compare_exchange(ref, expected, new)
-- Either succeeds completely or fails completely
-- No partial updates visible to other threads
```

### 12.4 Runtime Safety

#### 12.4.1 Bounds Checking
Array/buffer access is bounds-checked:

```
buf <- alloc_buf(r, 10)
x <- read_buf(buf, 15)    -- Runtime error: index out of bounds
```

#### 12.4.2 Division by Zero
Integer division checks for zero divisor:

```
x / 0    -- Runtime error: division by zero
```

#### 12.4.3 Effect Violations
Missing capabilities cause runtime errors:

```
-- Without mem(normal) capability:
alloc_ref(r, 42)    -- Runtime error: capability violation
```

## 14. Actor Model Semantics

### 13.1 Actor Lifecycle

#### 13.1.1 Actor Creation
Actors are created with initial state and behavior function:

```
spawn(initial_state, behavior_fn) : proc[concurrency] actor_ref
```

The behavior function has type: `(Msg, State) -> int` (simplified for current implementation)

The `initial_state` parameter must implement the `ActorState` trait (for named types only). The `actor_ref` return type is a primitive type (like `int` or `bool`), not parameterized by message type.

#### 13.1.2 Actor Execution Model
Each actor executes as an infinite loop in the runtime system:

```
actor_loop(state, behavior) {
    message <- recv()           -- runtime receives message from mailbox
    new_state <- behavior(message, state)  -- user behavior processes message
    actor_loop(new_state, behavior)        -- continue with new state
}
```

**Important**: The `recv()` operation is performed by the actor runtime system, not by user code. User-defined behavior functions only receive the message and current state as parameters - they never call `recv()` directly.

#### 13.1.3 Actor Identity
Each actor has a unique identity:

```
self() : proc[mailbox<Msg>, concurrency] actor_ref
```

The `actor_ref` type is a primitive type (like `int` or `bool`), representing a reference to an actor.

### 13.2 Actor Behavior Functions

#### 13.2.1 Behavior Function Signature
Behavior functions transform messages and state:

```
type Request = {command: string, reply_to: actor_ref};
type Response = {result: int};
impl ActorMessage for Request;
impl ActorMessage for Response;

fn counter(msg: Request, state: int)
    : proc[mailbox<Request>, concurrency] int {

    case msg of
        {command: "increment", reply_to} -> return state + 1
        {command: "get", reply_to} ->
            -- Send response back using cast
            cast(reply_to, Response {result: state})
            return state
        {command: "reset", reply_to} -> return 0
    end
}
```

#### 13.2.2 State Encapsulation
Actor state is private and can only be modified by the actor itself:

```
-- External code cannot access or modify actor state
actor_ref <- spawn(0, counter)
-- No way to read or write the counter value directly
```

#### 13.2.3 Behavior Hot-Swapping
Actors can change their behavior by returning a different behavior function type (future extension).

### 13.3 Actor Failure and Supervision

#### 13.3.1 Actor Termination
Actors terminate when their behavior function cannot handle a message:

```
fn fragile_behavior(msg: string, state: unit) : proc[mailbox<string>] unit {
    case msg of
        "quit" -> -- terminate actor (no return)
        other -> return ()  -- continue
    end
}
```

#### 13.3.2 Failure Isolation
Actor failures don't affect other actors:

```
actor1 <- spawn((), fragile_behavior)
actor2 <- spawn((), robust_behavior)

send(actor1, "quit")    -- actor1 terminates
send(actor2, "ping")    -- actor2 continues normally
```

## 15. Message Passing

### 14.1 Message Send Semantics

#### 14.1.1 Asynchronous Send
Messages are sent asynchronously:

```
send(actor: actor_ref, message: ActorMessage) : proc[concurrency] unit
```

Send never blocks - messages are queued in the actor's mailbox. The `message` parameter must be a type that implements the `ActorMessage` trait (for named types only).

#### 14.1.2 Asynchronous Cast
Messages can be sent asynchronously without blocking, with success/failure indication:

```
cast(actor: actor_ref, message: ActorMessage) : proc[concurrency] bool
```

Cast never blocks - messages are queued in the actor's mailbox and the function returns immediately. Returns `true` if the message was successfully enqueued, `false` if the actor doesn't exist or the mailbox is full. The `message` parameter must be a type that implements the `ActorMessage` trait (for named types only).

#### 14.1.3 Message Ordering
Messages from the same sender maintain order:

```
send(actor, msg1)
send(actor, msg2)
-- actor receives msg1, then msg2
```

#### 14.1.4 Message Delivery
Messages are delivered exactly once, in FIFO order per sender.

### 14.2 Message Receive Semantics

#### 14.2.1 Runtime Message Reception
Message reception is handled automatically by the actor runtime system. The `recv()` operation is not available for direct use in user code:

```
recv() : proc[mailbox<Msg>, concurrency] Msg  -- Runtime internal function
```

User behavior functions receive messages as parameters rather than calling `recv()` directly.

#### 14.2.2 Mailbox Semantics
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

#### 14.2.3 Message Patterns in Behavior Functions
Behavior functions receive messages as parameters and can use pattern matching on them:

```
fn selective_receiver(msg: msg_type, state: unit) : proc[mailbox<msg_type>] unit {
    case msg of
        {request, data} -> handle_request(data)
        ping -> handle_ping()
        quit -> terminate()
    end
    return ()
}
```

The message parameter is automatically provided by the actor runtime when a message is received.

### 14.2.4 Cast vs Send
Both `cast()` and `send()` send messages asynchronously without blocking:

- **`send()`**: Returns `unit` - fire-and-forget message sending
- **`cast()`**: Returns `bool` - indicates success/failure of message enqueueing

Use `cast()` when you need to know if the message was successfully enqueued. Use `send()` for simple fire-and-forget messaging.

### 14.3 Message Types and Serialization

#### 14.3.1 ActorState Trait
The `ActorState` trait is a marker trait that must be implemented by types used as actor initial state:

```
trait ActorState {
    // No methods required - marker trait for type safety
}
```

Only the `initial_state` parameter in `spawn(initial_state, ...)` must implement `ActorState`. The trait is only implemented for named types (structs, type aliases) - no blanket implementations for primitive types.

#### 14.3.2 ActorMessage Trait
The `ActorMessage` trait is a marker trait that must be implemented by types used as messages:

```
trait ActorMessage {
    // No methods required - marker trait for type safety
}
```

All types used in `send()` or `cast()` must implement `ActorMessage`. The trait is only implemented for named types (structs, type aliases) - no blanket implementations for primitive types.

#### 14.3.3 Message Type Safety
Messages must implement the `ActorMessage` trait:

```
type Request = {data: int, reply_to: actor_ref};
impl ActorMessage for Request;

actor_ref <- spawn(0, handler)
cast(actor_ref, Request {data: 42, reply_to: some_actor})  -- ✓ correct type
cast(actor_ref, 42)  -- ✗ type error: int doesn't implement ActorMessage
```

#### 14.3.4 Cast-Back Pattern
Messages can include a `reply_to` field containing an `actor_ref` for sending responses back:

```
type Request = {data: int, reply_to: actor_ref};
type Response = {result: int};
impl ActorMessage for Request;
impl ActorMessage for Response;

fn handler(msg: Request, state: State) -> State {
    case msg of
        {data, reply_to} ->
            -- Process request and send response back
            cast(reply_to, Response {result: data * 2})
            -- ... update state ...
    end
}
```

The `reply_to` field is optional - messages without it cannot be used for cast-back, but this is enforced at compile time through field access checks. Attempting to access `reply_to` on a message type that doesn't have it results in a compile-time error.

#### 14.3.5 Message Passing Guarantees
- **Type Safety**: Messages are type-checked at compile time - must implement `ActorMessage` trait
- **Trait Checking**: All type inference and trait checking happens at compile time, not runtime
- **Compile-Time Verification**: Field access (e.g., `reply_to`) is verified at compile time - attempting to access a field that doesn't exist in the message type results in a compile-time error
- **Immutability**: Message data cannot be mutated after sending
- **Isolation**: Message contents are copied between actors
- **Cast Success Indication**: `cast()` returns `bool` indicating success/failure of message enqueueing
- **Actor Reference Type**: `actor_ref` is a primitive type (like `int` or `bool`), not parameterized by message type

## 16. Atomic Operations

### 15.1 Atomic Types and Memory Spaces

#### 15.1.1 Atomic References
Atomic references provide thread-safe shared memory:

```
atomic_ref(R, Space, T)    -- atomic reference to type T
```

#### 15.1.2 Atomic Memory Spaces
Atomic operations work in designated memory spaces:

```
alloc_atomic(region, initial_value) : proc[mem(Space), atomic] atomic_ref(R, Space, T)
```

### 15.2 Memory Ordering Semantics

#### 15.2.1 Ordering Levels
Silica supports standard memory orderings:

```
type order = relaxed | acquire | release | acq_rel | seq_cst
```

#### 15.2.2 Ordering Guarantees

**relaxed**: No ordering constraints
```
atomic_load(ref, relaxed)     -- no synchronization
atomic_store(ref, value, relaxed)
```

**acquire**: Synchronizes with release operations
```
atomic_load(ref, acquire)     -- establishes happens-before with prior releases
```

**release**: Synchronizes with acquire operations
```
atomic_store(ref, value, release)  -- establishes happens-before for future acquires
```

**acq_rel**: Both acquire and release semantics
```
atomic_fetch_add(ref, delta, acq_rel)
```

**seq_cst**: Sequential consistency
```
atomic_load(ref, seq_cst)     -- participates in global total order
```

### 15.3 Atomic Primitives

#### 15.3.1 Load and Store
```
atomic_load(aref, order) : proc[mem(Space), atomic] T
atomic_store(aref, value, order) : proc[mem(Space), atomic] unit
```

#### 15.3.2 Read-Modify-Write Operations
```
atomic_fetch_add(aref, delta, order) : proc[mem(Space), atomic] T
atomic_fetch_sub(aref, delta, order) : proc[mem(Space), atomic] T
atomic_fetch_and(aref, mask, order) : proc[mem(Space), atomic] T
atomic_fetch_or(aref, mask, order) : proc[mem(Space), atomic] T
atomic_fetch_xor(aref, mask, order) : proc[mem(Space), atomic] T
```

#### 15.3.3 Compare and Exchange
```
atomic_compare_exchange(aref, expected, new_val, order)
    : proc[mem(Space), atomic] {ok, T} | {fail, T}
```

Returns `{ok, old_value}` if successful, `{fail, current_value}` if the value wasn't expected.

### 15.4 Lock-Free Data Structures

#### 15.4.1 SPSC Queue Example
```
type spsc_queue<R, T> = {
    buf: buf(R, normal, T, Capacity),
    capacity: int,
    head: atomic_ref(R, normal, int),
    tail: atomic_ref(R, normal, int)
}

fn spsc_send(queue, item) : proc[mem(normal), atomic] bool {
    tail <- atomic_load(queue.tail, acquire)
    head <- atomic_load(queue.head, acquire)

    next_tail = (tail + 1) % queue.capacity
    if next_tail == head {
        return false    -- queue full
    }

    write_buf(queue.buf, tail, item)
    atomic_store(queue.tail, next_tail, release)
    return true
}

fn spsc_recv(queue) : proc[mem(normal), atomic] option<T> {
    head <- atomic_load(queue.head, acquire)
    tail <- atomic_load(queue.tail, acquire)

    if head == tail {
        return None     -- queue empty
    }

    item <- read_buf(queue.buf, head)
    next_head = (head + 1) % queue.capacity
    atomic_store(queue.head, next_head, release)
    return Some(item)
}
```

## 17. Synchronization Guarantees

### 16.1 Happens-Before Relationships

#### 16.1.1 Actor Message Ordering
```
send(actorA, msg1)
send(actorA, msg2)
```
establishes: `msg1` happens-before `msg2` in actorA

#### 16.1.2 Atomic Synchronization
```
atomic_store(ref, value, release)  -- in actor A
atomic_load(ref, acquire)          -- in actor B
```
establishes: store happens-before load

#### 16.1.3 Transitive Ordering
Happens-before is transitive:
```
A → B and B → C implies A → C
```

### 16.2 Memory Consistency Models

#### 16.2.1 Per-Actor Sequential Consistency
Within a single actor, all operations appear sequentially consistent:

```
-- Inside actor, this appears atomic to external observers
x <- read_ref(ref1)
y <- read_ref(ref2)
write_ref(ref3, x + y)
```

#### 16.2.2 Cross-Actor Ordering
Between actors, only explicit synchronization establishes ordering:

```
-- Actor 1
atomic_store(flag, true, release)
send(actor2, data)

-- Actor 2 behavior function
fn process_message(msg: Data, state: unit) -> unit {
    flag_value <- atomic_load(flag, acquire)
    -- flag_value is guaranteed to be true
}
```

### 16.3 Race Condition Prevention

#### 16.3.1 Atomic Operations
Atomic operations prevent data races:

```
counter <- alloc_atomic(region, 0)

-- Multiple actors can safely increment
fn increment_counter() {
    atomic_fetch_add(counter, 1, seq_cst)
}
```

#### 16.3.2 Actor Isolation
Actors cannot directly share mutable state:

```
-- This is not possible in Silica
actor1_state = actor2.state.field  -- ✗ No shared state access
```

#### 16.3.3 Message Immutability
Messages cannot be mutated after sending:

```
mutable_data = {value: 42}
send(actor, mutable_data)
-- Cannot modify mutable_data.value here
-- Actor receives immutable copy
```

### 16.4 Deadlock Freedom

#### 16.4.1 No Blocking Sends
Send operations never block - no send-side deadlocks.

#### 16.4.2 Actor Autonomy
Actors process messages independently - no circular wait conditions.

#### 16.4.3 Atomic Operation Atomicity
Atomic RMW operations are indivisible - no partial update deadlocks.

### 16.5 Performance Guarantees

#### 16.5.1 Lock-Free Algorithms
Atomic operations enable lock-free data structures:

```
-- SPSC queue: no locks, wait-free for single producer/consumer
-- MPSC queue: lock-free, wait-free for producers
```

#### 16.5.2 Composable Concurrency
Actors compose without synchronization overhead:

```
-- Independent actors scale linearly
-- No global locks or shared state bottlenecks
```

#### 16.5.3 Hardware Utilization
Direct mapping to AArch64 concurrency features:

```
load acquire  → LDAR (load-acquire)
store release → STLR (store-release)
RMW operations → LDXR/STXR loops with barriers
```

## 18. Module System

### 17.1 Module Structure

#### 17.1.1 Filename-Based Modules
Modules are implicitly created from source file names. A file named `math_utils.silica` automatically creates a module named `math_utils`. No explicit module declarations are required in the source code.

#### 17.1.2 Module Naming
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
├── main.silica          -- module 'main'
├── math_utils.silica    -- module 'math_utils'
└── utils/
    ├── string.silica    -- module 'string'
    └── list.silica      -- module 'list'
```

### 17.2 Export System

#### 17.2.1 Export Declarations
Functions are exported using the `export` keyword with function name and arity:

```
export add/2, multiply/2, factorial/1;
```

- Functions must be defined in the same module to be exported
- Arity specifies the number of parameters (e.g., `add/2` for binary addition)
- Only exported functions are visible to importing modules
- All exported symbols are available to importers (no selective imports)

#### 17.2.2 Export Validation
The compiler validates that:
- All exported symbols exist in the module
- Arities match the actual function definitions
- No duplicate exports in the same module

### 17.3 Import System

#### 17.3.1 Module Imports
Import modules using the `use` keyword with comma-separated module names:

```
use math_utils;                    -- import single module
use collections, io, string;      -- import multiple modules
```

- All exported functions from imported modules become available in the current scope
- No selective imports - all exports are imported
- No module renaming - imported modules are accessed by their original names
- Imports must appear at the top level of a module (before any function definitions)

#### 17.3.2 Name Resolution
Imported functions are accessed directly by name:

```
use math_utils;

fn main() -> int {
    do
        result:int <- add(3, 4);   -- 'add' from math_utils module
        multiply(result, 2)        -- 'multiply' from math_utils module
    end
}
```

#### 17.3.3 Name Conflicts
- If two imported modules export functions with the same name, it's a compiler error
- Variable shadowing is not allowed; attempting to shadow a variable causes a compilation error
- Explicit qualification is not supported - conflicts must be resolved by renaming or restructuring

#### 17.3.4 Module System Design Principles
Silica's module system is designed to be:
- **Simple**: No complex hierarchical namespaces or selective imports
- **Explicit**: All exports and imports are clearly declared
- **Safe**: Name conflicts are caught at compile time
- **Scalable**: Separate compilation with proper dependency tracking

### 17.4 Module Dependencies

#### 17.4.1 Dependency Resolution
Modules can depend on other modules through imports:

```
-- math_utils.silica (module name: math_utils)
export add/2, multiply/2;

fn add(x: int, y: int) -> int { x + y }
fn multiply(x: int, y: int) -> int { x * y }

-- main.silica (module name: main)
use math_utils;

fn main() -> int {
    do
        sum:int <- add(3, 4);        -- uses function from math_utils
        product:int <- multiply(sum, 2);  -- uses another function from math_utils
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

#### 17.4.3 Example Project Structure
```
my_project/
├── silica-comp -I modules -I stdlib main.silica
├── main.silica           -- Entry point module
├── modules/
│   ├── math_utils.silica -- Math utilities
│   └── collections.silica -- Data structures
└── stdlib/
    ├── io.silica         -- Input/output
    └── string.silica     -- String operations
```

#### 17.4.2 Compilation Order
Modules are compiled in dependency order:
1. Parse all module files
2. Build dependency graph from import declarations
3. Compile modules with no dependencies first
4. Compile dependent modules after their dependencies

#### 17.4.3 Cyclic Dependencies
Cyclic module dependencies are not allowed:

```
-- a.silica
use b;        -- A depends on B

-- b.silica
use a;        -- B depends on A (creates cycle)
```
This results in a compilation error.

### 17.5 Module Search Paths

#### 17.5.1 Search Path Configuration
Module files are located using configurable search paths:

```
silica-comp --search-path ./modules --search-path ./stdlib main.silica
```

- Search paths are specified with `--search-path` or `-I` flags
- Multiple paths can be specified
- Paths are searched in the order given
- Default search path is the current directory (`.`)

#### 17.5.2 Module Resolution Algorithm
When resolving a module import:
1. Extract module name from `use module_name;` declaration
2. For each search path in order:
   - Check if `path/module_name.silica` exists
   - If found, load and parse the module
3. If not found in any search path, report compilation error

#### 17.5.3 Search Path Examples
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

### 17.6 Module Validation and Errors

#### 17.6.1 Module Resolution Errors
- **Module Not Found**: When a `use module_name;` declaration cannot locate `module_name.silica` in any search path
- **Invalid Module Name**: Module names must be valid identifiers
- **Circular Dependencies**: Import cycles between modules

#### 17.6.2 Export Validation Errors
- **Undefined Export**: Exporting a function that doesn't exist in the module
- **Wrong Arity**: Export arity doesn't match the actual function definition
- **Duplicate Exports**: Same function exported multiple times

#### 17.6.3 Import Validation Errors
- **Name Conflicts**: Multiple imported modules export the same function name
- **Invalid Module Reference**: Importing a module that fails to parse or type-check

#### 17.6.4 Error Examples
```
-- Error: module 'nonexistent' not found in search paths
use nonexistent;

-- Error: function 'divide' not defined in this module
export divide/2;

-- Error: both 'math' and 'advanced_math' export 'add'
use math, advanced_math;  -- if both export add/2
```

## 19. Standard Library

### 18.1 Core Types

#### 18.1.1 Option Type
Represents optional values:

```
type option<T> = Some(T) | None

fn find_index(list: list<T>, item: T) -> option<int> {
    -- returns Some(index) or None
}
```

#### 18.1.2 Result Type
Represents computation results or errors:

```
type result<T, E> = Ok(T) | Error(E)

fn parse_int(s: string) -> result<int, string> {
    -- returns Ok(value) or Error("invalid number")
}
```

#### 18.1.3 List Type
Dynamic arrays with automatic memory management:

```
type list<T> = {data: buf(R, normal, T, capacity), size: int, capacity: int}

fn cons(list: list<T>, item: T) -> list<T>
fn head(list: list<T>) -> option<T>
fn tail(list: list<T>) -> list<T>
```

### 18.2 Core Functions

#### 18.2.1 Arithmetic Functions
```
fn abs(x: int) -> int
fn min(a: int, b: int) -> int
fn max(a: int, b: int) -> int
fn pow(base: int, exp: int) -> int
```

#### 18.2.2 String Functions
```
fn length(s: string) -> int
fn concat(s1: string, s2: string) -> string
fn substring(s: string, start: int, len: int) -> string
fn contains(s: string, substr: string) -> bool
```

#### 18.2.3 List Functions
```
fn length<T>(list: list<T>) -> int
fn is_empty<T>(list: list<T>) -> bool
fn nth<T>(list: list<T>, index: int) -> option<T>
fn append<T>(list1: list<T>, list2: list<T>) -> list<T>
fn map<T, U>(list: list<T>, f: (T) -> U) -> list<U>
fn filter<T>(list: list<T>, pred: (T) -> bool) -> list<T>
fn fold<T, U>(list: list<T>, init: U, f: (U, T) -> U) -> U
```

#### 18.2.4 IO Functions
```
fn print(s: string) -> proc[device_io] unit
fn println(s: string) -> proc[device_io] unit
fn read_line() -> proc[device_io] string

#### 18.2.5 Debug and Assertion Functions
```
fn debug_print(value: T) -> proc[] unit        -- Print any value for debugging
fn debug_println(value: T) -> proc[] unit     -- Print any value with newline
fn assert(condition: bool, message: string)  -- Terminate process if condition false
    -> proc[] unit
```

**Assertion Semantics:**
Assertions check for programming errors during development and testing. When an assertion fails:
- The current process/actor terminates with an `AssertionError`
- Supervisors can catch this and decide whether to restart or escalate
- Failed assertions should not occur in production code
- Unlike exceptions, assertions are for catching logic errors, not recoverable runtime conditions
```

### 18.3 Actor Utilities

#### 18.3.1 Actor Registry
```
fn register(name: string, actor: actor_ref<Msg>) -> proc[concurrency] unit
fn lookup(name: string) -> proc[concurrency] option<actor_ref<Msg>>
```

#### 18.3.2 Message Broadcasting
```
fn broadcast(actors: list<actor_ref<Msg>>, message: Msg) -> proc[concurrency] unit
```

#### 18.3.3 Actor Monitoring
```
fn monitor(target: actor_ref<any>, monitor: actor_ref<down_msg>)
    -> proc[concurrency] unit
```

### 18.4 Networking

Silica provides optional networking capabilities through effect-gated modules. Networking is not part of the core language but available as standard library modules that require the `networking` effect.

#### 18.4.1 Core Networking Types
```
type socket_addr = {
    ip: ip_addr,
    port: int
}

type ip_addr = ipv4_addr | ipv6_addr
type ipv4_addr = (int, int, int, int)  -- IPv4 tuple
type ipv6_addr = buf(R, normal, int, 16)  -- 16-byte IPv6 address

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

#### 18.4.2 Socket Module
```
module net.socket {

    pub type socket<T: protocol_type>  -- Protocol-specific socket

    pub fn create_socket(protocol: protocol_type)
        -> proc[networking] result<socket<protocol>, net_error>

    pub fn bind_socket(sock: socket<T>, addr: socket_addr)
        -> proc[networking] result<unit, net_error>

    pub fn close_socket(sock: socket<T>)
        -> proc[networking] unit

    pub fn get_socket_addr(sock: socket<T>)
        -> proc[networking] socket_addr

    pub fn set_socket_option<T>(sock: socket<T>, option: socket_option, value: T)
        -> proc[networking] result<unit, net_error>
}
```

#### 18.4.3 TCP Module
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
        -> proc[networking] result<tcp_connection, net_error>

    pub fn listen(sock: tcp_socket, backlog: int)
        -> proc[networking] result<unit, net_error>

    pub fn accept(sock: tcp_socket)
        -> proc[networking] result<tcp_connection, net_error>

    pub fn send(sock: tcp_connection, data: buf(R, normal, byte, size))
        -> proc[networking] result<int, net_error>

    pub fn receive(sock: tcp_connection, buffer: buf(R, normal, byte, max_size))
        -> proc[networking] result<int, net_error>

    pub fn shutdown(sock: tcp_connection, direction: shutdown_direction)
        -> proc[networking] result<unit, net_error>
}
```

#### 18.4.4 UDP Module
```
module net.udp {

    use module net.socket

    pub type udp_socket = socket<udp>
    pub type udp_endpoint = socket_addr

    pub fn send_to(sock: udp_socket, data: buf(R, normal, byte, size), dest: socket_addr)
        -> proc[networking] result<int, net_error>

    pub fn receive_from(sock: udp_socket, buffer: buf(R, normal, byte, max_size))
        -> proc[networking] result<(int, socket_addr), net_error>

    pub fn join_multicast_group(sock: udp_socket, group_addr: ip_addr, interface: ip_addr)
        -> proc[networking] result<unit, net_error>

    pub fn leave_multicast_group(sock: udp_socket, group_addr: ip_addr, interface: ip_addr)
        -> proc[networking] result<unit, net_error>
}
```

#### 18.4.5 Packet Processing Module
```
module net.packet {

    use module arch.sve  -- Optional: for SIMD acceleration

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
        -> proc[] result<ethernet_frame, parse_error>

    pub fn parse_ipv4_packet(data: buf(R, normal, byte, packet_size))
        -> proc[] result<ipv4_packet, parse_error>

    pub fn calculate_ipv4_checksum(packet: ipv4_packet)
        -> proc[] int

    pub fn validate_packet(packet: ipv4_packet)
        -> proc[] result<unit, validation_error>

    -- SIMD-accelerated batch processing (when SVE available)
    pub fn process_packet_batch(packets: buf(R, normal, packet, batch_size))
        -> proc[] processed_results
}
```

#### 18.4.6 Networking Utilities
```
module net.utils {

    pub fn resolve_hostname(hostname: string)
        -> proc[networking] result<ip_addr, resolve_error>

    pub fn get_network_interfaces()
        -> proc[networking] list<network_interface>

    pub fn create_network_buffer(size: int)
        -> proc[networking, mem(normal)] buf(R, normal, byte, size)

    pub fn optimize_buffer_for_nic(buffer: buf(R, normal, T, size), nic_device: device_ref)
        -> proc[networking] buf(R, normal, T, size)
}
```

### 18.5 Networking Integration with Chip Features

Silica's networking modules leverage AArch64 chip capabilities for optimal performance:

#### 18.5.1 NUMA-Aware Networking
Network buffers and processing can be NUMA-optimized:
```silica
-- Place network buffers close to NIC for minimal latency
nic_numa_node <- get_nic_numa_node(network_interface)
region <- alloc_region_on_numa_node(nic_numa_node, normal)
rx_buffers <- alloc_buf(region, buffer_count)
```

#### 18.5.2 CPU Affinity for Network Processing
Network actors can be pinned to optimal cores:
```silica
-- Pin network processing to efficiency cores (continuous I/O)
network_actor <- spawn_actor(network_state, packet_processor)
pin_actor_to_efficiency_core(network_actor)

-- Pin application logic to performance cores (bursty processing)
app_actor <- spawn_actor(app_state, app_logic)
pin_actor_to_performance_core(app_actor)
```

#### 18.5.3 SIMD-Accelerated Packet Processing
When SVE is available, packet processing is automatically vectorized:
```silica
use module net.packet
use module arch.sve  -- Enables SIMD acceleration

-- Automatic vectorization for batch packet processing
results <- net.packet.process_packet_batch(packet_batch)
```

#### 18.5.4 Hardware-Assisted Security
Network buffers can use memory tagging for security:
```silica
use module arch.mte

-- Tagged network buffers prevent overflow exploits
secure_buffer <- net.utils.create_secure_network_buffer(size)
```

## 20. Architecture-Specific Modules

### 19.1 SVE (Scalable Vector Extension)

#### 19.1.1 Vector Types
```
module arch.sve {

    pub type Vec<T> where T: sve_supported_type
    pub type Pred    -- predicate mask for conditional operations

    -- Supported element types: int8, int16, int32, int64, float16, float32, float64
}
```

#### 19.1.2 Vector Operations
```
module arch.sve {

    pub fn load_vector<T>(ptr: *T, pred: option<Pred>) -> Vec<T>
    pub fn store_vector<T>(ptr: *T, vec: Vec<T>, pred: option<Pred>) -> unit
    pub fn add_vectors<T>(a: Vec<T>, b: Vec<T>) -> Vec<T>
    pub fn mul_vectors<T>(a: Vec<T>, b: Vec<T>) -> Vec<T>

    -- Predicate operations
    pub fn create_pred_true(len: int) -> Pred
    pub fn create_pred_from_mask(mask: Vec<bool>) -> Pred
    pub fn test_any_true(pred: Pred) -> bool
    pub fn test_all_true(pred: Pred) -> bool
}
```

#### 19.1.3 Scalable Width
SVE vectors automatically scale with hardware vector length:

```
use module arch.sve

fn vector_add(a: *int, b: *int, len: int) -> proc[] unit {
    pred = sve.create_pred_true(len)
    va = sve.load_vector(a, Some(pred))
    vb = sve.load_vector(b, Some(pred))
    result = sve.add_vectors(va, vb)
    sve.store_vector(a, result, Some(pred))  -- a = a + b
}
```

### 19.2 NEON (Fixed-Width SIMD)

#### 19.2.1 Fixed-Width Vectors
```
module arch.neon {

    pub type Vec128<T>  -- 128-bit vectors
    pub type Vec64<T>   -- 64-bit vectors (limited use)

    -- Supported for: int8, int16, int32, int64, float32
}
```

#### 19.2.2 NEON Operations
```
module arch.neon {

    pub fn load_128<T>(ptr: *T) -> Vec128<T>
    pub fn store_128<T>(ptr: *T, vec: Vec128<T>) -> unit
    pub fn add_128<T>(a: Vec128<T>, b: Vec128<T>) -> Vec128<T>
    pub fn mul_128<T>(a: Vec128<T>, b: Vec128<T>) -> Vec128<T>

    -- Lane access
    pub fn extract_lane_128<T>(vec: Vec128<T>, lane: int) -> T
    pub fn insert_lane_128<T>(vec: Vec128<T>, lane: int, value: T) -> Vec128<T>
}
```

### 19.3 Memory Tagging Extensions (MTE)

#### 19.3.1 Tagged Pointers
```
module arch.mte {

    pub type tagged_ptr<T>  -- pointer with memory tag

    pub fn alloc_tagged<T>(size: int) -> proc[mem(normal)] tagged_ptr<T>
    pub fn free_tagged<T>(ptr: tagged_ptr<T>) -> proc[mem(normal)] unit

    -- Tag operations
    pub fn set_tag<T>(ptr: tagged_ptr<T>, tag: int) -> tagged_ptr<T>
    pub fn get_tag<T>(ptr: tagged_ptr<T>) -> int
    pub fn check_tag<T>(ptr: tagged_ptr<T>) -> bool
}
```

### 19.4 Pointer Authentication (PAC)

#### 19.4.1 Authenticated Pointers
```
module arch.pac {

    pub type pac_ptr<T>  -- pointer authentication code

    pub fn sign_ptr<T>(ptr: *T, context: int) -> pac_ptr<T>
    pub fn auth_ptr<T>(ptr: pac_ptr<T>, context: int) -> *T
    pub fn auth_fail<T>(ptr: pac_ptr<T>, context: int) -> bool
}
```

### 19.5 Apple Silicon Extensions

#### 19.5.1 AMX (Apple Matrix Engine)
```
module arch.apple.amx {

    pub type Matrix<T>  -- AMX matrix registers

    pub fn load_matrix<T>(data: *T, rows: int, cols: int) -> Matrix<T>
    pub fn store_matrix<T>(matrix: Matrix<T>, data: *T) -> unit
    pub fn matmul<T>(a: Matrix<T>, b: Matrix<T>) -> Matrix<T>
}
```

## 21. Built-in Functions

### 20.1 Memory Management
```
alloc_region(space: memory_space) -> proc[mem(space)] region(any, space)
alloc_ref(region, initial_value) -> proc[mem(space)] ref(region, space, T)
alloc_buf(region, capacity) -> proc[mem(space)] buf(region, space, T, capacity)
alloc_atomic(region, initial_value) -> proc[mem(space), atomic] atomic_ref(region, space, T)
```

### 20.2 Reference Operations
```
read_ref(reference) -> proc[mem(space)] T
write_ref(reference, value) -> proc[mem(space)] unit
```

### 20.3 Buffer Operations
```
read_buf(buffer, index) -> proc[mem(space)] T
write_buf(buffer, index, value) -> proc[mem(space)] unit
buffer_length(buffer) -> int
buffer_capacity(buffer) -> int
```

### 20.4 Actor Operations
```
spawn(initial_state, behavior) -> proc[concurrency] actor_ref<Msg>
send(actor, message) -> proc[concurrency] unit
recv() -> proc[mailbox<Msg>, concurrency] Msg          -- Runtime internal
self() -> proc[mailbox<Msg>, concurrency] actor_ref<Msg>
```

**Note**: `recv()` is a runtime internal function and cannot be called directly from user code.

### 20.4.1 CPU Affinity Operations

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
    latency: int  -- relative latency to this NUMA node
}

type cache_info = {
    levels: list<cache_level>
}

type cache_level = {
    level: int,  -- L1, L2, L3
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
    core_type: core_type,  -- efficiency | performance
    capabilities: list<string>,
    frequency_mhz: int
}

type thermal_info = {
    temperatures: list<int>,  -- per-core temperatures
    throttling_active: bool,
    cooling_policy: cooling_policy
}

type power_info = {
    battery_level: option<int>,
    power_source: power_source,  -- battery | ac_power
    power_policy: power_policy
}

type priority_level = low | normal | high | realtime
type thermal_policy = conservative | balanced | performance
type power_policy = power_saver | balanced | performance
type cooling_policy = passive | active
type power_source = battery | ac_power
type core_type = efficiency | performance

type affinity_error =
    InvalidCoreId
  | CoreUnavailable
  | ThermalLimitExceeded
  | PermissionDenied
  | ResourceExhausted
```

**CPU Affinity Functions:**
```
-- CPU topology and status discovery
get_cpu_topology() -> proc[] cpu_topology
get_efficiency_cores() -> proc[] list<int>
get_performance_cores() -> proc[] list<int>
get_core_capabilities(core_id: int) -> proc[] core_info
get_thermal_status() -> proc[] thermal_info
get_power_status() -> proc[] power_info

-- Actor pinning operations
pin_actor_to_core(actor: actor_ref<any>, core_id: int)
    -> proc[cpu_affinity] result<unit, affinity_error>
pin_actor_to_efficiency_core(actor: actor_ref<any>)
    -> proc[cpu_affinity] result<unit, affinity_error>
pin_actor_to_performance_core(actor: actor_ref<any>)
    -> proc[cpu_affinity] result<unit, affinity_error>
pin_actor_realtime(actor: actor_ref<any>, priority: int)
    -> proc[cpu_affinity] result<unit, affinity_error>
unpin_actor(actor: actor_ref<any>) -> proc[cpu_affinity] unit

-- Advanced scheduling hints
set_actor_priority(actor: actor_ref<any>, priority: priority_level)
    -> proc[cpu_affinity] unit
set_actor_thermal_policy(actor: actor_ref<any>, policy: thermal_policy)
    -> proc[cpu_affinity] unit
set_actor_power_policy(actor: actor_ref<any>, policy: power_policy)
    -> proc[cpu_affinity] unit
```

### 20.5 Atomic Operations
```
atomic_load(ref, order) -> proc[mem(space), atomic] T
atomic_store(ref, value, order) -> proc[mem(space), atomic] unit
atomic_fetch_add(ref, delta, order) -> proc[mem(space), atomic] T
atomic_compare_exchange(ref, expected, new_val, order)
    -> proc[mem(space), atomic] {ok, T} | {fail, T}
```

### 20.6 Type Operations
```
size_of<T>() -> int                    -- size in bytes
align_of<T>() -> int                   -- alignment requirement
type_name<T>() -> string               -- type name as string
```

### 20.7 Runtime Operations
```
current_time() -> proc[] int           -- milliseconds since epoch
random_int(min: int, max: int) -> proc[] int
hash<T>(value: T) -> int               -- stable hash function
```

### 20.8 String Operations
```
string_length(s: string) -> int
string_concat(s1: string, s2: string) -> string
string_slice(s: string, start: int, end: int) -> string
string_to_int(s: string) -> option<int>
int_to_string(n: int) -> string
```

### 20.9 Control Flow
```
panic(message: string) -> proc[] !          -- terminate with error
assert(condition: bool, message: string) -> proc[] unit
unreachable() -> proc[] !                   -- mark unreachable code
```

## 22. Runtime System

### 21.1 Execution Environment

#### 21.1.1 Process Scheduler
The runtime provides a scheduler for process execution:

- **Fair Scheduling**: Processes are scheduled fairly across available cores
- **Preemptive**: Long-running processes can be preempted
- **Priority Support**: Optional priority hints for process scheduling
- **Load Balancing**: Automatic distribution across CPU cores

#### 21.1.2 Actor Runtime
Actors are managed by the runtime:

- **Mailbox Management**: Each actor has a dedicated message queue
- **Message Delivery**: Asynchronous message delivery with ordering guarantees
- **Failure Isolation**: Actor failures don't affect the runtime or other actors
- **Resource Limits**: Optional memory and message queue limits per actor

#### 21.1.2 CPU Scheduling and Affinity
The runtime provides intelligent CPU scheduling with optional affinity controls:

- **NUMA-Aware Scheduling**: Automatic scheduling considers memory locality to minimize cross-NUMA communication latency
- **Optional CPU Pinning**: Developers can optionally pin actors to specific cores or core types when needed
- **Core Type Awareness**: Distinguishes between efficiency cores (power-optimized) and performance cores (speed-optimized)
- **Load Balancing**: Automatic distribution across available cores with affinity constraints
- **Thermal Management**: Runtime monitors thermal conditions and migrates actors to prevent overheating while respecting affinity settings
- **Real-Time Scheduling**: Optional real-time priority scheduling for latency-critical actors with CPU affinity guarantees
- **Power Management**: Automatic migration between core types based on battery level and power constraints

#### 21.1.3 Memory Manager
Region-based memory management:

- **Region Tracking**: Runtime tracks region lifetimes and ownership
- **Garbage-Free**: No garbage collection - explicit region deallocation
- **Safety Checks**: Bounds checking and region isolation enforcement
- **Optimization**: Region coalescing and memory layout optimization

### 21.2 Capability System

#### 21.2.1 Effect Capabilities
Runtime enforces effect capabilities:

- **Capability Tokens**: Processes carry capability tokens for allowed effects
- **Stack Inspection**: Runtime checks capabilities on effectful operations
- **Dynamic Checking**: Effect violations caught at runtime with clear errors
- **Performance**: Capability checks optimized for common cases

#### 21.2.2 Memory Capabilities
Memory access requires appropriate capabilities:

```
Memory Spaces:
- normal: General-purpose memory allocation
- atomic: Memory for atomic operations
- device: Memory-mapped device access
```

#### 21.2.3 Concurrency Capabilities
Concurrency operations require capabilities:

- `concurrency`: Actor spawning and management
- `mailbox`: Message send/receive operations
- `atomic`: Atomic memory operations
- `cpu_affinity`: CPU pinning and affinity controls
- `networking`: Network device access and communication

### 21.3 Error Handling and Recovery

#### 21.3.1 Runtime Errors
Runtime catches and reports errors:

- **Effect Violations**: Missing capability for operation
- **Memory Errors**: Bounds violations, invalid references
- **Type Errors**: Pattern match failures, invalid operations
- **Resource Exhaustion**: Out of memory, too many actors

#### 21.3.2 Error Propagation
Errors propagate through the process system:

```
fn safe_divide(x: int, y: int) -> proc[] result<int, string> {
    if y == 0 {
        return Error("division by zero")
    }
    return Ok(x / y)
}

-- Usage
do
    result <- safe_divide(10, 0)
    case result of
        Ok(value) -> print(value)
        Error(msg) -> print("Error: " + msg)
    end
end
```

## 23. Implementation Requirements

### 22.1 Compiler Obligations

#### 22.1.1 Type Safety
The compiler must ensure:

- **Type Soundness**: Well-typed programs don't go wrong
- **Effect Tracking**: All effects properly tracked and enforced
- **Memory Safety**: No dangling pointers or use-after-free
- **Concurrency Safety**: No data races or invalid message sends

#### 22.1.2 Optimization Requirements
The compiler should perform:

- **Effect Inference**: Automatic effect inference where possible
- **Region Optimization**: Minimize region allocation overhead
- **Actor Optimization**: Optimize message passing and actor scheduling
- **Vectorization**: Automatic vectorization for suitable loops (when safe)

#### 22.1.3 Code Generation
Generated code must:

- **Preserve Semantics**: Maintain operational semantics of the specification
- **Runtime Integration**: Properly interface with the Silica runtime
- **Platform Specific**: Generate optimal AArch64 code
- **Debuggable**: Support debugging with source-level information

### 22.2 Runtime Requirements

#### 22.2.1 Memory Management
The runtime must provide:

- **Region Allocation**: Safe region creation and deallocation
- **Reference Tracking**: Valid reference checking
- **Bounds Checking**: Array and buffer bounds validation
- **Atomic Operations**: Hardware-accelerated atomic primitives

#### 22.2.2 Concurrency Management
The runtime must support:

- **Actor Scheduling**: Fair and efficient actor execution
- **Message Delivery**: Reliable, ordered message passing
- **Synchronization**: Proper happens-before relationships
- **Failure Handling**: Graceful actor failure and cleanup

#### 22.2.3 Effect Enforcement
The runtime must enforce:

- **Capability Checking**: Effect capability validation
- **Isolation**: Actor and process isolation
- **Resource Limits**: Prevent resource exhaustion attacks
- **Performance**: Efficient capability checking

### 22.3 Conformance Testing

#### 22.3.1 Language Conformance
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

## 24. Error Handling

### 23.1 Error Types

#### 23.1.1 Runtime Errors
```
type runtime_error =
    EffectViolation(string)        -- missing capability
  | MemoryError(string)           -- memory corruption
  | BoundsError(string)           -- array bounds violation
  | TypeError(string)             -- type mismatch at runtime
  | ActorError(string)            -- actor failure
```

#### 23.1.2 Compilation Errors
```
type compile_error =
    TypeError(string)             -- static type error
  | EffectError(string)           -- effect mismatch
  | SyntaxError(string)           -- parse error
  | ModuleError(string)           -- module resolution error
```

### 23.2 Error Propagation

#### 23.2.1 Result-Based Error Handling
Functions return results to indicate success or failure:

```
fn parse_number(s: string) -> result<int, string> {
    -- attempt parsing, return Ok(value) or Error(message)
}

fn safe_operation() -> proc[] result<unit, runtime_error> {
    -- operations that might fail at runtime
}
```

#### 23.2.2 Panic Mechanism
For unrecoverable errors:

```
fn panic(message: string) -> ! {
    -- terminates the current process with error message
    -- '!' indicates this function never returns normally
}
```

#### 23.2.3 Actor Failure
Actors can fail and notify monitors:

```
type down_message = Down(actor_ref<any>, exit_reason)

fn failing_actor(msg: unit, state: unit) : proc[mailbox<unit>] unit {
    case msg of
        () -> panic("intentional failure")
    end
}
```

### 23.3 Error Recovery

#### 23.3.1 Supervision
Actors can supervise other actors:

```
fn supervisor(child_failure: down_message, state: supervisor_state)
    : proc[mailbox<down_message>] supervisor_state {

    case child_failure of
        Down(child_ref, reason) ->
            new_child <- spawn_actor(initial_state, child_behavior)
            -- restart failed child
            return updated_state
    end
}
```

#### 23.3.2 Try-Catch Style
Monadic error handling:

```
do
    x <- operation_that_might_fail()
    case x of
        Ok(result) -> continue_with(result)
        Error(err) -> handle_error(err)
    end
end
```

## 25. Platform Integration

### 24.1 AArch64 Architecture Support

#### 24.1.0 Hardware Architecture Integration

Silica's design fundamentally aligns with modern AArch64 chip architectures, providing optimizations that traditional languages cannot achieve.

**Cache Hierarchy Utilization:**
Silica's region-based memory model is designed to work optimally with modern CPU cache hierarchies (L1/L2/L3). Regions are allocated and managed to minimize cache thrashing and maximize cache locality:

- **Region Placement**: Runtime allocates regions to optimize for specific cache levels based on access patterns
- **Cache-Aware Scheduling**: Actor scheduling considers cache affinity to reduce cross-cache communication
- **Memory Layout**: Region-based allocation enables optimal memory layout for cache line utilization

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

#### 24.1.1 Performance Guarantees

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

#### 24.1.2 Instruction Selection

#### 24.1.1 Instruction Selection
The compiler generates optimal AArch64 code:

- **Load/Store**: Efficient memory access patterns
- **Atomic Operations**: Direct mapping to LDXR/STXR instructions
- **Vector Instructions**: SVE/NEON code generation
- **Branch Prediction**: Optimal branch patterns

#### 24.1.2 Memory Model Mapping
Direct mapping to AArch64 memory model:

```
Silica Ordering    AArch64 Instruction
relaxed            LDR/STR
acquire            LDAR
release            STLR
acq_rel            LDAXR/STLXR + barriers
seq_cst            DMB + LDAR/STLR
```

#### 24.1.3 Hardware Features
Utilization of AArch64 hardware features:

- **Large Address Space**: 64-bit addressing
- **Memory Tagging**: Hardware-assisted bounds checking (MTE)
- **Pointer Authentication**: Code pointer protection (PAC)
- **Scalable Vectors**: SVE for data parallelism

### 24.2 Operating System Integration

#### 24.2.1 Thread Management
Runtime integrates with OS threading:

- **Native Threads**: Uses OS threads for actor scheduling
- **CPU Affinity**: Optional thread pinning to CPU cores
- **Priority Mapping**: Maps Silica priorities to OS priorities
- **Signal Handling**: Proper signal handling for runtime control

#### 24.2.2 Memory Management
Integration with OS memory facilities:

- **Virtual Memory**: Uses OS virtual memory for regions
- **Huge Pages**: Support for large page sizes
- **Memory Locking**: Optional memory locking for real-time use
- **NUMA Awareness**: NUMA-aware memory allocation

### 24.3 Foreign Function Interface

#### 24.3.1 C Interoperability (Future)
While Silica doesn't target C interop by design, future extensions might include:

- **Safe Wrappers**: Type-safe C function wrappers
- **Memory Layout**: Compatible data layout with C
- **Calling Convention**: AArch64 calling convention compliance
- **Error Propagation**: C error code to Silica result conversion

#### 24.3.2 Runtime Linking
Dynamic loading of Silica modules:

- **Module Loading**: Runtime module loading and linking
- **Version Compatibility**: Module version checking
- **Dependency Resolution**: Automatic dependency loading
- **Security**: Module signature verification

### 24.4 Performance Characteristics

#### 24.4.1 Memory Efficiency
- **Zero GC Overhead**: No garbage collection pauses
- **Compact Representations**: Efficient type representations
- **Cache-Friendly**: Optimized data layout for cache performance
- **Memory Reuse**: Region-based memory reuse patterns

#### 24.4.2 Concurrency Performance
- **Lock-Free Operations**: Where possible, lock-free algorithms
- **Message Batchinge**: Optimized message passing
- **Actor Locality**: NUMA-aware actor scheduling
- **Vector Acceleration**: Hardware vector utilization

#### 24.4.3 Startup and Compilation
- **Fast Compilation**: Incremental compilation support
- **Small Binaries**: Efficient code generation
- **Quick Startup**: Minimal runtime initialization
- **Cross-Compilation**: Support for cross-compiling to AArch64

## 26. Compilation and Linking

### 25.1 Chip-Centric Compilation Strategy

Silica's compilation process is fundamentally different from traditional compilers, designed specifically for AArch64 chip architectures rather than retrofitted from x86-era tools.

#### 25.1.1 Region-Aware Code Generation

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

#### 25.1.2 Effect-Driven Optimization

**Capability-Aware Code Generation:**
The compiler uses effect information to generate hardware-optimized code:
- **Memory Barrier Insertion**: Automatic generation of appropriate barriers based on effect annotations
- **Atomic Operation Selection**: Chooses optimal AArch64 atomic instructions based on ordering requirements
- **Cache Coherency Hints**: Uses AArch64 cache maintenance instructions for effect-tracked operations

**Speculative Execution Control:**
Modern AArch64 chips have sophisticated speculative execution. Silica controls this through effects:
- **Speculation Barriers**: Automatic insertion for security-sensitive operations
- **Branch Prediction Hints**: Compiler hints based on actor behavior patterns
- **Memory Ordering**: Hardware memory model exploitation for actor communication

### 25.2 AArch64-Specific Optimizations

#### 25.2.1 Asymmetric Core Utilization

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

#### 25.2.2 Hardware-Assisted Concurrency

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

### 25.3 Linking and Module Resolution

#### 25.3.1 Module Linking Process

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

#### 25.3.2 Runtime Linking

**Dynamic Module Loading:**
Runtime module loading leverages AArch64 capabilities:
- **Just-In-Time Compilation**: Chip-specific optimizations at load time
- **Relocation Optimization**: Hardware-assisted address relocation
- **Security Validation**: PAC and MTE verification during loading

### 25.4 Performance Profiling and Adaptation

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

#### 25.4.2 Thermal and Power Awareness

**Chip Temperature Integration:**
Compiler adapts to thermal conditions:
- **Dynamic Voltage Scaling**: Code generation considers power states
- **Core Migration Planning**: Pre-planned migration paths for thermal events
- **Workload Balancing**: Distribute computation to avoid thermal hotspots

### 25.5 Compilation Phases

#### 25.5.1 Frontend: Language-Centric

1. **Module Resolution**: Locate and parse module dependencies using search paths
2. **Parsing**: UTF-8 aware, AI-assisted error recovery for all modules
3. **Import/Export Validation**: Verify module interfaces and resolve cross-module references
4. **Type Checking**: Effect-aware type system with hardware capability validation across all modules
5. **Region Analysis**: Lifetime and ownership verification across module boundaries
6. **Effect Inference**: Automatic effect annotation where possible

#### 25.5.2 Middle-End: Architecture-Aware

1. **Region Optimization**: NUMA and cache-aware memory layout
2. **Actor Optimization**: Message passing and scheduling optimization
3. **Effect Lowering**: Translation of high-level effects to hardware primitives
4. **Vectorization**: Automatic SVE code generation

#### 25.5.3 Backend: Chip-Native

1. **Instruction Selection**: AArch64-specific instruction choice
2. **Register Allocation**: Region-lifetime aware register assignment
3. **Code Layout**: Cache and TLB optimized code placement
4. **Link-Time Optimization**: Cross-module hardware-aware optimization

### 25.6 Build System Integration

#### 25.6.1 Hardware-Aware Build Configuration

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

## 27. Compiler Infrastructure

### 27.1 Optimization Passes

#### 27.1.1 Constant Folding
Compile-time evaluation of constant expressions:

```silica
fn compute() -> int {
    -- This becomes: return 42
    return 6 * 7
}
```

#### 27.1.2 Dead Code Elimination
Removal of unreachable code:

```silica
fn example(flag: bool) -> int {
    if flag {
        return 1
    } else {
        return 2
    }
    print("This is never reached")  -- Eliminated
}
```

#### 27.1.3 Function Inlining
Inlining of small functions for performance:

```silica
fn small_function(x: int) -> int { x + 1 }

fn caller() -> int {
    -- May be inlined to: return (42 + 1)
    return small_function(42)
}
```

### 27.2 Incremental Compilation

#### 27.2.1 Dependency Tracking
Only recompiling changed modules and their dependents:

```
main.silica ──┐
              ├── math.silica (changed) ──┐
              │                           ├── utils.silica (recompile)
              └── io.silica ────┘
```

#### 27.2.2 Module Caching
Persistent caching of compiled modules:

```silica
-- Module cache structure
.cache/
├── math.silica.bc       -- Compiled bytecode
├── math.silica.deps     -- Dependency information
└── math.silica.types    -- Type information
```

## 28. IDE & Developer Experience

### 28.1 Language Server

#### 28.1.1 Syntax Highlighting
Editor integration for Silica syntax highlighting:

```silica
keywords: fn, if, case, actor, effect, type
types: int, bool, string, actor_ref
effects: [mem(normal)], [concurrency]
```

#### 28.1.2 Go to Definition
Navigate to symbol definitions across modules:

```
fn main() {
    result <- add(1, 2)  -- Ctrl+click on 'add' jumps to math.silica
}
```

#### 28.1.3 Hover Information
Display type and documentation information:

```silica
fn add(x: int, y: int) -> int  -- Hover shows signature
```

#### 28.1.4 Auto-completion
Context-aware code completion:

```silica
use math_
      -- Suggests: math_utils
```

### 28.2 Debugging Support

#### 28.2.1 Source-Level Debugging
Step through Silica code with source line mapping:

```silica
fn factorial(n: int) -> int {
    if n <= 1 {           -- Breakpoint here
        return 1
    }
    return n * factorial(n - 1)
}
```

#### 28.2.2 Actor State Inspection
Examine actor internal state during debugging:

```silica
actor counter {
    state: int = 0

    increment() -> unit {
        state = state + 1  -- Inspect 'state' variable
    }
}
```

#### 28.2.3 Message Tracing
Trace message passing between actors:

```
Actor A sends: increment()
  ↓
Actor B receives: increment()
  ↓
Actor B state: 0 → 1
```

## 29. Advanced Type System

### 29.1 Traits

#### 29.1.1 Trait Definitions
Traits define interfaces that types can implement:

```silica
trait Display {
    fn to_string(self) -> string
}

trait Comparable {
    fn equals(self, other) -> bool
    fn less_than(self, other) -> bool
}
```

#### 29.1.2 Trait Inheritance and Sub-traits
Traits can be created using other traits as sub-traits. Sub-traits are independent traits that can be implemented separately, and traits can accumulate functionality by including multiple sub-traits.

**Sub-trait Inheritance:**
```silica
trait Printable {
    fn to_string(self) -> string
}

trait Debug includes Printable {
    fn debug_string(self) -> string
}

// Debug automatically includes Printable's methods
```

**Trait Composition through Accumulation:**
```silica
trait Serializable {
    fn serialize(self) -> bytes
}

trait Comparable {
    fn equals(self, other) -> bool
}

// A trait that accumulates multiple sub-traits
trait FullFeatured includes Printable, Serializable, Comparable {
    fn version(self) -> int
}

// FullFeatured includes methods from all three sub-traits:
// - to_string() from Printable
// - serialize() from Serializable
// - equals() from Comparable
// - version() (its own method)
```

**Important Notes:**
- Sub-traits remain independent and can be implemented separately
- When implementing a trait that includes others, you must implement all methods from the trait and its sub-traits
- Sub-traits can be used independently of their extending traits

#### 29.1.3 Implementation Requirements for Inherited Traits
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

#### 29.1.4 Actor System Traits
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

-- ActorMessage can be used as a type
cast(actor_ref, message: ActorMessage) : proc[concurrency] bool
```

#### 29.1.5 Trait Bounds
Functions can require trait implementations:

```silica
fn print_value(x) where Display {
    print(to_string(x))
}
```

#### 29.1.6 Implementing Inherited Traits
When implementing a trait that includes other traits, you must implement all methods from the trait and its sub-traits:

```silica
trait Printable {
    fn to_string(self) -> string
}

trait Debug includes Printable {
    fn debug_string(self) -> string
}

// Implementation must provide both methods
impl Debug for int {
    fn to_string(self) = int_to_string(self)        // From Printable sub-trait
    fn debug_string(self) = format("int: {}", self) // From Debug trait
}

// Sub-traits can be implemented independently
impl Printable for bool {
    fn to_string(self) = if self { "true" } else { "false" }
}
```

### 29.2 Trait Composition

#### 29.2.1 Multiple Trait Implementation
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

#### 29.2.2 Complex Trait Relationships
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
- Advanced effect system with composition and inheritance
- Compiler optimizations and incremental compilation
- IDE support with language server and debugging
- Advanced type system with traits

*Specification Complete: All Major Features Specified*

The Silica programming language specification now includes comprehensive coverage of:
- Core language features (Phases 1-7)
- Advanced language features and type system (Phase 8)
- Compiler infrastructure and tooling (Phase 9)

This specification serves as the definitive reference for Silica implementation and usage.
