# Silica Compiler Build Plan

## Overview

This document provides a comprehensive plan for implementing the complete Silica compiler, written in Silica itself. The plan includes both phases already implemented in the bootstrap compiler (for reimplementation and comparison) and the missing phases identified as gaps.

**Target**: Implement complete Silica compiler in Silica language
**Output**: Production-ready Silica compiler with native AArch64 backend
**Reference**: See specification-analysis-state.md, bootstrap-analysis-state.md, gap-analysis-state.md

**Implementation Strategy**: 
1. First implement bootstrap compiler phases in Silica (Phases 0-5) to match bootstrap behavior
2. Compare Silica compiler output with bootstrap compiler output for validation
3. Then implement missing phases (Phases 6-13) to achieve full specification compliance

**Phase Structure**:
- **Phases 0-5**: Bootstrap Compiler Phases (already implemented in Rust, reimplement in Silica)
  - Phase 0: Lexical Analysis ✅ Bootstrap
  - Phase 1: Parsing ✅ Bootstrap
  - Phase 2: Module Resolution ✅ Bootstrap
  - Phase 3: Type Checking ✅ Bootstrap
  - Phase 4: Effect Checking ✅ Bootstrap
  - Phase 5: Code Generation (LLVM) ✅ Bootstrap
- **Phases 6-13**: Missing Phases (not in bootstrap, implement in Silica)
  - Phase 6: Region Analysis ❌ Missing
  - Phase 7: Pattern Matching Exhaustiveness Enhancement ⚠️ Partial
  - Phase 8: Hardware Capability Validation ❌ Missing
  - Phase 9: Region Optimization ❌ Missing
  - Phase 10: Actor Optimization ❌ Missing
  - Phase 11: Effect Lowering ⚠️ Delegated to LLVM
  - Phase 12: Vectorization ❌ Missing
  - Phase 13: Native Backend Phases ⚠️ Delegated to LLVM

## Planning Structure

This plan uses a dual hierarchy:
- **Phase Hierarchy**: Phases → Sub-phases → Tasks → Subtasks
- **Component Hierarchy**: Components → Modules → Functions

Cross-references link phase tasks to component modules that implement them.

---

## Toolchain and Assembler Integration

The Silica compiler integrates with external toolchains for assembly, linking, and platform runtime integration. The compiler emits AArch64 assembly code that is compatible with the target platform's toolchain.

**Note**: Toolchain selection and configuration is handled by build systems (Makefiles, CMake, etc.). The compiler accepts toolchain configuration via command-line flags and generates assembly compatible with the specified toolchain.

### Toolchain Configuration Strategy

Build systems (Makefiles) detect and select toolchains based on the target platform, then pass toolchain configuration to the compiler:

**Apple Silicon (macOS arm64)**:
- **Assembler**: Apple-provided Clang integrated assembler
- **Linker**: Apple-provided Clang linker (`ld64`)
- **Runtime Integration**: Apple platform runtime libraries
- **Target Triple**: `arm64-apple-darwin` (or `aarch64-apple-darwin`)
- **Toolchain Driver**: `clang` (Apple Clang)
- **Build System**: Makefiles detect Apple Clang and configure compiler flags accordingly

**Non-Apple AArch64 Platforms**:
- **Assembler**: LLVM integrated assembler (`llvm-as`)
- **Linker**: LLVM linker (`lld`) or platform-specific linker
- **Runtime Integration**: Platform-specific runtime libraries
- **Target Triple**: `aarch64-unknown-linux-gnu`, `aarch64-unknown-linux-musl`, or platform-specific
- **Toolchain Driver**: `clang` (upstream LLVM)
- **Build System**: Makefiles detect LLVM toolchain and configure compiler flags accordingly

### Implementation Requirements

#### Task TC.1: Target Triple Parsing and Toolchain Configuration
**Component**: `toolchain` module

**Subtask TC.1.1**: Implement Target Triple Parsing
- Parse target triple format (`arch-vendor-os-abi`) from command-line `--target` flag
- Identify Apple Silicon targets (`*-apple-*`)
- Identify non-Apple AArch64 targets (`aarch64-*-*`)
- **Specification**: Section 1.4 (Target Platform), Section 1.5 (Compiler Interface), Section 27 (Compilation and Linking)
- **Note**: Build systems (Makefiles) pass target triple via `--target` flag
- **Silica Pattern**: Use target triple parsing:
  ```silica
  struct TargetTriple {
      arch: string,
      vendor: string,
      os: string,
      abi: OptionString,
  }
  
  fn parse_target_triple(triple: string) -> TargetTriple proc[mem(normal)]
  fn is_apple_target(triple: TargetTriple) -> bool proc[mem(normal)]
  ```

**Subtask TC.1.2**: Implement Toolchain Configuration Parsing
- Parse toolchain type from command-line `--toolchain` flag (e.g., `--toolchain=apple-clang` or `--toolchain=llvm`)
- Parse toolchain paths from command-line flags (e.g., `--clang-path`, `--assembler-path`, `--linker-path`)
- Validate toolchain configuration
- **Specification**: Section 1.5 (Compiler Interface), Section 27
- **Note**: Build systems (Makefiles) detect toolchains and pass configuration to compiler
- **Silica Pattern**: Use toolchain configuration parsing:
  ```silica
  enum Toolchain {
      AppleClang { clang_path: string },
      LLVM { clang_path: string, llvm_as_path: string, lld_path: string },
  }
  
  fn parse_toolchain_config(flags: CompilerFlags) -> ResultToolchainToolchainError proc[mem(normal)]
  ```

#### Task TC.2: Assembly Code Generation Compatibility
**Component**: `codegen` module

**Subtask TC.2.1**: Implement Apple Clang Assembly Compatibility
- Generate AArch64 assembly compatible with Apple Clang integrated assembler
- Use Apple-specific assembly directives and syntax
- Handle Apple-specific calling conventions
- **Specification**: Section 26 (Platform Integration), Section 27
- **Silica Pattern**: Use assembly generation with toolchain awareness:
  ```silica
  fn generate_assembly(
      ir: IR,
      toolchain: Toolchain
  ) -> string proc[mem(normal)]
  
  fn generate_apple_assembly(ir: IR) -> string proc[mem(normal)]
  ```

**Subtask TC.2.2**: Implement LLVM Assembly Compatibility
- Generate AArch64 assembly compatible with LLVM integrated assembler
- Use LLVM-specific assembly directives and syntax
- Handle standard AArch64 calling conventions
- **Specification**: Section 26, Section 27
- **Silica Pattern**: Use LLVM-compatible assembly generation:
  ```silica
  fn generate_llvm_assembly(ir: IR) -> string proc[mem(normal)]
  ```

#### Task TC.3: Toolchain Driver Invocation (Optional)
**Component**: `toolchain` module

**Note**: Build systems (Makefiles) typically handle toolchain driver invocation. The compiler may optionally support invoking toolchain drivers directly when `--assemble` or `--link` flags are provided.

**Subtask TC.3.1**: Implement Clang Driver Invocation (Optional)
- Invoke `clang` as the toolchain driver when `--assemble` or `--link` flags are provided
- Pass target triple via `-target` flag
- Use toolchain configuration from command-line flags
- **Specification**: Section 1.5 (Compiler Interface), Section 27
- **Note**: Typically handled by build systems; compiler provides optional support
- **Silica Pattern**: Use toolchain invocation:
  ```silica
  fn invoke_clang_driver(
      toolchain: Toolchain,
      target: TargetTriple,
      assembly_file: string,
      output_file: string,
      flags: ListString
  ) -> ResultUnitToolchainError proc[device_io, mem(normal)]
  ```

**Subtask TC.3.2**: Implement Assembly Invocation (Optional)
- Invoke Apple Clang integrated assembler for Apple targets when `--assemble` flag provided
- Invoke LLVM integrated assembler for non-Apple targets when `--assemble` flag provided
- Handle assembly errors and diagnostics
- **Specification**: Section 27
- **Note**: Typically handled by build systems; compiler provides optional support
- **Silica Pattern**: Use assembler invocation:
  ```silica
  fn invoke_assembler(
      toolchain: Toolchain,
      assembly_file: string,
      object_file: string
  ) -> ResultUnitToolchainError proc[device_io, mem(normal)]
  ```

**Subtask TC.3.3**: Implement Linker Invocation (Optional)
- Invoke Apple linker (`ld64`) for Apple targets when `--link` flag provided
- Invoke LLVM linker (`lld`) or platform linker for non-Apple targets when `--link` flag provided
- Configure linking flags and library paths from command-line flags
- **Specification**: Section 27, Section 28 (Linking)
- **Note**: Typically handled by build systems; compiler provides optional support
- **Silica Pattern**: Use linker invocation:
  ```silica
  fn invoke_linker(
      toolchain: Toolchain,
      object_files: ListString,
      output_file: string,
      libraries: ListString
  ) -> ResultUnitToolchainError proc[device_io, mem(normal)]
  ```

#### Task TC.4: Platform Runtime Integration
**Component**: `toolchain` module

**Subtask TC.4.1**: Implement Apple Runtime Integration
- Link against Apple platform runtime libraries
- Handle Apple-specific runtime requirements
- Configure Apple-specific linker flags
- **Specification**: Section 26, Section 28
- **Silica Pattern**: Use platform-specific runtime configuration:
  ```silica
  fn configure_apple_runtime(
      toolchain: Toolchain,
      target: TargetTriple
  ) -> ListString proc[mem(normal)]
  ```

**Subtask TC.4.2**: Implement Non-Apple Runtime Integration
- Link against platform-specific runtime libraries
- Handle platform-specific runtime requirements
- Configure platform-specific linker flags
- **Specification**: Section 26, Section 28
- **Silica Pattern**: Use platform runtime configuration:
  ```silica
  fn configure_platform_runtime(
      toolchain: Toolchain,
      target: TargetTriple
  ) -> ListString proc[mem(normal)]
  ```

### Build System Integration

**Build Systems (Makefiles)** handle:
- Toolchain detection (detecting available Apple Clang or LLVM toolchains)
- Toolchain selection based on target platform
- Passing toolchain configuration to compiler via command-line flags
- Invoking assembler and linker after compiler generates assembly
- Managing toolchain-specific flags and library paths

**Compiler** handles:
- Parsing toolchain configuration from command-line flags
- Generating assembly compatible with specified toolchain
- Optionally invoking toolchain drivers when `--assemble` or `--link` flags provided

### Toolchain Component Integration

**Component**: `toolchain` module
**Implements**: Toolchain configuration parsing, assembly generation compatibility, and optional toolchain driver invocation
**Specification**: Section 1.4 (Target Platform), Section 1.5 (Compiler Interface), Section 26 (Platform Integration), Section 27 (Compilation and Linking)

#### Module TC.1: `toolchain`
**Functions**:
- `parse_target_triple(triple: string) -> TargetTriple proc[mem(normal)]`
- `is_apple_target(triple: TargetTriple) -> bool proc[mem(normal)]`
- `select_toolchain(triple: TargetTriple) -> ResultToolchainToolchainError proc[device_io, mem(normal)]`
- `generate_assembly(ir: IR, toolchain: Toolchain) -> string proc[mem(normal)]`
- `generate_apple_assembly(ir: IR) -> string proc[mem(normal)]`
- `generate_llvm_assembly(ir: IR) -> string proc[mem(normal)]`
- `invoke_clang_driver(toolchain: Toolchain, target: TargetTriple, assembly_file: string, output_file: string, flags: ListString) -> ResultUnitToolchainError proc[device_io, mem(normal)]`
- `invoke_assembler(toolchain: Toolchain, assembly_file: string, object_file: string) -> ResultUnitToolchainError proc[device_io, mem(normal)]`
- `invoke_linker(toolchain: Toolchain, object_files: ListString, output_file: string, libraries: ListString) -> ResultUnitToolchainError proc[device_io, mem(normal)]`
- `configure_apple_runtime(toolchain: Toolchain, target: TargetTriple) -> ListString proc[mem(normal)]`
- `configure_platform_runtime(toolchain: Toolchain, target: TargetTriple) -> ListString proc[mem(normal)]`

### Cross-References

**Toolchain Integration ↔ Phase 13 (Native Backend Phases)**:
- **Task TC.2.1** → **Phase 13.1** (Instruction Selection Phase) - Assembly generation
- **Task TC.3.1** → **Phase 13.4** (Link-Time Optimization Phase) - Toolchain driver invocation
- **Task TC.4.1** → **Phase 13.4** (Link-Time Optimization Phase) - Runtime integration

**Toolchain Integration ↔ Phase 5 (Code Generation Phase)**:
- **Task TC.2.1** → **Phase 5.4** (LLVM Output Generation) - Assembly compatibility for LLVM backend

---

## Phase Hierarchy

### Phase 0: Lexical Analysis Phase
**Priority**: Foundation (Bootstrap Phase)
**Status**: ✅ Implemented in Bootstrap
**Specification References**: Section 2 (Lexical Structure), Section 27.5.1 (Frontend Phases)
**Dependencies**: None (first phase)
**Blocks**: Parsing Phase

#### Sub-phase 0.1: Token Recognition
**Component**: `lexer` module

**Task 0.1.1**: Implement UTF-8 Character Processing
- **Subtask 0.1.1.1**: Read UTF-8 encoded source files
- **Subtask 0.1.1.2**: Handle Unicode characters in strings and comments
- **Subtask 0.1.1.3**: Track character positions for error reporting
- **Specification**: Section 2.1
- **Silica Pattern**: Use string processing with explicit types:
  ```silica
  struct SourceFile {
      content: string,
      file_path: string,
  }
  
  fn read_source_file(path: string) -> SourceFile proc[device_io, mem(normal)]
  ```

**Task 0.1.2**: Implement Keyword Recognition
- **Subtask 0.1.2.1**: Define keyword set (33 keywords)
- **Subtask 0.1.2.2**: Match keywords during tokenization
- **Subtask 0.1.2.3**: Distinguish keywords from identifiers
- **Specification**: Section 2.2.1
- **Silica Pattern**: Use pattern matching for keyword recognition:
  ```silica
  fn recognize_keyword(lexeme: string) -> OptionTokenKind proc[mem(normal)]
  ```

