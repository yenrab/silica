# Silica Bootstrap Compiler Design Roadmap

## Executive Summary

This document outlines the detailed implementation plan for the Silica bootstrap compiler. The bootstrap compiler is a minimal but complete Silica compiler written in Rust with LLVM backend, capable of compiling itself and enabling self-hosting.

**Goals:**
- Implement Phase 1 bootstrap compiler from bootstrapping.md
- Support minimal Silica subset for self-hosting
- Generate efficient LLVM IR with memory safety guarantees
- Provide foundation for Phase 2 complete Silica compiler

## Architecture Overview

```
Frontend (Silica) → Lexer → Parser → AST → Type Check → Effect Check → LLVM IR → Executable
     ↑                                                                        ↓
Runtime System ←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←
```

**Core Components:**
1. **Lexer** (`lexer.rs`) - UTF-8 lexical analysis
2. **Parser** (`parser.rs`) - Recursive descent parsing
3. **AST** (`ast.rs`) - Abstract syntax tree definitions
4. **Types** (`types.rs`) - Type system and checking
5. **Effects** (`effects.rs`) - Effect system implementation
6. **Codegen** (`codegen.rs`) - LLVM IR generation
7. **Runtime** (`runtime.rs`) - Minimal runtime components

## 1. Lexer Implementation Plan

### 1.1 Core Data Structures

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords (15 total)
    Fn, Let, If, Else, Type, Effect, Proc, Mem, Ref, Region,
    Actor, Spawn, Send, Recv, Self_, True, False, Unit,

    // Literals
    Integer(i64),
    String(String),
    Char(char),

    // Identifiers
    Identifier(String),

    // Operators (15 total)
    Plus, Minus, Star, Slash, Percent, Equal, NotEqual,
    Less, LessEqual, Greater, GreaterEqual, And, Or, Not,
    Assign, Arrow,

    // Punctuation
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Comma, Semicolon, Colon, DoubleColon, Dot, Pipe,

    // Special
    EOF,
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}
```

### 1.2 Lexer Implementation

```rust
pub struct Lexer {
    source: String,
    chars: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    file: String,
}

impl Lexer {
    pub fn new(source: String, file: String) -> Self { ... }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> { ... }

    // Core tokenization methods
    fn next_token(&mut self) -> Result<Token, LexerError> { ... }
    fn read_identifier(&mut self) -> String { ... }
    fn read_number(&mut self) -> Result<i64, LexerError> { ... }
    fn read_string(&mut self) -> Result<String, LexerError> { ... }
    fn read_char(&mut self) -> Result<char, LexerError> { ... }
    fn skip_whitespace(&mut self) { ... }
    fn skip_comment(&mut self) { ... }
}
```

### 1.3 Error Recovery Strategy

- **Panic Mode**: Skip tokens until synchronization point (statement boundary)
- **Error Tokens**: Continue parsing with error markers
- **Context Preservation**: Maintain source locations for all tokens
- **Diagnostic Quality**: Provide clear error messages with suggestions

### 1.4 Performance Targets

- **Throughput**: < 1ms per 1K lines of code
- **Memory**: O(n) space complexity
- **UTF-8**: Full Unicode support with efficient decoding

## 2. Parser Implementation Plan

### 2.1 Parser Architecture

```rust
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self { ... }

    pub fn parse(&mut self) -> Result<Vec<Declaration>, ParseError> { ... }

    // Core parsing methods (25+ functions)
    fn parse_declaration(&mut self) -> Result<Declaration, ParseError> { ... }
    fn parse_function_declaration(&mut self) -> Result<FunctionDecl, ParseError> { ... }
    fn parse_type_declaration(&mut self) -> Result<TypeDecl, ParseError> { ... }
    fn parse_expression(&mut self) -> Result<Expression, ParseError> { ... }
    fn parse_if_expression(&mut self) -> Result<IfExpr, ParseError> { ... }
    fn parse_function_call(&mut self) -> Result<CallExpr, ParseError> { ... }
    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> { ... }
}
```

### 2.2 Grammar Implementation

**Expression Precedence (from lowest to highest):**
1. Assignment (`<-`)
2. Logical OR (`or`)
3. Logical AND (`and`)
4. Comparison (`==`, `!=`, `<`, `<=`, `>`, `>=`)
5. Arithmetic (`+`, `-`)
6. Multiplicative (`*`, `/`, `%`)
7. Unary (`not`, `-`)
8. Function application
9. Literals, identifiers, grouping

**Declaration Types:**
- Function declarations: `fn name(params) [: return_type] { body }`
- Type declarations: `type Name = TypeDefinition`
- Effect declarations: `effect Name = [effects]`
- Module declarations: `use module path`

### 2.3 Error Handling Strategy

- **Synchronized Recovery**: Skip to next statement boundary
- **Error Propagation**: Continue parsing to find more errors
- **Context Tracking**: Maintain parse context for better messages
- **Recovery Hints**: Suggest likely corrections

### 2.4 AST Construction

Direct AST construction during parsing with:
- Source location tracking
- Symbol table population
- Type annotation collection
- Effect specification parsing

## 3. AST and Type System Implementation Plan

### 3.1 AST Node Definitions (20+ Types)

```rust
#[derive(Debug, Clone)]
pub enum Declaration {
    Function(FunctionDecl),
    Type(TypeDecl),
    Effect(EffectDecl),
    Module(ModuleDecl),
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Expression,
    pub effects: Vec<Effect>,
    pub location: SourceLocation,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    Binary(BinaryExpr),
    If(IfExpr),
    Call(CallExpr),
    Do(DoExpr),
    // ... 15+ expression variants
}

