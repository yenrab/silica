use crate::errors::SourceLocation;

/// Program is the top-level AST node containing all declarations
#[derive(Debug, Clone)]
pub struct Program {
    pub declarations: Vec<Declaration>,
    pub location: SourceLocation,
}

/// Declaration represents top-level declarations in Silica
#[derive(Debug, Clone)]
pub enum Declaration {
    Function(FunctionDecl), // includes extern functions
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

/// Function declaration
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>, // Generic type parameters with bounds
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub where_clause: Option<WhereClause>,
    pub body: Vec<Statement>,
    pub effects: Vec<Effect>,
    pub location: SourceLocation,
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_: Type,
    pub location: SourceLocation,
    pub pattern: Option<Pattern>,
}

/// Type declaration
#[derive(Debug, Clone)]
pub struct TypeDecl {
    pub name: String,
    pub type_: Type,
    pub location: SourceLocation,
}

/// Effect declaration
#[derive(Debug, Clone)]
pub struct EffectDecl {
    pub name: String,
    pub effects: Vec<Effect>,
    pub location: SourceLocation,
}

/// Export item with name and arity
#[derive(Debug, Clone)]
pub struct ExportItem {
    pub name: String,
    pub arity: u32,
    pub location: SourceLocation,
}

/// Import declaration
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub modules: Vec<String>,
    pub location: SourceLocation,
}

/// Export declaration
#[derive(Debug, Clone)]
pub struct ExportDecl {
    pub items: Vec<ExportItem>,
    pub location: SourceLocation,
}

/// Struct declaration
#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>, // Generic type parameters with bounds
    pub fields: Vec<StructField>,
    pub location: SourceLocation,
}

/// Struct field
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
    pub location: SourceLocation,
}

/// Enum declaration
#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>, // Generic type parameters with bounds
    pub variants: Vec<EnumVariant>,
    pub location: SourceLocation,
}

/// Enum variant
#[derive(Debug, Clone)]
pub enum EnumVariant {
    Unit { name: String, location: SourceLocation },
    Tuple { name: String, fields: Vec<Type>, location: SourceLocation },
    Struct { name: String, fields: Vec<StructField>, location: SourceLocation },
}

/// Associated type declaration in traits
#[derive(Debug, Clone)]
pub struct AssociatedType {
    pub name: String,
    pub bounds: Vec<String>, // Trait bounds like ["Eq", "Ord"]
    pub location: SourceLocation,
}

/// Trait declaration
#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>, // Generic type parameters with bounds
    pub associated_types: Vec<AssociatedType>,
    pub methods: Vec<TraitMethod>,
    pub location: SourceLocation,
}

/// Trait method
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub location: SourceLocation,
}

/// Associated type definition in impl blocks
#[derive(Debug, Clone)]
pub struct AssociatedTypeDef {
    pub name: String,
    pub type_: Type,
    pub location: SourceLocation,
}

/// Type parameter with optional trait bounds
#[derive(Debug, Clone)]
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<TraitBound>, // Trait bounds like T: Eq + Ord
}

impl PartialEq for TypeParam {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
        // For now, we only compare names for equality
        // Bounds comparison would be more complex
    }
}

/// Trait bound for generic constraints
#[derive(Debug, Clone)]
pub struct TraitBound {
    pub trait_name: String,
    pub type_args: Vec<Type>, // For generic traits like Iterator<Item = T>
}

/// Where clause predicate
#[derive(Debug, Clone)]
pub enum WherePredicate {
    TraitBound {
        type_: Type,
        bounds: Vec<TraitBound>,
    },
}

/// Where clause
#[derive(Debug, Clone)]
pub struct WhereClause {
    pub predicates: Vec<WherePredicate>,
}

/// Implementation declaration
#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub trait_name: Option<String>, // None for inherent impls
    pub type_params: Vec<TypeParam>,
    pub for_type: Type,
    pub associated_types: Vec<AssociatedTypeDef>,
    pub methods: Vec<FunctionDecl>,
    pub location: SourceLocation,
}

/// Type alias declaration
#[derive(Debug, Clone)]
pub struct TypeAliasDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>, // Generic type parameters with bounds
    pub aliased_type: Type,
    pub location: SourceLocation,
}

/// Expression represents all expression forms in Silica
#[derive(Debug, Clone)]
pub enum Expression {
    // Literals
    Literal(Literal),

    // Identifiers and references
    Identifier(String),

    // Control flow
    If(IfExpr),
    Case(CaseExpr),
    Do(DoExpr),

    // Function calls and applications
    Call(CallExpr),
    FunctionLiteral(FunctionLiteralExpr),

    // Operators
    Unary(UnaryExpr),
    Binary(BinaryExpr),