**Task 0.1.3**: Implement Literal Parsing
- **Subtask 0.1.3.1**: Parse integer literals (decimal, hex, binary)
- **Subtask 0.1.3.2**: Parse floating-point literals
- **Subtask 0.1.3.3**: Parse string and character literals with escape sequences
- **Subtask 0.1.3.4**: Parse boolean and unit literals
- **Specification**: Section 2.2.3
- **Silica Pattern**: Use recursive parsing with explicit types:
  ```silica
  fn parse_integer_literal(chars: ListChar, start: int64) -> (Token, int64) proc[mem(normal)]
  fn parse_string_literal(chars: ListChar, start: int64) -> (Token, int64) proc[mem(normal)]
  ```

**Task 0.1.4**: Implement Operator and Punctuation Recognition
- **Subtask 0.1.4.1**: Recognize arithmetic operators (+, -, *, /, %)
- **Subtask 0.1.4.2**: Recognize comparison operators (==, !=, <, <=, >, >=)
- **Subtask 0.1.4.3**: Recognize logical operators (and, or, not)
- **Subtask 0.1.4.4**: Recognize assignment/binding operators (<-, =, ->)
- **Subtask 0.1.4.5**: Recognize punctuation (parentheses, braces, brackets, commas, semicolons)
- **Specification**: Section 2.2.4
- **Silica Pattern**: Use pattern matching for operator recognition:
  ```silica
  fn recognize_operator(chars: ListChar, start: int64) -> OptionToken proc[mem(normal)]
  ```

**Task 0.1.5**: Implement Source Location Tracking
- **Subtask 0.1.5.1**: Track line numbers
- **Subtask 0.1.5.2**: Track column numbers
- **Subtask 0.1.5.3**: Track byte offsets
- **Subtask 0.1.5.4**: Attach location to each token
- **Specification**: Section 1.6 (Error Messages)
- **Silica Pattern**: Use struct types for location tracking:
  ```silica
  struct SourceLocation {
      file: string,
      line: int64,
      column: int64,
      offset: int64,
  }
  
  struct Token {
      kind: TokenKind,
      lexeme: string,
      location: SourceLocation,
  }
  ```

**Task 0.1.6**: Implement Error Recovery
- **Subtask 0.1.6.1**: Handle invalid escape sequences
- **Subtask 0.1.6.2**: Handle unterminated strings
- **Subtask 0.1.6.3**: Handle invalid numeric literals
- **Subtask 0.1.6.4**: Generate structured error messages
- **Specification**: Section 2.5, Section 1.6
- **Silica Pattern**: Use error types with structured metadata:
  ```silica
  enum LexerError {
      InvalidEscapeSequence { location: SourceLocation, sequence: string },
      UnterminatedString { location: SourceLocation },
      InvalidNumericLiteral { location: SourceLocation, literal: string },
  }
  ```

#### Sub-phase 0.2: Token Stream Generation
**Component**: `lexer` module

**Task 0.2.1**: Implement Tokenization Loop
- **Subtask 0.2.1.1**: Iterate through source characters
- **Subtask 0.2.1.2**: Skip whitespace and comments
- **Subtask 0.2.1.3**: Generate token stream
- **Subtask 0.2.1.4**: Add EOF token at end
- **Specification**: Section 2
- **Silica Pattern**: Use recursive tokenization:
  ```silica
  fn tokenize(source: SourceFile) -> ResultListTokenLexerError proc[mem(normal)]
  ```

---

### Phase 1: Parsing Phase
**Priority**: Foundation (Bootstrap Phase)
**Status**: ✅ Implemented in Bootstrap
**Specification References**: Section 3 (Syntax), Section 27.5.1 (Frontend Phases)
**Dependencies**: Requires Lexical Analysis Phase completion
**Blocks**: Module Resolution Phase, Type Checking Phase

#### Sub-phase 1.1: AST Infrastructure
**Component**: `ast` module

**Task 1.1.1**: Define AST Node Types
- **Subtask 1.1.1.1**: Define Program type (top-level AST)
- **Subtask 1.1.1.2**: Define Declaration types (Function, Type, Effect, Import, Export, Struct, Enum, Trait, Impl, TypeAlias)
- **Subtask 1.1.1.3**: Define Expression types (Literal, Identifier, Call, Case, If, Do, Binary, Unary, etc.)
- **Subtask 1.1.1.4**: Define Statement types
- **Subtask 1.1.1.5**: Define Pattern types
- **Subtask 1.1.1.6**: Define Type types
- **Specification**: Section 3
- **Silica Pattern**: Use enum types for AST nodes:
  ```silica
  enum Declaration {
      Function(FunctionDecl),
      Type(TypeDecl),
      Effect(EffectDecl),
      Import(ImportDecl),
      Export(ExportDecl),
      Struct(StructDecl),
      Enum(EnumDecl),
      Trait(TraitDecl),
      Impl(ImplDecl),
      TypeAlias(TypeAliasDecl),
  }
  
  struct Program {
      declarations: ListDeclaration,
      location: SourceLocation,
  }
  ```

**Task 1.1.2**: Define Expression Types
- **Subtask 1.1.2.1**: Define literal expressions
- **Subtask 1.1.2.2**: Define identifier expressions
- **Subtask 1.1.2.3**: Define function call expressions
- **Subtask 1.1.2.4**: Define case expressions with patterns
- **Subtask 1.1.2.5**: Define if expressions
- **Subtask 1.1.2.6**: Define do expressions (process monad)
- **Subtask 1.1.2.7**: Define binary and unary expressions
- **Subtask 1.1.2.8**: Define memory operations (region, alloc_ref, read_ref, write_ref)
- **Subtask 1.1.2.9**: Define actor operations (spawn, send, recv, cast)
- **Specification**: Section 3.3
- **Silica Pattern**: Use enum types with explicit type annotations:
  ```silica
  enum Expression {
      Literal(Literal),
      Identifier(string),
      Call(CallExpr),
      Case(CaseExpr),
      If(IfExpr),
      Do(DoExpr),
      Binary(BinaryExpr),
      Unary(UnaryExpr),
      Region(RegionExpr),
      AllocRef(AllocRefExpr),
      ReadRef(ReadRefExpr),
      WriteRef(WriteRefExpr),
      Spawn(SpawnExpr),
      Send(SendExpr),
      Recv(RecvExpr),
      Cast(CastExpr),
  }
  ```

#### Sub-phase 1.2: Recursive Descent Parsing
**Component**: `parser` module

**Task 1.2.1**: Implement Declaration Parsing
- **Subtask 1.2.1.1**: Parse function declarations
- **Subtask 1.2.1.2**: Parse type declarations
- **Subtask 1.2.1.3**: Parse effect declarations
- **Subtask 1.2.1.4**: Parse import declarations
- **Subtask 1.2.1.5**: Parse export declarations
- **Subtask 1.2.1.6**: Parse struct declarations
- **Subtask 1.2.1.7**: Parse enum declarations
- **Subtask 1.2.1.8**: Parse trait declarations
- **Subtask 1.2.1.9**: Parse impl declarations
- **Subtask 1.2.1.10**: Parse type alias declarations
- **Specification**: Section 3.4
- **Silica Pattern**: Use recursive parsing functions:
  ```silica
  fn parse_declaration(parser: ref(R, normal, Parser)) -> ResultDeclarationParseError proc[mem(normal)]
  fn parse_function_declaration(parser: ref(R, normal, Parser)) -> ResultFunctionDeclParseError proc[mem(normal)]
  ```

**Task 1.2.2**: Implement Expression Parsing
- **Subtask 1.2.2.1**: Parse literals
- **Subtask 1.2.2.2**: Parse identifiers
- **Subtask 1.2.2.3**: Parse function calls
- **Subtask 1.2.2.4**: Parse case expressions with patterns
- **Subtask 1.2.2.5**: Parse if expressions
- **Subtask 1.2.2.6**: Parse do expressions
- **Subtask 1.2.2.7**: Parse binary expressions (with precedence)
- **Subtask 1.2.2.8**: Parse unary expressions
- **Subtask 1.2.2.9**: Parse memory operations
- **Subtask 1.2.2.10**: Parse actor operations
- **Specification**: Section 3.3
- **Silica Pattern**: Use precedence-based parsing:
  ```silica
  fn parse_expression(parser: ref(R, normal, Parser)) -> ResultExpressionParseError proc[mem(normal)]
  fn parse_expression_with_precedence(parser: ref(R, normal, Parser), min_precedence: int64) -> ResultExpressionParseError proc[mem(normal)]
  ```

**Task 1.2.3**: Implement Pattern Parsing
- **Subtask 1.2.3.1**: Parse literal patterns
- **Subtask 1.2.3.2**: Parse identifier patterns
- **Subtask 1.2.3.3**: Parse wildcard patterns (with type annotations)
- **Subtask 1.2.3.4**: Parse tuple patterns
- **Subtask 1.2.3.5**: Parse struct patterns
- **Subtask 1.2.3.6**: Parse enum variant patterns
- **Specification**: Section 3.5
- **Silica Pattern**: Use pattern matching for pattern parsing:
  ```silica
  fn parse_pattern(parser: ref(R, normal, Parser)) -> ResultPatternParseError proc[mem(normal)]
  ```

**Task 1.2.4**: Implement Type Parsing
- **Subtask 1.2.4.1**: Parse primitive types
- **Subtask 1.2.4.2**: Parse named types
- **Subtask 1.2.4.3**: Parse function types
- **Subtask 1.2.4.4**: Parse tuple types
- **Subtask 1.2.4.5**: Parse record types
- **Subtask 1.2.4.6**: Parse region types
- **Subtask 1.2.4.7**: Parse reference types
- **Subtask 1.2.4.8**: Parse process types with effects
- **Specification**: Section 3.7
- **Silica Pattern**: Use recursive type parsing:
  ```silica
  fn parse_type(parser: ref(R, normal, Parser)) -> ResultTypeParseError proc[mem(normal)]
  ```

**Task 1.2.5**: Implement Error Recovery
- **Subtask 1.2.5.1**: Handle syntax errors gracefully
- **Subtask 1.2.5.2**: Generate structured parse errors
- **Subtask 1.2.5.3**: Provide error suggestions
- **Specification**: Section 1.6, Section 27.5.1 (AI-assisted error recovery)
- **Silica Pattern**: Use structured error types:
  ```silica
  enum ParseError {
      UnexpectedToken { location: SourceLocation, expected: ListTokenKind, found: TokenKind },
      MissingToken { location: SourceLocation, expected: TokenKind },
      SyntaxError { location: SourceLocation, message: string },
  }
  ```

---

### Phase 2: Module Resolution Phase
**Priority**: Foundation (Bootstrap Phase)
**Status**: ✅ Implemented in Bootstrap
**Specification References**: Section 19 (Module System), Section 1.5 (Compiler Interface), Section 27.5.1 (Frontend Phases)
**Dependencies**: Requires Parsing Phase completion
**Blocks**: Type Checking Phase

#### Sub-phase 2.1: Module Discovery
**Component**: `module_resolver` module

**Task 2.1.1**: Implement Filename-Based Module Naming
- **Subtask 2.1.1.1**: Extract module name from filename (remove .silica extension)
- **Subtask 2.1.1.2**: Validate module name follows identifier rules
- **Subtask 2.1.1.3**: Handle module name conflicts
- **Specification**: Section 19.1
- **Silica Pattern**: Use string processing:
  ```silica
  fn extract_module_name(file_path: string) -> ResultStringModuleError proc[mem(normal)]
  ```

**Task 2.1.2**: Implement Module Search Path Management
- **Subtask 2.1.2.1**: Parse --search-path and -I command-line options
- **Subtask 2.1.2.2**: Maintain search path list
- **Subtask 2.1.2.3**: Default to current directory
- **Subtask 2.1.2.4**: Search paths in order
- **Specification**: Section 1.5.2
- **Silica Pattern**: Use list of search paths:
  ```silica
  struct ModuleResolver {
      search_paths: ListString,
      loaded_modules: MapModuleNameLoadedModule,
  }
  
  fn new_module_resolver(search_paths: ListString) -> ModuleResolver proc[mem(normal)]
  ```

#### Sub-phase 2.2: Module Loading and Caching
**Component**: `module_resolver` module

**Task 2.2.1**: Implement Module File Finding
- **Subtask 2.2.1.1**: Search for module file in search paths
- **Subtask 2.2.1.2**: Verify file exists and is readable
- **Subtask 2.2.1.3**: Return file path or error
- **Specification**: Section 19.4.1
- **Silica Pattern**: Use file system operations:
  ```silica
  fn find_module_file(resolver: ref(R, normal, ModuleResolver), module_name: string) -> ResultStringModuleError proc[device_io, mem(normal)]
  ```

**Task 2.2.2**: Implement Recursive Module Loading
- **Subtask 2.2.2.1**: Load module file
- **Subtask 2.2.2.2**: Parse module AST
- **Subtask 2.2.2.3**: Extract import declarations
- **Subtask 2.2.2.4**: Recursively load imported modules
- **Subtask 2.2.2.5**: Detect circular dependencies
- **Specification**: Section 19.4.5
- **Silica Pattern**: Use recursive loading with cycle detection:
  ```silica
  fn load_module(resolver: ref(R, normal, ModuleResolver), module_name: string) -> ResultUnitModuleError proc[device_io, mem(normal)]
  ```

**Task 2.2.3**: Implement Module Caching
- **Subtask 2.2.3.1**: Cache loaded modules
- **Subtask 2.2.3.2**: Avoid reloading already-loaded modules
- **Subtask 2.2.3.3**: Invalidate cache on file changes
- **Specification**: Section 28.1.2
- **Silica Pattern**: Use map for caching:
  ```silica
  struct LoadedModule {
      name: string,
      path: string,
      ast: Program,
      exports: ListExportItem,
  }
  
  fn cache_module(resolver: ref(R, normal, ModuleResolver), module: LoadedModule) -> unit proc[mem(normal)]
  ```