#[derive(Debug, Clone)]
pub enum Type {
    Primitive(PrimitiveType),
    Function(FunctionType),
    Tuple(Vec<Type>),
    Record(Vec<Field>),
    Variant(Vec<Variant>),
    Process(ProcessType),
    Region(RegionType),
    Reference(ReferenceType),
}

#[derive(Debug, Clone)]
pub enum Effect {
    Memory(MemorySpace),
    Mailbox(MessageType),
    Concurrency,
    Atomic,
    DeviceIO,
}
```

### 3.2 Type Checking Implementation

**Hindley-Milner Inference Algorithm:**
```rust
pub struct TypeChecker {
    environment: HashMap<String, TypeScheme>,
    constraints: Vec<Constraint>,
    substitution: Substitution,
    effect_context: EffectContext,
}

impl TypeChecker {
    pub fn check_program(&mut self, declarations: &[Declaration]) -> Result<(), TypeError> {
        for decl in declarations {
            self.check_declaration(decl)?;
        }
        self.solve_constraints()?;
        Ok(())
    }

    fn check_expression(&mut self, expr: &Expression) -> Result<(Type, EffectSet), TypeError> { ... }
    fn unify(&mut self, t1: &Type, t2: &Type) -> Result<(), TypeError> { ... }
    fn generalize(&self, ty: &Type) -> TypeScheme { ... }
    fn instantiate(&self, scheme: &TypeScheme) -> Type { ... }
}
```

**Type Checking Rules:**
- Expression typing with effect tracking
- Function application with effect union
- Process creation with effect capture
- Pattern matching exhaustiveness
- Region and reference safety

### 3.3 Effect System

**Effect Checking:**
```rust
pub struct EffectChecker {
    active_effects: HashSet<Effect>,
    capability_stack: Vec<Capability>,
}

impl EffectChecker {
    pub fn check_effect_safety(&self, required: &EffectSet) -> Result<(), EffectError> { ... }
    pub fn with_capability<F, R>(&mut self, effect: Effect, f: F) -> R
    where F: FnOnce() -> R { ... }
}
```

### 3.4 Visitor Pattern for Code Generation

```rust
pub trait AstVisitor<T> {
    fn visit_program(&mut self, program: &Program) -> T;
    fn visit_declaration(&mut self, decl: &Declaration) -> T;
    fn visit_expression(&mut self, expr: &Expression) -> T;
    // ... specific visit methods for each AST node
}

pub trait AstTransformer {
    fn transform_program(&mut self, program: &mut Program);
    fn transform_expression(&mut self, expr: &mut Expression);
    // ... transformation methods
}
```

## 4. LLVM Code Generation Implementation Plan

### 4.1 LLVM Integration Architecture

```rust
pub struct CodeGenerator {
    context: LLVMContext,
    module: LLVMModule,
    builder: LLVMBuilder,
    named_values: HashMap<String, LLVMValue>,
    type_map: TypeMap,
    effect_runtime: EffectRuntime,
}

impl CodeGenerator {
    pub fn new(module_name: &str) -> Self { ... }

    pub fn generate_program(&mut self, program: &Program) -> Result<(), CodegenError> { ... }

    pub fn write_to_file(&self, filename: &str) -> Result<(), CodegenError> { ... }
}
```

### 4.2 Type Mapping (Silica → LLVM)

```rust
pub struct TypeMap {
    // Primitive mappings
    int_type: LLVMType,      // i64
    bool_type: LLVMType,     // i1
    char_type: LLVMType,     // i32 (Unicode scalar)
    unit_type: LLVMType,     // void

    // Compound type builders
    function_types: HashMap<FunctionSignature, LLVMType>,
    struct_types: HashMap<String, LLVMType>,
}

