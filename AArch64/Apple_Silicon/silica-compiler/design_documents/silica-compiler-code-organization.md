# Silica Compiler Code Organization

## Overview

This document defines the code organization structure for the Phase 2 Silica compiler written in Silica. The organization is designed to be both human-friendly and LLM-friendly, following Silica's design principles of explicit structure and clear naming conventions.

**Key Principles:**
- Feature-group modularity: Each language feature has dedicated files in lexer, parser, and codegen
- Clear separation: Lexer, parser, and codegen are separate directories
- Consistent naming: Files use descriptive names with grouping prefixes to avoid conflicts
- Hierarchical organization: Subdirectories group related features
- Incremental development: New features can be added without modifying existing files

## Directory Structure

```
silica-compiler/src/
├── lexer/
│   ├── lexer_core.silica              -- Core token types, source location, basic tokenization
│   ├── lexer_keywords.silica          -- Keyword recognition
│   ├── lexer_literals.silica           -- Integer, float, string, char, boolean, unit literals
│   ├── lexer_operators.silica         -- Operator recognition (arithmetic, comparison, logical)
│   └── lexer_comments.silica           -- Comment handling
│
├── parser/
│   ├── parser_core.silica              -- Parser state, basic parsing utilities, error types
│   ├── declarations/
│   │   ├── parser_declarations_functions.silica     -- Function declarations
│   │   ├── parser_declarations_types.silica         -- Type declarations, type aliases
│   │   ├── parser_declarations_structs.silica       -- Struct declarations
│   │   ├── parser_declarations_enums.silica         -- Enum declarations
│   │   ├── parser_declarations_traits.silica        -- Trait declarations
│   │   ├── parser_declarations_impls.silica         -- Impl declarations
│   │   ├── parser_declarations_modules.silica       -- Module, import, export declarations
│   │   └── parser_declarations_effects.silica       -- Effect declarations
│   ├── expressions/
│   │   ├── parser_expressions_literals.silica        -- Literal expressions
│   │   ├── parser_expressions_calls.silica          -- Function calls, qualified method calls
│   │   ├── parser_expressions_function_literals.silica  -- Function literal expressions
│   │   ├── parser_expressions_case.silica           -- Case expressions
│   │   ├── parser_expressions_if.silica             -- If expressions
│   │   ├── parser_expressions_do.silica             -- Do expressions
│   │   ├── parser_expressions_structs.silica        -- Struct literals, field access
│   │   ├── parser_expressions_tuples.silica         -- Tuple literals
│   │   ├── parser_expressions_constructors.silica  -- Constructor calls
│   │   ├── parser_expressions_casts.silica          -- Cast and as-type expressions
│   │   ├── parser_expressions_operators.silica      -- Binary and unary operators
│   │   └── parser_expressions_lists.silica          -- List literals and list operations
│   ├── patterns/
│   │   ├── parser_patterns_basic.silica            -- Basic patterns (identifiers, literals, wildcards)
│   │   ├── parser_patterns_structs.silica          -- Struct patterns
│   │   ├── parser_patterns_tuples.silica          -- Tuple patterns
│   │   ├── parser_patterns_enums.silica            -- Enum/constructor patterns
│   │   └── parser_patterns_lists.silica            -- List patterns (empty, cons)
│   ├── types/
│   │   ├── parser_types_primitives.silica          -- Primitive types
│   │   ├── parser_types_functions.silica           -- Function types
│   │   ├── parser_types_tuples.silica              -- Tuple types
│   │   ├── parser_types_records.silica              -- Record types
│   │   ├── parser_types_variants.silica            -- Variant/enum types
│   │   ├── parser_types_processes.silica           -- Process types
│   │   ├── parser_types_regions.silica             -- Region and reference types
│   │   ├── parser_types_actors.silica              -- Actor types
│   │   └── parser_types_lists.silica               -- List types
│   └── effects/
│       └── parser_effects_effects.silica           -- Effect parsing (device_io, concurrency, mem)
│
├── codegen/
│   ├── codegen_core.silica                         -- Code generation state, basic utilities
│   ├── declarations/
│   │   ├── codegen_declarations_functions.silica   -- Function code generation
│   │   ├── codegen_declarations_types.silica       -- Type code generation
│   │   ├── codegen_declarations_structs.silica     -- Struct code generation
│   │   ├── codegen_declarations_enums.silica       -- Enum code generation
│   │   ├── codegen_declarations_traits.silica      -- Trait method dispatch code generation
│   │   └── codegen_declarations_modules.silica     -- Module-level code generation
│   ├── expressions/
│   │   ├── codegen_expressions_literals.silica     -- Literal code generation
│   │   ├── codegen_expressions_calls.silica        -- Function call code generation
│   │   ├── codegen_expressions_function_literals.silica  -- Closure code generation
│   │   ├── codegen_expressions_case.silica         -- Case expression code generation
│   │   ├── codegen_expressions_if.silica           -- If expression code generation
│   │   ├── codegen_expressions_do.silica           -- Do expression code generation
│   │   ├── codegen_expressions_structs.silica       -- Struct literal and field access code generation
│   │   ├── codegen_expressions_tuples.silica       -- Tuple code generation
│   │   ├── codegen_expressions_constructors.silica -- Constructor call code generation
│   │   ├── codegen_expressions_casts.silica        -- Cast code generation
│   │   ├── codegen_expressions_operators.silica    -- Operator code generation
│   │   └── codegen_expressions_lists.silica        -- List operation code generation
│   ├── patterns/
│   │   └── codegen_patterns_matching.silica       -- Pattern matching code generation
│   ├── effects/
│   │   └── codegen_effects_effects.silica         -- Effect handling code generation
│   ├── actors/
│   │   ├── codegen_actors_spawn.silica            -- Actor spawn code generation
│   │   ├── codegen_actors_messaging.silica         -- Send/recv/cast code generation
│   │   └── codegen_actors_mailbox.silica           -- Mailbox code generation
│   └── memory/
│       ├── codegen_memory_regions.silica          -- Region allocation code generation
│       └── codegen_memory_references.silica       -- Reference operations code generation
│
├── ast/
│   ├── ast_core.silica                            -- Core AST types (Program, SourceLocation)
│   ├── ast_declarations.silica                    -- Declaration AST nodes
│   ├── ast_expressions.silica                     -- Expression AST nodes
│   ├── ast_patterns.silica                        -- Pattern AST nodes
│   ├── ast_types.silica                           -- Type AST nodes
│   └── ast_effects.silica                         -- Effect AST nodes
│
└── compiler.silica                                -- Main compiler entry point, pipeline orchestration
```