#### Sub-phase 2.3: Dependency Graph Construction
**Component**: `module_resolver` module

**Task 2.3.1**: Implement Dependency Graph Building
- **Subtask 2.3.1.1**: Build graph from import declarations
- **Subtask 2.3.1.2**: Create edges from module to imported modules
- **Subtask 2.3.1.3**: Detect cycles in dependency graph
- **Specification**: Section 19.4.5
- **Silica Pattern**: Use graph data structures:
  ```silica
  struct DependencyGraph {
      nodes: ListModuleName,
      edges: ListDependencyEdge,
  }
  
  struct DependencyEdge {
      from: string,
      to: string,
  }
  
  fn build_dependency_graph(modules: ListLoadedModule) -> DependencyGraph proc[mem(normal)]
  fn detect_cycles(graph: DependencyGraph) -> ResultUnitCycleError proc[mem(normal)]
  ```

**Task 2.3.2**: Implement Topological Sorting
- **Subtask 2.3.2.1**: Sort modules in dependency order
- **Subtask 2.3.2.2**: Dependencies come before dependents
- **Subtask 2.3.2.3**: Handle parallel compilation opportunities
- **Specification**: Section 19.4.5
- **Silica Pattern**: Use topological sort algorithm:
  ```silica
  fn topological_sort(graph: DependencyGraph) -> ListModuleName proc[mem(normal)]
  ```

#### Sub-phase 2.4: Symbol Table Population
**Component**: `module_resolver` module

**Task 2.4.1**: Implement Export Extraction
- **Subtask 2.4.1.1**: Extract export declarations from AST
- **Subtask 2.4.1.2**: Validate exported symbols exist
- **Subtask 2.4.1.3**: Validate arities match function definitions
- **Specification**: Section 19.2
- **Silica Pattern**: Use export extraction:
  ```silica
  fn extract_exports(ast: Program) -> ListExportItem proc[mem(normal)]
  ```

**Task 2.4.2**: Implement Symbol Table Building
- **Subtask 2.4.2.1**: Create symbol table structure
- **Subtask 2.4.2.2**: Add module symbols to table
- **Subtask 2.4.2.3**: Support cross-module symbol lookup
- **Specification**: Section 19.3
- **Silica Pattern**: Use symbol table structure:
  ```silica
  struct SymbolTable {
      modules: MapModuleNameMapSymbolNameSymbolInfo,
  }
  
  struct SymbolInfo {
      name: string,
      arity: int64,
      module: string,
      type_: Type,
  }
  
  fn add_module_symbols(table: ref(R, normal, SymbolTable), module: LoadedModule) -> ResultUnitSymbolError proc[mem(normal)]
  fn lookup_symbol(table: ref(R, normal, SymbolTable), module_name: string, symbol_name: string) -> OptionSymbolInfo proc[mem(normal)]
  ```

---

### Phase 3: Type Checking Phase
**Priority**: Foundation (Bootstrap Phase)
**Status**: ✅ Implemented in Bootstrap
**Specification References**: Section 8 (Type System), Section 10 (Type Checking), Section 30 (Advanced Type System), Section 27.5.1 (Frontend Phases)
**Dependencies**: Requires Module Resolution Phase completion
**Blocks**: Effect Checking Phase, Region Analysis Phase

#### Sub-phase 3.1: Type Environment Management
**Component**: `type_checker` module

**Task 3.1.1**: Implement Type Environment Structure
- **Subtask 3.1.1.1**: Define type environment type
- **Subtask 3.1.1.2**: Support variable bindings
- **Subtask 3.1.1.3**: Support type variable bindings
- **Subtask 3.1.1.4**: Support nested scopes
- **Specification**: Section 10.1.1
- **Silica Pattern**: Use type environment with explicit types:
  ```silica
  struct TypeEnv {
      bindings: MapStringTypeScheme,
      constraints: ListConstraint,
  }
  
  struct TypeScheme {
      vars: ListTypeVar,
      type_: Type,
  }
  
  fn create_type_env() -> TypeEnv proc[mem(normal)]
  fn add_binding(env: ref(R, normal, TypeEnv), name: string, scheme: TypeScheme) -> unit proc[mem(normal)]
  fn lookup_binding(env: ref(R, normal, TypeEnv), name: string) -> OptionTypeScheme proc[mem(normal)]
  ```

**Task 3.1.2**: Implement Type Constraint Management
- **Subtask 3.1.2.1**: Create type constraints
- **Subtask 3.1.2.2**: Solve type constraints
- **Subtask 3.1.2.3**: Apply type substitutions
- **Specification**: Section 10.1.1
- **Silica Pattern**: Use constraint solving:
  ```silica
  struct Constraint {
      left: Type,
      right: Type,
  }
  
  fn add_constraint(env: ref(R, normal, TypeEnv), constraint: Constraint) -> unit proc[mem(normal)]
  fn solve_constraints(env: ref(R, normal, TypeEnv)) -> ResultSubstitutionTypeError proc[mem(normal)]
  ```

#### Sub-phase 3.2: Expression Type Checking
**Component**: `type_checker` module

**Task 3.2.1**: Implement Literal Type Checking
- **Subtask 3.2.1.1**: Check integer literals → int
- **Subtask 3.2.1.2**: Check boolean literals → bool
- **Subtask 3.2.1.3**: Check character literals → char
- **Subtask 3.2.1.4**: Check string literals → string
- **Subtask 3.2.1.5**: Check unit literal → unit
- **Specification**: Section 10.1.2
- **Silica Pattern**: Use pattern matching for literal types:
  ```silica
  fn check_literal_type(literal: Literal) -> Type proc[mem(normal)]
  ```

**Task 3.2.2**: Implement Variable Type Checking
- **Subtask 3.2.2.1**: Lookup variable in type environment
- **Subtask 3.2.2.2**: Verify variable is in scope
- **Subtask 3.2.2.3**: Return variable type
- **Specification**: Section 10.1.3
- **Silica Pattern**: Use environment lookup:
  ```silica
  fn check_variable_type(env: ref(R, normal, TypeEnv), name: string) -> ResultTypeTypeError proc[mem(normal)]
  ```

**Task 3.2.3**: Implement Function Application Type Checking
- **Subtask 3.2.3.1**: Check function type
- **Subtask 3.2.3.2**: Check argument types
- **Subtask 3.2.3.3**: Verify arity matches
- **Subtask 3.2.3.4**: Compute effect union
- **Subtask 3.2.3.5**: Return result type
- **Specification**: Section 10.1.4
- **Silica Pattern**: Use function type checking:
  ```silica
  fn check_function_call(
      env: ref(R, normal, TypeEnv),
      func_expr: Expression,
      args: ListExpression
  ) -> (Type, ListEffect) proc[mem(normal)]
  ```

**Task 3.2.4**: Implement Trait-Constrained Function Application
- **Subtask 3.2.4.1**: Check trait constraint on function parameter
- **Subtask 3.2.4.2**: Verify argument type implements trait
- **Subtask 3.2.4.3**: Lookup trait implementation
- **Subtask 3.2.4.4**: Resolve trait method calls
- **Specification**: Section 10.1.4.1
- **Silica Pattern**: Use trait checking:
  ```silica
  fn check_trait_constraint(
      env: ref(R, normal, TypeEnv),
      trait_name: string,
      type_: Type
  ) -> ResultUnitTypeError proc[mem(normal)]
  
  fn lookup_trait_implementation(
      trait_name: string,
      type_: Type,
      impls: ListTraitImpl
  ) -> OptionTraitImpl proc[mem(normal)]
  ```

#### Sub-phase 3.3: Declaration Type Checking
**Component**: `type_checker` module

**Task 3.3.1**: Implement Function Declaration Checking
- **Subtask 3.3.1.1**: Check function body type
- **Subtask 3.3.1.2**: Verify return type matches body type
- **Subtask 3.3.1.3**: Check parameter types
- **Subtask 3.3.1.4**: Check effect declarations
- **Specification**: Section 10.2.1
- **Silica Pattern**: Use function declaration checking:
  ```silica
  fn check_function_declaration(
      env: ref(R, normal, TypeEnv),
      decl: FunctionDecl
  ) -> ResultUnitTypeError proc[mem(normal)]
  ```

**Task 3.3.2**: Implement Type Declaration Checking
- **Subtask 3.3.2.1**: Check type definition well-formedness
- **Subtask 3.3.2.2**: Add type to environment
- **Subtask 3.3.2.3**: Check type aliases
- **Specification**: Section 10.2.2
- **Silica Pattern**: Use type declaration checking:
  ```silica
  fn check_type_declaration(
      env: ref(R, normal, TypeEnv),
      decl: TypeDecl
  ) -> ResultUnitTypeError proc[mem(normal)]
  ```

**Task 3.3.3**: Implement Trait Declaration Checking
- **Subtask 3.3.3.1**: Check trait method signatures
- **Subtask 3.3.3.2**: Check trait inheritance (includes)
- **Subtask 3.3.3.3**: Handle diamond inheritance
- **Subtask 3.3.3.4**: Add trait to environment
- **Specification**: Section 30.1
- **Silica Pattern**: Use trait checking:
  ```silica
  fn check_trait_declaration(
      env: ref(R, normal, TypeEnv),
      decl: TraitDecl
  ) -> ResultUnitTypeError proc[mem(normal)]
  ```

**Task 3.3.4**: Implement Trait Implementation Checking
- **Subtask 3.3.4.1**: Verify trait exists
- **Subtask 3.3.4.2**: Verify all trait methods implemented
- **Subtask 3.3.4.3**: Check method signatures match trait
- **Subtask 3.3.4.4**: Register implementation
- **Specification**: Section 30.1
- **Silica Pattern**: Use implementation checking:
  ```silica
  fn check_impl_declaration(
      env: ref(R, normal, TypeEnv),
      decl: ImplDecl
  ) -> ResultUnitTypeError proc[mem(normal)]
  ```

#### Sub-phase 3.4: Cross-Module Type Checking
**Component**: `type_checker` module

**Task 3.4.1**: Implement Global Type Environment Construction
- **Subtask 3.4.1.1**: Collect types from all modules
- **Subtask 3.4.1.2**: Build global type environment
- **Subtask 3.4.1.3**: Resolve cross-module type references
- **Specification**: Section 10, Section 19.4.5
- **Silica Pattern**: Use global environment:
  ```silica
  fn build_global_type_environment(modules: ListModule) -> TypeEnv proc[mem(normal)]
  ```

**Task 3.4.2**: Implement Cross-Module Trait Lookup
- **Subtask 3.4.2.1**: Lookup trait implementations across modules
- **Subtask 3.4.2.2**: Handle trait inheritance across modules
- **Subtask 3.4.2.3**: Verify trait consistency
- **Specification**: Section 30
- **Silica Pattern**: Use cross-module lookup:
  ```silica
  fn lookup_trait_across_modules(
      env: ref(R, normal, TypeEnv),
      trait_name: string,
      type_: Type
  ) -> OptionTraitImpl proc[mem(normal)]
  ```

**Task 3.4.3**: Implement Type Consistency Verification
- **Subtask 3.4.3.1**: Verify exported types match implementations
- **Subtask 3.4.3.2**: Verify imported types are compatible
- **Subtask 3.4.3.3**: Check cross-module type errors
- **Specification**: Section 10, Section 19
- **Silica Pattern**: Use consistency checking:
  ```silica
  fn verify_cross_module_types(env: ref(R, normal, TypeEnv)) -> ResultUnitTypeError proc[mem(normal)]
  ```

---

### Phase 4: Effect Checking Phase
**Priority**: Foundation (Bootstrap Phase)
**Status**: ✅ Implemented in Bootstrap
**Specification References**: Section 9 (Effect System), Section 27.5.1 (Frontend Phases)
**Dependencies**: Requires Type Checking Phase completion
**Blocks**: Effect Lowering Phase

#### Sub-phase 4.1: Effect Context Management
**Component**: `effect_checker` module

**Task 4.1.1**: Implement Effect Context Structure
- **Subtask 4.1.1.1**: Define effect context type
- **Subtask 4.1.1.2**: Track active effects
- **Subtask 4.1.1.3**: Track capability stack
- **Subtask 4.1.1.4**: Support effect variables
- **Specification**: Section 9
- **Silica Pattern**: Use effect context structure:
  ```silica
  struct EffectContext {
      active_effects: ListEffect,
      capability_stack: ListCapability,
      effect_variables: MapStringListEffect,
  }
  
  struct Capability {
      effect: Effect,
      location: SourceLocation,
  }
  
  fn create_effect_context() -> EffectContext proc[mem(normal)]
  ```

**Task 4.1.2**: Implement Capability Stack Management
- **Subtask 4.1.2.1**: Push capabilities onto stack
- **Subtask 4.1.2.2**: Pop capabilities from stack
- **Subtask 4.1.2.3**: Query active capabilities
- **Specification**: Section 9
- **Silica Pattern**: Use stack operations:
  ```silica
  fn push_capability(context: ref(R, normal, EffectContext), capability: Capability) -> unit proc[mem(normal)]
  fn pop_capability(context: ref(R, normal, EffectContext)) -> OptionCapability proc[mem(normal)]
  fn get_active_effects(context: ref(R, normal, EffectContext)) -> ListEffect proc[mem(normal)]
  ```

#### Sub-phase 4.2: Effect Declaration Validation
**Component**: `effect_checker` module

**Task 4.2.1**: Implement Effect Declaration Checking
- **Subtask 4.2.1.1**: Verify all effects are explicitly declared
- **Subtask 4.2.1.2**: Check effect syntax (proc[...])
- **Subtask 4.2.1.3**: Validate effect names
- **Subtask 4.2.1.4**: Reject effect inference (explicit requirement)
- **Specification**: Section 9.3.1
- **Silica Pattern**: Use effect validation:
  ```silica
  fn validate_effect_declaration(
      effects: ListEffect,
      function_body: ListStatement
  ) -> ResultUnitEffectError proc[mem(normal)]
  ```

