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
    Function(FunctionDecl),
    Type(TypeDecl),
    Effect(EffectDecl),
    Module(ModuleDecl),
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
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Expression,
    pub effects: Vec<Effect>,
    pub location: SourceLocation,
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_: Type,
    pub location: SourceLocation,
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

/// Module declaration
#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub path: Vec<String>,
    pub location: SourceLocation,
}

/// Import declaration
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub location: SourceLocation,
}

/// Export declaration
#[derive(Debug, Clone)]
pub struct ExportDecl {
    pub name: String,
    pub location: SourceLocation,
}

/// Struct declaration
#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub type_params: Vec<String>, // Generic type parameters
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
    pub type_params: Vec<String>, // Generic type parameters
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

/// Trait declaration
#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub name: String,
    pub type_params: Vec<String>, // Generic type parameters
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

/// Implementation declaration
#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub trait_name: Option<String>, // None for inherent impls
    pub type_params: Vec<String>,
    pub for_type: Type,
    pub methods: Vec<FunctionDecl>,
    pub location: SourceLocation,
}

/// Type alias declaration
#[derive(Debug, Clone)]
pub struct TypeAliasDecl {
    pub name: String,
    pub type_params: Vec<String>, // Generic type parameters
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

    // Operators
    Unary(UnaryExpr),
    Binary(BinaryExpr),

    // Memory operations
    AllocRef(AllocRefExpr),
    ReadRef(ReadRefExpr),
    WriteRef(WriteRefExpr),

    // Actor operations
    Spawn(SpawnExpr),
    Send(SendExpr),
    Recv(RecvExpr),
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
    pub arguments: Vec<Expression>,
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

    // Compound types
    Tuple(Vec<Type>),
    Record(Vec<(String, Type)>),
    Variant(Vec<(String, Option<Type>)>),

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

    // Generic types
    Generic {
        name: String,
        type_args: Vec<Type>,
    },

    // Type variables (for polymorphism)
    Variable(String),

    // User-defined types
    Named(String),
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