## Naming Convention

Files follow the pattern: `{phase}_{group}_{feature}.silica`

Where:
- `{phase}` is one of: `lexer`, `parser`, `codegen`, `ast`
- `{group}` is optional and represents a subdirectory grouping (e.g., `declarations`, `expressions`, `patterns`, `types`, `effects`, `actors`, `memory`)
- `{feature}` is the specific feature name (e.g., `functions`, `structs`, `case`, `lists`)

Examples:
- `lexer_core.silica` - Core lexer functionality
- `parser_declarations_functions.silica` - Function declaration parsing
- `parser_expressions_case.silica` - Case expression parsing
- `codegen_expressions_lists.silica` - List expression code generation
- `ast_declarations.silica` - Declaration AST nodes

## Module Structure

Each file defines a module following Silica's module naming conventions:

```silica
-- parser/expressions/parser_expressions_case.silica
module parser.expressions.case;

use ast_expressions;
use parser_core;
use parser_patterns_basic;

-- Parse case expression
fn parse_case_expression(
    parser: ref(R, normal, Parser)
) -> ResultCaseExprParseError proc[mem(normal)] {
    -- Implementation
}

-- Parse case branch
fn parse_case_branch(
    parser: ref(R, normal, Parser)
) -> ResultCaseBranchParseError proc[mem(normal)] {
    -- Implementation
}
```

## Feature Groups

### Lexer Features
- **Core**: Token types, source location, basic tokenization
- **Keywords**: Keyword recognition (fn, struct, trait, etc.)
- **Literals**: Integer, float, string, char, boolean, unit literals
- **Operators**: Arithmetic, comparison, logical operators
- **Comments**: Line and block comment handling

### Parser Declarations
- **Functions**: Function declarations with parameters, return types, effects
- **Types**: Type declarations and type aliases
- **Structs**: Struct declarations with fields
- **Enums**: Enum declarations with variants
- **Traits**: Trait declarations with method signatures
- **Impls**: Trait implementation declarations
- **Modules**: Module, import, and export declarations
- **Effects**: Effect declarations

