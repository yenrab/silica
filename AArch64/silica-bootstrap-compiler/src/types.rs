use crate::ast::*;
use crate::errors::{Result, CompilerError, type_error, type_error_with_metadata, SourceLocation, ErrorMetadataBuilder, ErrorSeverity, TypeInfo};
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
    trait_impls: Vec<TraitImpl>, // All trait implementations
    trait_defs: HashMap<String, TraitDecl>, // Trait definitions
    type_aliases: HashMap<String, Type>, // Type alias definitions (expanded)
    type_alias_decls: HashMap<String, TypeAliasDecl>, // Complete type alias declarations
    pub expression_types: HashMap<SourceLocation, Type>, // Types of expressions for code generation
    pub actor_mailbox_types: HashMap<SourceLocation, Type>, // Map from spawn locations to message types
}

impl<'a> TypeChecker<'a> {
    /// Resolve a type through type aliases to its canonical form
    pub fn resolve_type(&self, type_: &Type) -> Result<Type> {
        self.resolve_type_with_location(type_, None)
    }

    /// Resolve a type through type aliases to its canonical form with location for error reporting
    pub fn resolve_type_with_location(&self, type_: &Type, location: Option<SourceLocation>) -> Result<Type> {
        match type_ {
            Type::Named(name) => {
                // Check if this is a type alias
                if let Some(alias_target) = self.type_aliases.get(name) {
                    // Recursively resolve the alias target (preserve location)
                    self.resolve_type_with_location(alias_target, location)
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
                        _ => {
                            let error_location = location.unwrap_or_else(|| SourceLocation::unknown());
                            let metadata = ErrorMetadataBuilder::new("E2002".to_string())
                                .severity(ErrorSeverity::Error)
                                .specification("§6.1".to_string(), None)
                                .suggestion(format!("Check if type '{}' is imported or declared", name))
                                .build();
                            type_error_with_metadata(error_location, format!("Undefined type: {}", name), metadata)
                        }
                    }
                }
            }
            // For complex types, resolve their component types
            Type::Tuple(types) => {
                let mut resolved_types = Vec::new();
                for t in types {
                    // For tuples, we don't have per-element location, so pass None
                    resolved_types.push(self.resolve_type_with_location(t, None)?);
                }
                Ok(Type::Tuple(resolved_types))
            }
            Type::Record(fields) => {
                let mut resolved_fields = Vec::new();
                for (name, t) in fields {
                    // For records, we don't have per-field location, so pass None
                    resolved_fields.push((name.clone(), self.resolve_type_with_location(t, None)?));
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
        self.resolve_type_name_with_location(name, None)
    }

    /// Resolve a type name to its canonical type with location for error reporting
    pub fn resolve_type_name_with_location(&self, name: &str, location: Option<SourceLocation>) -> Result<Type> {
        if let Some(alias_target) = self.type_aliases.get(name) {
            // Pass location through to resolve_type
            self.resolve_type_with_location(alias_target, location)
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
                _ => {
                    let error_location = location.unwrap_or_else(|| SourceLocation::unknown());
                    let metadata = ErrorMetadataBuilder::new("E2002".to_string())
                        .severity(ErrorSeverity::Error)
                        .specification("§6.1".to_string(), None)
                        .suggestion(format!("Check if type '{}' is imported or declared", name))
                        .build();
                    type_error_with_metadata(error_location, format!("Undefined type: {}", name), metadata)
                }
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
            let metadata = ErrorMetadataBuilder::new("E2004".to_string())
                .severity(ErrorSeverity::Error)
                .specification("§6".to_string(), None)
                .suggestion(format!("Rename variable '{}' to avoid shadowing", name))
                .build();
            return type_error_with_metadata(
                location.clone(),
                format!("Variable '{}' shadows an existing binding. Variable shadowing is not allowed in Silica.", name),
                metadata
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
            Expression::Cast(cast) => &cast.location,
            Expression::Recv(recv) => &recv.location,
            Expression::ReadFile(read_file) => &read_file.location,
            Expression::WriteFile(write_file) => &write_file.location,
            Expression::Print(print) => &print.location,
            Expression::PrintLn(println) => &println.location,
            Expression::PrintInt(print_int) => &print_int.location,
            Expression::PrintBool(print_bool) => &print_bool.location,
            Expression::PrintChar(print_char) => &print_char.location,
            Expression::GetCpuTopologyInfo(get_topology) => &get_topology.location,
            Expression::ExecCommand(exec_cmd) => &exec_cmd.location,
            Expression::StructLiteral(struct_lit) => &struct_lit.location,
            Expression::FieldAccess(field_access) => &field_access.location,
            Expression::Tuple(_) => panic!("Tuple location should be handled specially"),
            Expression::ConstructorCall(ctor) => &ctor.location,
            Expression::FunctionLiteral(func) => &func.location,
            Expression::ReadLines(read_lines) => &read_lines.location,
            Expression::AppendFile(append_file) => &append_file.location,
            Expression::FileExists(file_exists) => &file_exists.location,
            Expression::DeleteFile(delete_file) => &delete_file.location,
            Expression::GetFileSize(get_file_size) => &get_file_size.location,
            Expression::CreateDirectory(create_dir) => &create_dir.location,
            Expression::RemoveDirectory(remove_dir) => &remove_dir.location,
            Expression::ListDirectory(list_dir) => &list_dir.location,
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
            // Handle named types that refer to structs as equal to their record representations
            (Type::Named(name), Type::Record(fields)) |
            (Type::Record(fields), Type::Named(name)) => {
                if let Some(struct_def) = self.struct_defs.get(name) {
                    // Check if the record fields match the struct definition
                    struct_def.len() == fields.len() &&
                    struct_def.iter().zip(fields.iter()).all(|(struct_field, (record_name, record_type))|
                        struct_field.name == *record_name && self.types_equal(&struct_field.ty, record_type)
                    )
                } else {
                    false
                }
            }
            (Type::Int, Type::Int) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Char, Type::Char) => true,
            (Type::String, Type::String) => true,
            (Type::Unit, Type::Unit) => true,
            (Type::ActorRef, Type::ActorRef) => true,
            (Type::Tuple(types1), Type::Tuple(types2)) => {
                types1.len() == types2.len() &&
                types1.iter().zip(types2.iter()).all(|(t1, t2)| self.types_equal(t1, t2))
            }
            (Type::Record(fields1), Type::Record(fields2)) => {
                fields1.len() == fields2.len() &&
                fields1.iter().zip(fields2.iter()).all(|((name1, ty1), (name2, ty2))|
                    name1 == name2 && self.types_equal(ty1, ty2)
                )
            }
            (Type::Function { parameters: params1, return_type: ret1 },
             Type::Function { parameters: params2, return_type: ret2 }) => {
                params1.len() == params2.len() &&
                params1.iter().zip(params2.iter()).all(|(p1, p2)| self.types_equal(p1, p2)) &&
                self.types_equal(ret1, ret2)
            }
            (Type::Region { space: ref space1 }, Type::Region { space: ref space2 }) => space1 == space2,
            (Type::Reference { region: ref reg1, space: ref space1, element_type: ref elem1 },
             Type::Reference { region: ref reg2, space: ref space2, element_type: ref elem2 }) => {
                let regions_compatible = match (&**reg1, &**reg2) {
                    (&Type::Named(ref name1), &Type::Region { space: ref space2 }) => {
                        // Check if the named type refers to a region variable
                        if let Some(scheme) = self.env.get(name1) {
                            matches!(&scheme.ty, Type::Region { space } if space == space2)
                        } else {
                            false
                        }
                    },
                    (&Type::Region { space: ref space1 }, &Type::Named(ref name2)) => {
                        // Check if the named type refers to a region variable
                        if let Some(scheme) = self.env.get(name2) {
                            matches!(&scheme.ty, Type::Region { space } if space == space1)
                        } else {
                            false
                        }
                    },
                    _ => self.types_equal(reg1, reg2)
                };
                space1 == space2 &&
                regions_compatible &&
                self.types_equal(elem1, elem2)
            }
            (Type::Buffer { region: reg1, space: space1, element_type: elem1, capacity: cap1 },
             Type::Buffer { region: reg2, space: space2, element_type: elem2, capacity: cap2 }) => {
                space1 == space2 &&
                cap1 == cap2 &&
                self.types_equal(reg1, reg2) &&
                self.types_equal(elem1, elem2)
            }
            _ => false, // Doesn't handle all complex types yet
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
        
        // Initialize built-in traits
        let mut trait_defs = HashMap::new();
        
        // Add ActorMessage trait (marker trait for actor messages)
        trait_defs.insert("ActorMessage".to_string(), TraitDecl {
            name: "ActorMessage".to_string(),
            included_traits: Vec::new(),
            associated_types: Vec::new(),
            methods: Vec::new(), // Marker trait - no methods
            location: SourceLocation::unknown(),
        });
        
        // Add ActorState trait (marker trait for actor initial state)
        trait_defs.insert("ActorState".to_string(), TraitDecl {
            name: "ActorState".to_string(),
            included_traits: Vec::new(),
            associated_types: Vec::new(),
            methods: Vec::new(), // Marker trait - no methods
            location: SourceLocation::unknown(),
        });
        
        // Add trait types to environment
        env.insert("ActorMessage".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Named("ActorMessage".to_string()),
        });
        env.insert("ActorState".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Named("ActorState".to_string()),
        });
        
        TypeChecker {
            env,
            constraints: Vec::new(),
            substitution: Substitution::new(),
            struct_defs: HashMap::new(),
            trait_impls: Vec::new(),
            trait_defs,
            type_aliases: HashMap::new(),
            type_alias_decls: HashMap::new(),
            symbol_table,
            expression_types: HashMap::new(),
            actor_mailbox_types: HashMap::new(),
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
        // Validate parameter types with location BEFORE expanding aliases
        for param in &func.parameters {
            self.validate_type_with_location(&param.type_, Some(param.location.clone()))?;
        }

        // Validate return type with location BEFORE expanding aliases
        if let Some(ref rt) = func.return_type {
            // Use function location as fallback for return type (return type doesn't have its own location in AST)
            self.validate_type_with_location(rt, Some(func.location.clone()))?;
        }

        // Convert parameter types, expanding aliases first
        let param_types: Vec<Type> = func.parameters.iter()
            .map(|param| self.expand_type_aliases(&param.type_))
            .collect();

        // Convert return type, expanding aliases first
        let return_type = func.return_type.as_ref()
            .map(|rt| self.expand_type_aliases(rt))
            .unwrap_or(Type::Unit);

        // Create function type
        let func_type = Type::Function {
            parameters: param_types.clone(),
            return_type: Box::new(return_type),
        };

        // Add function to environment
        let scheme = TypeScheme {
            vars: vec![],
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

        let body_type = self.infer_statements(&func.body)?;
        let expected_return = func.return_type.as_ref()
            .map(|rt| rt.clone())
            .unwrap_or(Type::Unit);

        // Restore environment
        self.env = saved_env;

        // Check return type directly to provide better error messages with location
        // First try to unify, and if it fails, provide a better error message
        if let Err(e) = self.unify_with_location(&body_type, &expected_return, Some(func.location.clone())) {
            // If unification failed, provide a better error message with function location
            if let CompilerError::TypeError { message, .. } = &e {
                if message.contains("Cannot unify types") {
                    let metadata = ErrorMetadataBuilder::new("E2008".to_string())
                        .severity(ErrorSeverity::Error)
                        .specification("§6.3".to_string(), None)
                        .expected_actual(
                            format!("{:?}", expected_return),
                            format!("{:?}", body_type)
                        )
                        .build();
                    return type_error_with_metadata(
                        func.location.clone(),
                        format!("Function '{}' declared to return {:?}, but body returns {:?}", 
                                func.name, expected_return, body_type),
                        metadata,
                    );
                }
            }
            return Err(e);
        }

        Ok(())
    }

    /// Collect captured variables from statements (for type checking)
    fn collect_captured_variables_from_statements(&self, statements: &[Statement], parameters: &[Parameter], captured: &mut Vec<String>) -> Result<()> {
        let mut local_vars = std::collections::HashSet::new();

        // Add parameters as local variables
        for param in parameters {
            local_vars.insert(param.name.clone());
        }

        // Collect bound variables from statements
        for statement in statements {
            if let Statement::Bind { pattern, .. } = statement {
                self.collect_bound_vars_from_pattern_typecheck(pattern, &mut local_vars);
            }
        }

        // Collect used variables from all expressions in statements
        for statement in statements {
            match statement {
                Statement::Bind { expr, .. } => {
                    self.collect_used_variables(expr, captured)?;
                }
                Statement::Expr(expr) => {
                    self.collect_used_variables(expr, captured)?;
                }
            }
        }

        // Remove duplicates and filter out local variables
        let mut unique_captured = std::collections::HashSet::new();
        for var in captured.iter() {
            if !local_vars.contains(var) {
                unique_captured.insert(var.clone());
            }
        }
        captured.clear();
        captured.extend(unique_captured);

        Ok(())
    }

    /// Collect bound variables from a pattern (for type checking)
    fn collect_bound_vars_from_pattern_typecheck(&self, pattern: &Pattern, bound_vars: &mut std::collections::HashSet<String>) {
        match pattern {
            Pattern::Identifier(name) => {
                bound_vars.insert(name.clone());
            }
            Pattern::TypedIdentifier { name, .. } => {
                if name != "_" {
                    bound_vars.insert(name.clone());
                }
            }
            Pattern::Tuple(patterns) => {
                for pattern in patterns {
                    self.collect_bound_vars_from_pattern_typecheck(pattern, bound_vars);
                }
            }
            Pattern::Literal(_) => {
                // Literals don't bind variables
            }
            Pattern::Record(fields) => {
                for (_, field_pattern) in fields {
                    self.collect_bound_vars_from_pattern_typecheck(field_pattern, bound_vars);
                }
            }
            Pattern::Variant { payload, .. } => {
                if let Some(payload_pattern) = payload {
                    self.collect_bound_vars_from_pattern_typecheck(payload_pattern, bound_vars);
                }
            }
            Pattern::Alternative(patterns) => {
                for pattern in patterns {
                    self.collect_bound_vars_from_pattern_typecheck(pattern, bound_vars);
                }
            }
        }
    }

    /// Recursively collect used variables from an expression
    fn collect_used_variables(&self, expr: &Expression, used: &mut Vec<String>) -> Result<()> {
        match expr {
            Expression::Identifier(name) => {
                used.push(name.clone());
            }
            Expression::Binary(binary) => {
                self.collect_used_variables(&binary.left, used)?;
                self.collect_used_variables(&binary.right, used)?;
            }
            Expression::Unary(unary) => {
                self.collect_used_variables(&unary.operand, used)?;
            }
            Expression::Call(call) => {
                self.collect_used_variables(&call.function, used)?;
                for arg in &call.arguments {
                    self.collect_used_variables(arg, used)?;
                }
            }
            Expression::If(if_expr) => {
                self.collect_used_variables(&if_expr.condition, used)?;
                self.collect_used_variables(&if_expr.then_branch, used)?;
                self.collect_used_variables(&if_expr.else_branch, used)?;
            }
            Expression::Case(case) => {
                self.collect_used_variables(&case.scrutinee, used)?;
                for branch in &case.branches {
                    self.collect_used_variables(&branch.body, used)?;
                }
            }
            Expression::Do(do_expr) => {
                for statement in &do_expr.statements {
                    match statement {
                        Statement::Bind { expr, .. } => {
                            self.collect_used_variables(expr, used)?;
                        }
                        Statement::Expr(expr) => {
                            self.collect_used_variables(expr, used)?;
                        }
                    }
                }
            }
            Expression::FunctionLiteral(func_lit) => {
                // For nested function literals, recursively collect their used variables
                self.collect_captured_variables_from_statements(&func_lit.body, &func_lit.parameters, used)?;
            }
            // Other expression types don't introduce variable usage
            _ => {}
        }
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
            Expression::Cast(cast) => self.infer_cast(cast)?,
            Expression::Recv(recv) => self.infer_recv(recv)?,
            Expression::ReadFile(read_file) => self.infer_read_file(read_file)?,
            Expression::WriteFile(write_file) => self.infer_write_file(write_file)?,
            Expression::Print(print) => self.infer_print(print)?,
            Expression::PrintLn(println) => self.infer_println(println)?,
            Expression::PrintInt(print_int) => self.infer_print_int(print_int)?,
            Expression::PrintBool(print_bool) => self.infer_print_bool(print_bool)?,
            Expression::PrintChar(print_char) => self.infer_print_char(print_char)?,
            Expression::ReadLines(read_lines) => self.infer_read_lines(read_lines)?,
            Expression::AppendFile(append_file) => self.infer_append_file(append_file)?,
            Expression::FileExists(file_exists) => self.infer_file_exists(file_exists)?,
            Expression::DeleteFile(delete_file) => self.infer_delete_file(delete_file)?,
            Expression::GetFileSize(get_file_size) => self.infer_get_file_size(get_file_size)?,
            Expression::CreateDirectory(create_dir) => self.infer_create_directory(create_dir)?,
            Expression::RemoveDirectory(remove_dir) => self.infer_remove_directory(remove_dir)?,
            Expression::ListDirectory(list_dir) => self.infer_list_directory(list_dir)?,
            Expression::ExecCommand(exec_cmd) => self.infer_exec_command(exec_cmd)?,
            Expression::Tuple(exprs) => self.infer_tuple(exprs)?,
            Expression::StructLiteral(struct_lit) => {
                // eprintln!("DEBUG INFER: StructLiteral case hit for type {}", struct_lit.type_name);
                self.infer_struct_literal(struct_lit)?
            },
            Expression::FieldAccess(field_access) => self.infer_field_access(field_access)?,
            Expression::GetCpuTopologyInfo(_) => Type::String, // Returns string pointer
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
        let resolved_type = self.resolve_type_name_with_location(&struct_lit.type_name, Some(struct_lit.location.clone()))?;
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
                        // Resolve both types through type aliases before comparison
                        let resolved_field_type = self.resolve_type_with_location(&field_type, Some(struct_lit.location.clone()))?;
                        let resolved_expected_type = self.resolve_type_with_location(expected_type, Some(struct_lit.location.clone()))?;
                        if !self.types_equal(&resolved_field_type, &resolved_expected_type) {
                            return type_error(
                                struct_lit.location.clone(),
                                format!(
                                    "Field '{}' expects type {:?} but got {:?}",
                                    field_name, resolved_expected_type, resolved_field_type
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

                // Resolve both types through type aliases before comparison
                let resolved_field_type = self.resolve_type_with_location(&field_type, Some(struct_lit.location.clone()))?;
                let resolved_expected_type = self.resolve_type_with_location(&expected_field.ty, Some(struct_lit.location.clone()))?;
                if !self.types_equal(&resolved_field_type, &resolved_expected_type) {
                    return type_error(
                        struct_lit.location.clone(),
                        format!(
                            "Field '{}' expects type {:?} but got {:?}",
                            field_name, resolved_expected_type, resolved_field_type
                        ),
                    );
                }
            }

            // Return the named struct type
            Ok(Type::Named(struct_lit.type_name.clone()))
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
        // Expand type aliases to get the actual type
        let expanded_object_type = self.expand_type_aliases(&object_type);

        // Check if it's a record/struct type or a named type that refers to a struct
        match &expanded_object_type {
            Type::Record(fields) => {
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
            }
            Type::Named(name) => {
                // Check if this named type refers to a struct
                if let Some(struct_def) = self.struct_defs.get(name) {
                    // Find the field in the struct definition
                    for field in struct_def {
                        if field.name == field_access.field {
                            return Ok(field.ty.clone());
                        }
                    }
                    return type_error(
                        field_access.location.clone(),
                        format!("Field '{}' not found in struct '{}'", field_access.field, name),
                    );
                } else {
                    return type_error(
                        field_access.location.clone(),
                        format!("Cannot access field '{}' on non-struct type {:?}", field_access.field, object_type),
                    );
                }
            }
            _ => {
                return type_error(
                    field_access.location.clone(),
                    format!("Cannot access field '{}' on non-struct type {:?}", field_access.field, object_type),
                );
            }
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
            Expression::Cast(cast) => Some(&cast.location),
            Expression::Recv(recv) => Some(&recv.location),
            Expression::ReadFile(read_file) => Some(&read_file.location),
            Expression::WriteFile(write_file) => Some(&write_file.location),
            Expression::Print(print) => Some(&print.location),
            Expression::PrintLn(println) => Some(&println.location),
            Expression::PrintInt(print_int) => Some(&print_int.location),
            Expression::PrintBool(print_bool) => Some(&print_bool.location),
            Expression::PrintChar(print_char) => Some(&print_char.location),
            Expression::GetCpuTopologyInfo(get_topology) => Some(&get_topology.location),
            Expression::ReadLines(read_lines) => Some(&read_lines.location),
            Expression::AppendFile(append_file) => Some(&append_file.location),
            Expression::FileExists(file_exists) => Some(&file_exists.location),
            Expression::DeleteFile(delete_file) => Some(&delete_file.location),
            Expression::GetFileSize(get_file_size) => Some(&get_file_size.location),
            Expression::CreateDirectory(create_dir) => Some(&create_dir.location),
            Expression::RemoveDirectory(remove_dir) => Some(&remove_dir.location),
            Expression::ListDirectory(list_dir) => Some(&list_dir.location),
            Expression::ExecCommand(exec_cmd) => Some(&exec_cmd.location),
            Expression::StructLiteral(struct_lit) => Some(&struct_lit.location),
            Expression::FieldAccess(field_access) => Some(&field_access.location),
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
        // Check core affinity types
        if name == "core_id" {
            return Ok(Type::CoreId);
        } else if name == "core_set" {
            return Ok(Type::Function {
                parameters: vec![], // Variable arguments for core IDs
                return_type: Box::new(Type::CoreSet(vec![])),
            });
        } else if name == "any_core" {
            return Ok(Type::AnyCore);
        } else if name == "performance_cores" {
            return Ok(Type::PerformanceCores);
        } else if name == "efficiency_cores" {
            return Ok(Type::EfficiencyCores);
        } else if name == "core_id" {
            return Ok(Type::Function {
                parameters: vec![Type::Int],
                return_type: Box::new(Type::CoreId),
            });
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

        // Special handling for I/O functions
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
                let path_location = Self::try_get_expression_location(&call.arguments[0]).cloned();
                self.unify_with_location(&path_type, &Type::Named("string".to_string()), path_location)?;
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
                let path_location = Self::try_get_expression_location(&call.arguments[0]).cloned();
                self.unify_with_location(&path_type, &Type::Named("string".to_string()), path_location)?;
                let content_type = self.infer_expression(&call.arguments[1])?;
                let content_location = Self::try_get_expression_location(&call.arguments[1]).cloned();
                self.unify_with_location(&content_type, &Type::Named("string".to_string()), content_location)?;
                return Ok(Type::Named("Result".to_string()));
            }
        }

        // Handle special built-in function calls
        if let Expression::Identifier(func_name) = &*call.function {
            if func_name == "core_id" {
                if call.arguments.len() != 1 {
                    return type_error(
                        call.location.clone(),
                        "core_id expects exactly 1 argument".to_string(),
                    );
                }
                let arg_type = self.infer_expression(&call.arguments[0])?;
                self.add_constraint(arg_type, Type::Int);
                return Ok(Type::CoreId);
            }
        }

        // Try to infer the function type, and provide a better error message
        // if it's an undefined identifier used as a function
        let func_type = match self.infer_expression(&call.function) {
            Ok(ty) => ty,
            Err(e) => {
                // If it's an identifier and the error is about undefined variable,
                // provide a better error message for function calls
                if let Expression::Identifier(func_name) = &*call.function {
                    if let CompilerError::TypeError { message, .. } = &e {
                        if message.contains("Undefined variable") {
                            return type_error(
                                call.location.clone(),
                                format!("Undefined function: {}", func_name),
                            );
                        }
                    }
                }
                return Err(e);
            }
        };

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
        // Convert parameter types
        let param_types: Vec<Type> = func.parameters.iter()
            .map(|param| param.type_.clone())
            .collect();

        // Convert return type
        let return_type = func.return_type.clone().unwrap_or(Type::Unit);

        // Create function type
        let func_type = Type::Function {
            parameters: param_types.clone(),
            return_type: Box::new(return_type.clone()),
        };

        // Analyze captured variables by examining the function body
        let mut captured_vars = Vec::new();
        self.collect_captured_variables_from_statements(&func.body, &func.parameters, &mut captured_vars)?;

        // Create local environment for function body checking
        let mut local_env = self.env.clone();

        // Add captured variables to local environment
        for captured_var in &captured_vars {
            if let Some(var_scheme) = self.env.get(captured_var) {
                local_env.insert(captured_var.clone(), var_scheme.clone());
            } else {
                return type_error(
                    func.location.clone(),
                    format!("Undefined variable '{}' in function literal", captured_var)
                );
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

        let body_type = self.infer_statements(&func.body)?;
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
        // Infer scrutinee type for pattern checking
        let scrutinee_type = self.infer_expression(&case.scrutinee)?;

        if case.branches.is_empty() {
            return type_error(case.location.clone(), "Case expression must have at least one branch".to_string());
        }


        // Basic exhaustiveness checking for bootstrap compiler
        self.check_exhaustiveness(&scrutinee_type, &case.branches, &case.location)?;

        // Infer return type from case branches
        // All branches must have the same type
        let mut first_branch_type = None;

        for (branch_idx, branch) in case.branches.iter().enumerate() {
            // Create pattern environment for this branch
            let mut pattern_env = HashMap::new();
            self.check_pattern(&branch.pattern, &scrutinee_type, &case.location, &mut pattern_env)?;

            // Check guard expression if present (must be Bool type)
            // Guards are evaluated with pattern variables in scope
            if let Some(ref guard) = branch.guard {
                // Set up pattern variables in environment for guard evaluation
                let saved_env = self.env.clone();
                for (var_name, type_scheme) in &pattern_env {
                    self.env.insert(var_name.clone(), type_scheme.clone());
                }

                // Guards must evaluate to boolean type
                let guard_type = self.infer_expression(guard)?;
                self.add_constraint(Type::Bool, guard_type);

                // Restore environment
                self.env = saved_env;
            }

            // Set up pattern variables in environment for body type checking
            let saved_env = self.env.clone();
            for (var_name, type_scheme) in &pattern_env {
                self.env.insert(var_name.clone(), type_scheme.clone());
            }

            // Infer branch body type with pattern variables in scope
            let branch_type = self.infer_expression(&branch.body)?;

            // Restore environment
            self.env = saved_env;

            // Check type consistency
            if branch_idx == 0 {
                first_branch_type = Some(branch_type.clone());
            } else {
                // For bootstrap compiler, skip unification and assume all branches have the same type
                // The code generator will handle this properly
            }
        }

        Ok(first_branch_type.unwrap())
    }

    /// Check that a pattern is compatible with the expected type
    fn check_pattern_type(&mut self, pattern: &Pattern, expected_type: &Type, location: &SourceLocation) -> Result<()> {
        match pattern {
            Pattern::Identifier(_) => {
                // Untyped identifiers accept any type
            }
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
            Pattern::TypedIdentifier { type_: pattern_type, .. } => {
                // Check that the pattern type matches the expected type
                self.add_constraint(expected_type.clone(), pattern_type.clone());
            }
            Pattern::TypedIdentifier { type_, .. } => {
                // Check that the pattern type matches the expected type
                self.add_constraint(expected_type.clone(), type_.clone());
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

                    // Require explicit type annotations for ALL bindings
                    if let crate::ast::Pattern::Identifier(_) = pattern {
                        let metadata = ErrorMetadataBuilder::new("E2000".to_string())
                            .severity(ErrorSeverity::Error)
                            .specification("§6".to_string(), None)
                            .suggestion("Add explicit type annotation: variable:type <- expression".to_string())
                            .build();
                        return Err(CompilerError::TypeError {
                            location: do_expr.location.clone(),
                            message: format!("Variable bindings must have explicit type annotations. Use 'variable:type <- expression' instead of 'variable <- expression'"),
                            metadata,
                        });
                    }

                    // Bind pattern variables to the type environment (keep named types for method dispatch)
                    self.bind_pattern_variables(pattern, &expr_type, &do_expr.location)?;

                    // Bind statements don't contribute to the return type - they just bind variables
                    // The return type comes from the final expression, or Unit if there is none
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
                // Untyped identifier - bind to the actual type
                if self.env.contains_key(name) {
                    return type_error(location.clone(), format!("Pattern variable '{}' shadows an existing binding", name));
                }
                self.env.insert(name.clone(), TypeScheme {
                    vars: vec![],
                    ty: ty.clone(),
                });
                Ok(())
            }
            Pattern::TypedIdentifier { name, type_ } => {
                // Check for variable shadowing before binding (skip for wildcards)
                if name != "_" {
                    self.check_variable_shadowing(name, location)?;
                }
                // Verify that the declared type matches the actual type
                let expanded_declared_ty = self.expand_type_aliases(type_);
                let expanded_actual_ty = self.expand_type_aliases(ty);
                let types_match = self.types_equal(&expanded_actual_ty, &expanded_declared_ty);

                if !types_match {
                    return type_error(location.clone(),
                        format!("BIND: Pattern declares type {:?} (expanded: {:?}) but value has type {:?}",
                               type_, expanded_declared_ty, expanded_actual_ty));
                }
                // Bind identifier pattern to the declared type (skip for wildcards)
                if name != "_" {
                    self.env.insert(name.clone(), TypeScheme {
                        vars: vec![], // No type variables for now
                        ty: type_.clone(), // Use declared type
                    });
                }
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
        self.unify_with_location(t1, t2, None)
    }

    /// Unify two types with location for error reporting
    fn unify_with_location(&mut self, t1: &Type, t2: &Type, location: Option<SourceLocation>) -> Result<()> {
        // Try to expand Named types to their underlying types before unification
        // This handles type aliases and struct names
        let expanded_t1 = self.expand_type_aliases(t1);
        let expanded_t2 = self.expand_type_aliases(t2);

        match (&expanded_t1, &expanded_t2) {
            // Identical types unify trivially
            (Type::Unit, Type::Unit) |
            (Type::Bool, Type::Bool) |
            (Type::Int, Type::Int) |
            (Type::Char, Type::Char) |
            (Type::String, Type::String) |
            (Type::ActorRef, Type::ActorRef) => Ok(()),

            // Variable unification
            (Type::Variable(var), ty) | (ty, Type::Variable(var)) => {
                self.unify_variable(var, ty)
            }

            // Function unification
            (Type::Function { parameters: params1, return_type: ret1 },
             Type::Function { parameters: params2, return_type: ret2 }) => {
                if params1.len() != params2.len() {
                    let error_location = location.unwrap_or_else(|| SourceLocation::unknown());
                    let metadata = ErrorMetadataBuilder::new("E2007".to_string())
                        .severity(ErrorSeverity::Error)
                        .specification("§6.3".to_string(), None)
                        .expected_actual(
                            format!("Function with {} parameters", params1.len()),
                            format!("Function with {} parameters", params2.len())
                        )
                        .build();
                    return type_error_with_metadata(
                        error_location,
                        format!("Function arity mismatch: expected {} parameters, found {}", params1.len(), params2.len()),
                        metadata,
                    );
                }
                for (p1, p2) in params1.iter().zip(params2) {
                    self.unify_with_location(p1, p2, location.clone())?;
                }
                self.unify_with_location(ret1, ret2, location)
            }

            // Closure unification
            (Type::Closure { parameters: params1, return_type: ret1, captured_types: caps1 },
             Type::Closure { parameters: params2, return_type: ret2, captured_types: caps2 }) => {
                if params1.len() != params2.len() {
                    let error_location = location.unwrap_or_else(|| SourceLocation::unknown());
                    let metadata = ErrorMetadataBuilder::new("E2007".to_string())
                        .severity(ErrorSeverity::Error)
                        .specification("§6.3".to_string(), None)
                        .expected_actual(
                            format!("Closure with {} parameters", params1.len()),
                            format!("Closure with {} parameters", params2.len())
                        )
                        .build();
                    return type_error_with_metadata(
                        error_location,
                        format!("Closure arity mismatch: expected {} parameters, found {}", params1.len(), params2.len()),
                        metadata,
                    );
                }
                if caps1.len() != caps2.len() {
                    let error_location = location.unwrap_or_else(|| SourceLocation::unknown());
                    let metadata = ErrorMetadataBuilder::new("E2007".to_string())
                        .severity(ErrorSeverity::Error)
                        .specification("§6.3".to_string(), None)
                        .expected_actual(
                            format!("Closure with {} captured variables", caps1.len()),
                            format!("Closure with {} captured variables", caps2.len())
                        )
                        .build();
                    return type_error_with_metadata(
                        error_location,
                        format!("Closure capture count mismatch: expected {} captures, found {}", caps1.len(), caps2.len()),
                        metadata,
                    );
                }
                for (p1, p2) in params1.iter().zip(params2) {
                    self.unify_with_location(p1, p2, location.clone())?;
                }
                for (c1, c2) in caps1.iter().zip(caps2) {
                    self.unify_with_location(c1, c2, location.clone())?;
                }
                self.unify_with_location(ret1, ret2, location)
            }

            // Unify Closure with Function (closure can be used as function)
            (Type::Closure { parameters: params1, return_type: ret1, .. },
             Type::Function { parameters: params2, return_type: ret2 }) |
            (Type::Function { parameters: params2, return_type: ret2 },
             Type::Closure { parameters: params1, return_type: ret1, .. }) => {
                if params1.len() != params2.len() {
                    let error_location = location.unwrap_or_else(|| SourceLocation::unknown());
                    let metadata = ErrorMetadataBuilder::new("E2007".to_string())
                        .severity(ErrorSeverity::Error)
                        .specification("§6.3".to_string(), None)
                        .expected_actual(
                            format!("Function/closure with {} parameters", params1.len()),
                            format!("Function/closure with {} parameters", params2.len())
                        )
                        .build();
                    return type_error_with_metadata(
                        error_location,
                        format!("Function/closure arity mismatch: expected {} parameters, found {}", params1.len(), params2.len()),
                        metadata,
                    );
                }
                for (p1, p2) in params1.iter().zip(params2) {
                    self.unify_with_location(p1, p2, location.clone())?;
                }
                self.unify_with_location(ret1, ret2, location)
            }

            // Process unification
            (Type::Process { effects: e1, result_type: r1 },
             Type::Process { effects: e2, result_type: r2 }) => {
                // TODO: Effect unification
                self.unify_with_location(r1, r2, location)
            }

            // Tuple unification
            (Type::Tuple(types1), Type::Tuple(types2)) => {
                if types1.len() != types2.len() {
                    let error_location = location.unwrap_or_else(|| SourceLocation::unknown());
                    let metadata = ErrorMetadataBuilder::new("E2005".to_string())
                        .severity(ErrorSeverity::Error)
                        .specification("§6.1".to_string(), None)
                        .expected_actual(
                            format!("Tuple with {} elements", types1.len()),
                            format!("Tuple with {} elements", types2.len())
                        )
                        .build();
                    return type_error_with_metadata(
                        error_location,
                        format!("Tuple arity mismatch: expected {} elements, found {}", types1.len(), types2.len()),
                        metadata,
                    );
                }
                for (t1, t2) in types1.iter().zip(types2) {
                    self.unify_with_location(t1, t2, location.clone())?;
                }
                Ok(())
            }

            // Record unification
            (Type::Record(fields1), Type::Record(fields2)) => {
                if fields1.len() != fields2.len() {
                    let error_location = location.unwrap_or_else(|| SourceLocation::unknown());
                    let metadata = ErrorMetadataBuilder::new("E2006".to_string())
                        .severity(ErrorSeverity::Error)
                        .specification("§6.1".to_string(), None)
                        .expected_actual(
                            format!("Record with {} fields", fields1.len()),
                            format!("Record with {} fields", fields2.len())
                        )
                        .build();
                    return type_error_with_metadata(
                        error_location,
                        format!("Record field count mismatch: expected {} fields, found {}", fields1.len(), fields2.len()),
                        metadata,
                    );
                }
                // TODO: Field name matching
                for ((_, t1), (_, t2)) in fields1.iter().zip(fields2) {
                    self.unify_with_location(t1, t2, location.clone())?;
                }
                Ok(())
            }

            // Named type unification - handle both original types and expanded types
            // First check if both are Named with same name
            (Type::Named(name1), Type::Named(name2)) if name1 == name2 => Ok(()),
            
            // Handle Named with Record (after expansion, Named might still be present if it's not a struct/alias)
            // Check if Named type refers to a struct that matches the Record
            (Type::Named(name), Type::Record(fields)) |
            (Type::Record(fields), Type::Named(name)) => {
                // Check if the Named type refers to a struct definition
                // Clone the struct_def to avoid borrow checker issues
                let struct_def_opt = self.struct_defs.get(name).cloned();
                if let Some(struct_def) = struct_def_opt {
                    // Check if the record fields match the struct definition
                    if struct_def.len() != fields.len() {
                        let error_location = location.unwrap_or_else(|| SourceLocation::unknown());
                        let metadata = ErrorMetadataBuilder::new("E2006".to_string())
                            .severity(ErrorSeverity::Error)
                            .specification("§6.1".to_string(), None)
                            .expected_actual(
                                format!("Struct {} with {} fields", name, struct_def.len()),
                                format!("Record with {} fields", fields.len())
                            )
                            .build();
                        return type_error_with_metadata(
                            error_location,
                            format!("Type mismatch: struct {} expects {} fields, but record has {}", name, struct_def.len(), fields.len()),
                            metadata,
                        );
                    }
                    // Unify each field type
                    for (struct_field, (record_name, record_type)) in struct_def.iter().zip(fields.iter()) {
                        if struct_field.name != *record_name {
                            let error_location = location.unwrap_or_else(|| SourceLocation::unknown());
                            let metadata = ErrorMetadataBuilder::new("E2006".to_string())
                                .severity(ErrorSeverity::Error)
                                .specification("§6.1".to_string(), None)
                                .expected_actual(
                                    format!("Field '{}'", struct_field.name),
                                    format!("Field '{}'", record_name)
                                )
                                .build();
                            return type_error_with_metadata(
                                error_location,
                                format!("Field name mismatch: expected '{}', found '{}'", struct_field.name, record_name),
                                metadata,
                            );
                        }
                        self.unify_with_location(&struct_field.ty, record_type, location.clone())?;
                    }
                    Ok(())
                } else {
                    // Named type doesn't refer to a struct, try to unify the original types
                    // This handles cases where expansion didn't change the types
                    self.unify_named_with_type(t1, t2, location)
                }
            }

            // Fallback: try to unify original types (in case expansion didn't help)
            _ => {
                // If expanded types are different from original, recursively try with expanded types
                // Otherwise, use the fallback error handler
                if t1 != &expanded_t1 || t2 != &expanded_t2 {
                    // Types were expanded, recursively try with expanded types
                    self.unify_with_location(&expanded_t1, &expanded_t2, location)
                } else {
                    // Types weren't expanded, use fallback error handler
                    self.unify_named_with_type(t1, t2, location)
                }
            }
        }
    }

    /// Helper to unify Named types with other types (fallback)
    fn unify_named_with_type(&mut self, t1: &Type, t2: &Type, location: Option<SourceLocation>) -> Result<()> {
        match (t1, t2) {
            (Type::Named(name1), Type::Named(name2)) if name1 == name2 => Ok(()),
            _ => {
                let error_location = location.unwrap_or_else(|| SourceLocation::unknown());
                let metadata = ErrorMetadataBuilder::new("E2003".to_string())
                    .severity(ErrorSeverity::Error)
                    .specification("§6.3".to_string(), None)
                    .expected_actual(
                        format!("{:?}", t1),
                        format!("{:?}", t2)
                    )
                    .build();
                type_error_with_metadata(
                    error_location,
                    format!("Cannot unify types: {:?} and {:?}", t1, t2),
                    metadata,
                )
            }
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
        let region_type = self.infer_expression(&alloc.region)?;
        let element_type = self.infer_expression(&alloc.initial_value)?;

        // For now, assume normal memory space
        let space = MemorySpace::Normal;

        // If the region is a named variable, look up its actual type from the environment
        let resolved_region_type = if let Type::Named(name) = &region_type {
            if let Some(scheme) = self.env.get(name) {
                scheme.ty.clone()
            } else {
                region_type // Keep as Named if not found
            }
        } else {
            region_type
        };

        Ok(Type::Reference {
            region: Box::new(resolved_region_type),
            space,
            element_type: Box::new(element_type),
        })
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
        // spawn(initial_state, behavior) returns an actor_ref (primitive type)
        // Check that initial_state implements ActorState trait (for named types only)
        let initial_state_type = self.infer_expression(&spawn.initial_state)?;
        
        // Check ActorState trait implementation for named types
        if let Type::Named(_) | Type::Record(_) = initial_state_type {
            // For named types, verify ActorState trait implementation
            if !self.type_implements_trait(&initial_state_type, "ActorState") {
                let error_location = Self::try_get_expression_location(&spawn.initial_state)
                    .unwrap_or(&spawn.location);
                return Err(CompilerError::type_error(
                    error_location.clone(),
                    format!("Type used as actor initial_state must implement ActorState trait")
                ));
            }
        }
        
        // Extract message type from behavior function for mailbox effect tracking
        // Try to extract from function literal first (most common case)
        let message_type = if let Expression::FunctionLiteral(func) = &*spawn.behavior {
            // Behavior function should have (message, state) -> state signature
            // First parameter is the message type
            if let Some(first_param) = func.parameters.first() {
                first_param.type_.clone()
            } else {
                Type::Unit // Fallback if no parameters
            }
        } else {
            // Behavior is not a function literal - try to get type from type inference
            let behavior_type = self.infer_expression(&spawn.behavior)?;
            if let Type::Function { parameters, .. } = behavior_type {
                // First parameter is the message type
                if let Some(first_param) = parameters.first() {
                    first_param.clone()
                } else {
                    Type::Unit // Fallback if no parameters
                }
            } else {
                Type::Unit // Fallback
            }
        };
        
        // Track actor mailbox type for effect checking
        self.actor_mailbox_types.insert(spawn.location.clone(), message_type);
        
        Ok(Type::ActorRef) // Return primitive actor_ref type
    }

    /// Infer type for send expression
    fn infer_send(&mut self, send: &SendExpr) -> Result<Type> {
        // send(actor, message) returns unit
        // Verify message implements ActorMessage trait (for named types)
        let message_type = self.infer_expression(&send.message)?;
        
        // Check ActorMessage trait implementation for named types
        if let Type::Named(_) | Type::Record(_) = message_type {
            // For named types, verify ActorMessage trait implementation
            if !self.type_implements_trait(&message_type, "ActorMessage") {
                let error_location = Self::try_get_expression_location(&send.message)
                    .unwrap_or(&send.location);
                return Err(CompilerError::type_error(
                    error_location.clone(),
                    format!("Message type must implement ActorMessage trait")
                ));
            }
        }
        
        Ok(Type::Unit)
    }

    /// Infer type for recv expression
    fn infer_recv(&mut self, recv: &RecvExpr) -> Result<Type> {
        // recv() returns the received message type
        // For now, assume it returns an integer
        // In a full implementation, this would depend on the actor's mailbox type
        Ok(Type::Int)
    }

    /// Infer type for cast expression
    fn infer_cast(&mut self, cast: &CastExpr) -> Result<Type> {
        // cast(actor: actor_ref, message: ActorMessage) : proc[concurrency] bool
        // Verify actor is actor_ref (primitive type)
        let actor_type = self.infer_expression(&cast.actor)?;
        self.add_constraint(actor_type, Type::ActorRef);
        
        // Verify message implements ActorMessage trait (for named types)
        let message_type = self.infer_expression(&cast.message)?;
        
        // Check ActorMessage trait implementation for named types
        if let Type::Named(_) | Type::Record(_) = message_type {
            // For named types, verify ActorMessage trait implementation
            if !self.type_implements_trait(&message_type, "ActorMessage") {
                let error_location = Self::try_get_expression_location(&cast.message)
                    .unwrap_or(&cast.location);
                return Err(CompilerError::type_error(
                    error_location.clone(),
                    format!("Message type must implement ActorMessage trait")
                ));
            }
        } else {
            // For non-named types, check if it's typed as ActorMessage trait directly
            // This would be handled by trait-as-type resolution
            // For now, we require named types to implement the trait
            let error_location = Self::try_get_expression_location(&cast.message)
                .unwrap_or(&cast.location);
            return Err(CompilerError::type_error(
                error_location.clone(),
                format!("Message must be a named type implementing ActorMessage trait")
            ));
        }
        
        // cast returns bool
        Ok(Type::Bool)
    }

    /// Check if a type implements a given trait
    fn type_implements_trait(&self, ty: &Type, trait_name: &str) -> bool {
        // Check if there's a trait implementation for this type and trait
        for trait_impl in &self.trait_impls {
            if trait_impl.trait_name == trait_name {
                // Check if the types match (using types_equal which handles type aliases)
                if self.types_equal(&trait_impl.for_type, ty) {
                    return true;
                }
            }
        }
        false
    }

    fn infer_read_file(&mut self, read_file: &ReadFileExpr) -> Result<Type> {
        // Check that path is a string
        let path_type = self.infer_expression(&read_file.path)?;
        let path_location = Self::try_get_expression_location(&read_file.path).cloned();
        self.unify_with_location(&path_type, &Type::Named("string".to_string()), path_location)?;

        // read_file returns Result<string, string>
        // For now, we'll represent this as a generic type
        Ok(Type::Named("Result".to_string()))
    }

    fn infer_write_file(&mut self, write_file: &WriteFileExpr) -> Result<Type> {
        // Check that path is a string
        let path_type = self.infer_expression(&write_file.path)?;
        let path_location = Self::try_get_expression_location(&write_file.path).cloned();
        self.unify_with_location(&path_type, &Type::Named("string".to_string()), path_location)?;

        // Check that content is a string
        let content_type = self.infer_expression(&write_file.content)?;
        let content_location = Self::try_get_expression_location(&write_file.content).cloned();
        self.unify_with_location(&content_type, &Type::Named("string".to_string()), content_location)?;

        // write_file returns Result<unit, string>
        Ok(Type::Named("Result".to_string()))
    }

    fn infer_print(&mut self, print: &PrintExpr) -> Result<Type> {
        // Check that value is a string
        let value_type = self.infer_expression(&print.value)?;
        self.unify(&value_type, &Type::String)?;
        // print returns unit
        Ok(Type::Unit)
    }

    fn infer_println(&mut self, println: &PrintLnExpr) -> Result<Type> {
        // Check that value is a string
        let value_type = self.infer_expression(&println.value)?;
        self.unify(&value_type, &Type::String)?;
        // println returns unit
        Ok(Type::Unit)
    }

    fn infer_print_int(&mut self, print_int: &PrintIntExpr) -> Result<Type> {
        // Check that value is an int
        let value_type = self.infer_expression(&print_int.value)?;
        self.unify(&value_type, &Type::Int)?;
        // print_int returns unit
        Ok(Type::Unit)
    }

    fn infer_print_bool(&mut self, print_bool: &PrintBoolExpr) -> Result<Type> {
        // Check that value is a bool
        let value_type = self.infer_expression(&print_bool.value)?;
        self.unify(&value_type, &Type::Bool)?;
        // print_bool returns unit
        Ok(Type::Unit)
    }

    fn infer_print_char(&mut self, print_char: &PrintCharExpr) -> Result<Type> {
        // Check that value is a char
        let value_type = self.infer_expression(&print_char.value)?;
        self.unify(&value_type, &Type::Char)?;
        // print_char returns unit
        Ok(Type::Unit)
    }

    fn infer_read_lines(&mut self, read_lines: &ReadLinesExpr) -> Result<Type> {
        // Check that path is a string
        let path_type = self.infer_expression(&read_lines.path)?;
        self.unify(&path_type, &Type::String)?;
        // read_lines returns string
        Ok(Type::String)
    }

    fn infer_append_file(&mut self, append_file: &AppendFileExpr) -> Result<Type> {
        // Check that path and content are strings
        let path_type = self.infer_expression(&append_file.path)?;
        self.unify(&path_type, &Type::String)?;
        let content_type = self.infer_expression(&append_file.content)?;
        self.unify(&content_type, &Type::String)?;
        // append_file returns bool
        Ok(Type::Bool)
    }

    fn infer_file_exists(&mut self, file_exists: &FileExistsExpr) -> Result<Type> {
        // Check that path is a string
        let path_type = self.infer_expression(&file_exists.path)?;
        self.unify(&path_type, &Type::String)?;
        // file_exists returns bool
        Ok(Type::Bool)
    }

    fn infer_delete_file(&mut self, delete_file: &DeleteFileExpr) -> Result<Type> {
        // Check that path is a string
        let path_type = self.infer_expression(&delete_file.path)?;
        self.unify(&path_type, &Type::String)?;
        // delete_file returns bool
        Ok(Type::Bool)
    }

    fn infer_get_file_size(&mut self, get_file_size: &GetFileSizeExpr) -> Result<Type> {
        // Check that path is a string
        let path_type = self.infer_expression(&get_file_size.path)?;
        self.unify(&path_type, &Type::String)?;
        // get_file_size returns int
        Ok(Type::Int)
    }

    fn infer_create_directory(&mut self, create_dir: &CreateDirectoryExpr) -> Result<Type> {
        // Check that path is a string
        let path_type = self.infer_expression(&create_dir.path)?;
        self.unify(&path_type, &Type::String)?;
        // create_directory returns bool
        Ok(Type::Bool)
    }

    fn infer_remove_directory(&mut self, remove_dir: &RemoveDirectoryExpr) -> Result<Type> {
        // Check that path is a string
        let path_type = self.infer_expression(&remove_dir.path)?;
        self.unify(&path_type, &Type::String)?;
        // remove_directory returns bool
        Ok(Type::Bool)
    }

    fn infer_list_directory(&mut self, list_dir: &ListDirectoryExpr) -> Result<Type> {
        // Check that path is a string
        let path_type = self.infer_expression(&list_dir.path)?;
        self.unify(&path_type, &Type::String)?;
        // list_directory returns string
        Ok(Type::String)
    }

    fn infer_exec_command(&mut self, exec_cmd: &ExecCommandExpr) -> Result<Type> {
        // Check that command is a string
        let cmd_type = self.infer_expression(&exec_cmd.command)?;
        self.unify(&cmd_type, &Type::String)?;

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
            self.validate_type_with_location(&field.ty, Some(field.location.clone()))?;
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
                EnumVariant::Tuple { fields, location, .. } => {
                    for field_type in fields {
                        self.validate_type_with_location(field_type, Some(location.clone()))?;
                    }
                }
                EnumVariant::Struct { fields, location, .. } => {
                    for field in fields {
                        self.validate_type_with_location(&field.ty, Some(field.location.clone()))?;
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
        // Check that all included traits exist
        for included_trait in &trait_decl.included_traits {
            if !self.trait_defs.contains_key(included_trait) {
                return Err(CompilerError::type_error(
                    trait_decl.location.clone(),
                    format!("Trait '{}' includes unknown trait '{}'", trait_decl.name, included_trait)
                ));
            }
        }

        // Check that all method signatures are valid

        // eprintln!("DEBUG TRAIT: check_trait_declaration called for trait {:?}", trait_decl.name);
        // eprintln!("DEBUG TRAIT: methods.len() = {}", trait_decl.methods.len());
        for method in &trait_decl.methods {
            // eprintln!("DEBUG TRAIT: method = {:?}", method.name);
            // eprintln!("DEBUG TRAIT: params.len() = {}", method.params.len());
            
            
            for param in &method.params {
                self.validate_type_with_location(&param.type_, Some(param.location.clone()))?;
            }
            if let Some(ref return_type) = method.return_type {
                // eprintln!("DEBUG TRAIT: return_type = {:?}", return_type);
                // Return type doesn't have direct location, use method location
                self.validate_type_with_location(return_type, Some(method.location.clone()))?;
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
        // Try to extract location from for_type if it's Named
        let for_type_location = match &impl_decl.for_type {
            Type::Named(_) => Some(impl_decl.location.clone()), // Use impl location as fallback
            _ => Some(impl_decl.location.clone()),
        };
        self.validate_type_with_location(&impl_decl.for_type, for_type_location)?;

        // Check all method implementations (validate types without adding to environment)
        for method in &impl_decl.methods {
            // Validate parameter types
            for param in &method.parameters {
                self.validate_type_with_location(&param.type_, Some(param.location.clone()))?;
            }
            // Validate return type
            if let Some(ref return_type) = method.return_type {
                self.validate_type_with_location(return_type, Some(method.location.clone()))?;
            }
        }

        // Check associated type definitions
        for assoc_type_def in &impl_decl.associated_types {
            self.validate_type_with_location(&assoc_type_def.type_, Some(assoc_type_def.location.clone()))?;
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
        // Validate the aliased type - use alias declaration location
        self.validate_type_with_location(&alias_decl.aliased_type, Some(alias_decl.location.clone()))?;

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
                } else if let Some(struct_def) = self.struct_defs.get(name) {
                    // Expand struct names to their record representations
                    Type::Record(
                        struct_def.iter().map(|f| (f.name.clone(), f.ty.clone())).collect()
                    )
                } else if let Some(scheme) = self.env.get(name) {
                    // Check if it's a variable in the environment
                    self.expand_type_aliases(&scheme.ty)
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
        self.validate_type_with_location(ty, None)
    }

    /// Validate that a type is well-formed with location for error reporting
    fn validate_type_with_location(&self, ty: &Type, location: Option<SourceLocation>) -> Result<()> {
        match ty {
            Type::Named(name) => {
                // Allow "Self" as a special type in trait contexts
                if name == "Self" {
                    return Ok(());
                }
                // Check if the named type exists in the environment
                if !self.env.contains_key(name) {
                    let error_location = location.unwrap_or_else(|| SourceLocation::unknown());
                    let metadata = ErrorMetadataBuilder::new("E2002".to_string())
                        .severity(ErrorSeverity::Error)
                        .specification("§6.1".to_string(), None)
                        .suggestion(format!("Check if type '{}' is imported or declared", name))
                        .build();
                    return type_error_with_metadata(
                        error_location,
                        format!("Unknown type: {} (env contains: {:?})", name, self.env.keys().collect::<Vec<_>>()),
                        metadata,
                    );
                }
                Ok(())
            }
            Type::Function { parameters, return_type } => {
                for param in parameters {
                    self.validate_type_with_location(param, location.clone())?;
                }
                self.validate_type_with_location(return_type, location)
            }
            Type::Closure { parameters, return_type, captured_types } => {
                for param in parameters {
                    self.validate_type_with_location(param, location.clone())?;
                }
                for captured in captured_types {
                    self.validate_type_with_location(captured, location.clone())?;
                }
                self.validate_type_with_location(return_type, location)
            }
            Type::Tuple(types) => {
                for ty in types {
                    self.validate_type_with_location(ty, location.clone())?;
                }
                Ok(())
            }
            Type::Record(fields) => {
                for (_, ty) in fields {
                    self.validate_type_with_location(ty, location.clone())?;
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
    fn infer_type_application(&mut self, _constructor: &Type, _args: &[Type]) -> Result<Type> {
        // Since generics are removed, just return a placeholder type
        Ok(Type::Named("TypeApplication".to_string()))
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
            Pattern::TypedIdentifier { name, type_ } => {
                // Skip shadowing check and binding for wildcards
                if name != "_" {
                    if env.contains_key(name) {
                        return type_error(location.clone(), format!("Pattern variable '{}' shadows an existing binding", name));
                    }
                    // Check that the declared type matches the expected type
                    let expanded_declared_type = self.expand_type_aliases(type_);
                    let types_match = self.types_equal(&expanded_expected_type, &expanded_declared_type);

                    if !types_match {
                        return type_error(location.clone(),
                            format!("CHECK: Pattern declares type {:?} (expanded: {:?}) but value has type {:?}",
                                   type_, expanded_declared_type, expanded_expected_type));
                    }
                    env.insert(name.clone(), TypeScheme {
                        vars: vec![],
                        ty: expanded_declared_type,
                    });
                } else {
                    // For wildcards, just check type compatibility but don't bind
                    let expanded_declared_type = self.expand_type_aliases(type_);
                    if !self.types_equal(&expanded_expected_type, &expanded_declared_type) {
                        return type_error(location.clone(),
                            format!("Wildcard pattern declares type {:?} but expected type {:?}",
                                   type_, expanded_expected_type));
                    }
                }
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
            Pattern::Record(fields) => {
                if let Type::Record(record_fields) = &expanded_expected_type {
                    // Create a map of field name to type for easy lookup
                    let mut field_type_map = std::collections::HashMap::new();
                    for (field_name, field_type) in record_fields {
                        field_type_map.insert(field_name, field_type);
                    }

                    // Check that all pattern fields exist in the record type
                    for (field_name, field_pattern) in fields {
                        match field_type_map.get(field_name) {
                            Some(field_type) => {
                                self.check_pattern(field_pattern, field_type, location, env)?;
                            }
                            None => {
                                return type_error(location.clone(),
                                    format!("Record pattern field '{}' does not exist in type {:?}", field_name, expanded_expected_type));
                            }
                        }
                    }
                } else {
                    return type_error(location.clone(),
                        format!("Record pattern expected record type, found {:?}", expanded_expected_type));
                }
            }
            Pattern::Variant { constructor, payload } => {
                if let Type::Variant(variants) = &expanded_expected_type {
                    // Find the variant with the matching constructor
                    let mut found_variant = None;
                    for (variant_name, variant_payload_type) in variants {
                        if variant_name == constructor {
                            found_variant = Some(variant_payload_type);
                            break;
                        }
                    }

                    match (found_variant, payload) {
                        (Some(Some(expected_payload_type)), Some(pattern_payload)) => {
                            // Variant has payload, pattern has payload
                            self.check_pattern(pattern_payload, expected_payload_type, location, env)?;
                        }
                        (Some(None), None) => {
                            // Variant has no payload, pattern has no payload - this is correct
                        }
                        _ => {
                            return type_error(location.clone(),
                                format!("Variant pattern '{}' payload mismatch for type {:?}", constructor, expanded_expected_type));
                        }
                    }
                } else {
                    return type_error(location.clone(),
                        format!("Variant pattern expected variant type, found {:?}", expanded_expected_type));
                }
            }
            Pattern::Alternative(patterns) => {
                if let Type::Sum(sum_types) = &expanded_expected_type {
                    // Alternative patterns should match one of the sum types
                    // For now, require that all alternatives match the same sum type
                    // TODO: More sophisticated alternative pattern checking
                    if patterns.len() != sum_types.len() {
                        return type_error(location.clone(),
                            format!("Alternative pattern has {} alternatives but sum type has {} types",
                                patterns.len(), sum_types.len()));
                    }

                    for (pattern, sum_type) in patterns.iter().zip(sum_types) {
                        self.check_pattern(pattern, sum_type, location, env)?;
                    }
                } else {
                    return type_error(location.clone(),
                        format!("Alternative pattern expected sum type, found {:?}", expanded_expected_type));
                }
            }
            Pattern::Literal(lit) => {
                // Literal patterns should match the literal type
                let lit_type = match lit {
                    crate::ast::Literal::Unit => Type::Unit,
                    crate::ast::Literal::Bool(_) => Type::Bool,
                    crate::ast::Literal::Int(_) => Type::Int,
                    crate::ast::Literal::Char(_) => Type::Char,
                    crate::ast::Literal::String(_) => Type::String,
                };

                if !self.types_equal(&expanded_expected_type, &lit_type) {
                    return type_error(location.clone(),
                        format!("Literal pattern type {:?} does not match expected type {:?}", lit_type, expanded_expected_type));
                }
            }
            _ => return type_error(location.clone(), format!("Unsupported pattern type: {:?}", pattern)),
        }
        Ok(())
    }

    /// Check exhaustiveness of case patterns
    fn check_exhaustiveness(&self, scrutinee_type: &Type, branches: &[CaseBranch], location: &SourceLocation) -> Result<()> {
        // Basic exhaustiveness checking for bootstrap compiler
        match scrutinee_type {
            Type::Bool => {
                // For boolean, must cover true and false (or have wildcard)
                let mut covers_true = false;
                let mut covers_false = false;
                let mut has_wildcard = false;

                for branch in branches {
                    match &branch.pattern {
                        Pattern::Literal(Literal::Bool(true)) => covers_true = true,
                        Pattern::Literal(Literal::Bool(false)) => covers_false = true,
                        Pattern::Identifier(_) => {
                            // Variable pattern covers everything
                            has_wildcard = true;
                        }
                        Pattern::TypedIdentifier { .. } => {
                            // Typed variable pattern covers everything
                            has_wildcard = true;
                        }
                        _ => {
                            // Other patterns might be exhaustive, but for now treat as non-exhaustive
                        }
                    }

                    // If guard is present, this branch might not cover all cases
                    if branch.guard.is_some() {
                        // Guards make exhaustiveness checking more complex
                        // For bootstrap compiler, skip detailed analysis
                        has_wildcard = true;
                    }
                }

                if !has_wildcard && (!covers_true || !covers_false) {
                    return type_error(location.clone(),
                        format!("Non-exhaustive patterns for boolean type. Missing cases: {}{}",
                            if !covers_true { "true" } else { "" },
                            if !covers_false { if !covers_true { ", false" } else { "false" } } else { "" }));
                }
            }
            Type::Variant(variants) => {
                // For variant types, check if all constructors are covered
                let mut covered_constructors = std::collections::HashSet::new();
                let mut has_wildcard = false;

                for branch in branches {
                    match &branch.pattern {
                        Pattern::Variant { constructor, .. } => {
                            covered_constructors.insert(constructor.clone());
                        }
                        Pattern::Identifier(_) | Pattern::TypedIdentifier { .. } => {
                            has_wildcard = true;
                        }
                        _ => {
                            // Other patterns don't cover variant constructors
                        }
                    }

                    // Guards complicate exhaustiveness
                    if branch.guard.is_some() {
                        has_wildcard = true;
                    }
                }

                if !has_wildcard {
                    let all_constructors: std::collections::HashSet<String> =
                        variants.iter().map(|(name, _)| name.clone()).collect();

                    let missing: Vec<String> = all_constructors.difference(&covered_constructors)
                        .cloned().collect();

                    if !missing.is_empty() {
                        return type_error(location.clone(),
                            format!("Non-exhaustive patterns for variant type. Missing constructors: {}",
                                missing.join(", ")));
                    }
                }
            }
            _ => {
                // For other types, we don't do exhaustiveness checking in bootstrap compiler
                // This includes complex types like tuples, records, etc.
            }
        }

        Ok(())
    }

    /// Get the type aliases for code generation
    pub fn get_type_aliases(&self) -> &HashMap<String, Type> {
        &self.type_aliases
    }

    /// Infer types for a sequence of statements, returning the type of the last expression
    fn infer_statements(&mut self, statements: &[crate::ast::Statement]) -> Result<Type> {
        let mut last_type = Type::Unit;
        let original_env = self.env.clone();

        for statement in statements {
            match statement {
                crate::ast::Statement::Bind { pattern, expr } => {
                    let expr_type = self.infer_expression(expr)?;
                    // Require explicit type annotations for ALL bindings
                    if let crate::ast::Pattern::Identifier(_) = pattern {
                        let metadata = ErrorMetadataBuilder::new("E2000".to_string())
                            .severity(ErrorSeverity::Error)
                            .specification("§6".to_string(), None)
                            .suggestion("Add explicit type annotation: variable:type <- expression".to_string())
                            .build();
                        return Err(CompilerError::TypeError {
                            location: SourceLocation::unknown(), // TODO: get proper location
                            message: format!("Variable bindings must have explicit type annotations. Use 'variable:type <- expression' instead of 'variable <- expression'"),
                            metadata,
                        });
                    }
                    // Check pattern against expression type and bind variables
                    let mut pattern_env = HashMap::new();
                    self.check_pattern(pattern, &expr_type, &SourceLocation::unknown(), &mut pattern_env)?;
                    // Merge the pattern bindings into the main environment
                    for (name, scheme) in pattern_env {
                        self.env.insert(name, scheme);
                    }
                }
                crate::ast::Statement::Expr(expr) => {
                    last_type = self.infer_expression(expr)?;
                }
            }
        }

        // Restore the original environment (bindings are local to this statement block)
        self.env = original_env;

        Ok(last_type)
    }

}