**Task 4.2.2**: Implement Effect Propagation
- **Subtask 4.2.2.1**: Collect effects from expressions
- **Subtask 4.2.2.2**: Compute effect union
- **Subtask 4.2.2.3**: Propagate effects through function calls
- **Subtask 4.2.2.4**: Propagate effects through bindings
- **Specification**: Section 9.4
- **Silica Pattern**: Use effect propagation:
  ```silica
  fn collect_expression_effects(expr: Expression) -> ListEffect proc[mem(normal)]
  fn union_effects(effects1: ListEffect, effects2: ListEffect) -> ListEffect proc[mem(normal)]
  fn propagate_effects(context: ref(R, normal, EffectContext), expr: Expression) -> ListEffect proc[mem(normal)]
  ```

#### Sub-phase 4.3: Effect Capability Enforcement
**Component**: `effect_checker` module

**Task 4.3.1**: Implement Capability Checking
- **Subtask 4.3.1.1**: Check expression effects against active capabilities
- **Subtask 4.3.1.2**: Verify effect subeffecting relationships
- **Subtask 4.3.1.3**: Generate capability errors
- **Specification**: Section 9.4.2
- **Silica Pattern**: Use capability checking:
  ```silica
  fn check_capability(
      context: ref(R, normal, EffectContext),
      required_effect: Effect
  ) -> ResultUnitEffectError proc[mem(normal)]
  
  fn is_subeffect(effect1: Effect, effect2: Effect) -> bool proc[mem(normal)]
  ```

**Task 4.3.2**: Implement Effect Error Reporting
- **Subtask 4.3.2.1**: Generate missing effect declaration errors
- **Subtask 4.3.2.2**: Generate effect mismatch errors
- **Subtask 4.3.2.3**: Generate capability violation errors
- **Subtask 4.3.2.4**: Provide effect suggestions
- **Specification**: Section 1.6, Section 9
- **Silica Pattern**: Use structured errors:
  ```silica
  enum EffectError {
      MissingEffectDeclaration { location: SourceLocation, required: ListEffect },
      EffectMismatch { location: SourceLocation, expected: ListEffect, actual: ListEffect },
      CapabilityViolation { location: SourceLocation, required: Effect, available: ListEffect },
  }
  ```

---

### Phase 5: Code Generation Phase (LLVM Backend)
**Priority**: Foundation (Bootstrap Phase)
**Status**: ✅ Implemented in Bootstrap (LLVM)
**Specification References**: Section 27 (Compilation and Linking), Section 27.5.1 (Frontend Phases)
**Dependencies**: Requires Type Checking Phase and Effect Checking Phase completion
**Blocks**: Native Backend Phases (will be replaced)

#### Sub-phase 5.1: LLVM IR Generation Infrastructure
**Component**: `codegen` module

**Task 5.1.1**: Implement LLVM Module and Context Setup
- **Subtask 5.1.1.1**: Create LLVM context
- **Subtask 5.1.1.2**: Create LLVM module
- **Subtask 5.1.1.3**: Initialize LLVM builder
- **Subtask 5.1.1.4**: Configure optimization level
- **Specification**: Section 27
- **Note**: This phase uses LLVM temporarily. Native backend (Phase 13) will replace this.

**Task 5.1.2**: Implement Type Mapping
- **Subtask 5.1.2.1**: Map Silica types to LLVM types
- **Subtask 5.1.2.2**: Map primitive types (int64 → i64, bool → i1, etc.)
- **Subtask 5.1.2.3**: Map struct types to LLVM struct types
- **Subtask 5.1.2.4**: Map function types to LLVM function types
- **Specification**: Section 27
- **Silica Pattern**: Use type mapping functions:
  ```silica
  fn map_silica_type_to_llvm(type_: Type) -> LLVMType proc[mem(normal)]
  ```

#### Sub-phase 5.2: Function Code Generation
**Component**: `codegen` module

**Task 5.2.1**: Implement Function Signature Generation
- **Subtask 5.2.1.1**: Generate LLVM function declarations
- **Subtask 5.2.1.2**: Map parameter types
- **Subtask 5.2.1.3**: Map return types
- **Subtask 5.2.1.4**: Handle function effects (for runtime)
- **Specification**: Section 27
- **Silica Pattern**: Use function generation:
  ```silica
  fn generate_function_signature(
      func: FunctionDecl,
      type_info: TypeInfo
  ) -> LLVMFunction proc[mem(normal)]
  ```

**Task 5.2.2**: Implement Function Body Generation
- **Subtask 5.2.2.1**: Generate code for function statements
- **Subtask 5.2.2.2**: Generate code for expressions
- **Subtask 5.2.2.3**: Handle variable scoping
- **Subtask 5.2.2.4**: Generate return statements
- **Specification**: Section 27
- **Silica Pattern**: Use recursive code generation:
  ```silica
  fn generate_function_body(
      func: FunctionDecl,
      llvm_func: LLVMFunction,
      type_info: TypeInfo
  ) -> ResultUnitCodegenError proc[mem(normal)]
  ```

#### Sub-phase 5.3: Expression Code Generation
**Component**: `codegen` module

**Task 5.3.1**: Implement Literal Code Generation
- **Subtask 5.3.1.1**: Generate LLVM constants for literals
- **Subtask 5.3.1.2**: Handle integer literals
- **Subtask 5.3.1.3**: Handle string literals (global constants)
- **Subtask 5.3.1.4**: Handle boolean and unit literals
- **Specification**: Section 27
- **Silica Pattern**: Use constant generation:
  ```silica
  fn generate_literal_code(literal: Literal, type_: Type) -> LLVMValue proc[mem(normal)]
  ```

**Task 5.3.2**: Implement Function Call Code Generation
- **Subtask 5.3.2.1**: Generate argument code
- **Subtask 5.3.2.2**: Generate function call instruction
- **Subtask 5.3.2.3**: Handle cross-module function calls
- **Subtask 5.3.2.4**: Handle trait method calls
- **Specification**: Section 27
- **Silica Pattern**: Use call generation:
  ```silica
  fn generate_call_code(
      call_expr: CallExpr,
      type_info: TypeInfo
  ) -> LLVMValue proc[mem(normal)]
  ```

**Task 5.3.3**: Implement Control Flow Code Generation
- **Subtask 5.3.3.1**: Generate code for case expressions
- **Subtask 5.3.3.2**: Generate code for if expressions
- **Subtask 5.3.3.3**: Generate code for do expressions
- **Subtask 5.3.3.4**: Handle pattern matching compilation
- **Specification**: Section 27
- **Silica Pattern**: Use control flow generation:
  ```silica
  fn generate_case_code(
      case_expr: CaseExpr,
      type_info: TypeInfo
  ) -> LLVMValue proc[mem(normal)]
  ```

#### Sub-phase 5.4: LLVM Output Generation
**Component**: `codegen` module

**Task 5.4.1**: Implement LLVM IR Text Output
- **Subtask 5.4.1.1**: Convert LLVM module to text IR
- **Subtask 5.4.1.2**: Write to output file
- **Subtask 5.4.1.3**: Verify LLVM IR validity
- **Specification**: Section 27
- **Note**: Temporary LLVM backend. Native backend will replace this.

**Task 5.4.2**: Implement LLVM Bitcode Output
- **Subtask 5.4.2.1**: Generate LLVM bitcode
- **Subtask 5.4.2.2**: Write bitcode to .bc file
- **Subtask 5.4.2.3**: Support LLVM linking
- **Specification**: Section 27
- **Note**: Temporary LLVM backend. Native backend will replace this.

---

---

### Phase 6: Region Analysis Phase
**Priority**: Critical (GAP-001)
**Status**: ❌ Missing (Not in Bootstrap)
**Specification References**: Section 12 (Memory Model), Section 12.1.4 (Static Region Lifetime Analysis), Section 27.5.1 (Frontend Phases)
**Dependencies**: Requires Phase 3 (Type Checking Phase) completion
**Blocks**: Phase 9 (Region Optimization Phase)

#### Sub-phase 6.1: Lifetime Environment Infrastructure
**Component**: `region_analyzer` module

**Task 6.1.1**: Define Lifetime Environment Types
- **Subtask 6.1.1.1**: Define `LifetimeEnv` type mapping region identifiers to scopes
- **Subtask 6.1.1.2**: Define `DependencySet` type tracking references and creation scopes
- **Subtask 6.1.1.3**: Define `ScopeId` type for lexical scope identification
- **Specification**: Section 12.1.4
- **Silica Pattern**: Use struct types with explicit type annotations:
  ```silica
  struct LifetimeEnv {
      regions: MapRegionIdScope,
      current_scope: ScopeId,
  }
  ```

**Task 6.1.2**: Implement Scope Tracking
- **Subtask 6.1.2.1**: Create scope entry function
- **Subtask 6.1.2.2**: Create scope exit function
- **Subtask 6.1.2.3**: Implement scope hierarchy tracking
- **Specification**: Section 12.1.4
- **Silica Pattern**: Use region-based memory for scope tracking:
  ```silica
  fn enter_scope(env: ref(R, normal, LifetimeEnv)) -> ScopeId proc[mem(normal)]
  fn exit_scope(env: ref(R, normal, LifetimeEnv), scope: ScopeId) -> unit proc[mem(normal)]
  ```

#### Sub-phase 6.2: Region Allocation Analysis
**Component**: `region_analyzer` module

**Task 6.2.1**: Implement Region Allocation Rule
- **Subtask 6.2.1.1**: Analyze `alloc_region` expressions
- **Subtask 6.2.1.2**: Add region to lifetime environment
- **Subtask 6.2.1.3**: Assign current scope to region
- **Specification**: Section 12.1.4
- **Silica Pattern**: Pattern match on AST expression types:
  ```silica
  fn analyze_region_allocation(
      expr: RegionExpr,
      env: ref(R, normal, LifetimeEnv),
      deps: ref(R, normal, DependencySet)
  ) -> (Type, LifetimeEnv, DependencySet) proc[mem(normal)]
  ```

**Task 6.2.2**: Implement Reference Allocation Rule
- **Subtask 6.2.2.1**: Analyze `alloc_ref` expressions
- **Subtask 6.2.2.2**: Verify region exists in lifetime environment
- **Subtask 6.2.2.3**: Add reference to dependency set
- **Specification**: Section 12.1.4
- **Silica Pattern**: Use case expressions for pattern matching:
  ```silica
  fn analyze_ref_allocation(
      expr: AllocRefExpr,
      env: ref(R, normal, LifetimeEnv),
      deps: ref(R, normal, DependencySet)
  ) -> (Type, LifetimeEnv, DependencySet) proc[mem(normal)]
  ```

#### Sub-phase 6.3: Reference Usage Analysis
**Component**: `region_analyzer` module

**Task 6.3.1**: Implement Reference Read Rule
- **Subtask 6.3.1.1**: Analyze `read_ref` expressions
- **Subtask 6.3.1.2**: Verify region exists in lifetime environment
- **Subtask 6.3.1.3**: Verify reference exists in dependency set
- **Subtask 6.3.1.4**: Verify scope constraints (scope_current ≤ scope_r, scope_current ≤ scope_ref)
- **Specification**: Section 12.1.4
- **Silica Pattern**: Use explicit type annotations and effect declarations:
  ```silica
  fn analyze_ref_read(
      expr: ReadRefExpr,
      env: ref(R, normal, LifetimeEnv),
      deps: ref(R, normal, DependencySet)
  ) -> (Type, LifetimeEnv, DependencySet) proc[mem(normal)]
  ```

**Task 6.3.2**: Implement Reference Write Rule
- **Subtask 6.3.2.1**: Analyze `write_ref` expressions
- **Subtask 6.3.2.2**: Verify reference and region constraints
- **Subtask 6.3.2.3**: Verify value type matches reference type
- **Specification**: Section 12.1.4
- **Silica Pattern**: Similar to read_ref with type checking:
  ```silica
  fn analyze_ref_write(
      expr: WriteRefExpr,
      env: ref(R, normal, LifetimeEnv),
      deps: ref(R, normal, DependencySet)
  ) -> (Type, LifetimeEnv, DependencySet) proc[mem(normal)]
  ```

#### Sub-phase 6.4: Cross-Function Analysis
**Component**: `region_analyzer` module

**Task 6.4.1**: Implement Function Parameter Lifetime Extension
- **Subtask 6.4.1.1**: Analyze function parameters with region types
- **Subtask 6.4.1.2**: Extend region lifetimes to function return scope
- **Specification**: Section 12.1.4
- **Silica Pattern**: Use function types with explicit effects:
  ```silica
  fn analyze_function_lifetime(
      func: FunctionDecl,
      env: ref(R, normal, LifetimeEnv),
      deps: ref(R, normal, DependencySet)
  ) -> (LifetimeEnv, DependencySet) proc[mem(normal)]
  ```

**Task 6.4.2**: Implement Function Return Lifetime Constraint
- **Subtask 6.4.2.1**: Analyze function return types with reference types
- **Subtask 6.4.2.2**: Verify region outlives function call
- **Specification**: Section 12.1.4
- **Silica Pattern**: Pattern match on return type:
  ```silica
  fn check_return_lifetime(
      return_type: Type,
      env: ref(R, normal, LifetimeEnv)
  ) -> bool proc[mem(normal)]
  ```

#### Sub-phase 6.5: Cross-Module Region Analysis
**Component**: `region_analyzer` module

**Task 6.5.1**: Implement Cross-Module Lifetime Tracking
- **Subtask 6.5.1.1**: Track regions across module boundaries
- **Subtask 6.5.1.2**: Verify cross-module reference usage
- **Specification**: Section 12 (Memory Model)
- **Silica Pattern**: Use module-scoped lifetime environments:
  ```silica
  struct CrossModuleLifetimeEnv {
      module_envs: MapModuleNameLifetimeEnv,
      global_regions: SetRegionId,
  }
  ```

**Task 6.5.2**: Implement Module Boundary Safety Checks
- **Subtask 6.5.2.1**: Verify exported functions don't leak regions
- **Subtask 6.5.2.2**: Verify imported functions respect region lifetimes
- **Specification**: Section 12 (Memory Model)
- **Silica Pattern**: Use trait-based validation:
  ```silica
  trait RegionSafe {
      fn verify_region_safety(self: Self) -> bool proc[mem(normal)];
  }
  ```