### Parser Expressions
- **Literals**: Literal expression parsing
- **Calls**: Function calls and qualified method calls
- **Function Literals**: Anonymous function/closure parsing
- **Case**: Case expression parsing with pattern matching
- **If**: If expression parsing
- **Do**: Do expression parsing (process monad)
- **Structs**: Struct literal and field access parsing
- **Tuples**: Tuple literal parsing
- **Constructors**: Constructor call parsing
- **Casts**: Cast and as-type expression parsing
- **Operators**: Binary and unary operator parsing
- **Lists**: List literal and list operation parsing

### Parser Patterns
- **Basic**: Identifier, literal, and wildcard patterns
- **Structs**: Struct pattern matching
- **Tuples**: Tuple pattern matching
- **Enums**: Enum/constructor pattern matching
- **Lists**: List pattern matching (empty, cons)

### Parser Types
- **Primitives**: Primitive type parsing (int64, bool, etc.)
- **Functions**: Function type parsing
- **Tuples**: Tuple type parsing
- **Records**: Record type parsing
- **Variants**: Variant/enum type parsing
- **Processes**: Process type parsing with effects
- **Regions**: Region and reference type parsing
- **Actors**: Actor type parsing
- **Lists**: List type parsing

### Parser Effects
- **Effects**: Effect parsing (device_io, concurrency, mem(normal))

### Codegen Declarations
- **Functions**: Function code generation
- **Types**: Type code generation
- **Structs**: Struct code generation
- **Enums**: Enum code generation
- **Traits**: Trait method dispatch code generation
- **Modules**: Module-level code generation

### Codegen Expressions
- **Literals**: Literal code generation
- **Calls**: Function call code generation
- **Function Literals**: Closure code generation
- **Case**: Case expression code generation
- **If**: If expression code generation
- **Do**: Do expression code generation
- **Structs**: Struct literal and field access code generation
- **Tuples**: Tuple code generation
- **Constructors**: Constructor call code generation
- **Casts**: Cast code generation
- **Operators**: Operator code generation
- **Lists**: List operation code generation

### Codegen Patterns
- **Matching**: Pattern matching code generation (decision trees, jump tables)

### Codegen Effects
- **Effects**: Effect handling code generation

### Codegen Actors
- **Spawn**: Actor spawn code generation
- **Messaging**: Send/recv/cast code generation
- **Mailbox**: Mailbox code generation

### Codegen Memory
- **Regions**: Region allocation code generation
- **References**: Reference operations code generation

### AST Nodes
- **Core**: Program, SourceLocation, basic AST infrastructure
- **Declarations**: Declaration AST node types
- **Expressions**: Expression AST node types
- **Patterns**: Pattern AST node types
- **Types**: Type AST node types
- **Effects**: Effect AST node types

## Adding a New Feature

When adding a new language feature (e.g., "match expressions"):

1. **Add AST nodes**: Update `ast/ast_expressions.silica` to add `MatchExpr` variant
2. **Add lexer support**: Create `lexer/lexer_match_expressions.silica` (if new tokens needed)
3. **Add parser support**: Create `parser/expressions/parser_expressions_match.silica`
4. **Add codegen support**: Create `codegen/expressions/codegen_expressions_match.silica`
5. **Update core**: Update `parser/parser_core.silica` to wire in the new parser function

This keeps changes localized and makes the compiler easy to extend and maintain.

## Benefits

1. **Easy Navigation**: Developers and LLMs can quickly locate code for any feature
2. **Incremental Development**: Features can be added without modifying existing files
3. **LLM-Friendly**: Clear file names and structure make code easy to understand
4. **Human-Friendly**: Small, focused files are easier to read and maintain
5. **Testable**: Each feature can be tested independently
6. **Scalable**: New features follow the same pattern, maintaining consistency

## Relationship to Specification

This organization maps directly to the Silica language specification sections:

- **Section 2 (Lexical Structure)** → `lexer/` directory
- **Section 3 (Syntax)** → `parser/` directory
- **Section 8 (Type System)** → `parser/types/` and `ast/ast_types.silica`
- **Section 9 (Effect System)** → `parser/effects/` and `codegen/effects/`
- **Section 15 (Actor Model)** → `codegen/actors/`
- **Section 12 (Memory Model)** → `codegen/memory/`

Each feature group corresponds to a specific section or subsection of the specification, making it easy to verify completeness and correctness.
