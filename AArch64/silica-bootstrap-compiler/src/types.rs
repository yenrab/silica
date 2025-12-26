use crate::ast::*;
use crate::errors::{Result, type_error, SourceLocation};
use std::collections::HashMap;

/// Type variable for polymorphism
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar(String);

impl TypeVar {
    pub fn new(name: String) -> Self {
        TypeVar(name)
    }

    pub fn fresh() -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        TypeVar(format!("t{}", id))
    }
}

/// Type scheme (forall quantification)
#[derive(Debug, Clone)]
pub struct TypeScheme {
    pub vars: Vec<TypeVar>,
    pub ty: Type,
}

/// Substitution mapping type variables to types
pub type Substitution = HashMap<String, Type>;

/// Type environment mapping identifiers to type schemes
pub type TypeEnv = HashMap<String, TypeScheme>;

/// Type constraint
#[derive(Debug, Clone)]
pub struct Constraint(pub Type, pub Type);

/// Type checker implementing Hindley-Milner inference
pub struct TypeChecker {
    env: TypeEnv,
    constraints: Vec<Constraint>,
    substitution: Substitution,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut env = TypeEnv::new();

        // Add built-in types
        env.insert("int".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Int,
        });
        env.insert("bool".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Bool,
        });
        env.insert("char".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Char,
        });
        env.insert("string".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::String,
        });
        env.insert("unit".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Unit,
        });

        TypeChecker {
            env,
            constraints: Vec::new(),
            substitution: Substitution::new(),
        }
    }

    /// Type check a program
    pub fn check_program(&mut self, program: &Program) -> Result<()> {
        for decl in &program.declarations {
            self.check_declaration(decl)?;
        }
        self.solve_constraints()?;
        Ok(())
    }

    /// Check a declaration
    fn check_declaration(&mut self, decl: &Declaration) -> Result<()> {
        match decl {
            Declaration::Function(func) => self.check_function_declaration(func),
            Declaration::Type(ty) => self.check_type_declaration(ty),
            Declaration::Effect(effect) => self.check_effect_declaration(effect),
            Declaration::Module(module) => self.check_module_declaration(module),
            Declaration::Import(import) => self.check_import_declaration(import),
            Declaration::Export(export) => self.check_export_declaration(export),
            Declaration::Struct(struct_decl) => self.check_struct_declaration(struct_decl),
            Declaration::Enum(enum_decl) => self.check_enum_declaration(enum_decl),
            Declaration::Trait(trait_decl) => self.check_trait_declaration(trait_decl),
            Declaration::Impl(impl_decl) => self.check_impl_declaration(impl_decl),
            Declaration::TypeAlias(alias_decl) => self.check_type_alias_declaration(alias_decl),
        }
    }

    /// Check function declaration
    fn check_function_declaration(&mut self, func: &FunctionDecl) -> Result<()> {
        // Create parameter types
        let param_types: Vec<Type> = func.parameters.iter()
            .map(|param| param.type_.clone())
            .collect();

        // Create function type
        let func_type = Type::Function {
            parameters: param_types.clone(),
            return_type: Box::new(func.return_type.clone().unwrap_or(Type::Unit)),
        };

        // Add function to environment
        let scheme = TypeScheme {
            vars: vec![], // TODO: Add type variables for polymorphism
            ty: func_type,
        };
        self.env.insert(func.name.clone(), scheme);

        // Create local environment with parameters
        let mut local_env = self.env.clone();
        for (param, param_type) in func.parameters.iter().zip(param_types) {
            local_env.insert(param.name.clone(), TypeScheme {
                vars: vec![],
                ty: param_type,
            });
        }

        // Check function body with local environment
        let saved_env = self.env.clone();
        self.env = local_env;

        let body_type = self.infer_expression(&func.body)?;
        let expected_return = func.return_type.as_ref().unwrap_or(&Type::Unit);

        // Restore environment
        self.env = saved_env;

        // Add constraint for return type
        self.add_constraint(body_type, expected_return.clone());

        Ok(())
    }

    /// Check type declaration
    fn check_type_declaration(&mut self, _ty: &TypeDecl) -> Result<()> {
        // Type declarations are currently just declarations
        // TODO: Add type checking for type declarations
        Ok(())
    }

    /// Check effect declaration
    fn check_effect_declaration(&mut self, _effect: &EffectDecl) -> Result<()> {
        // Effect declarations are currently just declarations
        // TODO: Add effect checking for effect declarations
        Ok(())
    }

    /// Check module declaration
    fn check_module_declaration(&mut self, _module: &ModuleDecl) -> Result<()> {
        // Module declarations are currently just declarations
        // TODO: Add module system checking
        Ok(())
    }

    /// Infer type for expression
    fn infer_expression(&mut self, expr: &Expression) -> Result<Type> {
        match expr {
            Expression::Literal(lit) => Ok(self.infer_literal(lit)),
            Expression::Identifier(name) => self.infer_identifier(name),
            Expression::Binary(binary) => self.infer_binary(binary),
            Expression::Unary(unary) => self.infer_unary(unary),
            Expression::Call(call) => self.infer_call(call),
            Expression::If(if_expr) => self.infer_if(if_expr),
            Expression::Case(case) => self.infer_case(case),
            Expression::Do(do_expr) => self.infer_do(do_expr),
            Expression::AllocRef(alloc) => self.infer_alloc_ref(alloc),
            Expression::ReadRef(read) => self.infer_read_ref(read),
            Expression::WriteRef(write) => self.infer_write_ref(write),
            Expression::Spawn(spawn) => self.infer_spawn(spawn),
            Expression::Send(send) => self.infer_send(send),
            Expression::Recv(recv) => self.infer_recv(recv),
            _ => type_error(
                SourceLocation::unknown(),
                format!("Type inference not implemented for: {:?}", expr),
            ),
        }
    }

    /// Infer type for literal
    fn infer_literal(&self, lit: &Literal) -> Type {
        match lit {
            Literal::Unit => Type::Unit,
            Literal::Bool(_) => Type::Bool,
            Literal::Int(_) => Type::Int,
            Literal::Char(_) => Type::Char,
            Literal::String(_) => Type::String,
        }
    }

    /// Infer type for identifier
    fn infer_identifier(&mut self, name: &str) -> Result<Type> {
        if let Some(scheme) = self.env.get(name) {
            // TODO: Instantiate type scheme with fresh variables
            Ok(scheme.ty.clone())
        } else {
            type_error(
                SourceLocation::unknown(),
                format!("Undefined variable: {}", name),
            )
        }
    }

    /// Infer type for binary expression
    fn infer_binary(&mut self, binary: &BinaryExpr) -> Result<Type> {
        let left_type = self.infer_expression(&binary.left)?;
        let right_type = self.infer_expression(&binary.right)?;

        match binary.operator {
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                // Arithmetic operators require int operands and return int
                self.add_constraint(left_type, Type::Int);
                self.add_constraint(right_type, Type::Int);
                Ok(Type::Int)
            }
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::LessEqual |
            BinaryOp::Greater | BinaryOp::GreaterEqual => {
                // Comparison operators work on any type but both operands must be the same
                self.add_constraint(left_type.clone(), right_type);
                Ok(Type::Bool)
            }
            BinaryOp::And | BinaryOp::Or => {
                // Logical operators require bool operands and return bool
                self.add_constraint(left_type, Type::Bool);
                self.add_constraint(right_type, Type::Bool);
                Ok(Type::Bool)
            }
        }
    }

    /// Infer type for unary expression
    fn infer_unary(&mut self, unary: &UnaryExpr) -> Result<Type> {
        let operand_type = self.infer_expression(&unary.operand)?;

        match unary.operator {
            UnaryOp::Not => {
                self.add_constraint(operand_type, Type::Bool);
                Ok(Type::Bool)
            }
            UnaryOp::Negate => {
                self.add_constraint(operand_type, Type::Int);
                Ok(Type::Int)
            }
        }
    }

    /// Infer type for function call
    fn infer_call(&mut self, call: &CallExpr) -> Result<Type> {
        let func_type = self.infer_expression(&call.function)?;

        // Create fresh type variables for arguments and return type
        let arg_types: Vec<Type> = call.arguments.iter()
            .map(|_| Type::Variable(TypeVar::fresh().0))
            .collect();
        let return_type = Type::Variable(TypeVar::fresh().0);

        // Constrain function type
        let expected_func_type = Type::Function {
            parameters: arg_types.clone(),
            return_type: Box::new(return_type.clone()),
        };
        self.add_constraint(func_type, expected_func_type);

        // Constrain arguments
        for (arg_expr, expected_type) in call.arguments.iter().zip(arg_types) {
            let actual_type = self.infer_expression(arg_expr)?;
            self.add_constraint(actual_type, expected_type);
        }

        Ok(return_type)
    }

    /// Infer type for if expression
    fn infer_if(&mut self, if_expr: &IfExpr) -> Result<Type> {
        let cond_type = self.infer_expression(&if_expr.condition)?;
        self.add_constraint(cond_type, Type::Bool);

        let then_type = self.infer_expression(&if_expr.then_branch)?;
        let else_type = self.infer_expression(&if_expr.else_branch)?;

        // Both branches must have the same type
        self.add_constraint(then_type.clone(), else_type);

        Ok(then_type)
    }

    /// Infer type for case expression
    fn infer_case(&mut self, case: &CaseExpr) -> Result<Type> {
        let _scrutinee_type = self.infer_expression(&case.scrutinee)?;

        // TODO: Implement pattern type checking
        if case.branches.is_empty() {
            return type_error(case.location.clone(), "Case expression must have at least one branch".to_string());
        }

        // All branches must have the same type
        let first_branch_type = self.infer_expression(&case.branches[0].body)?;
        for branch in &case.branches[1..] {
            let branch_type = self.infer_expression(&branch.body)?;
            self.add_constraint(first_branch_type.clone(), branch_type);
        }

        Ok(first_branch_type)
    }

    /// Infer type for do expression
    fn infer_do(&mut self, do_expr: &DoExpr) -> Result<Type> {
        let mut last_type = Type::Unit;

        for statement in &do_expr.statements {
            match statement {
                Statement::Bind { pattern: _, expr } => {
                    last_type = self.infer_expression(expr)?;
                }
                Statement::Expr(expr) => {
                    last_type = self.infer_expression(expr)?;
                }
            }
        }

        Ok(last_type)
    }

    /// Add a type constraint
    fn add_constraint(&mut self, t1: Type, t2: Type) {
        self.constraints.push(Constraint(t1, t2));
    }

    /// Solve type constraints using unification
    fn solve_constraints(&mut self) -> Result<()> {
        for constraint in &self.constraints.clone() {
            self.unify(&constraint.0, &constraint.1)?;
        }
        Ok(())
    }

    /// Unify two types
    fn unify(&mut self, t1: &Type, t2: &Type) -> Result<()> {
        match (t1, t2) {
            // Identical types unify trivially
            (Type::Unit, Type::Unit) |
            (Type::Bool, Type::Bool) |
            (Type::Int, Type::Int) |
            (Type::Char, Type::Char) |
            (Type::String, Type::String) => Ok(()),

            // Variable unification
            (Type::Variable(var), ty) | (ty, Type::Variable(var)) => {
                self.unify_variable(var, ty)
            }

            // Function unification
            (Type::Function { parameters: params1, return_type: ret1 },
             Type::Function { parameters: params2, return_type: ret2 }) => {
                if params1.len() != params2.len() {
                    return type_error(
                        SourceLocation::unknown(),
                        "Function arity mismatch".to_string(),
                    );
                }
                for (p1, p2) in params1.iter().zip(params2) {
                    self.unify(p1, p2)?;
                }
                self.unify(ret1, ret2)
            }

            // Process unification
            (Type::Process { effects: e1, result_type: r1 },
             Type::Process { effects: e2, result_type: r2 }) => {
                // TODO: Effect unification
                self.unify(r1, r2)
            }

            // Tuple unification
            (Type::Tuple(types1), Type::Tuple(types2)) => {
                if types1.len() != types2.len() {
                    return type_error(
                        SourceLocation::unknown(),
                        "Tuple arity mismatch".to_string(),
                    );
                }
                for (t1, t2) in types1.iter().zip(types2) {
                    self.unify(t1, t2)?;
                }
                Ok(())
            }

            // Record unification (simplified)
            (Type::Record(fields1), Type::Record(fields2)) => {
                if fields1.len() != fields2.len() {
                    return type_error(
                        SourceLocation::unknown(),
                        "Record field count mismatch".to_string(),
                    );
                }
                // TODO: Field name matching
                for ((_, t1), (_, t2)) in fields1.iter().zip(fields2) {
                    self.unify(t1, t2)?;
                }
                Ok(())
            }

            // Named type unification (simplified)
            (Type::Named(name1), Type::Named(name2)) if name1 == name2 => Ok(()),

            _ => type_error(
                SourceLocation::unknown(),
                format!("Cannot unify types: {:?} and {:?}", t1, t2),
            ),
        }
    }

    /// Unify a type variable
    fn unify_variable(&mut self, var: &String, ty: &Type) -> Result<()> {
        // TODO: Occurs check
        if let Some(existing) = self.substitution.get(var).cloned() {
            self.unify(&existing, ty)?;
        } else {
            self.substitution.insert(var.clone(), ty.clone());
        }
        Ok(())
    }

    /// Infer type for alloc_ref expression
    fn infer_alloc_ref(&mut self, alloc: &AllocRefExpr) -> Result<Type> {
        // alloc_ref(region, initial_value) returns a reference
        // For now, we'll return a simple reference type
        // In a full implementation, this would create a proper reference type
        // with region and element type information
        Ok(Type::Int) // Placeholder - should be a reference type
    }

    /// Infer type for read_ref expression
    fn infer_read_ref(&mut self, read: &ReadRefExpr) -> Result<Type> {
        // read_ref(reference) returns the element type
        // For now, assume references contain integers
        // In a full implementation, this would extract the element type from the reference
        Ok(Type::Int)
    }

    /// Infer type for write_ref expression
    fn infer_write_ref(&mut self, write: &WriteRefExpr) -> Result<Type> {
        // write_ref(reference, value) returns unit
        Ok(Type::Unit)
    }

    /// Infer type for spawn expression
    fn infer_spawn(&mut self, spawn: &SpawnExpr) -> Result<Type> {
        // spawn(initial_state, behavior) returns an actor_ref
        // For now, we'll return a placeholder type
        // In a full implementation, this would create a proper ActorRef type
        Ok(Type::Int) // Placeholder - should be ActorRef type
    }

    /// Infer type for send expression
    fn infer_send(&mut self, send: &SendExpr) -> Result<Type> {
        // send(actor, message) returns unit
        Ok(Type::Unit)
    }

    /// Infer type for recv expression
    fn infer_recv(&mut self, recv: &RecvExpr) -> Result<Type> {
        // recv() returns the received message type
        // For now, assume it returns an integer
        // In a full implementation, this would depend on the actor's mailbox type
        Ok(Type::Int)
    }

    /// Check import declaration
    fn check_import_declaration(&mut self, _import: &ImportDecl) -> Result<()> {
        // For now, imports are accepted without validation
        // In a full implementation, this would check if the imported module exists
        // and resolve the imported symbols
        Ok(())
    }

    /// Check export declaration
    fn check_export_declaration(&mut self, _export: &ExportDecl) -> Result<()> {
        // For now, exports are accepted without validation
        // In a full implementation, this would check if the exported symbol exists
        Ok(())
    }

    /// Check struct declaration
    fn check_struct_declaration(&mut self, struct_decl: &StructDecl) -> Result<()> {
        // Check that all field types are valid
        for field in &struct_decl.fields {
            self.validate_type(&field.ty)?;
        }

        // Add the struct type to the environment
        let struct_type = Type::Named(struct_decl.name.clone());
        self.env.insert(struct_decl.name.clone(), TypeScheme { vars: Vec::new(), ty: struct_type });

        Ok(())
    }

    /// Check enum declaration
    fn check_enum_declaration(&mut self, enum_decl: &EnumDecl) -> Result<()> {
        // Check that all variant types are valid
        for variant in &enum_decl.variants {
            match variant {
                EnumVariant::Unit { .. } => {}
                EnumVariant::Tuple { fields, .. } => {
                    for field_type in fields {
                        self.validate_type(field_type)?;
                    }
                }
                EnumVariant::Struct { fields, .. } => {
                    for field in fields {
                        self.validate_type(&field.ty)?;
                    }
                }
            }
        }

        // Add the enum type to the environment
        let enum_type = Type::Named(enum_decl.name.clone());
        self.env.insert(enum_decl.name.clone(), TypeScheme { vars: Vec::new(), ty: enum_type });

        Ok(())
    }

    /// Check trait declaration
    fn check_trait_declaration(&mut self, trait_decl: &TraitDecl) -> Result<()> {
        // Check that all method signatures are valid
        for method in &trait_decl.methods {
            for param in &method.params {
                self.validate_type(&param.type_)?;
            }
            if let Some(ref return_type) = method.return_type {
                self.validate_type(return_type)?;
            }
        }

        // Add the trait type to the environment
        let trait_type = Type::Named(trait_decl.name.clone());
        self.env.insert(trait_decl.name.clone(), TypeScheme { vars: Vec::new(), ty: trait_type });

        Ok(())
    }

    /// Check impl declaration
    fn check_impl_declaration(&mut self, impl_decl: &ImplDecl) -> Result<()> {
        // Validate the type being implemented for
        self.validate_type(&impl_decl.for_type)?;

        // Check all method implementations
        for method in &impl_decl.methods {
            self.check_function_declaration(method)?;
        }

        Ok(())
    }

    /// Check type alias declaration
    fn check_type_alias_declaration(&mut self, alias_decl: &TypeAliasDecl) -> Result<()> {
        // Validate the aliased type
        self.validate_type(&alias_decl.aliased_type)?;

        // Add the alias to the environment
        let alias_type = Type::Named(alias_decl.name.clone());
        self.env.insert(alias_decl.name.clone(), TypeScheme { vars: Vec::new(), ty: alias_type });

        Ok(())
    }

    /// Validate that a type is well-formed
    fn validate_type(&self, ty: &Type) -> Result<()> {
        match ty {
            Type::Named(name) => {
                // Check if the named type exists in the environment
                if !self.env.contains_key(name) {
                    return type_error(
                        SourceLocation::unknown(),
                        format!("Unknown type: {}", name),
                    );
                }
                Ok(())
            }
            Type::Generic { type_args, .. } => {
                // Validate all type arguments
                for arg in type_args {
                    self.validate_type(arg)?;
                }
                Ok(())
            }
            Type::Function { parameters, return_type } => {
                for param in parameters {
                    self.validate_type(param)?;
                }
                self.validate_type(return_type)
            }
            Type::Tuple(types) => {
                for ty in types {
                    self.validate_type(ty)?;
                }
                Ok(())
            }
            Type::Record(fields) => {
                for (_, ty) in fields {
                    self.validate_type(ty)?;
                }
                Ok(())
            }
            // Other types are assumed valid for now
            _ => Ok(()),
        }
    }
}
