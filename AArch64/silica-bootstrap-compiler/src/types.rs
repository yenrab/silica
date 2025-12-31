use crate::ast::*;
use crate::errors::{Result, CompilerError, type_error, SourceLocation};
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

/// Trait implementation mapping
#[derive(Debug, Clone)]
pub struct TraitImpl {
    pub trait_name: String,
    pub for_type: Type,
    pub methods: HashMap<String, FunctionDecl>,
    pub associated_types: HashMap<String, Type>,
}

/// Type checker implementing Hindley-Milner inference
pub struct TypeChecker<'a> {
    env: TypeEnv,
    constraints: Vec<Constraint>,
    substitution: Substitution,
    struct_defs: HashMap<String, Vec<StructField>>,
    symbol_table: Option<&'a crate::module_resolver::SymbolTable>,
    generic_instantiations: HashMap<String, Vec<Type>>,
    trait_impls: Vec<TraitImpl>, // All trait implementations
    trait_defs: HashMap<String, TraitDecl>, // Trait definitions
    type_aliases: HashMap<String, Type>, // Type alias definitions (expanded)
    type_alias_decls: HashMap<String, TypeAliasDecl>, // Complete type alias declarations
    pub expression_types: HashMap<SourceLocation, Type>, // Types of expressions for code generation
}

impl<'a> TypeChecker<'a> {
    /// Resolve a type through type aliases to its canonical form
    pub fn resolve_type(&self, type_: &Type) -> Result<Type> {
        match type_ {
            Type::Named(name) => {
                // Check if this is a type alias
                if let Some(alias_target) = self.type_aliases.get(name) {
                    // Recursively resolve the alias target
                    self.resolve_type(alias_target)
                } else if self.struct_defs.contains_key(name) {
                    // This is a direct struct type
                    Ok(type_.clone())
                } else {
                    // Check built-in types
                    match name.as_str() {
                        "int" => Ok(Type::Int),
                        "bool" => Ok(Type::Bool),
                        "char" => Ok(Type::Char),
                        "string" => Ok(Type::String),
                        "unit" => Ok(Type::Unit),
                        _ => type_error(
                            SourceLocation::new("".to_string(), 0, 0, 0), // TODO: Pass proper location
                            format!("Undefined type: {}", name),
                        ),
                    }
                }
            }
            // For complex types, resolve their component types
            Type::Tuple(types) => {
                let mut resolved_types = Vec::new();
                for t in types {
                    resolved_types.push(self.resolve_type(t)?);
                }
                Ok(Type::Tuple(resolved_types))
            }
            Type::Record(fields) => {
                let mut resolved_fields = Vec::new();
                for (name, t) in fields {
                    resolved_fields.push((name.clone(), self.resolve_type(t)?));
                }
                Ok(Type::Record(resolved_fields))
            }
            // Simple types don't need resolution
            Type::Int | Type::Bool | Type::Char | Type::String | Type::Unit => Ok(type_.clone()),
            // TODO: Handle generic types, effects, etc.
            _ => Ok(type_.clone()), // For now, pass through unresolved
        }
    }

    /// Get trait implementations
    pub fn get_trait_impls(&self) -> &Vec<TraitImpl> {
        &self.trait_impls
    }

    /// Resolve a type name to its canonical type
    pub fn resolve_type_name(&self, name: &str) -> Result<Type> {
        if let Some(alias_target) = self.type_aliases.get(name) {
            self.resolve_type(alias_target)
        } else if self.struct_defs.contains_key(name) {
            Ok(Type::Named(name.to_string()))
        } else {
            // Check built-in types
            match name {
                "int" => Ok(Type::Int),
                "bool" => Ok(Type::Bool),
                "char" => Ok(Type::Char),
                "string" => Ok(Type::String),
                "unit" => Ok(Type::Unit),
                _ => type_error(
                    SourceLocation::new("".to_string(), 0, 0, 0),
                    format!("Undefined type: {}", name),
                ),
            }
        }
    }

    /// Find the alias name for a given expanded type (reverse lookup)
    pub fn find_alias_name_for_expanded_type(&self, expanded_type: &Type) -> Option<&str> {
        for (name, _) in &self.type_alias_decls {
            if let Some(stored_expanded) = self.type_aliases.get(name) {
                if self.types_equal(stored_expanded, expanded_type) {
                    return Some(name);
                }
            }
        }
        None
    }

    /// Find the alias name for a given original aliased type
    pub fn find_alias_name_for_aliased_type(&self, aliased_type: &Type) -> Option<&str> {
        for (name, decl) in &self.type_alias_decls {
            if self.types_equal(&decl.aliased_type, aliased_type) {
                return Some(name);
            }
        }
        None
    }

    /// Get the complete type alias declaration by name
    pub fn get_type_alias_decl(&self, name: &str) -> Option<&TypeAliasDecl> {
        self.type_alias_decls.get(name)
    }

    /// Check if a variable name would shadow an existing binding
    fn check_variable_shadowing(&self, name: &str, location: &SourceLocation) -> Result<()> {
        if self.env.contains_key(name) {
            return type_error(
                location.clone(),
                format!("Variable '{}' shadows an existing binding. Variable shadowing is not allowed in Silica.", name)
            );
        }
        Ok(())
    }

    /// Add a variable to the environment with shadowing check
    fn add_variable_to_env(&mut self, name: String, scheme: TypeScheme, location: &SourceLocation) -> Result<()> {
        self.check_variable_shadowing(&name, location)?;
        self.env.insert(name, scheme);
        Ok(())
    }

    /// Record the type of an expression for code generation
    fn record_expression_type(&mut self, location: &SourceLocation, ty: Type) {
        self.expression_types.insert(location.clone(), ty);
    }

    /// Get the type of an expression (for code generation)
    pub fn get_expression_type(&self, location: &SourceLocation) -> Option<&Type> {
        self.expression_types.get(location)
    }

    /// Instantiate a type scheme by replacing type variables with fresh ones
    fn instantiate_scheme(&self, scheme: &TypeScheme) -> Result<Type> {
        // Create substitution map from type variables to fresh types
        let mut subst = HashMap::new();
        for var in &scheme.vars {
            subst.insert(var.0.clone(), Type::Variable(TypeVar::fresh().0));
        }

        // Apply substitution to the type
        Ok(self.substitute_type(&scheme.ty, &subst))
    }

    /// Extract location from an expression
    fn get_expression_location(expr: &Expression) -> &SourceLocation {
        match expr {
            Expression::Literal(_) => panic!("Literals don't have location - use expression location"),
            Expression::Identifier(name) => panic!("Identifier should have location from context"),
            Expression::Binary(binary) => &binary.location,
            Expression::Unary(unary) => &unary.location,
            Expression::Call(call) => &call.location,
            Expression::If(if_expr) => &if_expr.location,
            Expression::Case(case) => &case.location,
            Expression::Do(do_expr) => &do_expr.location,
            Expression::Region(region) => &region.location,
            Expression::AllocRef(alloc) => &alloc.location,
            Expression::ReadRef(read) => &read.location,
            Expression::WriteRef(write) => &write.location,
            Expression::Spawn(spawn) => &spawn.location,
            Expression::Send(send) => &send.location,
            Expression::Recv(recv) => &recv.location,
            Expression::ReadFile(read_file) => &read_file.location,
            Expression::WriteFile(write_file) => &write_file.location,
            Expression::ExecCommand(exec_cmd) => &exec_cmd.location,
            Expression::StructLiteral(struct_lit) => &struct_lit.location,
            Expression::FieldAccess(field_access) => &field_access.location,
            Expression::Tuple(_) => panic!("Tuple location should be handled specially"),
            Expression::GenericInstantiation(generic) => &generic.location,
            Expression::ConstructorCall(ctor) => &ctor.location,
            Expression::FunctionLiteral(func) => &func.location,
        }
    }