    // Memory operations
    Region(RegionExpr),
    AllocRef(AllocRefExpr),
    ReadRef(ReadRefExpr),
    WriteRef(WriteRefExpr),

    // Actor operations
    Spawn(SpawnExpr),
    Send(SendExpr),
    Recv(RecvExpr),

    // File I/O operations
    ReadFile(ReadFileExpr),
    WriteFile(WriteFileExpr),

    // Print operations
    Print(PrintExpr),
    PrintLn(PrintLnExpr),
    PrintInt(PrintIntExpr),
    PrintBool(PrintBoolExpr),
    PrintChar(PrintCharExpr),
    GetCpuTopologyInfo(GetCpuTopologyInfoExpr),

    // I/O operations
    ReadLines(ReadLinesExpr),
    AppendFile(AppendFileExpr),
    FileExists(FileExistsExpr),
    DeleteFile(DeleteFileExpr),
    GetFileSize(GetFileSizeExpr),
    CreateDirectory(CreateDirectoryExpr),
    RemoveDirectory(RemoveDirectoryExpr),
    ListDirectory(ListDirectoryExpr),

    // Process execution operations
    ExecCommand(ExecCommandExpr),

    // Data structures
    StructLiteral(StructLiteralExpr),
    FieldAccess(FieldAccessExpr),
    Tuple(Vec<Expression>), // Tuple literals: (expr1, expr2, ...)

    // Generic types
    GenericInstantiation(GenericInstantiationExpr),
    ConstructorCall(ConstructorCallExpr),
}

/// Literal values
#[derive(Debug, Clone)]
pub enum Literal {
    Unit,
    Bool(bool),
    Int(i64),
    Char(char),
    String(String),
}

/// If expression
#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: Box<Expression>,
    pub then_branch: Box<Expression>,
    pub else_branch: Box<Expression>,
    pub location: SourceLocation,
}

/// Case expression for pattern matching
#[derive(Debug, Clone)]
pub struct CaseExpr {
    pub scrutinee: Box<Expression>,
    pub branches: Vec<CaseBranch>,
    pub location: SourceLocation,
}

/// Case branch
#[derive(Debug, Clone)]
pub struct CaseBranch {
    pub pattern: Pattern,
    pub guard: Option<Box<Expression>>, // Optional guard expression
    pub body: Box<Expression>,
    pub location: SourceLocation,
}

/// Do expression (monadic sequencing)
#[derive(Debug, Clone)]
pub struct DoExpr {
    pub statements: Vec<Statement>,
    pub location: SourceLocation,
}

/// Statement in do expression
#[derive(Debug, Clone)]
pub enum Statement {
    Bind { pattern: Pattern, expr: Box<Expression> },
    Expr(Box<Expression>),
}

/// Function call expression
#[derive(Debug, Clone)]
pub struct CallExpr {
    pub function: Box<Expression>,
    pub type_args: Vec<Type>, // Optional type arguments for generic calls
    pub arguments: Vec<Expression>,
    pub location: SourceLocation,
}

/// Function literal expression (lambda/anonymous function)
#[derive(Debug, Clone)]
pub struct FunctionLiteralExpr {
    pub type_params: Vec<TypeParam>, // Generic type parameters with bounds
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub where_clause: Option<WhereClause>,
    pub body: Vec<Statement>,
    pub effects: Vec<Effect>,
    pub captured_vars: Vec<String>, // Variables captured from outer scope
    pub location: SourceLocation,
}

/// Unary expression
#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub operator: UnaryOp,
    pub operand: Box<Expression>,
    pub location: SourceLocation,
}

/// Binary expression
#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub left: Box<Expression>,
    pub operator: BinaryOp,
    pub right: Box<Expression>,
    pub location: SourceLocation,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Negate,
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    // Logical
    And,
    Or,
}

/// Region creation expression
#[derive(Debug, Clone)]
pub struct RegionExpr {
    pub space: MemorySpace,
    pub location: SourceLocation,
}

/// Memory allocation expression
#[derive(Debug, Clone)]
pub struct AllocRefExpr {
    pub region: Box<Expression>,
    pub initial_value: Box<Expression>,
    pub location: SourceLocation,
}

/// Reference read expression
#[derive(Debug, Clone)]
pub struct ReadRefExpr {
    pub reference: Box<Expression>,
    pub location: SourceLocation,
}

/// Reference write expression
#[derive(Debug, Clone)]
pub struct WriteRefExpr {
    pub reference: Box<Expression>,
    pub value: Box<Expression>,
    pub location: SourceLocation,
}

/// Actor spawn expression
#[derive(Debug, Clone)]
pub struct SpawnExpr {
    pub initial_state: Box<Expression>,
    pub behavior: Box<Expression>,
    pub core_affinity: Option<Box<Expression>>, // Optional core affinity specification
    pub location: SourceLocation,
}