---

---

### Phase 7: Pattern Matching Exhaustiveness Enhancement
**Priority**: Critical (GAP-002)
**Status**: ⚠️ Partial (Basic pattern matching in Bootstrap, missing guard exhaustiveness and AArch64 optimizations)
**Specification References**: Section 3.6 (Pattern Matching Semantics), Section 27.5.3 (Backend Phases)
**Dependencies**: Requires Phase 1 (Parsing Phase) completion
**Blocks**: None (enhancement)

#### Sub-phase 7.1: Guard Exhaustiveness Checking
**Component**: `pattern_analyzer` module

**Task 7.1.1**: Implement Guard Coverage Computation
- **Subtask 7.1.1.1**: Define guard coverage set computation
- **Subtask 7.1.1.2**: Implement guard exhaustiveness checking algorithm
- **Subtask 7.1.1.3**: Handle conservative coverage for function calls
- **Specification**: Section 3.6
- **Silica Pattern**: Use recursive pattern matching:
  ```silica
  fn compute_guard_coverage(
      pattern: Pattern,
      guard: OptionExpression,
      type_: Type
  ) -> CoverageSet proc[mem(normal)]
  ```

**Task 7.1.2**: Implement Pattern-Guard Exhaustiveness
- **Subtask 7.1.2.1**: Check pattern exhaustiveness
- **Subtask 7.1.2.2**: Check guard exhaustiveness for each pattern
- **Subtask 7.1.2.3**: Verify catch-all patterns when guards incomplete
- **Specification**: Section 3.6
- **Silica Pattern**: Use case expressions for exhaustiveness checking:
  ```silica
  fn check_exhaustiveness(
      patterns: ListPattern,
      matched_type: Type
  ) -> ResultUnitExhaustivenessError proc[mem(normal)]
  ```

#### Sub-phase 7.2: Pattern Compilation Strategy Selection
**Component**: `pattern_compiler` module

**Task 7.2.1**: Implement Decision Tree Generation
- **Subtask 7.2.1.1**: Build decision tree from patterns
- **Subtask 7.2.1.2**: Optimize decision tree structure
- **Subtask 7.2.1.3**: Generate code from decision tree
- **Specification**: Section 3.6
- **Silica Pattern**: Use recursive data structures:
  ```silica
  enum DecisionTree {
      Leaf { result: Expression },
      Branch { condition: Expression, true_branch: DecisionTree, false_branch: DecisionTree },
  }
  ```

**Task 7.2.2**: Implement Jump Table Generation
- **Subtask 7.2.2.1**: Detect dense integer/enum patterns
- **Subtask 7.2.2.2**: Generate jump table for dense patterns
- **Subtask 7.2.2.3**: Use PC-relative addressing for AArch64
- **Specification**: Section 3.6, Section 27.5.3
- **Silica Pattern**: Use AArch64-specific code generation:
  ```silica
  fn generate_jump_table(
      patterns: ListPattern,
      matched_type: Type
  ) -> JumpTable proc[mem(normal)]
  ```

#### Sub-phase 7.3: AArch64 Pattern Optimization
**Component**: `pattern_compiler` module

**Task 7.3.1**: Implement Conditional Select Optimization
- **Subtask 7.3.1.1**: Detect simple two-way pattern matches
- **Subtask 7.3.1.2**: Generate CSEL instructions
- **Subtask 7.3.1.3**: Eliminate branch misprediction penalties
- **Specification**: Section 27.5.3
- **Silica Pattern**: Use AArch64 instruction patterns:
  ```silica
  fn generate_csel_pattern(
      condition: Expression,
      true_value: Expression,
      false_value: Expression
  ) -> AArch64Instruction proc[mem(normal)]
  ```

**Task 7.3.2**: Implement Pattern-Guard Fusion
- **Subtask 7.3.2.1**: Detect patterns with simple guards
- **Subtask 7.3.2.2**: Fuse pattern matching and guard evaluation
- **Subtask 7.3.2.3**: Generate single conditional instruction sequence
- **Specification**: Section 27.5.3
- **Silica Pattern**: Combine pattern and guard analysis:
  ```silica
  fn fuse_pattern_guard(
      pattern: Pattern,
      guard: Expression
  ) -> FusedPattern proc[mem(normal)]
  ```

---

---

### Phase 8: Hardware Capability Validation
**Priority**: Critical (GAP-003)
**Status**: ❌ Missing (Not in Bootstrap)
**Specification References**: Section 27.5.1 (Frontend Phases), Section 1.4 (Target Platform), Section 27.6.1 (Hardware-Aware Build Configuration)
**Dependencies**: Requires Phase 3 (Type Checking Phase) completion
**Blocks**: Phase 12 (Vectorization Phase), Optimizations

#### Sub-phase 3.1: Hardware Feature Detection
**Component**: `hardware_detector` module

**Task 3.1.1**: Implement CPU Feature Detection
- **Subtask 3.1.1.1**: Detect SVE/SVE2 support
- **Subtask 3.1.1.2**: Detect NEON support
- **Subtask 3.1.1.3**: Detect MTE support
- **Subtask 3.1.1.4**: Detect PAC support
- **Specification**: Section 1.4, Section 27.6.1
- **Silica Pattern**: Use AArch64 system registers:
  ```silica
  struct HardwareCapabilities {
      sve: bool,
      sve2: bool,
      neon: bool,
      mte: bool,
      pac: bool,
  }
  
  fn detect_capabilities() -> HardwareCapabilities proc[device_io]
  ```

**Task 3.1.2**: Implement Cache Hierarchy Discovery
- **Subtask 3.1.2.1**: Detect cache levels and sizes
- **Subtask 3.1.2.2**: Detect NUMA topology
- **Subtask 3.1.2.3**: Map cache hierarchy structure
- **Specification**: Section 27.6.1
- **Silica Pattern**: Use system information structures:
  ```silica
  struct CacheHierarchy {
      levels: ListCacheLevel,
      numa_nodes: ListNUMANode,
  }
  
  fn discover_cache_hierarchy() -> CacheHierarchy proc[device_io]
  ```

#### Sub-phase 3.2: Capability Validation During Type Checking
**Component**: `type_checker` module (enhancement)

**Task 3.2.1**: Integrate Capability Checking into Type Checker
- **Subtask 3.2.1.1**: Add capability context to type checker
- **Subtask 3.2.1.2**: Validate vector types against capabilities
- **Subtask 3.2.1.3**: Validate atomic operations against capabilities
- **Specification**: Section 27.5.1
- **Silica Pattern**: Extend type checker context:
  ```silica
  struct TypeCheckerContext {
      type_env: TypeEnv,
      capabilities: HardwareCapabilities,
      constraints: ListConstraint,
  }
  
  fn check_type_with_capabilities(
      expr: Expression,
      context: ref(R, normal, TypeCheckerContext)
  ) -> (Type, TypeCheckerContext) proc[mem(normal)]
  ```

**Task 3.2.2**: Implement Capability Error Reporting
- **Subtask 3.2.2.1**: Generate capability mismatch errors
- **Subtask 3.2.2.2**: Suggest alternative implementations
- **Subtask 3.2.2.3**: Provide fallback options
- **Specification**: Section 1.6 (Compiler Error Messages)
- **Silica Pattern**: Use structured error types:
  ```silica
  enum CapabilityError {
      UnsupportedFeature { feature: string, required: HardwareCapabilities },
      MissingCapability { feature: string, available: HardwareCapabilities },
  }
  ```

---

---

### Phase 9: Region Optimization Phase
**Priority**: Major (GAP-004)
**Status**: ❌ Missing (Not in Bootstrap)
**Specification References**: Section 27.5.2 (Middle-End Phases), Section 12 (Memory Model), Section 27.6.1 (Hardware-Aware Build Configuration)
**Dependencies**: Requires Phase 6 (Region Analysis Phase) completion
**Blocks**: None

#### Sub-phase 4.1: NUMA-Aware Memory Layout
**Component**: `region_optimizer` module

**Task 4.1.1**: Implement NUMA Topology Analysis
- **Subtask 4.1.1.1**: Analyze NUMA node structure
- **Subtask 4.1.1.2**: Map regions to NUMA nodes
- **Subtask 4.1.1.3**: Optimize region placement for locality
- **Specification**: Section 27.6.1
- **Silica Pattern**: Use NUMA-aware region allocation:
  ```silica
  struct NUMARegionLayout {
      regions: MapRegionIdNUMANode,
      affinity: MapRegionIdCoreSet,
  }
  
  fn optimize_numa_layout(
      regions: ListRegion,
      numa_topology: NUMATopology
  ) -> NUMARegionLayout proc[mem(normal)]
  ```

**Task 4.1.2**: Implement Cache-Aware Memory Layout
- **Subtask 4.1.2.1**: Analyze cache hierarchy
- **Subtask 4.1.2.2**: Optimize region placement for cache locality
- **Subtask 4.1.2.3**: Minimize cache conflicts
- **Specification**: Section 27.6.1
- **Silica Pattern**: Use cache-aware optimization:
  ```silica
  fn optimize_cache_layout(
      regions: ListRegion,
      cache_hierarchy: CacheHierarchy
  ) -> CacheOptimizedLayout proc[mem(normal)]
  ```

#### Sub-phase 4.2: Region Allocation Optimization
**Component**: `region_optimizer` module

**Task 4.2.1**: Implement Region Pooling
- **Subtask 4.2.1.1**: Identify region reuse opportunities
- **Subtask 4.2.1.2**: Pool regions with similar lifetimes
- **Subtask 4.2.1.3**: Reduce allocation overhead
- **Specification**: Section 27.5.2
- **Silica Pattern**: Use region pooling strategies:
  ```silica
  struct RegionPool {
      available_regions: ListRegion,
      pool_size: int64,
  }
  
  fn pool_regions(
      regions: ListRegion,
      lifetime_analysis: LifetimeAnalysis
  ) -> RegionPool proc[mem(normal)]
  ```

---

---

### Phase 10: Actor Optimization Phase
**Priority**: Major (GAP-005)
**Status**: ❌ Missing (Not in Bootstrap)
**Specification References**: Section 27.5.2 (Middle-End Phases), Section 15 (Actor Model Semantics), Section 16 (Message Passing)
**Dependencies**: Requires Phase 3 (Type Checking Phase) completion
**Blocks**: None

#### Sub-phase 5.1: Actor Scheduling Optimization
**Component**: `actor_optimizer` module

**Task 5.1.1**: Implement Actor Affinity Analysis
- **Subtask 5.1.1.1**: Analyze actor communication patterns
- **Subtask 5.1.1.2**: Determine actor affinity to cores
- **Subtask 5.1.1.3**: Optimize actor placement
- **Specification**: Section 15, Section 27.5.2
- **Silica Pattern**: Use actor scheduling structures:
  ```silica
  struct ActorAffinity {
      actor_id: ActorId,
      preferred_cores: CoreSet,
      communication_partners: ListActorId,
  }
  
  fn analyze_actor_affinity(
      actors: ListActor,
      message_trace: MessageTrace
  ) -> ListActorAffinity proc[mem(normal)]
  ```

**Task 5.1.2**: Implement Message Passing Optimization
- **Subtask 5.1.2.1**: Analyze message patterns
- **Subtask 5.1.2.2**: Optimize mailbox layout
- **Subtask 5.1.2.3**: Reduce message copying
- **Specification**: Section 16, Section 27.5.2
- **Silica Pattern**: Use message optimization strategies:
  ```silica
  fn optimize_message_passing(
      actors: ListActor,
      messages: ListMessage
  ) -> OptimizedMessageLayout proc[mem(normal)]
  ```

---

---

### Phase 11: Effect Lowering Phase
**Priority**: Major (GAP-006)
**Status**: ⚠️ Delegated to LLVM (Not Native)
**Specification References**: Section 27.5.2 (Middle-End Phases), Section 9 (Effect System), Section 26 (Platform Integration)
**Dependencies**: Requires Phase 4 (Effect Checking Phase) completion
**Blocks**: Phase 13 (Native Backend Phases)

#### Sub-phase 6.1: Effect to Hardware Primitive Mapping
**Component**: `effect_lowering` module

**Task 6.1.1**: Implement Memory Effect Lowering
- **Subtask 6.1.1.1**: Map `mem(normal)` to AArch64 memory attributes
- **Subtask 6.1.1.2**: Map `mem(atomic)` to atomic memory operations
- **Subtask 6.1.1.3**: Configure MAIR_EL1 register
- **Specification**: Section 9, Section 26
- **Silica Pattern**: Use effect lowering functions:
  ```silica
  fn lower_memory_effect(
      effect: MemoryEffect,
      capabilities: HardwareCapabilities
  ) -> AArch64MemoryConfig proc[mem(normal)]
  ```

**Task 6.1.2**: Implement Concurrency Effect Lowering
- **Subtask 6.1.2.1**: Map `concurrency` to actor runtime primitives
- **Subtask 6.1.2.2**: Map `mailbox` to message queue primitives
- **Subtask 6.1.2.3**: Generate actor scheduling code
- **Specification**: Section 9, Section 15
- **Silica Pattern**: Use concurrency lowering:
  ```silica
  fn lower_concurrency_effect(
      effect: ConcurrencyEffect,
      actor_info: ActorInfo
  ) -> AArch64ConcurrencyConfig proc[mem(normal)]
  ```

---

---

### Phase 12: Vectorization Phase
**Priority**: Major (GAP-007)
**Status**: ❌ Missing (Not in Bootstrap)
**Specification References**: Section 27.5.2 (Middle-End Phases), Section 1.4 (Target Platform), Section 21 (Architecture-Specific Modules)
**Dependencies**: Requires Phase 8 (Hardware Capability Validation) completion
**Blocks**: None

#### Sub-phase 12.1: SVE Vectorization
**Component**: `vectorizer` module