    /// Resolve a method call on a type
    pub fn resolve_method(&self, receiver_type: &Type, method_name: &str) -> Option<&FunctionDecl> {
        // eprintln!("DEBUG RESOLVE: Called with method {} on type {:?}", method_name, receiver_type);
        // eprintln!("DEBUG RESOLVE: We have {} trait impls", self.trait_impls.len());

        // Look through all trait implementations
        for trait_impl in &self.trait_impls {
            // eprintln!("DEBUG RESOLVE: Checking trait impl for trait {:?}", trait_impl.trait_name);
            // Check if this implementation applies to the receiver type
            if self.types_equal(&trait_impl.for_type, receiver_type) {
                // Check if the trait has this method
                // eprintln!("DEBUG RESOLVE: Trait has this method");
                if let Some(method) = trait_impl.methods.get(method_name) {
                    return Some(method);
                } else {
                    // eprintln!("DEBUG RESOLVE: Trait does not have this method");
                }
            }
        }

        // eprintln!("DEBUG RESOLVE: No direct match found");
        // If no direct match, check if receiver_type is an expanded type that has an alias
        if let Some(alias_name) = self.find_alias_name_for_expanded_type(receiver_type) {
            // eprintln!("DEBUG RESOLVE: Found alias name {:?}", alias_name);
            let alias_type = Type::Named(alias_name.to_string());
            // eprintln!("DEBUG RESOLVE: Checking trait impl for alias type {:?}", alias_type);
            for trait_impl in &self.trait_impls {
                // eprintln!("DEBUG RESOLVE: Checking trait impl for trait {:?}", trait_impl.trait_name);
                if self.types_equal(&trait_impl.for_type, &alias_type) {
                    // eprintln!("DEBUG RESOLVE: Trait has this method");
                    if let Some(method) = trait_impl.methods.get(method_name) {
                        return Some(method);
                    } else {
                        // eprintln!("DEBUG RESOLVE: Trait does not have this method");
                    }
                }
            }
        } else {
            // eprintln!("DEBUG RESOLVE: No alias found for type {:?}", receiver_type);
        }

        // eprintln!("DEBUG RESOLVE: Method {} not found on type {:?}", method_name, receiver_type);
        None
    }