/// Message send expression
#[derive(Debug, Clone)]
pub struct SendExpr {
    pub actor: Box<Expression>,
    pub message: Box<Expression>,
    pub location: SourceLocation,
}

/// Message receive expression
#[derive(Debug, Clone)]
pub struct RecvExpr {
    pub actor: Option<Box<Expression>>, // Optional actor to receive from
    pub location: SourceLocation,
}

/// File read expression: read_file(path)
#[derive(Debug, Clone)]
pub struct ReadFileExpr {
    pub path: Box<Expression>,
    pub location: SourceLocation,
}

/// File write expression: write_file(path, content)
#[derive(Debug, Clone)]
pub struct WriteFileExpr {
    pub path: Box<Expression>,
    pub content: Box<Expression>,
    pub location: SourceLocation,
}

/// Print expression: print(value)
#[derive(Debug, Clone)]
pub struct PrintExpr {
    pub value: Box<Expression>,
    pub location: SourceLocation,
}

/// Print line expression: println(value)
#[derive(Debug, Clone)]
pub struct PrintLnExpr {
    pub value: Box<Expression>,
    pub location: SourceLocation,
}

/// Print int expression: print_int(value)
#[derive(Debug, Clone)]
pub struct PrintIntExpr {
    pub value: Box<Expression>,
    pub location: SourceLocation,
}

/// Print bool expression: print_bool(value)
#[derive(Debug, Clone)]
pub struct PrintBoolExpr {
    pub value: Box<Expression>,
    pub location: SourceLocation,
}

/// Print char expression: print_char(value)
#[derive(Debug, Clone)]
pub struct PrintCharExpr {
    pub value: Box<Expression>,
    pub location: SourceLocation,
}

/// Get CPU topology info expression: get_cpu_topology_info()
#[derive(Debug, Clone)]
pub struct GetCpuTopologyInfoExpr {
    pub location: SourceLocation,
}

/// Read lines expression: read_lines(path)
#[derive(Debug, Clone)]
pub struct ReadLinesExpr {
    pub path: Box<Expression>,
    pub location: SourceLocation,
}

/// Append file expression: append_file(path, content)
#[derive(Debug, Clone)]
pub struct AppendFileExpr {
    pub path: Box<Expression>,
    pub content: Box<Expression>,
    pub location: SourceLocation,
}

/// File exists expression: file_exists(path)
#[derive(Debug, Clone)]
pub struct FileExistsExpr {
    pub path: Box<Expression>,
    pub location: SourceLocation,
}

/// Delete file expression: delete_file(path)
#[derive(Debug, Clone)]
pub struct DeleteFileExpr {
    pub path: Box<Expression>,
    pub location: SourceLocation,
}

/// Get file size expression: get_file_size(path)
#[derive(Debug, Clone)]
pub struct GetFileSizeExpr {
    pub path: Box<Expression>,
    pub location: SourceLocation,
}

/// Create directory expression: create_directory(path)
#[derive(Debug, Clone)]
pub struct CreateDirectoryExpr {
    pub path: Box<Expression>,
    pub location: SourceLocation,
}

/// Remove directory expression: remove_directory(path)
#[derive(Debug, Clone)]
pub struct RemoveDirectoryExpr {
    pub path: Box<Expression>,
    pub location: SourceLocation,
}

/// List directory expression: list_directory(path)
#[derive(Debug, Clone)]
pub struct ListDirectoryExpr {
    pub path: Box<Expression>,
    pub location: SourceLocation,
}

/// Execute command expression: exec_command(command, args)
#[derive(Debug, Clone)]
pub struct ExecCommandExpr {
    pub command: Box<Expression>,
    pub args: Vec<Expression>,
    pub location: SourceLocation,
}

/// Struct literal expression: TypeName { field: value, ... }
#[derive(Debug, Clone)]
pub struct StructLiteralExpr {
    pub type_name: String,
    pub fields: Vec<(String, Expression)>,
    pub location: SourceLocation,
}

/// Field access expression: object.field
#[derive(Debug, Clone)]
pub struct FieldAccessExpr {
    pub object: Box<Expression>,
    pub field: String,
    pub location: SourceLocation,
}

/// Generic instantiation expression: TypeName<Arg1, Arg2>(value)
/// Creates a value of a generic type, e.g., Some(42) for Option<int>
#[derive(Debug, Clone)]
pub struct GenericInstantiationExpr {
    pub type_name: String,
    pub type_args: Vec<Type>,
    pub payload: Option<Box<Expression>>,
    pub location: SourceLocation,
}

/// Constructor call expression: TypeName::Constructor<Args>(payload)
/// Creates a value using a constructor, e.g., Option::Some<int>(42)
#[derive(Debug, Clone)]
pub struct ConstructorCallExpr {
    pub type_name: String,
    pub constructor: String,
    pub type_args: Vec<Type>,
    pub payload: Option<Box<Expression>>,
    pub location: SourceLocation,
}