impl TypeMap {
    pub fn silica_to_llvm(&self, silica_type: &Type) -> LLVMType { ... }
    pub fn llvm_to_silica(&self, llvm_type: LLVMType) -> Option<Type> { ... }
}
```

### 4.3 Memory Model Implementation

**Region Management:**
```rust
pub struct RegionManager {
    regions: HashMap<String, LLVMValue>, // region name -> allocation
    alloc_function: LLVMValue,
    dealloc_function: LLVMValue,
}

impl RegionManager {
    pub fn allocate_region(&mut self, name: &str, space: MemorySpace) -> LLVMValue { ... }
    pub fn allocate_reference(&mut self, region: LLVMValue, value: LLVMValue) -> LLVMValue { ... }
    pub fn load_reference(&self, reference: LLVMValue) -> LLVMValue { ... }
    pub fn store_reference(&self, reference: LLVMValue, value: LLVMValue) { ... }
}
```

### 4.4 Expression Code Generation

**Core Generation Methods:**
```rust
impl CodeGenerator {
    fn generate_literal(&mut self, lit: &Literal) -> LLVMValue { ... }
    fn generate_binary_op(&mut self, op: BinaryOp, lhs: LLVMValue, rhs: LLVMValue) -> LLVMValue { ... }
    fn generate_if_expression(&mut self, cond: LLVMValue, then_expr: LLVMValue, else_expr: LLVMValue) -> LLVMValue { ... }
    fn generate_function_call(&mut self, func: LLVMValue, args: &[LLVMValue]) -> LLVMValue { ... }
    fn generate_do_expression(&mut self, statements: &[Statement]) -> LLVMValue { ... }
}
```

### 4.5 Control Flow and Functions

**Function Generation:**
```rust
impl CodeGenerator {
    fn generate_function(&mut self, func_decl: &FunctionDecl) -> Result<LLVMValue, CodegenError> {
        let function = self.create_function(func_decl)?;
        let entry_bb = self.create_basic_block("entry")?;

        self.builder.position_at_end(entry_bb);

        // Generate function body
        let body_value = self.generate_expression(&func_decl.body)?;

        // Handle return
        if func_decl.return_type == Some(Type::Unit) {
            self.builder.build_ret_void();
        } else {
            self.builder.build_ret(body_value);
        }

        Ok(function)
    }
}
```

### 4.6 Effect Runtime Integration

```rust
pub struct EffectRuntime {
    capability_check_function: LLVMValue,
    effect_stack: LLVMValue,
    memory_capability: LLVMValue,
    concurrency_capability: LLVMValue,
}

impl EffectRuntime {
    pub fn check_effect(&self, effect: &Effect) -> LLVMValue { ... }
    pub fn push_effect_context(&self, effects: &EffectSet) { ... }
    pub fn pop_effect_context(&self) { ... }
}
```

## 5. Runtime System Implementation Plan

### 5.1 Memory Management Runtime

**Region Allocator:**
```rust
#[no_mangle]
pub extern "C" fn silica_alloc_region(space: MemorySpace) -> *mut Region {
    match space {
        MemorySpace::Normal => {
            // Allocate region structure
            let region = Box::new(Region {
                allocations: Vec::new(),
                capacity: INITIAL_REGION_SIZE,
                used: 0,
            });
            Box::into_raw(region)
        }
        MemorySpace::Atomic => {
            // Atomic memory allocation
            // ...
        }
    }
}
```

**Reference Operations:**
```rust
#[no_mangle]
pub extern "C" fn silica_alloc_ref(region: *mut Region, value: Value) -> *mut Reference {
    unsafe {
        (*region).allocate_ref(value)
    }
}

#[no_mangle]
pub extern "C" fn silica_read_ref(reference: *const Reference) -> Value {
    unsafe {
        (*reference).read()
    }
}

#[no_mangle]
pub extern "C" fn silica_write_ref(reference: *mut Reference, value: Value) {
    unsafe {
        (*reference).write(value);
    }
}
```

### 5.2 Effect System Runtime

**Capability Checking:**
```rust
pub struct EffectContext {
    active_effects: HashSet<Effect>,
    capability_tokens: HashMap<Effect, usize>,
}

#[no_mangle]
pub extern "C" fn silica_check_effect(effect: Effect) -> bool {
    EFFECT_CONTEXT.with(|ctx| {
        ctx.borrow().active_effects.contains(&effect)
    })
}

#[no_mangle]
pub extern "C" fn silica_push_effect_context(effects: *const EffectSet) {
    // Push new effect context
}