**Task 12.1.1**: Implement SVE Pattern Recognition
- **Subtask 12.1.1.1**: Detect vectorizable loops (recursive patterns)
- **Subtask 12.1.1.2**: Identify SVE-compatible operations
- **Subtask 12.1.1.3**: Analyze data dependencies
- **Specification**: Section 21, Section 27.5.2
- **Silica Pattern**: Use vectorization analysis:
  ```silica
  fn detect_sve_patterns(
      expr: Expression,
      capabilities: HardwareCapabilities
  ) -> OptionSVEVectorization proc[mem(normal)]
  ```

**Task 12.1.2**: Implement SVE Code Generation
- **Subtask 12.1.2.1**: Generate SVE vector instructions
- **Subtask 12.1.2.2**: Handle scalable vector types
- **Subtask 12.1.2.3**: Optimize vector operations
- **Specification**: Section 21, Section 27.5.2
- **Silica Pattern**: Use SVE instruction generation:
  ```silica
  fn generate_sve_code(
      vectorization: SVEVectorization,
      vector_type: VecType
  ) -> ListAArch64Instruction proc[mem(normal)]
  ```

#### Sub-phase 12.2: NEON Vectorization
**Component**: `vectorizer` module

**Task 12.2.1**: Implement NEON Pattern Recognition
- **Subtask 12.2.1.1**: Detect NEON-compatible operations
- **Subtask 12.2.1.2**: Identify 128-bit vector opportunities
- **Subtask 12.2.1.3**: Analyze alignment requirements
- **Specification**: Section 21, Section 27.5.2
- **Silica Pattern**: Similar to SVE but for fixed-width vectors:
  ```silica
  fn detect_neon_patterns(
      expr: Expression,
      capabilities: HardwareCapabilities
  ) -> OptionNEONVectorization proc[mem(normal)]
  ```

---

---

### Phase 13: Native Backend Phases
**Priority**: Major (GAP-008 through GAP-011)
**Status**: ⚠️ Delegated to LLVM (Not Native)
**Specification References**: Section 27.5.3 (Backend Phases), Section 26 (Platform Integration), Section 27 (Compilation and Linking)
**Dependencies**: Requires Phase 11 (Effect Lowering Phase) completion
**Blocks**: None (enables native backend)

#### Sub-phase 13.1: Instruction Selection Phase
**Component**: `instruction_selector` module

**Task 13.1.1**: Implement AArch64 Instruction Patterns
- **Subtask 13.1.1.1**: Define instruction pattern matching
- **Subtask 13.1.1.2**: Implement pattern selection algorithm
- **Subtask 13.1.1.3**: Optimize instruction sequences
- **Specification**: Section 27.5.3, Section 26
- **Silica Pattern**: Use instruction pattern structures:
  ```silica
  struct InstructionPattern {
      pattern: Expression,
      instruction: AArch64Instruction,
      cost: int64,
  }
  
  fn select_instruction(
      expr: Expression,
      patterns: ListInstructionPattern
  ) -> AArch64Instruction proc[mem(normal)]
  ```

#### Sub-phase 13.2: Register Allocation Phase
**Component**: `register_allocator` module

**Task 13.2.1**: Implement Region-Lifetime Aware Allocation
- **Subtask 13.2.1.1**: Integrate region lifetime information
- **Subtask 13.2.1.2**: Allocate registers considering lifetimes
- **Subtask 13.2.1.3**: Manage 32 general-purpose registers
- **Specification**: Section 27.5.3, Section 12
- **Silica Pattern**: Use register allocation with lifetime awareness:
  ```silica
  struct RegisterAllocator {
      available_registers: SetRegister,
      lifetime_info: LifetimeAnalysis,
      allocations: MapVariableRegister,
  }
  
  fn allocate_registers(
      variables: ListVariable,
      lifetimes: LifetimeAnalysis
  ) -> RegisterAllocation proc[mem(normal)]
  ```

#### Sub-phase 13.3: Code Layout Phase
**Component**: `code_layout` module

**Task 13.3.1**: Implement Cache-Aware Code Placement
- **Subtask 13.3.1.1**: Analyze code hot paths
- **Subtask 13.3.1.2**: Place hot code in cache-friendly locations
- **Subtask 13.3.1.3**: Optimize instruction cache usage
- **Specification**: Section 27.5.3
- **Silica Pattern**: Use code layout optimization:
  ```silica
  fn optimize_code_layout(
      functions: ListFunction,
      hot_paths: ListHotPath,
      cache_info: CacheHierarchy
  ) -> OptimizedLayout proc[mem(normal)]
  ```

**Task 13.3.2**: Implement TLB Optimization
- **Subtask 13.3.2.1**: Analyze page access patterns
- **Subtask 13.3.2.2**: Optimize page layout
- **Subtask 13.3.2.3**: Minimize TLB misses
- **Specification**: Section 27.5.3
- **Silica Pattern**: Use TLB-aware layout:
  ```silica
  fn optimize_tlb_layout(
      code: ListInstruction,
      page_size: int64
  ) -> TLBOptimizedLayout proc[mem(normal)]
  ```

#### Sub-phase 13.4: Link-Time Optimization Phase
**Component**: `link_time_optimizer` module

**Task 13.4.1**: Implement Cross-Module Optimization
- **Subtask 13.4.1.1**: Analyze cross-module call patterns
- **Subtask 13.4.1.2**: Inline cross-module functions
- **Subtask 13.4.1.3**: Optimize cross-module data access
- **Specification**: Section 27.5.3, Section 28
- **Silica Pattern**: Use cross-module analysis:
  ```silica
  fn optimize_cross_module(
      modules: ListModule,
      call_graph: CallGraph
  ) -> OptimizedModules proc[mem(normal)]
  ```

**Task 13.4.2**: Implement Hardware-Aware Linking
- **Subtask 13.4.2.1**: Apply hardware-specific optimizations
- **Subtask 13.4.2.2**: Generate hardware-aware code variants
- **Subtask 13.4.2.3**: Select optimal code paths
- **Specification**: Section 27.5.3, Section 27.6.1
- **Silica Pattern**: Use hardware-aware linking:
  ```silica
  fn link_with_hardware_awareness(
      modules: ListModule,
      capabilities: HardwareCapabilities
  ) -> LinkedModule proc[mem(normal)]
  ```

---

## Component Hierarchy

### Component 0: Lexer
**Purpose**: Tokenize source code into tokens
**Implements**: Phase 0 (Lexical Analysis Phase)
**Specification**: Section 2 (Lexical Structure)

#### Module 0.1: `lexer`
**Functions**:
- `read_source_file(path: string) -> SourceFile proc[device_io, mem(normal)]`
- `recognize_keyword(lexeme: string) -> OptionTokenKind proc[mem(normal)]`
- `parse_integer_literal(chars: ListChar, start: int64) -> (Token, int64) proc[mem(normal)]`
- `parse_string_literal(chars: ListChar, start: int64) -> (Token, int64) proc[mem(normal)]`
- `recognize_operator(chars: ListChar, start: int64) -> OptionToken proc[mem(normal)]`
- `tokenize(source: SourceFile) -> ResultListTokenLexerError proc[mem(normal)]`

### Component 1: Parser
**Purpose**: Build Abstract Syntax Tree from tokens
**Implements**: Phase 1 (Parsing Phase)
**Specification**: Section 3 (Syntax)

#### Module 1.1: `ast`
**Functions**:
- `create_program(declarations: ListDeclaration, location: SourceLocation) -> Program proc[mem(normal)]`
- `create_function_declaration(name: string, params: ListParameter, return_type: OptionType, body: ListStatement, effects: ListEffect, location: SourceLocation) -> FunctionDecl proc[mem(normal)]`

#### Module 1.2: `parser`
**Functions**:
- `parse_declaration(parser: ref(R, normal, Parser)) -> ResultDeclarationParseError proc[mem(normal)]`
- `parse_function_declaration(parser: ref(R, normal, Parser)) -> ResultFunctionDeclParseError proc[mem(normal)]`
- `parse_expression(parser: ref(R, normal, Parser)) -> ResultExpressionParseError proc[mem(normal)]`
- `parse_pattern(parser: ref(R, normal, Parser)) -> ResultPatternParseError proc[mem(normal)]`
- `parse_type(parser: ref(R, normal, Parser)) -> ResultTypeParseError proc[mem(normal)]`
- `parse_program(parser: ref(R, normal, Parser)) -> ResultProgramParseError proc[mem(normal)]`

### Component 2: Module Resolver
**Purpose**: Resolve module dependencies and build dependency graph
**Implements**: Phase 2 (Module Resolution Phase)
**Specification**: Section 19 (Module System)

#### Module 2.1: `module_resolver`
**Functions**:
- `extract_module_name(file_path: string) -> ResultStringModuleError proc[mem(normal)]`
- `new_module_resolver(search_paths: ListString) -> ModuleResolver proc[mem(normal)]`
- `find_module_file(resolver: ref(R, normal, ModuleResolver), module_name: string) -> ResultStringModuleError proc[device_io, mem(normal)]`
- `load_module(resolver: ref(R, normal, ModuleResolver), module_name: string) -> ResultUnitModuleError proc[device_io, mem(normal)]`
- `cache_module(resolver: ref(R, normal, ModuleResolver), module: LoadedModule) -> unit proc[mem(normal)]`
- `build_dependency_graph(modules: ListLoadedModule) -> DependencyGraph proc[mem(normal)]`
- `detect_cycles(graph: DependencyGraph) -> ResultUnitCycleError proc[mem(normal)]`
- `topological_sort(graph: DependencyGraph) -> ListModuleName proc[mem(normal)]`
- `extract_exports(ast: Program) -> ListExportItem proc[mem(normal)]`
- `add_module_symbols(table: ref(R, normal, SymbolTable), module: LoadedModule) -> ResultUnitSymbolError proc[mem(normal)]`
- `lookup_symbol(table: ref(R, normal, SymbolTable), module_name: string, symbol_name: string) -> OptionSymbolInfo proc[mem(normal)]`

### Component 3: Type Checker
**Purpose**: Verify type correctness
**Implements**: Phase 3 (Type Checking Phase)
**Specification**: Section 8 (Type System), Section 10 (Type Checking)

#### Module 3.1: `type_checker`
**Functions**:
- `create_type_env() -> TypeEnv proc[mem(normal)]`
- `add_binding(env: ref(R, normal, TypeEnv), name: string, scheme: TypeScheme) -> unit proc[mem(normal)]`
- `lookup_binding(env: ref(R, normal, TypeEnv), name: string) -> OptionTypeScheme proc[mem(normal)]`
- `add_constraint(env: ref(R, normal, TypeEnv), constraint: Constraint) -> unit proc[mem(normal)]`
- `solve_constraints(env: ref(R, normal, TypeEnv)) -> ResultSubstitutionTypeError proc[mem(normal)]`
- `check_literal_type(literal: Literal) -> Type proc[mem(normal)]`
- `check_variable_type(env: ref(R, normal, TypeEnv), name: string) -> ResultTypeTypeError proc[mem(normal)]`
- `check_function_call(env: ref(R, normal, TypeEnv), func_expr: Expression, args: ListExpression) -> (Type, ListEffect) proc[mem(normal)]`
- `check_trait_constraint(env: ref(R, normal, TypeEnv), trait_name: string, type_: Type) -> ResultUnitTypeError proc[mem(normal)]`
- `lookup_trait_implementation(trait_name: string, type_: Type, impls: ListTraitImpl) -> OptionTraitImpl proc[mem(normal)]`
- `check_function_declaration(env: ref(R, normal, TypeEnv), decl: FunctionDecl) -> ResultUnitTypeError proc[mem(normal)]`
- `check_type_declaration(env: ref(R, normal, TypeEnv), decl: TypeDecl) -> ResultUnitTypeError proc[mem(normal)]`
- `check_trait_declaration(env: ref(R, normal, TypeEnv), decl: TraitDecl) -> ResultUnitTypeError proc[mem(normal)]`
- `check_impl_declaration(env: ref(R, normal, TypeEnv), decl: ImplDecl) -> ResultUnitTypeError proc[mem(normal)]`
- `build_global_type_environment(modules: ListModule) -> TypeEnv proc[mem(normal)]`
- `lookup_trait_across_modules(env: ref(R, normal, TypeEnv), trait_name: string, type_: Type) -> OptionTraitImpl proc[mem(normal)]`
- `verify_cross_module_types(env: ref(R, normal, TypeEnv)) -> ResultUnitTypeError proc[mem(normal)]`

### Component 4: Effect Checker
**Purpose**: Verify effect declarations and propagation
**Implements**: Phase 4 (Effect Checking Phase)
**Specification**: Section 9 (Effect System)

#### Module 4.1: `effect_checker`
**Functions**:
- `create_effect_context() -> EffectContext proc[mem(normal)]`
- `push_capability(context: ref(R, normal, EffectContext), capability: Capability) -> unit proc[mem(normal)]`
- `pop_capability(context: ref(R, normal, EffectContext)) -> OptionCapability proc[mem(normal)]`
- `get_active_effects(context: ref(R, normal, EffectContext)) -> ListEffect proc[mem(normal)]`
- `validate_effect_declaration(effects: ListEffect, function_body: ListStatement) -> ResultUnitEffectError proc[mem(normal)]`
- `collect_expression_effects(expr: Expression) -> ListEffect proc[mem(normal)]`
- `union_effects(effects1: ListEffect, effects2: ListEffect) -> ListEffect proc[mem(normal)]`
- `propagate_effects(context: ref(R, normal, EffectContext), expr: Expression) -> ListEffect proc[mem(normal)]`
- `check_capability(context: ref(R, normal, EffectContext), required_effect: Effect) -> ResultUnitEffectError proc[mem(normal)]`
- `is_subeffect(effect1: Effect, effect2: Effect) -> bool proc[mem(normal)]`

### Component 5: Code Generator (LLVM Backend)
**Purpose**: Generate LLVM IR from validated AST
**Implements**: Phase 5 (Code Generation Phase - LLVM)
**Specification**: Section 27 (Compilation and Linking)
**Note**: Temporary LLVM backend. Will be replaced by native backend (Phase 13).