/// Pattern for pattern matching
#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(Literal),
    Identifier(String),
    Wildcard,
    Tuple(Vec<Pattern>),
    Record(Vec<(String, Pattern)>),
    Variant { constructor: String, payload: Option<Box<Pattern>> },
    GenericVariant { constructor: String, type_args: Vec<Type>, payload: Option<Box<Pattern>> },
    Alternative(Vec<Pattern>), // Pattern alternatives: pat1 | pat2 | pat3
}

/// Type system representation
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Primitive types
    Unit,
    Bool,
    Int,
    Char,
    String,

    // Function types
    Function {
        parameters: Vec<Type>,
        return_type: Box<Type>,
    },
    // Closure types (functions with captured environment)
    Closure {
        parameters: Vec<Type>,
        return_type: Box<Type>,
        captured_types: Vec<Type>, // Types of captured variables
    },
    // Polymorphic function types (higher-order)
    PolymorphicFunction {
        type_params: Vec<TypeParam>,  // Type parameters with bounds
        parameters: Vec<Type>,
        return_type: Box<Type>,
    },

    // Compound types
    Tuple(Vec<Type>),
    Record(Vec<(String, Type)>),
    Variant(Vec<(String, Option<Type>)>),
    Sum(Vec<Type>), // Sum types: A | B | C

    // Process types (monadic)
    Process {
        effects: Vec<Effect>,
        result_type: Box<Type>,
    },

    // Memory types
    Region {
        space: MemorySpace,
    },
    Reference {
        region: Box<Type>,
        space: MemorySpace,
        element_type: Box<Type>,
    },
    Buffer {
        region: Box<Type>,
        space: MemorySpace,
        element_type: Box<Type>,
        capacity: usize,
    },

    // Actor types
    ActorRef {
        message_type: Box<Type>,
    },

    // Core affinity types
    CoreId,        // Single CPU core identifier
    CoreSet(Vec<u32>), // Set of CPU cores
    AnyCore,       // Default: any available core
    PerformanceCores, // Built-in: high-performance cores
    EfficiencyCores,  // Built-in: low-power efficiency cores

    // Generic types
    Generic {
        name: String,
        type_args: Vec<Type>,
    },

    // Type variables (for polymorphism)
    Variable(String),

    // User-defined types
    Named(String),

    // Type schemes (polymorphic types with quantifiers)
    Scheme {
        vars: Vec<String>,
        ty: Box<Type>,
    },

    // Advanced type features
    // Type operators (type-level functions)
    TypeOperator {
        name: String,
        args: Vec<Type>,
    },

    // Existential types (exists T. Type)
    Existential {
        var: String,
        body: Box<Type>,
    },

    // Type application (applying type constructors)
    TypeApplication {
        constructor: Box<Type>,
        args: Vec<Type>,
    },
}

/// Memory spaces for region-based memory management
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemorySpace {
    Normal,
    Atomic,
}

/// Effect system representation
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    // Built-in effects
    Memory(MemorySpace),
    Mailbox(Box<Type>), // Message type
    Concurrency,
    Atomic,
    DeviceIO,

    // User-defined effects
    Named(String),

    // Higher-order effects (effects that take parameters)
    Parametric(String, Vec<Type>), // Effect name with type parameters
}

/// Visitor pattern for AST traversal
pub trait AstVisitor<T> {
    fn visit_program(&mut self, program: &Program) -> T;
    fn visit_declaration(&mut self, decl: &Declaration) -> T;
    fn visit_expression(&mut self, expr: &Expression) -> T;
    fn visit_type(&mut self, ty: &Type) -> T;
    fn visit_pattern(&mut self, pattern: &Pattern) -> T;
}

/// Default implementations for visitor methods
impl<T> AstVisitor<T> for () {
    fn visit_program(&mut self, _program: &Program) -> T {
        unimplemented!()
    }

    fn visit_declaration(&mut self, _decl: &Declaration) -> T {
        unimplemented!()
    }

    fn visit_expression(&mut self, _expr: &Expression) -> T {
        unimplemented!()
    }

    fn visit_type(&mut self, _ty: &Type) -> T {
        unimplemented!()
    }

    fn visit_pattern(&mut self, _pattern: &Pattern) -> T {
        unimplemented!()
    }
}

/// AST transformer trait
pub trait AstTransformer {
    fn transform_program(&mut self, program: &mut Program);
    fn transform_declaration(&mut self, decl: &mut Declaration);
    fn transform_expression(&mut self, expr: &mut Expression);
    fn transform_type(&mut self, ty: &mut Type);
    fn transform_pattern(&mut self, pattern: &mut Pattern);
}