#[no_mangle]
pub extern "C" fn silica_pop_effect_context() {
    // Pop effect context
}
```

### 5.3 Actor System (Minimal)

**Basic Actor Support:**
```rust
pub struct Actor {
    mailbox: VecDeque<Message>,
    behavior: BehaviorFunction,
    state: Value,
}

#[no_mangle]
pub extern "C" fn silica_spawn_actor(initial_state: Value, behavior: BehaviorFunction) -> ActorRef {
    // Create new actor
}

#[no_mangle]
pub extern "C" fn silica_send_message(actor: ActorRef, message: Message) {
    // Send message to actor mailbox
}

#[no_mangle]
pub extern "C" fn silica_receive_message() -> Message {
    // Blocking receive from current actor's mailbox
}
```

### 5.4 Runtime Integration

**Startup and Shutdown:**
```rust
#[no_mangle]
pub extern "C" fn silica_runtime_init() {
    // Initialize memory manager
    // Set up effect system
    // Start actor scheduler
}

#[no_mangle]
pub extern "C" fn silica_runtime_shutdown() {
    // Clean up resources
    // Stop actor scheduler
    // Free memory regions
}
```

## 6. Integration and Testing Plan

### 6.1 Component Integration

**Compilation Pipeline:**
```rust
pub fn compile_silica(source: &str, output_file: &str) -> Result<(), CompilerError> {
    // 1. Lexical analysis
    let tokens = Lexer::new(source.to_string(), "input.silica".to_string())
        .tokenize()?;

    // 2. Parsing
    let declarations = Parser::new(tokens).parse()?;

    // 3. Type checking
    let mut type_checker = TypeChecker::new();
    type_checker.check_program(&declarations)?;

    // 4. Code generation
    let mut codegen = CodeGenerator::new("silica_module")?;
    codegen.generate_program(&declarations)?;
    codegen.write_to_file(output_file)?;

    Ok(())
}
```

### 6.2 Testing Strategy

**Specification Compliance Tests:**
- Parse all examples from silica-specification.md
- Type check specification examples
- Generate and execute test programs
- Validate memory safety guarantees

**Self-Hosting Validation:**
- Bootstrap compiler compiles itself
- Generated code matches expected output
- Performance within acceptable range
- Memory usage verification

**Integration Tests:**
- End-to-end compilation pipeline
- Runtime system validation
- Actor system functionality
- Effect safety enforcement

### 6.3 Performance Benchmarks

**Compilation Speed:**
- Target: < 100ms per 1000 lines
- Measure: Lexer, parser, type checking, code generation times

**Runtime Performance:**
- Memory allocation/deallocation overhead
- Function call performance
- Actor messaging latency
- Effect checking overhead

## 7. Development Roadmap

### Phase 1A: Core Infrastructure (Weeks 1-2)
- [ ] Set up Rust project with LLVM dependencies
- [ ] Implement basic lexer with token definitions
- [ ] Create AST node definitions
- [ ] Set up error handling framework

### Phase 1B: Frontend Implementation (Weeks 3-8)
- [ ] Complete lexer with UTF-8 support
- [ ] Implement recursive descent parser
- [ ] Add type checking infrastructure
- [ ] Implement effect system

### Phase 1C: Backend Integration (Weeks 9-14)
- [ ] LLVM IR generation for expressions
- [ ] Memory model implementation
- [ ] Function and control flow generation
- [ ] Runtime system integration

### Phase 1D: Validation and Optimization (Weeks 15-16)
- [ ] Self-hosting validation
- [ ] Performance optimization
- [ ] Comprehensive testing
- [ ] Documentation completion

## 8. Risk Mitigation

### Technical Risks
- **LLVM Integration Complexity**: Mitigated by using well-tested llvm-sys crate
- **Type System Correctness**: Comprehensive test suite against specification
- **Memory Safety**: Region-based design with runtime checks
- **Performance**: Incremental optimization with profiling

### Schedule Risks
- **Underestimation**: 25% buffer added to all estimates
- **Dependency Issues**: Use stable Rust ecosystem components
- **Self-Hosting Complexity**: Validate each increment before proceeding

### Quality Assurance
- **Specification Compliance**: Regular validation against silica-specification.md
- **Code Reviews**: All components reviewed for correctness
- **Automated Testing**: 100% test coverage for critical paths
- **Performance Monitoring**: Continuous benchmarking against targets

## Conclusion

This implementation plan provides a complete roadmap for building the Silica bootstrap compiler. The modular architecture ensures maintainability, while the detailed specifications enable systematic implementation and validation. The design balances minimalism for bootstrap purposes with completeness required for self-hosting.

The bootstrap compiler will serve as the foundation for the complete Silica compiler, enabling the transition from Rust implementation to self-hosted Silica compiler in Phase 2.