#### Module 5.1: `codegen`
**Functions**:
- `map_silica_type_to_llvm(type_: Type) -> LLVMType proc[mem(normal)]`
- `generate_function_signature(func: FunctionDecl, type_info: TypeInfo) -> LLVMFunction proc[mem(normal)]`
- `generate_function_body(func: FunctionDecl, llvm_func: LLVMFunction, type_info: TypeInfo) -> ResultUnitCodegenError proc[mem(normal)]`
- `generate_literal_code(literal: Literal, type_: Type) -> LLVMValue proc[mem(normal)]`
- `generate_call_code(call_expr: CallExpr, type_info: TypeInfo) -> LLVMValue proc[mem(normal)]`
- `generate_case_code(case_expr: CaseExpr, type_info: TypeInfo) -> LLVMValue proc[mem(normal)]`
- `write_llvm_ir(module: LLVMModule, output_file: string) -> ResultUnitCodegenError proc[device_io, mem(normal)]`

### Component 6: Region Analyzer
**Purpose**: Analyze region-based memory management for lifetime and ownership verification
**Implements**: Phase 6 (Region Analysis Phase)
**Specification**: Section 12 (Memory Model), Section 12.1.4

#### Module 1.1: `lifetime_env`
**Functions**:
- `create_lifetime_env() -> LifetimeEnv proc[mem(normal)]`
- `add_region(env: ref(R, normal, LifetimeEnv), region_id: RegionId, scope: ScopeId) -> unit proc[mem(normal)]`
- `remove_region(env: ref(R, normal, LifetimeEnv), region_id: RegionId) -> unit proc[mem(normal)]`
- `lookup_region(env: ref(R, normal, LifetimeEnv), region_id: RegionId) -> OptionScopeId proc[mem(normal)]`

#### Module 1.2: `dependency_set`
**Functions**:
- `create_dependency_set() -> DependencySet proc[mem(normal)]`
- `add_reference(deps: ref(R, normal, DependencySet), ref_id: RefId, scope: ScopeId) -> unit proc[mem(normal)]`
- `remove_reference(deps: ref(R, normal, DependencySet), ref_id: RefId) -> unit proc[mem(normal)]`
- `lookup_reference(deps: ref(R, normal, DependencySet), ref_id: RefId) -> OptionScopeId proc[mem(normal)]`

#### Module 1.3: `region_analysis`
**Functions**:
- `analyze_region_allocation(expr: RegionExpr, env: ref(R, normal, LifetimeEnv), deps: ref(R, normal, DependencySet)) -> (Type, LifetimeEnv, DependencySet) proc[mem(normal)]`
- `analyze_ref_allocation(expr: AllocRefExpr, env: ref(R, normal, LifetimeEnv), deps: ref(R, normal, DependencySet)) -> (Type, LifetimeEnv, DependencySet) proc[mem(normal)]`
- `analyze_ref_read(expr: ReadRefExpr, env: ref(R, normal, LifetimeEnv), deps: ref(R, normal, DependencySet)) -> (Type, LifetimeEnv, DependencySet) proc[mem(normal)]`
- `analyze_ref_write(expr: WriteRefExpr, env: ref(R, normal, LifetimeEnv), deps: ref(R, normal, DependencySet)) -> (Type, LifetimeEnv, DependencySet) proc[mem(normal)]`
- `analyze_program(program: Program, type_info: TypeInfo) -> RegionAnalysisResult proc[mem(normal)]`

### Component 7: Pattern Analyzer
**Purpose**: Analyze and verify pattern matching exhaustiveness
**Implements**: Phase 7 (Pattern Matching Exhaustiveness Enhancement)
**Specification**: Section 3.6 (Pattern Matching Semantics)

#### Module 2.1: `exhaustiveness_checker`
**Functions**:
- `check_pattern_exhaustiveness(patterns: ListPattern, matched_type: Type) -> ResultUnitExhaustivenessError proc[mem(normal)]`
- `check_guard_exhaustiveness(patterns: ListPattern, matched_type: Type) -> ResultUnitExhaustivenessError proc[mem(normal)]`
- `compute_coverage_set(pattern: Pattern, guard: OptionExpression, type_: Type) -> CoverageSet proc[mem(normal)]`

#### Module 2.2: `pattern_compiler`
**Functions**:
- `build_decision_tree(patterns: ListPattern, matched_type: Type) -> DecisionTree proc[mem(normal)]`
- `build_jump_table(patterns: ListPattern, matched_type: Type) -> JumpTable proc[mem(normal)]`
- `generate_csel_pattern(condition: Expression, true_value: Expression, false_value: Expression) -> AArch64Instruction proc[mem(normal)]`
- `fuse_pattern_guard(pattern: Pattern, guard: Expression) -> FusedPattern proc[mem(normal)]`

### Component 8: Hardware Detector
**Purpose**: Detect and validate AArch64 hardware capabilities
**Implements**: Phase 8 (Hardware Capability Validation)
**Specification**: Section 1.4 (Target Platform), Section 27.6.1

#### Module 3.1: `capability_detection`
**Functions**:
- `detect_capabilities() -> HardwareCapabilities proc[device_io]`
- `detect_sve_support() -> bool proc[device_io]`
- `detect_neon_support() -> bool proc[device_io]`
- `discover_cache_hierarchy() -> CacheHierarchy proc[device_io]`
- `discover_numa_topology() -> NUMATopology proc[device_io]`

#### Module 3.2: `capability_validation`
**Functions**:
- `validate_vector_type(type_: Type, capabilities: HardwareCapabilities) -> ResultUnitCapabilityError proc[mem(normal)]`
- `validate_atomic_operation(op: AtomicOp, capabilities: HardwareCapabilities) -> ResultUnitCapabilityError proc[mem(normal)]`
- `check_capability_requirements(expr: Expression, capabilities: HardwareCapabilities) -> ResultUnitCapabilityError proc[mem(normal)]`

### Component 9: Region Optimizer
**Purpose**: Optimize region-based memory layout for NUMA and cache
**Implements**: Phase 9 (Region Optimization Phase)
**Specification**: Section 27.5.2 (Middle-End Phases)

#### Module 4.1: `numa_optimizer`
**Functions**:
- `optimize_numa_layout(regions: ListRegion, numa_topology: NUMATopology) -> NUMARegionLayout proc[mem(normal)]`
- `analyze_region_affinity(regions: ListRegion, access_patterns: AccessPatterns) -> MapRegionIdNUMANode proc[mem(normal)]`

#### Module 4.2: `cache_optimizer`
**Functions**:
- `optimize_cache_layout(regions: ListRegion, cache_hierarchy: CacheHierarchy) -> CacheOptimizedLayout proc[mem(normal)]`
- `pool_regions(regions: ListRegion, lifetime_analysis: LifetimeAnalysis) -> RegionPool proc[mem(normal)]`

### Component 10: Actor Optimizer
**Purpose**: Optimize actor scheduling and message passing
**Implements**: Phase 10 (Actor Optimization Phase)
**Specification**: Section 27.5.2 (Middle-End Phases)

#### Module 5.1: `actor_scheduler`
**Functions**:
- `analyze_actor_affinity(actors: ListActor, message_trace: MessageTrace) -> ListActorAffinity proc[mem(normal)]`
- `optimize_actor_placement(actors: ListActor, affinity: ListActorAffinity) -> ActorPlacement proc[mem(normal)]`

#### Module 5.2: `message_optimizer`
**Functions**:
- `optimize_message_passing(actors: ListActor, messages: ListMessage) -> OptimizedMessageLayout proc[mem(normal)]`
- `analyze_message_patterns(messages: ListMessage) -> MessagePatterns proc[mem(normal)]`

### Component 11: Effect Lowering
**Purpose**: Translate high-level effects to hardware primitives
**Implements**: Phase 11 (Effect Lowering Phase)
**Specification**: Section 27.5.2 (Middle-End Phases)

#### Module 6.1: `effect_lowering`
**Functions**:
- `lower_memory_effect(effect: MemoryEffect, capabilities: HardwareCapabilities) -> AArch64MemoryConfig proc[mem(normal)]`
- `lower_concurrency_effect(effect: ConcurrencyEffect, actor_info: ActorInfo) -> AArch64ConcurrencyConfig proc[mem(normal)]`
- `lower_effects(program: Program, capabilities: HardwareCapabilities) -> LoweredProgram proc[mem(normal)]`

### Component 12: Vectorizer
**Purpose**: Automatic SVE and NEON code generation
**Implements**: Phase 12 (Vectorization Phase)
**Specification**: Section 27.5.2 (Middle-End Phases)

#### Module 7.1: `sve_vectorizer`
**Functions**:
- `detect_sve_patterns(expr: Expression, capabilities: HardwareCapabilities) -> OptionSVEVectorization proc[mem(normal)]`
- `generate_sve_code(vectorization: SVEVectorization, vector_type: VecType) -> ListAArch64Instruction proc[mem(normal)]`

#### Module 7.2: `neon_vectorizer`
**Functions**:
- `detect_neon_patterns(expr: Expression, capabilities: HardwareCapabilities) -> OptionNEONVectorization proc[mem(normal)]`
- `generate_neon_code(vectorization: NEONVectorization, vector_type: Vec128Type) -> ListAArch64Instruction proc[mem(normal)]`

### Component 13: Instruction Selector
**Purpose**: AArch64-specific instruction choice
**Implements**: Phase 13.1 (Instruction Selection Phase)
**Specification**: Section 27.5.3 (Backend Phases)

#### Module 8.1: `instruction_selector`
**Functions**:
- `select_instruction(expr: Expression, patterns: ListInstructionPattern) -> AArch64Instruction proc[mem(normal)]`
- `match_pattern(expr: Expression, pattern: InstructionPattern) -> OptionAArch64Instruction proc[mem(normal)]`
- `optimize_instruction_sequence(instructions: ListAArch64Instruction) -> ListAArch64Instruction proc[mem(normal)]`

### Component 14: Register Allocator
**Purpose**: Region-lifetime aware register assignment
**Implements**: Phase 13.2 (Register Allocation Phase)
**Specification**: Section 27.5.3 (Backend Phases)

#### Module 9.1: `register_allocator`
**Functions**:
- `allocate_registers(variables: ListVariable, lifetimes: LifetimeAnalysis) -> RegisterAllocation proc[mem(normal)]`
- `allocate_register_for_variable(var: Variable, lifetimes: LifetimeAnalysis, available: SetRegister) -> OptionRegister proc[mem(normal)]`
- `spill_register(register: Register, variables: ListVariable) -> SpillStrategy proc[mem(normal)]`

### Component 15: Code Layout Optimizer
**Purpose**: Cache and TLB optimized code placement
**Implements**: Phase 13.3 (Code Layout Phase)
**Specification**: Section 27.5.3 (Backend Phases)

#### Module 10.1: `code_layout`
**Functions**:
- `optimize_code_layout(functions: ListFunction, hot_paths: ListHotPath, cache_info: CacheHierarchy) -> OptimizedLayout proc[mem(normal)]`
- `optimize_tlb_layout(code: ListInstruction, page_size: int64) -> TLBOptimizedLayout proc[mem(normal)]`
- `analyze_hot_paths(functions: ListFunction) -> ListHotPath proc[mem(normal)]`

### Component 16: Link-Time Optimizer
**Purpose**: Cross-module hardware-aware optimization
**Implements**: Phase 13.4 (Link-Time Optimization Phase)
**Specification**: Section 27.5.3 (Backend Phases)

#### Module 11.1: `link_time_optimizer`
**Functions**:
- `optimize_cross_module(modules: ListModule, call_graph: CallGraph) -> OptimizedModules proc[mem(normal)]`
- `link_with_hardware_awareness(modules: ListModule, capabilities: HardwareCapabilities) -> LinkedModule proc[mem(normal)]`
- `inline_cross_module_functions(modules: ListModule, call_graph: CallGraph) -> InlinedModules proc[mem(normal)]`

### Component 17: Toolchain Integration
**Purpose**: Toolchain selection, assembly generation compatibility, and toolchain driver invocation
**Implements**: Toolchain Integration (see Toolchain and Assembler Integration section)
**Specification**: Section 1.4 (Target Platform), Section 26 (Platform Integration), Section 27 (Compilation and Linking)

#### Module 17.1: `toolchain`
**Functions**:
- `parse_target_triple(triple: string) -> TargetTriple proc[mem(normal)]`
- `is_apple_target(triple: TargetTriple) -> bool proc[mem(normal)]`
- `parse_toolchain_config(flags: CompilerFlags) -> ResultToolchainToolchainError proc[mem(normal)]`
- `generate_assembly(ir: IR, toolchain: Toolchain) -> string proc[mem(normal)]`
- `generate_apple_assembly(ir: IR) -> string proc[mem(normal)]`
- `generate_llvm_assembly(ir: IR) -> string proc[mem(normal)]`
- `invoke_clang_driver(toolchain: Toolchain, target: TargetTriple, assembly_file: string, output_file: string, flags: ListString) -> ResultUnitToolchainError proc[device_io, mem(normal)]` (optional)
- `invoke_assembler(toolchain: Toolchain, assembly_file: string, object_file: string) -> ResultUnitToolchainError proc[device_io, mem(normal)]` (optional)
- `invoke_linker(toolchain: Toolchain, object_files: ListString, output_file: string, libraries: ListString) -> ResultUnitToolchainError proc[device_io, mem(normal)]` (optional)
- `configure_apple_runtime(toolchain: Toolchain, target: TargetTriple) -> ListString proc[mem(normal)]`
- `configure_platform_runtime(toolchain: Toolchain, target: TargetTriple) -> ListString proc[mem(normal)]`

---

## Cross-References Between Phase and Component Hierarchies

### Phase 0 ↔ Component 0 (Lexer)
- **Phase 0.1.1** → **Component 0.1** (`lexer` module)
- **Phase 0.2.1** → **Component 0.1** (`lexer` module)

### Phase 1 ↔ Component 1 (Parser)
- **Phase 1.1.1** → **Component 1.1** (`ast` module)
- **Phase 1.2.1** → **Component 1.2** (`parser` module)