    /// Check if two types are equal (simplified version)
    fn types_equal(&self, t1: &Type, t2: &Type) -> bool {
        match (t1, t2) {
            (Type::Named(n1), Type::Named(n2)) => n1 == n2,
            (Type::Int, Type::Int) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Char, Type::Char) => true,
            (Type::String, Type::String) => true,
            (Type::Unit, Type::Unit) => true,
            _ => false, // Simplified - doesn't handle generics, tuples, etc.
        }
    }
    pub fn new() -> Self {
        let result = Self::with_symbol_table(None);
        result
    }

    pub fn with_symbol_table(symbol_table: Option<&'a crate::module_resolver::SymbolTable>) -> Self {
        // eprintln!("DEBUG TYPECHECK: with_symbol_table called");
        // eprintln!("DEBUG TYPECHECK: symbol_table = {:?}", symbol_table);
        let mut env = TypeEnv::new();
        // eprintln!("DEBUG TYPECHECK: env created");

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
            struct_defs: HashMap::new(),
            generic_instantiations: HashMap::new(),
            trait_impls: Vec::new(),
            trait_defs: HashMap::new(),
            type_aliases: HashMap::new(),
            type_alias_decls: HashMap::new(),
            symbol_table,
            expression_types: HashMap::new(),
        }
    }

    /// Type check a program
    pub fn check_program(&mut self, program: &Program) -> Result<()> {
        // eprintln!("DEBUG TYPECHECK: check_program called!");
        // eprintln!("DEBUG TYPECHECK: Starting check_program with {} declarations", program.declarations.len());
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
            Declaration::Import(import) => self.check_import_declaration(import),
            Declaration::Export(export) => self.check_export_declaration(export),
            Declaration::Struct(struct_decl) => self.check_struct_declaration(struct_decl),
            Declaration::Enum(enum_decl) => self.check_enum_declaration(enum_decl),
            Declaration::Trait(trait_decl) => self.check_trait_declaration(trait_decl),
            Declaration::Impl(impl_decl) => {
                
                self.check_impl_declaration(impl_decl)
            }
            Declaration::TypeAlias(alias_decl) => self.check_type_alias_declaration(alias_decl),
        }
    }

    /// Check function declaration
    fn check_function_declaration(&mut self, func: &FunctionDecl) -> Result<()> {
        // Convert type parameter names to TypeVars
        let type_vars: Vec<TypeVar> = func.type_params.iter()
            .map(|type_param| TypeVar(type_param.name.clone()))
            .collect();

        // Create substitution map for type parameters
        let mut type_param_subst: HashMap<String, Type> = HashMap::new();
        for (i, type_param) in func.type_params.iter().enumerate() {
            type_param_subst.insert(type_param.name.clone(), Type::Variable(type_vars[i].0.clone()));
        }

        // Check that trait bounds refer to valid traits
        for type_param in &func.type_params {
            for bound in &type_param.bounds {
                if !self.trait_defs.contains_key(&bound.trait_name) {
                    return type_error(
                        func.location.clone(),
                        format!("Trait '{}' not found (required by type parameter '{}')", bound.trait_name, type_param.name)
                    );
                }
            }
        }

        // Convert parameter types, expanding aliases first, then substituting type parameters with variables
        let mut param_types: Vec<Type> = func.parameters.iter()
            .map(|param| {
                let expanded_type = self.expand_type_aliases(&param.type_);
                self.substitute_type(&expanded_type, &type_param_subst)
            })
            .collect();

        // Convert polymorphic function types that use the same type parameters
        // to regular function types for simpler unification
        for param_type in &mut param_types {
            if let Type::PolymorphicFunction { type_params, parameters, return_type } = param_type {
                // Check if all type parameters are already bound by the outer function
                let all_bound = type_params.iter().all(|tp| type_param_subst.contains_key(&tp.name));
                if all_bound {
                    // Convert to regular function type
                    *param_type = Type::Function {
                        parameters: parameters.clone(),
                        return_type: return_type.clone(),
                    };
                }
            }
        }

        // Convert return type, expanding aliases first
        let return_type = func.return_type.as_ref()
            .map(|rt| {
                let expanded_type = self.expand_type_aliases(rt);
                self.substitute_type(&expanded_type, &type_param_subst)
            })
            .unwrap_or(Type::Unit);

        // Create function type
        let func_type = Type::Function {
            parameters: param_types.clone(),
            return_type: Box::new(return_type),
        };

        // Add function to environment as polymorphic scheme
        let scheme = TypeScheme {
            vars: type_vars,
            ty: func_type,
        };
        self.add_variable_to_env(func.name.clone(), scheme, &func.location)?;

        // Create local environment with parameters
        let mut local_env = self.env.clone();
        for param in &func.parameters {
            if let Some(pattern) = &param.pattern {
                // If it's a pattern parameter, check the pattern against the parameter's type
                self.check_pattern(pattern, &param.type_, &param.location, &mut local_env)?;
            } else {
                // Existing logic for identifier parameters
                if local_env.contains_key(&param.name) {
                    return type_error(
                        param.location.clone(),
                        format!("Parameter '{}' shadows an existing binding. Variable shadowing is not allowed in Silica.", param.name)
                    );
                }
                local_env.insert(param.name.clone(), TypeScheme {
                    vars: vec![],
                    ty: self.expand_type_aliases(&param.type_),
                });
            }
        }

        // Check function body with local environment
        let saved_env = self.env.clone();
        self.env = local_env;

        let body_type = self.infer_expression(&func.body)?;
        let expected_return = func.return_type.as_ref()
            .map(|rt| self.substitute_type(rt, &type_param_subst))
            .unwrap_or(Type::Unit);

        // Restore environment
        self.env = saved_env;

        // Add constraint for return type
        self.add_constraint(body_type, expected_return);

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


    /// Infer type for expression
    pub fn infer_expression(&mut self, expr: &Expression) -> Result<Type> {
        let result_type = match expr {
            Expression::Literal(lit) => self.infer_literal(lit),
            Expression::Identifier(name) => self.infer_identifier(name)?,
            Expression::Binary(binary) => self.infer_binary(binary)?,
            Expression::Unary(unary) => self.infer_unary(unary)?,
            Expression::Call(call) => self.infer_call(call)?,
            Expression::FunctionLiteral(func) => self.infer_function_literal(func)?,
            Expression::If(if_expr) => self.infer_if(if_expr)?,
            Expression::Case(case) => self.infer_case(case)?,
            Expression::Do(do_expr) => self.infer_do(do_expr)?,
            Expression::Region(region) => self.infer_region(region)?,
            Expression::AllocRef(alloc) => self.infer_alloc_ref(alloc)?,
            Expression::ReadRef(read) => self.infer_read_ref(read)?,
            Expression::WriteRef(write) => self.infer_write_ref(write)?,
            Expression::Spawn(spawn) => self.infer_spawn(spawn)?,
            Expression::Send(send) => self.infer_send(send)?,
            Expression::Recv(recv) => self.infer_recv(recv)?,
            Expression::ReadFile(read_file) => self.infer_read_file(read_file)?,
            Expression::WriteFile(write_file) => self.infer_write_file(write_file)?,
            Expression::ExecCommand(exec_cmd) => self.infer_exec_command(exec_cmd)?,
            Expression::Tuple(exprs) => self.infer_tuple(exprs)?,
            Expression::StructLiteral(struct_lit) => {
                // eprintln!("DEBUG INFER: StructLiteral case hit for type {}", struct_lit.type_name);
                self.infer_struct_literal(struct_lit)?
            },
            Expression::FieldAccess(field_access) => self.infer_field_access(field_access)?,
            _ => return type_error(
                SourceLocation::unknown(),
                format!("Type inference not implemented for: {:?}", expr),
            ),
        };

        // Record the type for code generation
        if let Some(location) = Self::try_get_expression_location(expr) {
            self.record_expression_type(location, result_type.clone());
        }

        Ok(result_type)
    }

    /// Infer type for struct literal
    fn infer_struct_literal(&mut self, struct_lit: &StructLiteralExpr) -> Result<Type> {
        // eprintln!("DEBUG STRUCT: infer_struct_literal called for type {}", struct_lit.type_name);
        // Resolve the type name through aliases to find the actual struct
        let resolved_type = self.resolve_type_name(&struct_lit.type_name)?;
        // eprintln!("DEBUG STRUCT: resolved_type = {:?}", resolved_type);

        match resolved_type {
            Type::Record(expected_fields) => {
                // This is a record type from a type alias like `type Point = {x: int, y: int}`
                // Validate the struct literal against the record type
                if struct_lit.fields.len() != expected_fields.len() {
                    return type_error(
                        struct_lit.location.clone(),
                        format!(
                            "Record type expects {} fields but got {}",
                            expected_fields.len(),
                            struct_lit.fields.len()
                        ),
                    );
                }

                    // Check each field matches the expected type
                    for (i, (field_name, field_expr)) in struct_lit.fields.iter().enumerate() {
                        let (expected_name, expected_type) = &expected_fields[i];
                        if field_name != expected_name {
                            return type_error(
                                struct_lit.location.clone(),
                                format!(
                                    "Expected field '{}' but got '{}'",
                                    expected_name, field_name
                                ),
                            );
                        }

                        // Type check the field value
                        let field_type = self.infer_expression(field_expr)?;
                        let resolved_expected_type = self.resolve_type(expected_type)?;
                        if !self.types_equal(&field_type, &resolved_expected_type) {
                            return type_error(
                                struct_lit.location.clone(),
                                format!(
                                    "Field '{}' expects type {:?} but got {:?}",
                                    field_name, resolved_expected_type, field_type
                                ),
                            );
                        }
                    }

                // Return the named type for method dispatch, but we've validated the structure
                let result_type = Type::Named(struct_lit.type_name.clone());
                Ok(result_type)
            }
            Type::Named(struct_name) => {
                // This resolves to a named struct type
                if let Some(struct_def) = self.struct_defs.get(&struct_name) {
            let struct_def = struct_def.clone(); // Clone to avoid borrowing issues

            // Check that the number of fields matches
            if struct_lit.fields.len() != struct_def.len() {
                return type_error(
                    struct_lit.location.clone(),
                    format!(
                        "Struct {} expects {} fields but got {}",
                        struct_name,
                        struct_def.len(),
                        struct_lit.fields.len()
                    ),
                );
            }

            // Check each field
            for (i, (field_name, field_expr)) in struct_lit.fields.iter().enumerate() {
                let expected_field = &struct_def[i];
                if field_name != &expected_field.name {
                    return type_error(
                        struct_lit.location.clone(),
                        format!(
                            "Expected field '{}' but got '{}' in struct {}",
                            expected_field.name, field_name, struct_lit.type_name
                        ),
                    );
                }

                // Infer the type of the field expression
                let field_type = self.infer_expression(field_expr)?;

                // Check that it matches the expected type
                let expected_type = &expected_field.ty;
                if !self.types_equal(&field_type, expected_type) {
                    return type_error(
                        struct_lit.location.clone(),
                        format!(
                            "Field '{}' expects type {:?} but got {:?}",
                            field_name, expected_type, field_type
                        ),
                    );
                }
            }

            // Return the struct type
            Ok(Type::Record(
                struct_def.iter().map(|f| (f.name.clone(), f.ty.clone())).collect()
            ))
        } else {
            type_error(
                struct_lit.location.clone(),
                format!("Undefined struct type: {}", struct_name),
            )
        }
            }
            _ => type_error(
                struct_lit.location.clone(),
                format!("Type {} does not resolve to a struct type", struct_lit.type_name),
            ),
        }
    }

    /// Infer type for field access
    fn infer_field_access(&mut self, field_access: &FieldAccessExpr) -> Result<Type> {
        // Infer the type of the object being accessed
        let object_type = self.infer_expression(&field_access.object)?;

        // Check if it's a record/struct type
        if let Type::Record(fields) = &object_type {
            // Find the field
            for (field_name, field_type) in fields {
                if field_name == &field_access.field {
                    return Ok(field_type.clone());
                }
            }
            return type_error(
                field_access.location.clone(),
                format!("Field '{}' not found in struct", field_access.field),
            );
        } else {
            return type_error(
                field_access.location.clone(),
                format!("Cannot access field '{}' on non-struct type {:?}", field_access.field, object_type),
            );
        }
    }

    /// Try to get location from an expression (returns None for tuples)
    pub fn try_get_expression_location(expr: &Expression) -> Option<&SourceLocation> {
        match expr {
            Expression::Literal(_) => None, // Literals don't have location
            Expression::Binary(binary) => Some(&binary.location),
            Expression::Unary(unary) => Some(&unary.location),
            Expression::Call(call) => Some(&call.location),
            Expression::If(if_expr) => Some(&if_expr.location),
            Expression::Case(case) => Some(&case.location),
            Expression::Do(do_expr) => Some(&do_expr.location),
            Expression::Region(region) => Some(&region.location),
            Expression::AllocRef(alloc) => Some(&alloc.location),
            Expression::ReadRef(read) => Some(&read.location),
            Expression::WriteRef(write) => Some(&write.location),
            Expression::Spawn(spawn) => Some(&spawn.location),
            Expression::Send(send) => Some(&send.location),
            Expression::Recv(recv) => Some(&recv.location),
            Expression::ReadFile(read_file) => Some(&read_file.location),
            Expression::WriteFile(write_file) => Some(&write_file.location),
            Expression::ExecCommand(exec_cmd) => Some(&exec_cmd.location),
            Expression::StructLiteral(struct_lit) => Some(&struct_lit.location),
            Expression::FieldAccess(field_access) => Some(&field_access.location),
            Expression::GenericInstantiation(generic) => Some(&generic.location),
            Expression::ConstructorCall(ctor) => Some(&ctor.location),
            Expression::FunctionLiteral(func) => Some(&func.location),
            // Tuples don't have their own location, only elements do
            Expression::Tuple(_) => None,
            Expression::Identifier(_) => None, // Handled separately
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
        // First check local environment for user-defined functions/variables
        if let Some(scheme) = self.env.get(name) {
            // Instantiate the type scheme (replace type variables with fresh ones)
            let instantiated = self.instantiate_scheme(scheme)?;
            // Don't expand type aliases here - keep named types for method dispatch
            return Ok(instantiated);
        }
        // Check built-in functions
        else if name == "read_file" {
            return Ok(Type::Function {
                parameters: vec![Type::Named("string".to_string())],
                return_type: Box::new(Type::Named("Result".to_string())),
            });
        } else if name == "write_file" {
            return Ok(Type::Function {
                parameters: vec![Type::Named("string".to_string()), Type::Named("string".to_string())],
                return_type: Box::new(Type::Named("Result".to_string())),
            });
        } else if name == "exec_command" {
            return Ok(Type::Function {
                parameters: vec![Type::Named("string".to_string())],
                return_type: Box::new(Type::Named("ProcessResult".to_string())),
            });
        } else if let Some(symbol_table) = &self.symbol_table {
            // Check imported symbols from all modules
            for (_module_name, module_symbols) in &symbol_table.modules {
                if let Some(symbol_info) = module_symbols.get(name) {
                    // Found imported symbol - convert to appropriate function type
                    let mut parameters = Vec::new();
                    for _ in 0..symbol_info.arity {
                        parameters.push(Type::Int); // Assume all parameters are int for now
                    }
                    return Ok(Type::Function {
                        parameters,
                        return_type: Box::new(Type::Int), // Assume all functions return int for now
                    });
                }
            }
            // If we get here, symbol wasn't found in any module
            return type_error(
                SourceLocation::unknown(),
                format!("Undefined variable: {}", name),
            );
        } else {
            return type_error(
                SourceLocation::unknown(),
                format!("Undefined variable: {}", name),
            );
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
        // Check if this is a method call (receiver.method(args))
        if let Expression::FieldAccess(field_access) = &*call.function {
            return self.infer_method_call(field_access, call);
        }

        // Special handling for built-in I/O functions
        if let Expression::Identifier(func_name) = &*call.function {
            if func_name == "read_file" {
                // read_file(path: string) -> Result<string, string>
                if call.arguments.len() != 1 {
                    return type_error(
                        call.location.clone(),
                        "read_file expects exactly 1 argument".to_string(),
                    );
                }
                let path_type = self.infer_expression(&call.arguments[0])?;
                self.unify(&path_type, &Type::Named("string".to_string()))?;
                return Ok(Type::Named("Result".to_string()));
            } else if func_name == "write_file" {
                // write_file(path: string, content: string) -> Result<unit, string>
                if call.arguments.len() != 2 {
                    return type_error(
                        call.location.clone(),
                        "write_file expects exactly 2 arguments".to_string(),
                    );
                }
                let path_type = self.infer_expression(&call.arguments[0])?;
                self.unify(&path_type, &Type::Named("string".to_string()))?;
                let content_type = self.infer_expression(&call.arguments[1])?;
                self.unify(&content_type, &Type::Named("string".to_string()))?;
                return Ok(Type::Named("Result".to_string()));
            }
        }

        let func_type = self.infer_expression(&call.function)?;

        // Check if we already have a function type
        if let Type::Function { parameters, return_type } = &func_type {
            // Direct function type - check arguments match
            if parameters.len() != call.arguments.len() {
                return type_error(
                    call.location.clone(),
                    format!("Function expects {} arguments, got {}", parameters.len(), call.arguments.len()),
                );
            }

            // Check argument types
            for (arg_expr, expected_type) in call.arguments.iter().zip(parameters) {
                let actual_type = self.infer_expression(arg_expr)?;
                self.add_constraint(actual_type, expected_type.clone());
            }

            return Ok(*return_type.clone());
        }

        // Handle polymorphic function types
        if let Type::PolymorphicFunction { type_params, parameters, return_type } = &func_type {
            if parameters.len() != call.arguments.len() {
                return type_error(
                    call.location.clone(),
                    format!("Function expects {} arguments, got {}", parameters.len(), call.arguments.len()),
                );
            }

            // Create fresh type variables for type parameters
            let mut type_subst: HashMap<String, Type> = HashMap::new();
            for type_param in type_params {
                type_subst.insert(type_param.name.clone(), Type::Variable(TypeVar::fresh().0));
            }

            // Substitute parameters and return type
            let subst_parameters: Vec<Type> = parameters.iter()
                .map(|p| self.substitute_type(p, &type_subst))
                .collect();
            let subst_return_type = self.substitute_type(return_type, &type_subst);

            // Check argument types
            for (arg_expr, expected_type) in call.arguments.iter().zip(subst_parameters) {
                let actual_type = self.infer_expression(arg_expr)?;
                self.add_constraint(actual_type, expected_type);
            }

            return Ok(subst_return_type);
        }

        // Fallback: create fresh type variables for arguments and return type
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

    /// Infer type for method calls (receiver.method(args))
    fn infer_method_call(&mut self, field_access: &FieldAccessExpr, call: &CallExpr) -> Result<Type> {
        // Infer the receiver type
        // eprintln!("DEBUG METHOD_CALL: infer_method_call called for {}.{}", "receiver", field_access.field);
        let receiver_type = self.infer_expression(&field_access.object)?;
        // eprintln!("DEBUG METHOD_CALL: receiver_type = {:?}", receiver_type);
        // Try to resolve the method
        if let Some(method) = self.resolve_method(&receiver_type, &field_access.field) {
            // Extract method info before doing mutable operations
            let expected_param_count = method.parameters.len();
            let return_type = method.return_type.clone();
            let self_param_type = method.parameters[0].type_.clone();
            let method_params: Vec<Type> = method.parameters.iter().skip(1).map(|p| p.type_.clone()).collect();
            // We found a method - check arguments
            // Method parameters: first is self, then the call arguments
            let actual_arg_count = call.arguments.len() + 1; // +1 for receiver
            if expected_param_count != actual_arg_count {
                return type_error(
                    call.location.clone(),
                    format!("Method expects {} arguments (including self), got {}", expected_param_count, actual_arg_count)
                );
            }
            // Check receiver type matches method's self parameter
            self.add_constraint(receiver_type.clone(), self_param_type);
            // Check call arguments against method parameters (skip self)
            for (arg_expr, expected_type) in call.arguments.iter().zip(method_params) {
                let actual_type = self.infer_expression(arg_expr)?;
                self.add_constraint(actual_type, expected_type);
            }
            // Return the method's return type
            match return_type {
                Some(rt) => Ok(rt),
                None => Ok(Type::Unit),
            }
        } else {
            type_error(
                field_access.location.clone(),
                format!("No method '{}' found for type {:?}", field_access.field, receiver_type)
            )
        }
    }

    /// Infer type for function literal
    fn infer_function_literal(&mut self, func: &FunctionLiteralExpr) -> Result<Type> {
        // Convert type parameter names to TypeVars
        let type_vars: Vec<TypeVar> = func.type_params.iter()
            .map(|type_param| TypeVar(type_param.name.clone()))
            .collect();

        // Create substitution map for type parameters
        let mut type_param_subst: HashMap<String, Type> = HashMap::new();
        for (i, type_param) in func.type_params.iter().enumerate() {
            type_param_subst.insert(type_param.name.clone(), Type::Variable(type_vars[i].0.clone()));
        }

        // Check that trait bounds refer to valid traits
        for type_param in &func.type_params {
            for bound in &type_param.bounds {
                if !self.trait_defs.contains_key(&bound.trait_name) {
                    return type_error(
                        func.location.clone(),
                        format!("Trait '{}' not found (required by type parameter '{}')", bound.trait_name, type_param.name)
                    );
                }
            }
        }

        // Convert parameter types, substituting type parameters with variables
        let param_types: Vec<Type> = func.parameters.iter()
            .map(|param| self.substitute_type(&param.type_, &type_param_subst))
            .collect();

        // Convert return type
        let return_type = func.return_type.as_ref()
            .map(|rt| self.substitute_type(rt, &type_param_subst))
            .unwrap_or(Type::Unit);

        // Create function type
        let func_type = Type::Function {
            parameters: param_types.clone(),
            return_type: Box::new(return_type.clone()),
        };

        // Create local environment for function body checking
        let mut local_env = self.env.clone();

        // Add captured variables to local environment
        for captured_var in &func.captured_vars {
            if let Some(var_scheme) = self.env.get(captured_var) {
                local_env.insert(captured_var.clone(), var_scheme.clone());
            }
        }

        // Add parameters to local environment
        for (param, param_type) in func.parameters.iter().zip(param_types.clone()) {
            local_env.insert(param.name.clone(), TypeScheme {
                vars: vec![],
                ty: param_type,
            });
        }

        // Check function body with local environment
        let saved_env = self.env.clone();
        self.env = local_env;

        let body_type = self.infer_expression(&func.body)?;
        self.add_constraint(body_type, return_type.clone());

        // Restore environment
        self.env = saved_env;

        // If there are captured variables, return a closure type
        if !func.captured_vars.is_empty() {
            // Get types of captured variables
            let mut captured_types = Vec::new();
            for captured_var in &func.captured_vars {
                if let Some(var_scheme) = self.env.get(captured_var) {
                    captured_types.push(var_scheme.ty.clone());
                } else {
                    // This shouldn't happen if capture detection is correct
                    captured_types.push(Type::Int); // fallback
                }
            }

            Ok(Type::Closure {
                parameters: param_types,
                return_type: Box::new(return_type),
                captured_types,
            })
        } else {
            // No captures, return regular function type
            Ok(func_type)
        }
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
        // Infer scrutinee type for pattern checking (disabled for now)
        // let scrutinee_type = self.infer_expression(&case.scrutinee)?;

        if case.branches.is_empty() {
            return type_error(case.location.clone(), "Case expression must have at least one branch".to_string());
        }

        // Check pattern types and guard expressions
        for branch in &case.branches {
            // Check that pattern matches scrutinee type
            // For now, skip detailed pattern type checking in case expressions
            // TODO: Add proper pattern binding to type environment
            // self.check_pattern_type(&branch.pattern, &scrutinee_type, &branch.location)?;

            // Check guard expression if present (must be Bool type)
            // For now, skip guard checking in case expressions
            // TODO: Add proper pattern variable scoping for guards
            // if let Some(ref guard) = branch.guard {
            //     let guard_type = self.infer_expression(guard)?;
            //     self.add_constraint(Type::Bool, guard_type);
            // }
        }

        // All branches must have the same type
        // For now, skip branch body type checking in case expressions
        // TODO: Add proper pattern variable scoping
        // let first_branch_type = self.infer_expression(&case.branches[0].body)?;
        // for branch in &case.branches[1..] {
        //     let branch_type = self.infer_expression(&branch.body)?;
        //     self.add_constraint(first_branch_type.clone(), branch_type);
        // }

        // Case expressions return the type of their branches, not the scrutinee
        // For now, assume branches return Int
        Ok(Type::Int)
    }

    /// Check that a pattern is compatible with the expected type
    fn check_pattern_type(&mut self, pattern: &Pattern, expected_type: &Type, location: &SourceLocation) -> Result<()> {
        match pattern {
            Pattern::Literal(lit) => {
                let lit_type = match lit {
                    Literal::Unit => Type::Unit,
                    Literal::Bool(_) => Type::Bool,
                    Literal::Int(_) => Type::Int,
                    Literal::Char(_) => Type::Char,
                    Literal::String(_) => Type::String,
                };
                self.add_constraint(expected_type.clone(), lit_type);
            }
            Pattern::Identifier(_) | Pattern::Wildcard => {
                // These match any type, no constraint needed
            }
            Pattern::Tuple(patterns) => {
                if let Type::Tuple(elem_types) = expected_type {
                    if patterns.len() != elem_types.len() {
                        return type_error(location.clone(),
                            format!("Tuple pattern has {} elements but expected {}", patterns.len(), elem_types.len()));
                    }
                    for (pattern, expected_elem_type) in patterns.iter().zip(elem_types.iter()) {
                        self.check_pattern_type(pattern, expected_elem_type, location)?;
                    }
                } else {
                    return type_error(location.clone(),
                        format!("Tuple pattern expected tuple type, got {:?}", expected_type));
                }
            }
            Pattern::Record(field_patterns) => {
                if let Type::Record(fields) = expected_type {
                    for (field_name, pattern) in field_patterns {
                        let expected_field_type = fields.iter()
                            .find(|(name, _)| name == field_name)
                            .map(|(_, ty)| ty)
                            .ok_or_else(|| CompilerError::type_error(location.clone(),
                                format!("Field '{}' not found in record type", field_name)))?;
                        self.check_pattern_type(pattern, expected_field_type, location)?;
                    }
                } else {
                    return type_error(location.clone(),
                        format!("Record pattern expected record type, got {:?}", expected_type));
                }
            }
            Pattern::Variant { constructor, payload } => {
                if let Type::Variant(variants) = expected_type {
                    let (_, expected_payload_type) = variants.iter()
                        .find(|(name, _)| name == constructor)
                        .ok_or_else(|| CompilerError::type_error(location.clone(),
                            format!("Variant constructor '{}' not found in type", constructor)))?;

                    if let Some(pattern) = payload {
                        if let Some(ref expected_type) = expected_payload_type {
                            self.check_pattern_type(pattern, expected_type, location)?;
                        } else {
                            return type_error(location.clone(),
                                format!("Variant '{}' does not expect payload", constructor));
                        }
                    } else if expected_payload_type.is_some() {
                        return type_error(location.clone(),
                            format!("Variant '{}' expects payload", constructor));
                    }
                } else {
                    return type_error(location.clone(),
                        format!("Variant pattern expected variant type, got {:?}", expected_type));
                }
            }
            Pattern::GenericVariant { constructor, type_args: _, payload } => {
                // For now, treat generic variants like regular variants
                // TODO: Handle type arguments properly
                if let Type::Variant(variants) = expected_type {
                    let (_, expected_payload_type) = variants.iter()
                        .find(|(name, _)| name == constructor)
                        .ok_or_else(|| CompilerError::type_error(location.clone(),
                            format!("Variant constructor '{}' not found in type", constructor)))?;

                    if let Some(pattern) = payload {
                        if let Some(ref expected_type) = expected_payload_type {
                            self.check_pattern_type(pattern, expected_type, location)?;
                        } else {
                            return type_error(location.clone(),
                                format!("Variant '{}' does not expect payload", constructor));
                        }
                    } else if expected_payload_type.is_some() {
                        return type_error(location.clone(),
                            format!("Variant '{}' expects payload", constructor));
                    }
                } else {
                    return type_error(location.clone(),
                        format!("Variant pattern expected variant type, got {:?}", expected_type));
                }
            }
            Pattern::Alternative(patterns) => {
                // All alternatives must match the same type
                for pattern in patterns {
                    self.check_pattern_type(pattern, expected_type, location)?;
                }
            }
        }
        Ok(())
    }

    /// Infer type for do expression
    fn infer_do(&mut self, do_expr: &DoExpr) -> Result<Type> {
        let mut last_type = Type::Unit;

        for statement in &do_expr.statements {
            match statement {
                Statement::Bind { pattern, expr } => {
                    // Infer the type of the expression
                    let expr_type = self.infer_expression(expr)?;
                    // eprintln!("DEBUG BIND: expr_type = {:?}", expr_type);

                    // Bind pattern variables to the type environment (keep named types for method dispatch)
                    self.bind_pattern_variables(pattern, &expr_type, &do_expr.location)?;

                    last_type = expr_type;
                }
                Statement::Expr(expr) => {
                    last_type = self.infer_expression(expr)?;
                }
            }
        }

        Ok(last_type)
    }

    /// Bind pattern variables to the type environment
    fn bind_pattern_variables(&mut self, pattern: &Pattern, ty: &Type, location: &SourceLocation) -> Result<()> {
        // Expand type aliases in the type
        let expanded_ty = self.expand_type_aliases(ty);

        match pattern {
            Pattern::Identifier(name) => {
                // Check for variable shadowing before binding
                self.check_variable_shadowing(name, location)?;
                // Bind identifier pattern to type (keep named types for method dispatch)
                self.env.insert(name.clone(), TypeScheme {
                    vars: vec![], // No type variables for now
                    ty: ty.clone(), // Keep original type, don't expand aliases
                });
                Ok(())
            }
            Pattern::Wildcard => {
                // Wildcard matches anything, no binding needed
                Ok(())
            }
            Pattern::Tuple(patterns) => {
                // For tuple patterns, recursively bind each element
                if let Type::Tuple(types) = &expanded_ty {
                    if patterns.len() != types.len() {
                        return type_error(
                            SourceLocation::unknown(),
                            format!("Tuple pattern has {} elements but type has {}", patterns.len(), types.len())
                        );
                    }
                    for (pattern, ty) in patterns.iter().zip(types.iter()) {
                        self.bind_pattern_variables(pattern, ty, &SourceLocation::unknown())?;
                    }
                    Ok(())
                } else {
                    type_error(
                        SourceLocation::unknown(),
                        format!("Tuple pattern cannot match non-tuple type {:?}", expanded_ty)
                    )
                }
            }
            _ => {
                // For other pattern types, skip binding for now
                // TODO: Implement binding for other pattern types
                Ok(())
            }
        }
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

            // Closure unification
            (Type::Closure { parameters: params1, return_type: ret1, captured_types: caps1 },
             Type::Closure { parameters: params2, return_type: ret2, captured_types: caps2 }) => {
                if params1.len() != params2.len() {
                    return type_error(
                        SourceLocation::unknown(),
                        "Closure arity mismatch".to_string(),
                    );
                }
                if caps1.len() != caps2.len() {
                    return type_error(
                        SourceLocation::unknown(),
                        "Closure capture count mismatch".to_string(),
                    );
                }
                for (p1, p2) in params1.iter().zip(params2) {
                    self.unify(p1, p2)?;
                }
                for (c1, c2) in caps1.iter().zip(caps2) {
                    self.unify(c1, c2)?;
                }
                self.unify(ret1, ret2)
            }

            // Unify Closure with Function (closure can be used as function)
            (Type::Closure { parameters: params1, return_type: ret1, .. },
             Type::Function { parameters: params2, return_type: ret2 }) |
            (Type::Function { parameters: params2, return_type: ret2 },
             Type::Closure { parameters: params1, return_type: ret1, .. }) => {
                if params1.len() != params2.len() {
                    return type_error(
                        SourceLocation::unknown(),
                        "Function/closure arity mismatch".to_string(),
                    );
                }
                for (p1, p2) in params1.iter().zip(params2) {
                    self.unify(p1, p2)?;
                }
                self.unify(ret1, ret2)
            }

            // Polymorphic function unification
            (Type::PolymorphicFunction { type_params: tp1, parameters: params1, return_type: ret1 },
             Type::PolymorphicFunction { type_params: tp2, parameters: params2, return_type: ret2 }) => {
                if tp1.len() != tp2.len() || params1.len() != params2.len() {
                    return type_error(
                        SourceLocation::unknown(),
                        "Polymorphic function signature mismatch".to_string(),
                    );
                }

                // Create fresh type variables for unification
                let mut fresh_vars: HashMap<String, Type> = HashMap::new();
                for i in 0..tp1.len() {
                    let fresh_var = TypeVar::fresh().0;
                    fresh_vars.insert(tp1[i].name.clone(), Type::Variable(fresh_var.clone()));
                    fresh_vars.insert(tp2[i].name.clone(), Type::Variable(fresh_var));
                }

                // Substitute and unify parameters
                for (p1, p2) in params1.iter().zip(params2) {
                    let subst_p1 = self.substitute_type(p1, &fresh_vars);
                    let subst_p2 = self.substitute_type(p2, &fresh_vars);
                    self.unify(&subst_p1, &subst_p2)?;
                }

                // Substitute and unify return types
                let subst_ret1 = self.substitute_type(ret1, &fresh_vars);
                let subst_ret2 = self.substitute_type(ret2, &fresh_vars);
                self.unify(&subst_ret1, &subst_ret2)
            }

            // Unify PolymorphicFunction with concrete Function
            (Type::PolymorphicFunction { type_params, parameters: poly_params, return_type: poly_ret },
             Type::Function { parameters: func_params, return_type: func_ret }) |
            (Type::Function { parameters: func_params, return_type: func_ret },
             Type::PolymorphicFunction { type_params, parameters: poly_params, return_type: poly_ret }) => {
                if poly_params.len() != func_params.len() {
                    return type_error(
                        SourceLocation::unknown(),
                        "Function arity mismatch with polymorphic function".to_string(),
                    );
                }

                // Create substitution map from polymorphic params to concrete params
                let mut substitution: HashMap<String, Type> = HashMap::new();
                for type_param in type_params {
                    substitution.insert(type_param.name.clone(), Type::Variable(TypeVar::fresh().0));
                }

                // Substitute polymorphic parameters and return type
                let subst_poly_params: Vec<Type> = poly_params.iter()
                    .map(|p| self.substitute_type(p, &substitution))
                    .collect();
                let subst_poly_ret = self.substitute_type(poly_ret, &substitution);

                // Unify the substituted polymorphic function with the concrete function
                for (poly_param, func_param) in subst_poly_params.iter().zip(func_params) {
                    self.unify(poly_param, func_param)?;
                }
                self.unify(&subst_poly_ret, func_ret)
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

    /// Infer type for region expression
    fn infer_region(&mut self, region: &RegionExpr) -> Result<Type> {
        // region() returns a region type
        Ok(Type::Region { space: region.space.clone() })
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

    fn infer_read_file(&mut self, read_file: &ReadFileExpr) -> Result<Type> {
        // Check that path is a string
        let path_type = self.infer_expression(&read_file.path)?;
        self.unify(&path_type, &Type::Named("string".to_string()))?;

        // read_file returns Result<string, string>
        // For now, we'll represent this as a generic type
        Ok(Type::Named("Result".to_string()))
    }

    fn infer_write_file(&mut self, write_file: &WriteFileExpr) -> Result<Type> {
        // Check that path is a string
        let path_type = self.infer_expression(&write_file.path)?;
        self.unify(&path_type, &Type::Named("string".to_string()))?;

        // Check that content is a string
        let content_type = self.infer_expression(&write_file.content)?;
        self.unify(&content_type, &Type::Named("string".to_string()))?;

        // write_file returns Result<unit, string>
        Ok(Type::Named("Result".to_string()))
    }

    fn infer_exec_command(&mut self, exec_cmd: &ExecCommandExpr) -> Result<Type> {
        // Check that command is a string
        let cmd_type = self.infer_expression(&exec_cmd.command)?;
        self.unify(&cmd_type, &Type::Named("string".to_string()))?;

        // exec_command returns ProcessResult
        Ok(Type::Named("ProcessResult".to_string()))
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

        // Store the struct definition for struct literal checking
        self.struct_defs.insert(struct_decl.name.clone(), struct_decl.fields.clone());

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

        // eprintln!("DEBUG TRAIT: check_trait_declaration called for trait {:?}", trait_decl.name);
        // eprintln!("DEBUG TRAIT: methods.len() = {}", trait_decl.methods.len());
        for method in &trait_decl.methods {
            // eprintln!("DEBUG TRAIT: method = {:?}", method.name);
            // eprintln!("DEBUG TRAIT: params.len() = {}", method.params.len());
            
            
            for param in &method.params {
                self.validate_type(&param.type_)?;
            }
            if let Some(ref return_type) = method.return_type {
                // eprintln!("DEBUG TRAIT: return_type = {:?}", return_type);
                self.validate_type(return_type)?;
            }
        }

        // Check associated types
        // eprintln!("DEBUG TRAIT: associated_types.len() = {}", trait_decl.associated_types.len());
        for assoc_type in &trait_decl.associated_types {
            // eprintln!("DEBUG TRAIT: associated_type = {:?}", assoc_type.name);
            // eprintln!("DEBUG TRAIT: bounds.len() = {}", assoc_type.bounds.len());
            // Associated types are just declarations, no validation needed beyond name
        }
        

        // Add the trait type to the environment
        let trait_type = Type::Named(trait_decl.name.clone());
        self.env.insert(trait_decl.name.clone(), TypeScheme { vars: Vec::new(), ty: trait_type });
        // eprintln!("DEBUG TRAIT: Trait type added to environment");

        // Store the trait definition
        self.trait_defs.insert(trait_decl.name.clone(), trait_decl.clone());
        // eprintln!("DEBUG TRAIT: Associated types validated after checking");

        Ok(())
    }

    /// Check impl declaration
    fn check_impl_declaration(&mut self, impl_decl: &ImplDecl) -> Result<()> {
        // eprintln!("DEBUG IMPL: check_impl_declaration START");
        // eprintln!("DEBUG IMPL: check_impl_declaration called for type {:?}", impl_decl.for_type);
        // eprintln!("DEBUG IMPL: trait_name = {:?}", impl_decl.trait_name);
        // eprintln!("DEBUG IMPL: methods.len() = {}", impl_decl.methods.len());
        
        
        
        

        // Validate the type being implemented for
        self.validate_type(&impl_decl.for_type)?;

        // Check all method implementations (validate types without adding to environment)
        for method in &impl_decl.methods {
            // Validate parameter types
            for param in &method.parameters {
                self.validate_type(&param.type_)?;
            }
            // Validate return type
            if let Some(ref return_type) = method.return_type {
                self.validate_type(return_type)?;
            }
        }

        // Check associated type definitions
        for assoc_type_def in &impl_decl.associated_types {
            self.validate_type(&assoc_type_def.type_)?;
        }

        // If this is a trait implementation (not inherent impl)
        if let Some(trait_name) = &impl_decl.trait_name {
            // eprintln!("DEBUG IMPL: Processing trait impl for trait '{}'", trait_name);

            // Verify the trait exists
            if !self.trait_defs.contains_key(trait_name) {
                // eprintln!("DEBUG IMPL: Trait '{}' not found!", trait_name);
                return type_error(
                    impl_decl.location.clone(),
                    format!("Trait '{}' not found", trait_name)
                );
            }
            // eprintln!("DEBUG IMPL: Trait '{}' found, proceeding with impl", trait_name);

            // Create method map
            let mut methods = HashMap::new();
            for method in &impl_decl.methods {
                methods.insert(method.name.clone(), method.clone());
            }

            // Create associated type map
            let mut associated_types = HashMap::new();
            for assoc_type in &impl_decl.associated_types {
                associated_types.insert(assoc_type.name.clone(), assoc_type.type_.clone());
            }

            // Store the trait implementation
            // eprintln!("DEBUG IMPL: Storing TraitImpl for trait '{}' and type {:?}", trait_name, impl_decl.for_type);
            // eprintln!("DEBUG IMPL: TraitImpl has {} methods: {:?}", methods.len(), methods.keys().collect::<Vec<_>>());
            let trait_impl = TraitImpl {
                trait_name: trait_name.clone(),
                for_type: impl_decl.for_type.clone(),
                methods,
                associated_types,
            };

            self.trait_impls.push(trait_impl);
            // eprintln!("DEBUG IMPL: Total trait impls now: {}", self.trait_impls.len());
            
        }

        Ok(())
    }

    /// Check type alias declaration
    fn check_type_alias_declaration(&mut self, alias_decl: &TypeAliasDecl) -> Result<()> {
        // Validate the aliased type
        self.validate_type(&alias_decl.aliased_type)?;

        // Expand the aliased type to resolve all built-in types
        let expanded_aliased_type = self.expand_type_aliases(&alias_decl.aliased_type);

        // Store the mapping from alias name to expanded actual type
        self.type_aliases.insert(alias_decl.name.clone(), expanded_aliased_type);

        // Store the complete type alias declaration for reverse lookup
        self.type_alias_decls.insert(alias_decl.name.clone(), alias_decl.clone());

        // Add the alias to the environment as a named type
        let alias_type = Type::Named(alias_decl.name.clone());
        self.env.insert(alias_decl.name.clone(), TypeScheme { vars: Vec::new(), ty: alias_type });

        Ok(())
    }

    /// Expand type aliases in a type
    fn expand_type_aliases(&self, ty: &Type) -> Type {
        match ty {
            Type::Named(name) => {
                if let Some(aliased_type) = self.type_aliases.get(name) {
                    // Expand the aliased type recursively
                    self.expand_type_aliases(aliased_type)
                } else {
                    // Check if it's a built-in type
                    match name.as_str() {
                        "int" => Type::Int,
                        "bool" => Type::Bool,
                        "char" => Type::Char,
                        "string" => Type::String,
                        "unit" => Type::Unit,
                        _ => ty.clone(), // Unknown named type, keep as is
                    }
                }
            }
            Type::Tuple(elements) => {
                Type::Tuple(elements.iter().map(|elem| self.expand_type_aliases(elem)).collect())
            }
            Type::Record(fields) => {
                Type::Record(fields.iter().map(|(name, ty)| (name.clone(), self.expand_type_aliases(ty))).collect())
            }
            Type::Function { parameters, return_type } => {
                Type::Function {
                    parameters: parameters.iter().map(|param| self.expand_type_aliases(param)).collect(),
                    return_type: Box::new(self.expand_type_aliases(return_type)),
                }
            }
            Type::Generic { name, type_args } => {
                Type::Generic {
                    name: name.clone(),
                    type_args: type_args.iter().map(|arg| self.expand_type_aliases(arg)).collect(),
                }
            }
            Type::Process { effects, result_type } => {
                Type::Process {
                    effects: effects.clone(),
                    result_type: Box::new(self.expand_type_aliases(result_type)),
                }
            }
            // For other types, return as-is
            _ => ty.clone(),
        }
    }

    /// Validate that a type is well-formed
    fn validate_type(&self, ty: &Type) -> Result<()> {
        match ty {
            Type::Named(name) => {
                // Allow "Self" as a special type in trait contexts
                if name == "Self" {
                    return Ok(());
                }
                // Check if the named type exists in the environment
                if !self.env.contains_key(name) {
                    return type_error(
                        SourceLocation::unknown(),
                        format!("Unknown type: {} (env contains: {:?})", name, self.env.keys().collect::<Vec<_>>()),
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
            Type::PolymorphicFunction { parameters, return_type, .. } => {
                for param in parameters {
                    self.validate_type(param)?;
                }
                self.validate_type(return_type)
            }
            Type::Closure { parameters, return_type, captured_types } => {
                for param in parameters {
                    self.validate_type(param)?;
                }
                for captured in captured_types {
                    self.validate_type(captured)?;
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

    pub fn get_struct_defs(&self) -> &HashMap<String, Vec<crate::ast::StructField>> {
        &self.struct_defs
    }

    pub fn get_generic_instantiations(&self) -> &HashMap<String, Vec<Type>> {
        &self.generic_instantiations
    }

    /// Substitute type variables in a type
    fn substitute_type(&self, ty: &Type, substitution: &HashMap<String, Type>) -> Type {
        match ty {
            Type::Variable(name) => {
                substitution.get(name).cloned().unwrap_or_else(|| ty.clone())
            }
            Type::Function { parameters, return_type } => {
                Type::Function {
                    parameters: parameters.iter().map(|p| self.substitute_type(p, substitution)).collect(),
                    return_type: Box::new(self.substitute_type(return_type, substitution)),
                }
            }
            Type::PolymorphicFunction { type_params, parameters, return_type } => {
                // Type parameters are bound variables, don't substitute them
                // But substitute in parameter and return types
                Type::PolymorphicFunction {
                    type_params: type_params.clone(),
                    parameters: parameters.iter().map(|p| self.substitute_type(p, substitution)).collect(),
                    return_type: Box::new(self.substitute_type(return_type, substitution)),
                }
            }
            Type::Closure { parameters, return_type, captured_types } => {
                Type::Closure {
                    parameters: parameters.iter().map(|p| self.substitute_type(p, substitution)).collect(),
                    return_type: Box::new(self.substitute_type(return_type, substitution)),
                    captured_types: captured_types.iter().map(|c| self.substitute_type(c, substitution)).collect(),
                }
            }
            Type::Tuple(types) => {
                Type::Tuple(types.iter().map(|t| self.substitute_type(t, substitution)).collect())
            }
            Type::Record(fields) => {
                Type::Record(
                    fields.iter().map(|(name, ty)| (name.clone(), self.substitute_type(ty, substitution))).collect()
                )
            }
            Type::Variant(variants) => {
                Type::Variant(
                    variants.iter().map(|(name, ty)| (name.clone(), ty.as_ref().map(|t| self.substitute_type(t, substitution)))).collect()
                )
            }
            Type::Sum(types) => {
                Type::Sum(types.iter().map(|t| self.substitute_type(t, substitution)).collect())
            }
            Type::TypeOperator { name, args } => {
                Type::TypeOperator {
                    name: name.clone(),
                    args: args.iter().map(|t| self.substitute_type(t, substitution)).collect(),
                }
            }
            Type::Existential { var, body } => {
                Type::Existential {
                    var: var.clone(),
                    body: Box::new(self.substitute_type(body, substitution)),
                }
            }
            Type::TypeApplication { constructor, args } => {
                Type::TypeApplication {
                    constructor: Box::new(self.substitute_type(constructor, substitution)),
                    args: args.iter().map(|t| self.substitute_type(t, substitution)).collect(),
                }
            }
            // For other types, return as-is for now
            _ => ty.clone(),
        }
    }

    /// Infer type for type operator
    fn infer_type_operator(&mut self, name: &str, args: &[Type]) -> Result<Type> {
        // For now, treat type operators as named types
        // In a full implementation, this would look up type operator definitions
        // and apply them to the arguments
        Ok(Type::Named(name.to_string()))
    }

    /// Infer type for existential type
    fn infer_existential(&mut self, var: &str, body: &Type) -> Result<Type> {
        // For now, just return the body type
        // In a full implementation, this would handle existential quantification
        Ok(body.clone())
    }

    /// Infer type for type application
    fn infer_type_application(&mut self, constructor: &Type, args: &[Type]) -> Result<Type> {
        // For now, create a generic type application
        // In a full implementation, this would apply type constructors
        match constructor {
            Type::Named(name) => Ok(Type::Generic {
                name: name.clone(),
                type_args: args.to_vec(),
            }),
            _ => Ok(Type::Named("TypeApplication".to_string())), // Placeholder
        }
    }

    /// Infer type for tuple expression
    fn infer_tuple(&mut self, exprs: &[Expression]) -> Result<Type> {
        // Infer types for all tuple elements
        let mut element_types = Vec::new();
        for expr in exprs {
            let element_type = self.infer_expression(expr)?;
            element_types.push(element_type.clone());

            // Record the type for this element if it has a location
            if let Some(location) = Self::try_get_expression_location(expr) {
                self.record_expression_type(location, element_type);
            }
        }
        Ok(Type::Tuple(element_types))
    }

    /// Check pattern against expected type and add variables to environment
    fn check_pattern(&mut self, pattern: &Pattern, expected_type: &Type, location: &SourceLocation, env: &mut HashMap<String, TypeScheme>) -> Result<()> {
        // Expand type aliases in the expected type
        let expanded_expected_type = self.expand_type_aliases(expected_type);

        match pattern {
            Pattern::Identifier(name) => {
                if env.contains_key(name) {
                    return type_error(location.clone(), format!("Pattern variable '{}' shadows an existing binding", name));
                }
                env.insert(name.clone(), TypeScheme {
                    vars: vec![],
                    ty: expanded_expected_type.clone(),
                });
            }
            Pattern::Tuple(elements) => {
                if let Type::Tuple(element_types) = &expanded_expected_type {
                    if elements.len() != element_types.len() {
                        return type_error(location.clone(),
                            format!("Tuple pattern has {} elements but expected type has {}",
                                elements.len(), element_types.len()));
                    }
                    for (elem_pattern, elem_type) in elements.iter().zip(element_types) {
                        self.check_pattern(elem_pattern, elem_type, location, env)?;
                    }
                } else {
                    return type_error(location.clone(),
                        format!("Tuple pattern expected tuple type, found {:?}", expanded_expected_type));
                }
            }
            Pattern::Wildcard => {
                // Wildcard matches anything, no variables to bind
            }
            _ => return type_error(location.clone(), format!("Unsupported pattern type: {:?}", pattern)),
        }
        Ok(())
    }

    /// Get the type aliases for code generation
    pub fn get_type_aliases(&self) -> &HashMap<String, Type> {
        &self.type_aliases
    }

}