### Phase 2 ↔ Component 2 (Module Resolver)
- **Phase 2.1.1** → **Component 2.1** (`module_resolver` module)
- **Phase 2.2.1** → **Component 2.1** (`module_resolver` module)
- **Phase 2.3.1** → **Component 2.1** (`module_resolver` module)
- **Phase 2.4.1** → **Component 2.1** (`module_resolver` module)

### Phase 3 ↔ Component 3 (Type Checker)
- **Phase 3.1.1** → **Component 3.1** (`type_checker` module)
- **Phase 3.2.1** → **Component 3.1** (`type_checker` module)
- **Phase 3.3.1** → **Component 3.1** (`type_checker` module)
- **Phase 3.4.1** → **Component 3.1** (`type_checker` module)

### Phase 4 ↔ Component 4 (Effect Checker)
- **Phase 4.1.1** → **Component 4.1** (`effect_checker` module)
- **Phase 4.2.1** → **Component 4.1** (`effect_checker` module)
- **Phase 4.3.1** → **Component 4.1** (`effect_checker` module)

### Phase 5 ↔ Component 5 (Code Generator - LLVM)
- **Phase 5.1.1** → **Component 5.1** (`codegen` module)
- **Phase 5.2.1** → **Component 5.1** (`codegen` module)
- **Phase 5.3.1** → **Component 5.1** (`codegen` module)
- **Phase 5.4.1** → **Component 5.1** (`codegen` module)

### Phase 6 ↔ Component 6 (Region Analyzer)
- **Phase 6.1.1** → **Component 6.1** (`lifetime_env` module)
- **Phase 6.2.1** → **Component 6.3** (`region_analysis` module)
- **Phase 6.3.1** → **Component 6.3** (`region_analysis` module)
- **Phase 6.4.1** → **Component 6.3** (`region_analysis` module)

### Phase 7 ↔ Component 7 (Pattern Analyzer)
- **Phase 7.1.1** → **Component 7.1** (`exhaustiveness_checker` module)
- **Phase 7.2.1** → **Component 7.2** (`pattern_compiler` module)
- **Phase 7.3.1** → **Component 7.2** (`pattern_compiler` module)

### Phase 8 ↔ Component 8 (Hardware Detector)
- **Phase 8.1.1** → **Component 8.1** (`capability_detection` module)
- **Phase 8.2.1** → **Component 8.2** (`capability_validation` module)

### Phase 9 ↔ Component 9 (Region Optimizer)
- **Phase 9.1.1** → **Component 9.1** (`numa_optimizer` module)
- **Phase 9.1.2** → **Component 9.2** (`cache_optimizer` module)

### Phase 10 ↔ Component 10 (Actor Optimizer)
- **Phase 10.1.1** → **Component 10.1** (`actor_scheduler` module)
- **Phase 10.1.2** → **Component 10.2** (`message_optimizer` module)

### Phase 11 ↔ Component 11 (Effect Lowering)
- **Phase 11.1.1** → **Component 11.1** (`effect_lowering` module)
- **Phase 11.1.2** → **Component 11.1** (`effect_lowering` module)

### Phase 12 ↔ Component 12 (Vectorizer)
- **Phase 12.1.1** → **Component 12.1** (`sve_vectorizer` module)
- **Phase 12.2.1** → **Component 12.2** (`neon_vectorizer` module)

### Phase 13 ↔ Components 13-16 (Native Backend)
- **Phase 13.1.1** → **Component 13.1** (`instruction_selector` module)
- **Phase 13.2.1** → **Component 14.1** (`register_allocator` module)
- **Phase 13.3.1** → **Component 15.1** (`code_layout` module)
- **Phase 13.4.1** → **Component 16.1** (`link_time_optimizer` module)

### Toolchain Integration ↔ Component 17 (Toolchain Integration)
- **Task TC.2.1** → **Component 17.1** (`toolchain` module)
- **Task TC.3.1** → **Component 17.1** (`toolchain` module)
- **Task TC.4.1** → **Component 17.1** (`toolchain` module)

---

## Dependency Graph

### Phase Dependencies

```
Phase 0: Lexical Analysis
    ↓
Phase 1: Parsing
    ↓
Phase 2: Module Resolution
    ↓
Phase 3: Type Checking
    ├─→ Phase 4: Effect Checking → Phase 11: Effect Lowering → Phase 13: Native Backend
    ├─→ Phase 6: Region Analysis → Phase 9: Region Optimization
    ├─→ Phase 8: Hardware Capability Validation → Phase 12: Vectorization
    └─→ Phase 10: Actor Optimization
    ↓
Phase 1: Parsing → Phase 7: Pattern Matching Enhancement
    ↓
Phase 13: Native Backend (Instruction Selection → Register Allocation → Code Layout → Link-Time Optimization)
```

### Component Dependencies

```
lexer → parser → module_resolver → type_checker
                                    ├─→ effect_checker → effect_lowering
                                    ├─→ region_analyzer → region_optimizer
                                    └─→ hardware_detector → vectorizer
parser → pattern_analyzer
type_checker → actor_optimizer
region_analyzer → register_allocator
effect_lowering → instruction_selector
vectorizer → instruction_selector
instruction_selector → register_allocator → code_layout → link_time_optimizer
code_layout → toolchain
link_time_optimizer → toolchain
```

---

## Silica-Specific Implementation Patterns

### Pattern 1: Region-Based Memory Management
**Pattern Type**: memory_management
**Description**: Use region-based allocation for compiler data structures
**Applicable To**: All compiler components

**Pattern Structure**:
```silica
-- Allocate region for compiler analysis
analysis_region: region(R, normal) <- alloc_region(normal) proc[mem(normal)];

-- Allocate data structures in region
env: ref(R, normal, TypeEnv) <- alloc_ref(analysis_region, create_type_env()) proc[mem(normal)];

-- Use region-scoped data structures
result: Type <- check_type(expr, env) proc[mem(normal)];
```

**Usage Context**: All compiler phases that need to allocate data structures
**Benefits**: Memory safety without garbage collection, explicit lifetime management

### Pattern 2: Explicit Effect Declarations
**Pattern Type**: effect_system
**Description**: All compiler functions must explicitly declare effects
**Applicable To**: All compiler functions

**Pattern Structure**:
```silica
-- Pure function (no effects)
fn check_syntax(ast: AST) -> bool {
    -- syntax checking logic
}

-- Function with memory effects
fn analyze_types(ast: AST, env: ref(R, normal, TypeEnv)) -> Type proc[mem(normal)] {
    -- type analysis logic
}

-- Function with I/O effects
fn read_source_file(path: string) -> string proc[device_io, mem(normal)] {
    -- file reading logic
}
```

**Usage Context**: All compiler functions
**Benefits**: Explicit side effect tracking, enables effect-based optimizations

### Pattern 3: Trait-Based Polymorphism
**Pattern Type**: code_structure
**Description**: Use traits for polymorphic compiler components
**Applicable To**: Compiler component interfaces

**Pattern Structure**:
```silica
trait CompilerPhase {
    fn analyze(self: Self, input: PhaseInput) -> PhaseOutput proc[mem(normal)];
    fn get_name(self: Self) -> string;
}

impl CompilerPhase for RegionAnalyzer {
    fn analyze(self: RegionAnalyzer, input: PhaseInput) -> PhaseOutput proc[mem(normal)] {
        -- region analysis implementation
    }
    fn get_name(self: RegionAnalyzer) -> string {
        "RegionAnalysis"
    }
}
```

**Usage Context**: Compiler phase interfaces, extensible components
**Benefits**: Polymorphism without generics, explicit trait implementations

### Pattern 4: Pattern Matching for AST Traversal
**Pattern Type**: code_structure
**Description**: Use pattern matching for AST node analysis
**Applicable To**: AST analysis functions

**Pattern Structure**:
```silica
fn analyze_expression(expr: Expression) -> Type proc[mem(normal)] {
    case expr of {
        Literal(lit) -> analyze_literal(lit);
        Identifier(name) -> lookup_type(name);
        Call(call_expr) -> analyze_call(call_expr);
        _: Expression -> error("Unsupported expression");
    }
}
```

**Usage Context**: AST traversal, expression analysis
**Benefits**: Exhaustive pattern matching, clear control flow

### Pattern 5: Recursive Data Structures
**Pattern Type**: code_structure
**Description**: Use recursive types for compiler data structures
**Applicable To**: AST, type environments, analysis results

**Pattern Structure**:
```silica
enum DecisionTree {
    Leaf { result: Expression },
    Branch {
        condition: Expression,
        true_branch: DecisionTree,
        false_branch: DecisionTree,
    },
}

fn build_tree(patterns: ListPattern) -> DecisionTree proc[mem(normal)] {
    -- recursive tree construction
}
```

**Usage Context**: Tree structures, recursive algorithms
**Benefits**: Natural representation of hierarchical data, type-safe recursion

### Pattern 6: Explicit Type Annotations
**Pattern Type**: code_structure
**Description**: All function parameters and return types must be explicit
**Applicable To**: All compiler code

**Pattern Structure**:
```silica
fn optimize_region_layout(
    regions: ListRegion,
    numa_topology: NUMATopology,
    cache_hierarchy: CacheHierarchy
) -> OptimizedLayout proc[mem(normal)] {
    -- optimization logic
}
```

**Usage Context**: All compiler functions
**Benefits**: No type inference ambiguity, clear interfaces

---

## Implementation Priority

### Foundation Priority (Bootstrap Phases - Implement First)
1. **Phase 0**: Lexical Analysis Phase - Foundation for all parsing
2. **Phase 1**: Parsing Phase - Foundation for all analysis
3. **Phase 2**: Module Resolution Phase - Required for multi-module programs
4. **Phase 3**: Type Checking Phase - Required for type safety
5. **Phase 4**: Effect Checking Phase - Required for effect safety
6. **Phase 5**: Code Generation Phase (LLVM) - Required for code generation

**Note**: Implement Phases 0-5 first to match bootstrap compiler behavior, then compare outputs.

### Critical Priority (Missing Phases - Must Implement Next)
7. **Phase 6**: Region Analysis Phase - Blocks memory safety
8. **Phase 7**: Pattern Matching Exhaustiveness Enhancement - Affects correctness
9. **Phase 8**: Hardware Capability Validation - Blocks optimizations

### High Priority (Should Implement Next)
10. **Phase 9**: Region Optimization Phase - Performance impact
11. **Phase 10**: Actor Optimization Phase - Performance impact
12. **Phase 11**: Effect Lowering Phase - Blocks native backend
13. **Phase 12**: Vectorization Phase - Significant performance impact

### Medium Priority (Native Backend)
14. **Phase 13**: Native Backend Phases - Enables full native compiler

---

## Specification References Summary

- **Section 1.4**: Target Platform (AArch64 features)
- **Section 1.6**: Compiler Error Messages (error format)
- **Section 3.6**: Pattern Matching Semantics (exhaustiveness)
- **Section 9**: Effect System (effect tracking)
- **Section 10**: Type Checking (type system)
- **Section 12**: Memory Model (region-based memory)
- **Section 12.1.4**: Static Region Lifetime Analysis (lifetime rules)
- **Section 15**: Actor Model Semantics (actor system)
- **Section 16**: Message Passing (message primitives)
- **Section 19**: Module System (module resolution)
- **Section 21**: Architecture-Specific Modules (vector types)
- **Section 26**: Platform Integration (AArch64 integration)
- **Section 27**: Compilation and Linking (compilation phases)
- **Section 27.5.1**: Frontend Phases (language-centric phases)
- **Section 27.5.2**: Middle-End Phases (architecture-aware phases)
- **Section 27.5.3**: Backend Phases (chip-native phases)
- **Section 27.6.1**: Hardware-Aware Build Configuration (hardware detection)
- **Section 28**: Compiler Infrastructure (incremental compilation)

---

## Next Steps

### Step 1: Implement Bootstrap Phases (Phases 0-5)
1. Begin implementation with Phase 0 (Lexical Analysis Phase)
2. Implement Phase 1 (Parsing Phase)
3. Implement Phase 2 (Module Resolution Phase)
4. Implement Phase 3 (Type Checking Phase)
5. Implement Phase 4 (Effect Checking Phase)
6. Implement Phase 5 (Code Generation Phase - LLVM)
7. Compare Silica compiler output with bootstrap compiler output for validation
8. Ensure behavior matches bootstrap compiler exactly

### Step 2: Implement Missing Phases (Phases 6-13)
9. Implement Phase 6 (Region Analysis Phase)
10. Implement Phase 7 (Pattern Matching Exhaustiveness Enhancement)
11. Implement Phase 8 (Hardware Capability Validation)
12. Implement Phase 9 (Region Optimization Phase)
13. Implement Phase 10 (Actor Optimization Phase)
14. Implement Phase 11 (Effect Lowering Phase)
15. Implement Phase 12 (Vectorization Phase)
16. Implement Phase 13 (Native Backend Phases)

### Step 3: Validation and Optimization
17. Test each phase against specification requirements
18. Integrate all phases into compilation pipeline
19. Validate against bootstrap compiler output (for Phases 0-5)
20. Validate against specification requirements (for all phases)
21. Optimize for AArch64 hardware features

---

## Summary

This plan includes:
- **14 Phases Total**: 6 bootstrap phases (0-5) + 8 missing phases (6-13)
- **18 Components Total**: All compiler components needed for complete implementation, including toolchain integration
- **Dual Hierarchy**: Phase hierarchy and Component hierarchy with cross-references
- **Toolchain Integration**: Apple Clang and LLVM toolchain support for AArch64 targets
- **Dependencies**: All phase and component dependencies documented
- **Specification References**: Each phase/task references relevant specification sections
- **Silica Patterns**: Implementation patterns provided for all phases

The plan enables:
1. Reimplementation of bootstrap compiler functionality in Silica
2. Comparison with bootstrap compiler for validation
3. Implementation of missing phases for full specification compliance
4. Complete native AArch64 backend implementation
5. Integration with Apple Clang toolchain for Apple Silicon targets
6. Integration with LLVM toolchain for non-Apple AArch64 targets

---

*Created using AALang and Gab*
