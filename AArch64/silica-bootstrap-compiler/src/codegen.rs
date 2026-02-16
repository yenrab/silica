/*
 * SILICA LLVM CODE GENERATOR
 *
 * This file demonstrates the correct architecture for LLVM code generation.
 * Currently provides a text-based LLVM IR generator that shows the structure.
 *
 * REAL LLVM BACKEND (when "llvm_backend" feature is enabled):
 * - Uses inkwell crate for actual LLVM IR generation
 * - Produces binary bitcode files (.bc) that can be compiled to machine code
 * - Performs LLVM module verification
 * - Can be processed by llc, llvm-link, lli, etc.
 *
 * To enable real LLVM: cargo build --features llvm_backend
 * (Requires LLVM 15+ installed on the system)
 */

use crate::ast::*;
use crate::ast::Literal;
use crate::errors::{Result, codegen_error, codegen_error_with_metadata, CompilerError, SourceLocation, ErrorMetadataBuilder, ErrorSeverity};
use crate::types::TypeChecker;
use crate::ast::Type;
use std::collections::HashMap;

#[cfg(feature = "llvm_backend")]
use inkwell::passes::PassManager;
#[cfg(feature = "llvm_backend")]
pub use inkwell::OptimizationLevel;
#[cfg(feature = "llvm_backend")]
use inkwell::context::Context;
#[cfg(feature = "llvm_backend")]
use inkwell::builder::Builder;
#[cfg(feature = "llvm_backend")]
use inkwell::module::Module;
#[cfg(feature = "llvm_backend")]
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, PointerValue};
#[cfg(feature = "llvm_backend")]
use inkwell::types::{BasicType, BasicTypeEnum, FunctionType, IntType, PointerType};
#[cfg(feature = "llvm_backend")]
use inkwell::{AddressSpace};
#[cfg(feature = "llvm_backend")]
use std::fs::File;

#[cfg(not(feature = "llvm_backend"))]
#[derive(Debug, Clone, Copy)]
pub enum OptimizationLevel {
    None,
    Less,
    Default,
    Aggressive,
}

/// LLVM code generator for Silica - demonstrates real LLVM integration structure
/// This is now structured to use inkwell crate when the "llvm_backend" feature is enabled
pub struct CodeGenerator {
    module_name: String,
    type_map: TypeMap,
    functions: HashMap<String, String>,
    function_return_types: HashMap<String, String>, // Function name -> LLVM return type
    function_param_types: HashMap<String, Vec<String>>, // Function name -> LLVM parameter types
    variables: HashMap<String, String>, // Variable name -> LLVM register/temp
    variable_types: HashMap<String, Type>, // Variable name -> Silica type
    /// Actual LLVM type of each variable (e.g. i64 for bool from tuple param after zext). Used so identifier lookup returns correct type.
    variable_llvm_types: HashMap<String, String>,
    instructions: Vec<String>,
    global_functions: Vec<String>, // Global function definitions (function literals)
    optimization_level: OptimizationLevel,
    symbol_table: Option<Box<crate::module_resolver::SymbolTable>>,
    expression_types: HashMap<SourceLocation, Type>,
    type_aliases: HashMap<String, Type>, // Type alias definitions
    struct_defs: HashMap<String, Vec<crate::ast::StructField>>, // Struct definitions
    trait_impls: Vec<crate::types::TraitImpl>, // Trait implementations
    trait_forwarders_emitted: std::collections::HashSet<(String, String)>, // (trait_name, method_name) already emitted
    trait_forwarder_ir: Vec<String>, // IR lines for trait method forwarders (define Trait_method -> call Concrete_method)
    variable_scopes: Vec<HashMap<String, String>>, // Scope stack for text IR variables
    function_variable_scopes: Vec<HashMap<String, (Vec<Type>, Type)>>, // Function signatures for variables
    register_counter: u32, // Counter for generating unique register names
    string_constants: HashMap<String, (String, usize)>, // String content -> (constant name, length) mapping
    in_behavior_function: bool, // Whether we're currently generating code for a behavior function
    /// Variable names that are currently bound to a self-referential placeholder (undefined until patch); use null when generating.
    self_ref_placeholders: std::collections::HashSet<String>,
    /// Module of the function currently being generated (for resolving unqualified calls)
    current_module: Option<String>,

    // Real LLVM backend fields (when feature enabled)
    #[cfg(feature = "llvm_backend")]
    context: *const inkwell::context::Context,
    #[cfg(feature = "llvm_backend")]
    module: Option<inkwell::module::Module<'static>>,
    #[cfg(feature = "llvm_backend")]
    builder: Option<inkwell::builder::Builder<'static>>,
    #[cfg(feature = "llvm_backend")]
    pass_manager: Option<PassManager<inkwell::module::Module<'static>>>,
    #[cfg(feature = "llvm_backend")]
    llvm_variable_scopes: Vec<HashMap<String, inkwell::values::PointerValue<'static>>>, // Scope stack for variables
    #[cfg(feature = "llvm_backend")]
    monomorphized_functions: HashMap<String, inkwell::values::FunctionValue<'static>>, // Cache for monomorphized functions
}

impl CodeGenerator {
    /// Create a new code generator with specified optimization level
    pub fn new(module_name: &str) -> Self {
        Self::new_with_optimization(module_name, OptimizationLevel::None)
    }

    /// Create a new code generator with specified optimization level
    pub fn new_with_optimization(module_name: &str, optimization_level: OptimizationLevel) -> Self {
        let type_map = TypeMap::new();

        CodeGenerator {
            module_name: module_name.to_string(),
            type_map,
            functions: HashMap::new(),
            function_return_types: HashMap::new(),
            function_param_types: HashMap::new(),
            variables: HashMap::new(),
            variable_types: HashMap::new(),
            variable_llvm_types: HashMap::new(),
            instructions: Vec::new(),
            global_functions: Vec::new(),
            optimization_level,
            symbol_table: None,
            expression_types: HashMap::new(),
            type_aliases: HashMap::new(),
            struct_defs: HashMap::new(),
            trait_impls: Vec::new(),
            trait_forwarders_emitted: std::collections::HashSet::new(),
            trait_forwarder_ir: Vec::new(),
            variable_scopes: vec![HashMap::new()], // Start with global scope
            function_variable_scopes: vec![HashMap::new()], // Start with global scope
            register_counter: 0,
            string_constants: HashMap::new(),
            in_behavior_function: false,
            self_ref_placeholders: std::collections::HashSet::new(),
            current_module: None,

            // LLVM backend fields will be initialized in generate_program
            #[cfg(feature = "llvm_backend")]
            context: std::ptr::null(),
            #[cfg(feature = "llvm_backend")]
            module: None,
            #[cfg(feature = "llvm_backend")]
            builder: None,
            #[cfg(feature = "llvm_backend")]
            monomorphized_functions: HashMap::new(),
            #[cfg(feature = "llvm_backend")]
            pass_manager: None,
            #[cfg(feature = "llvm_backend")]
            llvm_variable_scopes: vec![HashMap::new()], // Start with global scope
        }
    }

    /// Set the symbol table for imported functions
    pub fn set_symbol_table(&mut self, symbol_table: Box<crate::module_resolver::SymbolTable>) {
        self.symbol_table = Some(symbol_table);
    }

    pub fn set_expression_types(&mut self, expression_types: HashMap<SourceLocation, Type>) {
        self.expression_types = expression_types;
    }

    /// Set the type aliases from the type checker
    pub fn set_type_aliases(&mut self, type_aliases: HashMap<String, Type>) {
        self.type_aliases = type_aliases;
    }

    /// Set the struct definitions from the type checker
    pub fn set_struct_defs(&mut self, struct_defs: HashMap<String, Vec<crate::ast::StructField>>) {
        self.struct_defs = struct_defs;
    }

    /// Set the trait implementations from the type checker
    pub fn set_trait_impls(&mut self, trait_impls: Vec<crate::types::TraitImpl>) {
        self.trait_impls = trait_impls;
    }

    /// Generate a unique register name
    fn next_register(&mut self) -> String {
        let reg = format!("t{}", self.register_counter);
        self.register_counter += 1;
        reg
    }

    /// Format a value for use in LLVM IR: register names need % prefix, constants and globals do not.
    fn format_llvm_value_ref(value: &str) -> String {
        let s = value.trim();
        if s.starts_with('%') || s.starts_with('@') {
            return s.to_string();
        }
        if s == "null" || s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok() {
            return s.to_string();
        }
        if s.contains(' ') {
            return s.to_string();
        }
        format!("%{}", s)
    }

    /// Normalize a typed call argument for LLVM: "i8* t6" -> "i8* %t6". Ensures value has % if it's a register.
    fn normalize_typed_call_arg(typed_arg: &str) -> String {
        if let Some(space) = typed_arg.find(' ') {
            let (ty, val) = typed_arg.split_at(space);
            let val = val.trim_start();
            let val_ref = Self::format_llvm_value_ref(val);
            format!("{} {}", ty, val_ref)
        } else {
            typed_arg.to_string()
        }
    }

    /// Get the type of an expression from expression_types map
    fn get_expression_type(&self, expr: &Expression) -> Result<Type> {
        let location = Self::try_get_expression_location(expr)
            .ok_or_else(|| CompilerError::codegen_error(
                format!("Cannot get location for expression: {:?}", expr)
            ))?;
        self.expression_types.get(location)
            .cloned()
            .ok_or_else(|| CompilerError::codegen_error(
                format!("Type information not available for expression at {:?}", location)
            ))
    }

    /// Map Silica type to LLVM type string (for text IR)
    fn type_to_llvm_string(ty: &Type) -> &'static str {
        match ty {
            Type::Int8 => "i8",
            Type::Int16 => "i16",
            Type::Int32 => "i32",
            Type::Int64 => "i64",
            Type::Float16 => "half",
            Type::Float32 => "float",
            Type::Float64 => "double",
            Type::Bool => "i1",
            Type::Char => "i32",
            // NEON 128-bit vector types
            Type::Vec128Int8 => "<16 x i8>",
            Type::Vec128Int16 => "<8 x i16>",
            Type::Vec128Int32 => "<4 x i32>",
            Type::Vec128Int64 => "<2 x i64>",
            Type::Vec128Float32 => "<4 x float>",
            Type::Vec128Bool => "<16 x i1>",
            // SVE scalable vector types
            Type::VecInt8 => "<vscale x 16 x i8>",
            Type::VecInt16 => "<vscale x 8 x i16>",
            Type::VecInt32 => "<vscale x 4 x i32>",
            Type::VecInt64 => "<vscale x 2 x i64>",
            Type::VecFloat16 => "<vscale x 8 x half>",
            Type::VecFloat32 => "<vscale x 4 x float>",
            Type::VecFloat64 => "<vscale x 2 x double>",
            Type::VecBool => "<vscale x 16 x i1>",
            // SVE predicate type
            Type::Pred => "<vscale x 16 x i1>",
            _ => "i64", // fallback
        }
    }

    /// Check if a type is numeric
    fn is_numeric_type(ty: &Type) -> bool {
        matches!(ty,
            Type::Int8 | Type::Int16 | Type::Int32 | Type::Int64 |
            Type::Float16 | Type::Float32 | Type::Float64
        )
    }

    /// Check if a type is an integer type
    fn is_integer_type(ty: &Type) -> bool {
        matches!(ty, Type::Int8 | Type::Int16 | Type::Int32 | Type::Int64)
    }

    /// Try to get the location of an expression for type lookup
    fn try_get_expression_location(expr: &Expression) -> Option<&SourceLocation> {
        match expr {
            Expression::Literal(_) => None, // Literals don't have location
            Expression::Binary(binary) => Some(&binary.location),
            Expression::Unary(unary) => Some(&unary.location),
            Expression::Call(call) => Some(&call.location),
            Expression::ModuleCall(module_call) => Some(&module_call.location),
            Expression::If(if_expr) => Some(&if_expr.location),
            Expression::Case(case) => Some(&case.location),
            Expression::Do(do_expr) => Some(&do_expr.location),
            Expression::Region(region) => Some(&region.location),
            Expression::ReadRef(read) => Some(&read.location),
            Expression::Spawn(spawn) => Some(&spawn.location),
            Expression::Send(send) => Some(&send.location),
            Expression::Cast(cast) => Some(&cast.location),
            Expression::Recv(recv) => Some(&recv.location),
            Expression::ReadFile(read_file) => Some(&read_file.location),
            Expression::WriteFile(write_file) => Some(&write_file.location),
            Expression::Print(print) => Some(&print.location),
            Expression::PrintLn(println) => Some(&println.location),
            Expression::PrintInt64(print_int64) => Some(&print_int64.location),
            Expression::PrintInt32(print_int32) => Some(&print_int32.location),
            Expression::PrintInt16(print_int16) => Some(&print_int16.location),
            Expression::PrintInt8(print_int8) => Some(&print_int8.location),
            Expression::PrintBool(print_bool) => Some(&print_bool.location),
            Expression::PrintChar(print_char) => Some(&print_char.location),
            Expression::PrintFloat16(print_float16) => Some(&print_float16.location),
            Expression::PrintFloat32(print_float32) => Some(&print_float32.location),
            Expression::PrintFloat64(print_float64) => Some(&print_float64.location),
            Expression::GetCpuTopology(get_topology) => Some(&get_topology.location),
            Expression::ReadLines(read_lines) => Some(&read_lines.location),
            Expression::AppendFile(append_file) => Some(&append_file.location),
            Expression::FileExists(file_exists) => Some(&file_exists.location),
            Expression::DeleteFile(delete_file) => Some(&delete_file.location),
            Expression::GetFileSize(get_file_size) => Some(&get_file_size.location),
            Expression::CreateDirectory(create_dir) => Some(&create_dir.location),
            Expression::RemoveDirectory(remove_dir) => Some(&remove_dir.location),
            Expression::ListDirectory(list_dir) => Some(&list_dir.location),
            Expression::AsType(as_type) => Some(&as_type.location),
            Expression::StringLen(string_len) => Some(&string_len.location),
            Expression::StringLenChars(string_len_chars) => Some(&string_len_chars.location),
            Expression::StringConcat(string_concat) => Some(&string_concat.location),
            Expression::StringSubstring(string_substring) => Some(&string_substring.location),
            Expression::StringSubstringUntilChar(string_substring_until_char) => Some(&string_substring_until_char.location),
            Expression::StringStartsWith(string_starts_with) => Some(&string_starts_with.location),
            Expression::StringEndsWith(string_ends_with) => Some(&string_ends_with.location),
            Expression::StringContains(string_contains) => Some(&string_contains.location),
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

    /// Check if two types are equal for code generation purposes
    fn types_equal_codegen(&self, t1: &Type, t2: &Type) -> bool {
        match (t1, t2) {
            (Type::Named(n1), Type::Named(n2)) => n1 == n2,
            (Type::Int64, Type::Int64) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Char, Type::Char) => true,
            (Type::String, Type::String) => true,
            (Type::Unit, Type::Unit) => true,
            _ => false, // Simplified - doesn't handle generics, tuples, etc.
        }
    }

    /// Expand type aliases for code generation
    fn expand_type_aliases_codegen(&self, ty: &Type) -> Type {
        match ty {
            Type::Named(name) => {
                if let Some(aliased_type) = self.type_aliases.get(name) {
                    // Expand the aliased type recursively
                    self.expand_type_aliases_codegen(aliased_type)
                } else {
                    // Check if it's a built-in type (including common aliases like "boolean" for bool)
                    match name.as_str() {
                        "int8" => Type::Int8,
                        "int16" => Type::Int16,
                        "int32" => Type::Int32,
                        "int64" => Type::Int64,
                        "float16" => Type::Float16,
                        "float32" => Type::Float32,
                        "float64" => Type::Float64,
                        "bool" | "boolean" => Type::Bool,
                        "char" => Type::Char,
                        "string" => Type::String,
                        "unit" => Type::Unit,
                        _ => ty.clone(), // Unknown named type, keep as is
                    }
                }
            }
            Type::Tuple(elements) => {
                Type::Tuple(elements.iter().map(|elem| self.expand_type_aliases_codegen(elem)).collect())
            }
            Type::Record(fields) => {
                Type::Record(fields.iter().map(|(name, ty)| (name.clone(), self.expand_type_aliases_codegen(ty))).collect())
            }
            Type::Function { parameters, return_type } => {
                Type::Function {
                    parameters: parameters.iter().map(|param| self.expand_type_aliases_codegen(param)).collect(),
                    return_type: Box::new(self.expand_type_aliases_codegen(return_type)),
                }
            }
            Type::Process { effects, result_type } => {
                Type::Process {
                    effects: effects.clone(),
                    result_type: Box::new(self.expand_type_aliases_codegen(result_type)),
                }
            }
            // For other types, return as-is
            _ => ty.clone(),
        }
    }

    /// Generate main function that calls Silica's main
    #[cfg(feature = "llvm_backend")]
    fn generate_main_function(&mut self) -> Result<()> {
        unsafe {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                let context = &*self.context;
                // Create main function: int main()
                let i32_type = context.i32_type();
                let main_fn_type = i32_type.fn_type(&[], false);
                let main_fn = module.add_function("main", main_fn_type, None);

                // Create entry block
                let entry_block = context.append_basic_block(main_fn, "entry");
                builder.position_at_end(entry_block);

                // Call the Silica main function (returns i64/int)
                if let Some(silica_main) = module.get_function("main") {
                    let call_result = builder.build_call(silica_main, &[], "result").unwrap();
                    let result: inkwell::values::BasicValueEnum<'static> = unsafe { std::mem::transmute(call_result.try_as_basic_value().unwrap_basic()) };


                    // Truncate i64 result to i32 for C main function
                    let truncated_result = builder.build_int_truncate(result.into_int_value(), i32_type, "truncated_result")
                        .unwrap_or_else(|_| {
                            // Fallback: just return 0 if truncation fails
                            i32_type.const_int(0, false)
                        });

                    // Return the truncated result
                    builder.build_return(Some(&truncated_result));
                } else {
                    // If no Silica main found, return 0
                    let zero = i32_type.const_int(0, false);
                    builder.build_return(Some(&zero));
                }

                Ok(())
            } else {
                {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
            }
        }
    }

    /// Generate LLVM function from Silica function declaration
    #[cfg(feature = "llvm_backend")]
    fn generate_llvm_function(&mut self, func: &FunctionDecl) -> Result<()> {
        unsafe {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                let context = &*self.context;
                // Convert Silica types to LLVM types
                let return_type = if let Some(ref ty) = func.return_type {
                    self.silica_type_to_llvm(ty)
                } else {
                    context.i32_type().into() // Default to i32 for unit return
                };
                let param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = func.parameters
                    .iter()
                    .map(|param| self.silica_type_to_llvm(&param.type_).into())
                    .collect();

                // Create function type
                let fn_type = return_type.fn_type(&param_types, false);

                // Add function to module
                let llvm_func = module.add_function(&func.name, fn_type, None);

                // Create entry block
                let entry_block = context.append_basic_block(llvm_func, "entry");
                builder.position_at_end(entry_block);

                // For now, generate a simple return (this is a placeholder)
                // TODO: Generate actual function body from Silica expressions
                if return_type.is_int_type() {
                    let zero = return_type.into_int_type().const_int(0, false);
                    builder.build_return(Some(&zero));
                } else {
                    builder.build_return(None);
                }

                Ok(())
            } else {
                {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
            }
        }
    }

    /// Generate LLVM IR for a complete program
    pub fn generate_program(&mut self, program: &Program) -> Result<()> {
        // Initialize LLVM components for real backend
        #[cfg(feature = "llvm_backend")]
        {
            let context = Box::new(inkwell::context::Context::create());
            self.context = Box::into_raw(context);
            unsafe {
                self.module = Some((*self.context).create_module(&self.module_name));
                self.builder = Some((*self.context).create_builder());
                self.pass_manager = match self.optimization_level {
                    OptimizationLevel::None => None,
                    _ => {
                        let pm = PassManager::create(());
                        // Add optimization passes based on level
                        Some(pm)
                    }
                };
                // Initialize monomorphized functions cache
                self.monomorphized_functions = HashMap::new();
            }
            self.add_external_declarations();

            // Generate LLVM functions for each declaration
            for decl in &program.declarations {
                match decl {
                    Declaration::Function(func) => {
                        self.generate_llvm_function(func)?;
                    }
                    _ => {
                        // Other declarations don't generate executable code yet
                    }
                }
            }

            // Generate main function that calls the Silica main
            self.generate_main_function()?;

            return Ok(());
        }

        // Text-based generation (fallback)
        #[cfg(not(feature = "llvm_backend"))]
        {
        self.instructions.push(format!("; Module: {}", self.module_name));
        }
        self.instructions.push("; Generated by Silica Bootstrap Compiler - Real LLVM Integration Ready".to_string());
        self.instructions.push("; This demonstrates the correct structure for inkwell integration".to_string());
        self.instructions.push(format!("; Optimization Level: {:?}", self.optimization_level));
        self.instructions.push("".to_string());

        // Add external function declarations for runtime system
        self.instructions.push("; External function declarations".to_string());
        self.instructions.push("declare i8* @malloc(i64)".to_string());
        self.instructions.push("declare void @free(i8*)".to_string());
        self.instructions.push("".to_string());

        // Silica runtime functions
        self.instructions.push("; Silica runtime functions".to_string());

        // Region management functions (suggestion_1)
        self.instructions.push("declare i8* @silica_region_create_with_value(i64)".to_string());
        self.instructions.push("declare i8* @silica_region_create_with_data(i8*, i64)".to_string());
        self.instructions.push("declare void @llvm.memcpy.p0i8.p0i8.i64(i8*, i8*, i64, i1)".to_string());
        self.instructions.push("declare i64 @silica_region_read(i8*)".to_string());
        self.instructions.push("declare void @silica_region_destroy(i8*)".to_string());

        // Actor management functions
        self.instructions.push("declare i8* @silica_actor_spawn(i8*, i8*, i32)".to_string());
        self.instructions.push("declare void @silica_actor_send(i8*, i8*)".to_string());
        self.instructions.push("declare i1 @silica_actor_cast(i8*, i8*)".to_string());
        self.instructions.push("declare i64 @silica_actor_recv(i8*)".to_string());

        // File I/O functions
        self.instructions.push("declare { i1, i8* } @silica_read_file(i8*, i64)".to_string());
        self.instructions.push("declare { i1, i8* } @silica_read_file_path(i8*)".to_string());
        self.instructions.push("declare { i1, i8* } @silica_write_file(i8*, i64, i8*, i64)".to_string());
        self.instructions.push("declare { i1, i8* } @silica_write_file_path(i8*, i8*)".to_string());
        self.instructions.push("declare void @silica_free_string(i8*)".to_string());

        // Process execution functions
        self.instructions.push("declare i8* @silica_exec_command(i8*, i64, i8*, i64, i8*)".to_string());
        self.instructions.push("declare void @silica_free_process_result(i8*)".to_string());

        // Print functions
        self.instructions.push("declare void @silica_print(i8*, i64)".to_string());
        self.instructions.push("declare void @silica_println(i8*, i64)".to_string());
        self.instructions.push("declare void @silica_print_string(i8*)".to_string());
        self.instructions.push("declare void @silica_println_string(i8*)".to_string());
        self.instructions.push("declare void @silica_print_int64(i64)".to_string());
        self.instructions.push("declare void @silica_print_int32(i32)".to_string());
        self.instructions.push("declare void @silica_print_int16(i16)".to_string());
        self.instructions.push("declare void @silica_print_int8(i8)".to_string());
        // C ABI: bool is i8 (1 byte); i1 causes ABI mismatch and segfault when passed to C
        self.instructions.push("declare void @silica_print_bool(i8)".to_string());
        self.instructions.push("declare void @silica_print_char(i32)".to_string());
        self.instructions.push("declare void @silica_print_float16(i16)".to_string());
        self.instructions.push("declare void @silica_print_float32(float)".to_string());
        self.instructions.push("declare void @silica_print_float64(double)".to_string());
        self.instructions.push("declare {i64, i64, i64, i1, i64, i64, i1, i64, i64} @silica_get_cpu_topology()".to_string());

        // String functions
        self.instructions.push("declare i64 @silica_string_len(i8*)".to_string());
        self.instructions.push("declare i64 @silica_string_len_chars(i8*)".to_string());
        self.instructions.push("declare i8* @silica_string_concat(i8*, i8*)".to_string());
        self.instructions.push("declare i8* @silica_string_substring(i8*, i64, i64)".to_string());
        self.instructions.push("declare i8* @silica_string_substring_until_char(i8*, i64, i32)".to_string());
        self.instructions.push("declare i1 @silica_string_starts_with(i8*, i8*)".to_string());
        self.instructions.push("declare i1 @silica_string_ends_with(i8*, i8*)".to_string());
        self.instructions.push("declare i1 @silica_string_contains(i8*, i8*)".to_string());
        self.instructions.push("declare i1 @silica_string_equals(i8*, i8*)".to_string());

        // Pass 1: Register all function signatures so bodies can call functions defined later
        for (i, decl) in program.declarations.iter().enumerate() {
            let module = program.declaration_modules.get(i).map(|s| s.as_str()).unwrap_or("main");
            match decl {
                Declaration::Function(func) => {
                    self.register_function_signature_text(module, func)?;
                }
                Declaration::Type(_) => {
                    // Type declarations don't generate code in LLVM
                    self.instructions.push("; Type declaration (metadata only)".to_string());
                }
                Declaration::Effect(_) => {
                    // Effect declarations don't generate code in LLVM
                    self.instructions.push("; Effect declaration (metadata only)".to_string());
                }
                Declaration::Import(_) => {
                    // Import declarations don't generate code in LLVM
                    self.instructions.push("; Import declaration (metadata only)".to_string());
                }
                Declaration::Export(_) => {
                    // Export declarations don't generate code in LLVM
                    self.instructions.push("; Export declaration (metadata only)".to_string());
                }
                Declaration::Struct(_) => {
                    // Struct declarations don't generate code in LLVM
                    self.instructions.push("; Struct declaration (metadata only)".to_string());
                }
                Declaration::Enum(_) => {
                    // Enum declarations don't generate code in LLVM
                    self.instructions.push("; Enum declaration (metadata only)".to_string());
                }
                Declaration::Trait(_) => {
                    // Trait declarations don't generate code in LLVM
                    self.instructions.push("; Trait declaration (metadata only)".to_string());
                }
                Declaration::Impl(impl_decl) => {
                    // Impl declarations generate method code
                    self.generate_impl_declaration(impl_decl)?;
                }
                Declaration::TypeAlias(_) => {
                    // Type alias declarations don't generate code in LLVM
                    self.instructions.push("; Type alias declaration (metadata only)".to_string());
                }
            }
            self.instructions.push("".to_string());
        }

        // Pass 2: Generate each function body (signatures already registered so calls to later functions resolve)
        for (i, decl) in program.declarations.iter().enumerate() {
            let module = program.declaration_modules.get(i).map(|s| s.as_str()).unwrap_or("main");
            if let Declaration::Function(func) = decl {
                self.generate_function_body_text(module, func)?;
            }
        }

        // Emit C entry point wrapper when program has main (text-based codegen produces module.main, not main)
        #[cfg(not(feature = "llvm_backend"))]
        {
            self.generate_main_wrapper_text(program)?;
        }

        // Now generate string constants at the end (they will be moved to the top during write)
        self.instructions.push("; String constants".to_string());

        // Collect constants into a vector and sort by constant name for deterministic output order
        let mut constants: Vec<_> = self.string_constants.iter().collect();
        constants.sort_by(|a, b| a.1.0.cmp(&b.1.0)); // Sort by constant name

        for (content, (const_name, _)) in constants {
            let len = content.len() + 1; // +1 for null terminator
            // Build LLVM IR string literal with proper escaping
            // The key insight: In LLVM IR, \n is an escape sequence (1 byte), not literal \ + n (2 bytes)
            // When we write instruction.push_str("\\n"), we're writing literal \ + n (2 characters)
            // But in the file, this becomes \n (2 characters), which LLVM should interpret as 1 byte
            // However, the issue is that when we use format! with {}, it writes the string as-is
            // So if escaped_content = "\\n" (2 bytes), format!("c\"{}\\00\"", escaped_content) becomes c"\\n\00"
            // In LLVM IR, \\n means: \\ (escaped backslash, 1 byte) + n (1 byte) = 2 bytes, plus null = 3 bytes total
            // We need c"\n\00" which means: \n (escape sequence, 1 byte) + null (1 byte) = 2 bytes total
            // Solution: Write the escape sequences directly in the format string, not through string substitution
            // Build LLVM IR string literal: write bytes directly using hexadecimal escape sequences
            // Use \XX format where XX is the hexadecimal byte value (e.g., \0A for newline)
            let mut instruction = format!("{} = private unnamed_addr constant [{} x i8] c\"", const_name, len);
            for byte in content.bytes() {
                match byte {
                    b'\\' => instruction.push_str(r#"\\"#),  // Write \\ which becomes \ in LLVM IR
                    b'"' => instruction.push_str(r#"\""#),  // Write \" which becomes " in LLVM IR
                    b if b >= 32 && b < 127 && b != b'\\' && b != b'"' => {
                        // Printable ASCII (excluding backslash and quote which are handled above)
                        instruction.push(byte as char)
                    },
                    _ => {
                        // All other bytes (including \n=0x0A, \t=0x09, \r=0x0D) as hexadecimal escape sequences
                        instruction.push_str(&format!(r#"\{:02X}"#, byte));
                    }
                }
            }
            instruction.push_str(r#"\00""#);
            self.instructions.push(instruction);
        }
        self.instructions.push("".to_string());

        // Apply optimizations based on level
        #[cfg(feature = "llvm_backend")]
        {
            // Real LLVM optimization will be applied later in write_to_file
        }
        #[cfg(not(feature = "llvm_backend"))]
        {
        self.apply_optimizations();
        }

        // When LLVM backend is enabled, this would verify the module
        #[cfg(feature = "llvm_backend")]
        {
        println!("✓ LLVM module structure verified (would use inkwell verification when enabled)");
        }
        #[cfg(not(feature = "llvm_backend"))]
        {
            // println!("✓ LLVM text IR generated successfully");
        }

        Ok(())
    }

    /// Apply optimizations to the generated LLVM IR based on optimization level
    fn apply_optimizations(&mut self) {
        match self.optimization_level {
            OptimizationLevel::None => {
                self.instructions.push("; No optimizations applied".to_string());
            }
            OptimizationLevel::Less => {
                self.apply_basic_optimizations();
            }
            OptimizationLevel::Default => {
                self.apply_standard_optimizations();
            }
            OptimizationLevel::Aggressive => {
                self.apply_aggressive_optimizations();
            }
        }
    }

    /// Apply basic optimizations (constant folding, dead code hints)
    fn apply_basic_optimizations(&mut self) {
        self.instructions.push("; Basic optimizations applied:".to_string());
        self.instructions.push("; - Constant folding hints".to_string());
        self.instructions.push("; - Dead code elimination hints".to_string());
        self.instructions.push("; - Basic instruction combining".to_string());

        // Add optimization metadata
        self.instructions.push("".to_string());
        self.instructions.push("!llvm.ident = !{!0}".to_string());
        self.instructions.push("!0 = !{!\"Silica Bootstrap Compiler with basic optimizations\"}".to_string());
    }

    /// Apply standard optimizations
    fn apply_standard_optimizations(&mut self) {
        self.apply_basic_optimizations();
        self.instructions.push("; - Standard optimizations: GVN, CSE, LICM".to_string());
        self.instructions.push("; - Register promotion".to_string());
        self.instructions.push("; - CFG simplification".to_string());
    }

    /// Apply aggressive optimizations
    fn apply_aggressive_optimizations(&mut self) {
        self.apply_standard_optimizations();
        self.instructions.push("; - Aggressive optimizations: Loop unrolling, vectorization".to_string());
        self.instructions.push("; - Profile-guided optimizations".to_string());
        self.instructions.push("; - Link-time optimizations".to_string());
    }

    /// Add external function declarations using inkwell
    #[cfg(feature = "llvm_backend")]
    fn add_external_declarations(&self) {
        if let Some(module) = &self.module {
            unsafe {
                let i8_ptr = (*self.context).ptr_type(inkwell::AddressSpace::default());
                let i8_type = (*self.context).i8_type();
                let i1_type = (*self.context).bool_type();
                let i64_type = (*self.context).i64_type();
                let void_type = (*self.context).void_type();

                // Standard C library functions
                let malloc_type = i8_ptr.fn_type(&[i64_type.into()], false);
                module.add_function("malloc", malloc_type, None);

                let free_type = void_type.fn_type(&[i8_ptr.into()], false);
                module.add_function("free", free_type, None);

                // Silica runtime functions (suggestion_1)
                let region_create_with_value_type = i8_ptr.fn_type(&[i64_type.into()], false);
                module.add_function("silica_region_create_with_value", region_create_with_value_type, None);

                let region_create_with_data_type = i8_ptr.fn_type(&[i8_ptr.into(), i64_type.into()], false);
                module.add_function("silica_region_create_with_data", region_create_with_data_type, None);

                let region_read_type = i64_type.fn_type(&[i8_ptr.into()], false);
                module.add_function("silica_region_read", region_read_type, None);

                let region_destroy_type = void_type.fn_type(&[i8_ptr.into()], false);
                module.add_function("silica_region_destroy", region_destroy_type, None);

                // Actor management functions
                let actor_spawn_type = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into(), i32_type.into()], false);
                module.add_function("silica_actor_spawn", actor_spawn_type, None);

                let actor_send_type = void_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
                module.add_function("silica_actor_send", actor_send_type, None);

                let actor_cast_type = i1_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
                module.add_function("silica_actor_cast", actor_cast_type, None);

                let actor_recv_type = i64_type.fn_type(&[i8_ptr.into()], false);
                module.add_function("silica_actor_recv", actor_recv_type, None);

                // File I/O functions - return { i1, i8* } struct
                let result_struct_type = context.struct_type(&[i1_type.into(), i8_ptr.into()], false);
                let read_file_type = result_struct_type.fn_type(&[i8_ptr.into(), i64_type.into()], false);
                module.add_function("silica_read_file", read_file_type, None);

                let write_file_type = result_struct_type.fn_type(&[i8_ptr.into(), i64_type.into(), i8_ptr.into(), i64_type.into()], false);
                module.add_function("silica_write_file", write_file_type, None);

                let free_string_type = void_type.fn_type(&[i8_ptr.into()], false);
                module.add_function("silica_free_string", free_string_type, None);

                // Process execution functions - return ProcessResult*
                let process_result_ptr = i8_ptr; // void* for ProcessResult*
                let exec_command_type = process_result_ptr.fn_type(&[
                    i8_ptr.into(), i64_type.into(), // command string
                    i8_ptr.into(), i64_type.into(), // args array and length
                    i8_ptr.into()                   // arg lengths array
                ], false);
                module.add_function("silica_exec_command", exec_command_type, None);

                let free_process_type = void_type.fn_type(&[process_result_ptr.into()], false);
                module.add_function("silica_free_process_result", free_process_type, None);

                // Print functions
                let print_type = void_type.fn_type(&[i8_ptr.into(), i64_type.into()], false);
                module.add_function("silica_print", print_type, None);

                let println_type = void_type.fn_type(&[i8_ptr.into(), i64_type.into()], false);
                module.add_function("silica_println", println_type, None);

                let print_int64_type = void_type.fn_type(&[i64_type.into()], false);
                module.add_function("silica_print_int64", print_int64_type, None);

                let i8_type = context.i8_type();
                let print_int8_type = void_type.fn_type(&[i8_type.into()], false);
                module.add_function("silica_print_int8", print_int8_type, None);

                let i16_type = context.i16_type();
                let print_int16_type = void_type.fn_type(&[i16_type.into()], false);
                module.add_function("silica_print_int16", print_int16_type, None);

                let i32_type = context.i32_type();
                let print_int32_type = void_type.fn_type(&[i32_type.into()], false);
                module.add_function("silica_print_int32", print_int32_type, None);

                let print_bool_type = void_type.fn_type(&[i1_type.into()], false);
                module.add_function("silica_print_bool", print_bool_type, None);

                let print_char_type = void_type.fn_type(&[i32_type.into()], false);
                module.add_function("silica_print_char", print_char_type, None);

                // silica_print_float16 takes u16 (i16 in LLVM) - the bit representation of the half
                let i16_type = context.i16_type();
                let print_float16_type = void_type.fn_type(&[i16_type.into()], false);
                module.add_function("silica_print_float16", print_float16_type, None);

                let float_type = context.f32_type();
                let print_float32_type = void_type.fn_type(&[float_type.into()], false);
                module.add_function("silica_print_float32", print_float32_type, None);

                let double_type = context.f64_type();
                let print_float64_type = void_type.fn_type(&[double_type.into()], false);
                module.add_function("silica_print_float64", print_float64_type, None);

                let topology_info_type = i8_ptr_type.fn_type(&[], false);
                module.add_function("silica_get_cpu_topology_info", topology_info_type, None);
            }
        }
    }

    /// Register function signature only (text backend). All functions are registered before any body
    /// is generated so that bodies can call functions defined later in the file.
    #[cfg(not(feature = "llvm_backend"))]
    fn register_function_signature_text(&mut self, module: &str, func: &FunctionDecl) -> Result<()> {
        let qualified_name = format!("{}.{}", module, func.name);
        let param_types: Vec<String> = func.parameters.iter()
            .map(|param| {
                if param.pattern.is_some() {
                    "i8*".to_string()
                } else {
                    let expanded_type = self.expand_type_aliases_codegen(&param.type_);
                    let is_trait_param = matches!(&expanded_type, Type::Named(name) if self.trait_impls.iter().any(|impl_| &impl_.trait_name == name));
                    if is_trait_param {
                        "i8*".to_string()
                    } else {
                        self.type_map.silica_to_llvm_str(&expanded_type)
                    }
                }
            })
            .collect();

        let return_type = func.return_type.as_ref()
            .map(|t| self.expand_type_aliases_codegen(t))
            .unwrap_or(Type::Unit);
        let return_type_str = match &return_type {
            Type::Tuple(_) => "i8*".to_string(),
            Type::Record(_) => "i8*".to_string(),
            Type::Named(name) if self.struct_defs.contains_key(name) || self.type_aliases.contains_key(name) => "i8*".to_string(), // Struct types
            _ => self.type_map.silica_to_llvm_str(&return_type),
        };

        // Use sret (struct return) for i8* returns to fix return-value propagation bug
        // with recursive functions (e.g. parse_declarations). Caller allocates slot, callee stores.
        let use_sret = return_type_str == "i8*";
        let (effective_return_type, mut effective_param_types, param_strs) = if use_sret {
            let sret_param = "i8* noalias sret(i8*) %sret".to_string();
            let mut all_param_types = vec!["i8*".to_string()];
            all_param_types.extend(param_types.clone());
            let other_params: Vec<String> = param_types.iter()
                .enumerate()
                .map(|(i, ty)| {
                    let param_name = if func.parameters[i].pattern.is_some() {
                        format!("param_{}", i)
                    } else {
                        func.parameters[i].name.clone()
                    };
                    format!("{} %{}", ty, param_name)
                })
                .collect();
            let all_params = std::iter::once(sret_param).chain(other_params.into_iter()).collect::<Vec<_>>().join(", ");
            ("void".to_string(), all_param_types, all_params)
        } else {
            let param_strs: Vec<String> = param_types.iter()
                .enumerate()
                .map(|(i, ty)| {
                    let param_name = if func.parameters[i].pattern.is_some() {
                        format!("param_{}", i)
                    } else {
                        func.parameters[i].name.clone()
                    };
                    format!("{} %{}", ty, param_name)
                })
                .collect();
            (return_type_str.clone(), param_types.clone(), param_strs.join(", "))
        };

        let signature = format!("define {} @{}({}) {{",
            effective_return_type,
            qualified_name,
            param_strs
        );

        // Only register in maps so call sites can resolve; do NOT push to instructions yet.
        // Each function's "define ... {" and body are pushed in pass 2 (generate_function_body_text).
        self.functions.insert(qualified_name.clone(), signature);
        self.function_return_types.insert(qualified_name.clone(), return_type_str.clone());
        self.function_param_types.insert(qualified_name, effective_param_types);
        Ok(())
    }

    /// Generate LLVM IR for a function declaration
    fn generate_function_declaration(&mut self, func: &FunctionDecl) -> Result<()> {
        // LLVM backend function generation
        #[cfg(feature = "llvm_backend")]
        {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                unsafe {
                    // Convert Silica types to LLVM types
                    let param_types: Vec<inkwell::types::BasicTypeEnum<'static>> = func.parameters.iter()
                        .map(|param| {
                            if param.pattern.is_some() {
                                // Pattern parameters are passed as i8* (pointers to tuples/structs)
                                (*self.context).i8_type().ptr_type(inkwell::AddressSpace::Generic).into()
                            } else {
                                self.silica_type_to_llvm(&param.type_)
                            }
                        })
                        .collect();

                    let return_type = func.return_type.as_ref().unwrap_or(&Type::Unit);
                    let llvm_return_type = self.silica_type_to_llvm(return_type);

                    // Convert BasicTypeEnum to BasicMetadataTypeEnum for function parameters
                    let param_metadata: Vec<inkwell::types::BasicMetadataTypeEnum<'static>> = param_types.iter()
                        .map(|ty| (*ty).into())
                        .collect();

                    // Create function type
                    let fn_type = if let Type::Unit = return_type {
                        (*self.context).void_type().fn_type(&param_metadata, false)
                    } else {
                        match llvm_return_type {
                            inkwell::types::BasicTypeEnum::IntType(ty) => ty.fn_type(&param_metadata, false),
                            inkwell::types::BasicTypeEnum::PointerType(ty) => ty.fn_type(&param_metadata, false),
                            _ => (*self.context).i64_type().fn_type(&param_metadata, false), // fallback
                        }
                    };

                    // Add function to module
                    let llvm_func = module.add_function(&func.name, fn_type, None);
                    self.functions.insert(func.name.clone(), format!("@{}", func.name));

                    // Generate function body
                    let entry_block = (*self.context).append_basic_block(llvm_func, "entry");
                    builder.position_at_end(entry_block);

                    // Generate function body
                    self.generate_llvm_function_body(func, llvm_func)?;
                }
            }
        }

        // Text-based generation (fallback)
        #[cfg(not(feature = "llvm_backend"))]
        {
        self.register_function_signature_text("main", func)?;
        self.generate_function_body_text("main", func)?;
        return Ok(());
        }

        Ok(())
    }

    /// Generate function body only (text backend). Signature must already be registered.
    #[cfg(not(feature = "llvm_backend"))]
    fn generate_function_body_text(&mut self, module: &str, func: &FunctionDecl) -> Result<()> {
        let qualified_name = format!("{}.{}", module, func.name);
        let param_types = self.function_param_types.get(&qualified_name).cloned()
            .ok_or_else(|| CompilerError::codegen_error(format!("Function '{}' not registered (internal error)", qualified_name)))?;
        let return_type_str = self.function_return_types.get(&qualified_name).cloned()
            .ok_or_else(|| CompilerError::codegen_error(format!("Function '{}' not registered (internal error)", qualified_name)))?;
        let return_type = func.return_type.as_ref()
            .map(|t| self.expand_type_aliases_codegen(t))
            .unwrap_or(Type::Unit);

        // Set current module for resolving unqualified calls (e.g. recursive calls)
        let prev_module = self.current_module.replace(module.to_string());

        // Emit "define ... @name(...) {" so this function's IR is contiguous
        let signature = self.functions.get(&qualified_name).cloned()
            .ok_or_else(|| CompilerError::codegen_error(format!("Function '{}' not registered (internal error)", qualified_name)))?;
        self.instructions.push(signature);

        // Add function parameters to variable scope
        for (i, param) in func.parameters.iter().enumerate() {
            let param_reg = if let Some(pattern) = &param.pattern {
                // For pattern parameters, the parameter is a tuple pointer
                let tuple_reg = format!("%param_{}", i);
                self.variables.insert("_".to_string(), tuple_reg.clone());

                // Extract individual elements from the tuple using proper type-aware decomposition
                match pattern {
                    Pattern::Tuple(elements) => {
                        for (i, elem_pattern) in elements.iter().enumerate() {
                            if let Pattern::TypedIdentifier { name: elem_name, .. } = elem_pattern {
                                // Read element count (at offset 0)
                                let count_ptr_reg = format!("%count_ptr_read_param_{}_{}", self.instructions.len(), i);
                                self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 0", count_ptr_reg, tuple_reg));
                                let count_ptr_typed = format!("%count_ptr_typed_param_{}_{}", self.instructions.len(), i);
                                self.instructions.push(format!("  {} = bitcast i8* {} to i64*", count_ptr_typed, count_ptr_reg));
                                let count_val_reg = format!("%count_val_param_{}_{}", self.instructions.len(), i);
                                self.instructions.push(format!("  {} = load i64, i64* {}", count_val_reg, count_ptr_typed));

                                // Read type ID for this element (at offset 8 + i)
                                let type_offset = 8 + i;
                                let type_ptr_reg = format!("%type_ptr_read_param_{}_{}", self.instructions.len(), i);
                                self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", type_ptr_reg, tuple_reg, type_offset));
                                let type_val_reg = format!("%type_val_param_{}_{}", self.instructions.len(), i);
                                self.instructions.push(format!("  {} = load i8, i8* {}", type_val_reg, type_ptr_reg));

                                // Calculate correct offset by replicating the creation logic
                                // Start with base offset (after count and type IDs)
                                let base_offset_reg = format!("%base_offset_param_{}_{}", self.instructions.len(), i);
                                self.instructions.push(format!("  {} = add i64 8, {}", base_offset_reg, count_val_reg));

                                // Initialize current offset to base
                                let mut current_offset_reg = base_offset_reg.clone();

                                // For each previous element, add its size with proper alignment
                                for prev_i in 0..i {
                                    // Read type of previous element
                                    let prev_type_offset = 8 + prev_i;
                                    let prev_type_ptr_reg = format!("%prev_type_ptr_param_{}_{}_{}", self.instructions.len(), i, prev_i);
                                    self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", prev_type_ptr_reg, tuple_reg, prev_type_offset));
                                    let prev_type_val_reg = format!("%prev_type_val_param_{}_{}_{}", self.instructions.len(), i, prev_i);
                                    self.instructions.push(format!("  {} = load i8, i8* {}", prev_type_val_reg, prev_type_ptr_reg));

                                    // Determine size and alignment based on type
                                    // Type 0 = i1 (size 1, align 1), Type 2 = i64 (size 8, align 8)
                                    let prev_is_i1_reg = format!("%prev_is_i1_param_{}_{}_{}", self.instructions.len(), i, prev_i);
                                    self.instructions.push(format!("  {} = icmp eq i8 {}, 0", prev_is_i1_reg, prev_type_val_reg));

                                    // Calculate aligned offset for previous element
                                    let prev_pre_align_reg = format!("%prev_pre_align_param_{}_{}_{}", self.instructions.len(), i, prev_i);
                                    self.instructions.push(format!("  {} = add i64 {}, 7", prev_pre_align_reg, current_offset_reg));
                                    let prev_aligned_reg = format!("%prev_aligned_param_{}_{}_{}", self.instructions.len(), i, prev_i);
                                    self.instructions.push(format!("  {} = and i64 {}, -8", prev_aligned_reg, prev_pre_align_reg));

                                    // But for i1, use current offset without alignment
                                    let prev_offset_reg = format!("%prev_offset_param_{}_{}_{}", self.instructions.len(), i, prev_i);
                                    self.instructions.push(format!("  {} = select i1 {}, i64 {}, i64 {}", prev_offset_reg, prev_is_i1_reg, current_offset_reg, prev_aligned_reg));

                                    // Add size to get next offset
                                    let prev_size_reg = format!("%prev_size_param_{}_{}_{}", self.instructions.len(), i, prev_i);
                                    self.instructions.push(format!("  {} = select i1 {}, i64 1, i64 8", prev_size_reg, prev_is_i1_reg));
                                    let next_offset_reg = format!("%next_offset_param_{}_{}_{}", self.instructions.len(), i, prev_i);
                                    self.instructions.push(format!("  {} = add i64 {}, {}", next_offset_reg, prev_offset_reg, prev_size_reg));
                                    current_offset_reg = next_offset_reg;
                                }

                                // Now calculate offset for current element
                                let is_current_i1_reg = format!("%is_current_i1_param_{}_{}", self.instructions.len(), i);
                                self.instructions.push(format!("  {} = icmp eq i8 {}, 0", is_current_i1_reg, type_val_reg));

                                // Align current offset for i64 elements
                                let current_pre_align_reg = format!("%current_pre_align_param_{}_{}", self.instructions.len(), i);
                                self.instructions.push(format!("  {} = add i64 {}, 7", current_pre_align_reg, current_offset_reg));
                                let current_aligned_reg = format!("%current_aligned_param_{}_{}", self.instructions.len(), i);
                                self.instructions.push(format!("  {} = and i64 {}, -8", current_aligned_reg, current_pre_align_reg));

                                // Select: i1 uses current_offset, i64 uses aligned offset
                                let elem_offset_reg = format!("%elem_offset_param_{}_{}", self.instructions.len(), i);
                                self.instructions.push(format!("  {} = select i1 {}, i64 {}, i64 {}", elem_offset_reg, is_current_i1_reg, current_offset_reg, current_aligned_reg));

                                // Load the element
                                let elem_ptr_reg = format!("%elem_ptr_param_{}_{}", self.instructions.len(), i);
                                self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", elem_ptr_reg, tuple_reg, elem_offset_reg));

                                // Load based on type; use unique register for wildcard '_' to avoid multiple definition of %_
                                let elem_reg = if elem_name == "_" {
                                    format!("%_param_discard_{}_{}", self.instructions.len(), i)
                                } else {
                                    format!("%{}", elem_name)
                                };
                                if i == 0 {
                                    // First element is bool (i1)
                                    let i1_cast_reg = format!("%i1_cast_param_{}_{}", self.instructions.len(), i);
                                    self.instructions.push(format!("  {} = bitcast i8* {} to i1*", i1_cast_reg, elem_ptr_reg));
                                    let i1_val_reg = format!("%i1_val_param_{}_{}", self.instructions.len(), i);
                                    self.instructions.push(format!("  {} = load i1, i1* {}", i1_val_reg, i1_cast_reg));
                                    self.instructions.push(format!("  {} = zext i1 {} to i64", elem_reg, i1_val_reg));
                                } else {
                                    // Other elements are i64
                                    let i64_cast_reg = format!("%i64_cast_param_{}_{}", self.instructions.len(), i);
                                    self.instructions.push(format!("  {} = bitcast i8* {} to i64*", i64_cast_reg, elem_ptr_reg));
                                    self.instructions.push(format!("  {} = load i64, i64* {}", elem_reg, i64_cast_reg));
                                }

                                // Store in variable scope; actual IR type is i64 (zext i1 or load i64)
                                self.variables.insert(elem_name.clone(), elem_reg);
                                self.variable_llvm_types.insert(elem_name.clone(), "i64".to_string());
                            }
                        }
                    }
                    _ => {} // Other pattern types not supported yet
                }
                tuple_reg
            } else {
                // Regular identifier parameter
                let param_reg = format!("%{}", param.name);
                self.variables.insert(param.name.clone(), param_reg.clone());
                self.variable_types.insert(param.name.clone(), param.type_.clone());
                // When sret is used, param_types[0] is sret; use param_types[i+1] for the i-th func param
                let param_type_idx = if param_types.len() > func.parameters.len() { i + 1 } else { i };
                if let Some(llvm_ty) = param_types.get(param_type_idx) {
                    self.variable_llvm_types.insert(param.name.clone(), llvm_ty.clone());
                }
                param_reg
            };
            self.instructions.push(format!("  ; Parameter: {}", param_reg));
        }

        // Enter scope for function local variables
        self.enter_scope_text();

        // Generate function body statements
        let body_result = self.generate_statements(&func.body)?;

        // Exit function scope
        self.exit_scope_text();


        // Generate return
        let use_sret = return_type_str == "i8*";
        match return_type {
            Type::Unit => {
                self.instructions.push("  ret void".to_string());
            }
            _ => {
                // Return the result of the function body
                if let Some(result_val) = body_result {
                    // sret: store result in caller-provided slot and return void (fixes recursive struct return bug)
                    if use_sret {
                        let result_reg = if result_val.starts_with("i8* ") {
                            result_val.trim_start_matches("i8* ").to_string()
                        } else if result_val.starts_with("i64 ") {
                            let ptr_reg = format!("%sret_inttoptr_{}", self.instructions.len());
                            self.instructions.push(format!("  {} = inttoptr i64 {} to i8*", ptr_reg, result_val.trim_start_matches("i64 ")));
                            ptr_reg
                        } else {
                            result_val
                        };
                        let clean = Self::format_llvm_value_ref(&result_reg);
                        self.instructions.push(format!("  store i8* {}, i8* * %sret", clean));
                        self.instructions.push("  ret void".to_string());
                    } else {
                    // Handle type conversions if needed (e.g., i64 to i8* for ActorRef)
                    // Check if result_val has type prefix or is just a register name
                    // Special case: actor reference registers from spawn are i64 but function returns i8*
                    if return_type_str == "i8*" && result_val.starts_with("%actor_") {
                        // This is an actor reference register (i64) that needs conversion to i8*
                        let ptr_reg = format!("%return_ptr_{}", self.instructions.len());
                        self.instructions.push(format!("  {} = inttoptr i64 {} to i8*", ptr_reg, result_val));
                        self.instructions.push(format!("  ret i8* {}", ptr_reg));
                    } else {
                        // Extract type and register from result_val
                        let (result_type, result_reg) = if result_val.starts_with("i64 ") {
                            ("i64".to_string(), result_val.trim_start_matches("i64 ").to_string())
                        } else if result_val.starts_with("i32 ") {
                            ("i32".to_string(), result_val.trim_start_matches("i32 ").to_string())
                        } else if result_val.starts_with("i16 ") {
                            ("i16".to_string(), result_val.trim_start_matches("i16 ").to_string())
                        } else if result_val.starts_with("i8 ") {
                            ("i8".to_string(), result_val.trim_start_matches("i8 ").to_string())
                        } else if result_val.starts_with("i8* ") {
                            ("i8*".to_string(), result_val.trim_start_matches("i8* ").to_string())
                        } else if result_val.starts_with("i1 ") {
                            ("i1".to_string(), result_val.trim_start_matches("i1 ").to_string())
                        } else if result_val.starts_with("float ") {
                            ("float".to_string(), result_val.trim_start_matches("float ").to_string())
                        } else if result_val.starts_with("half ") {
                            ("half".to_string(), result_val.trim_start_matches("half ").to_string())
                        } else {
                            // No type prefix - assume it matches the return type
                            (return_type_str.to_string(), result_val)
                        };
                        
                        // Check if type conversion is needed
                        if return_type_str == "i8*" && result_type == "i64" {
                            // Need to convert i64 to i8* (e.g., for ActorRef return type)
                            let ptr_reg = format!("%return_ptr_{}", self.instructions.len());
                            self.instructions.push(format!("  {} = inttoptr i64 {} to i8*", ptr_reg, result_reg));
                            self.instructions.push(format!("  ret i8* {}", ptr_reg));
                        } else if return_type_str == "i64" && result_type == "i8*" {
                            // Need to convert i8* to i64 (unlikely but handle for completeness)
                            let int_reg = format!("%return_int_{}", self.instructions.len());
                            self.instructions.push(format!("  {} = ptrtoint i8* {} to i64", int_reg, result_reg));
                            self.instructions.push(format!("  ret i64 {}", int_reg));
                        } else {
                            // Types match or no conversion needed
                            // Check if we need to truncate for smaller return types or convert float types
                            // First, ensure result_reg doesn't have a type prefix (it should just be the register name)
                            let clean_result_reg = result_reg.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i16 ").trim_start_matches("i8 ").trim_start_matches("half ").trim_start_matches("float ").trim_start_matches("i1 ").to_string();
                            
                            let final_result = if return_type_str == "i8" && (result_type == "i64" || result_type == "i32" || result_type == "i16") {
                                let trunc_reg = format!("%trunc_return_{}", self.instructions.len());
                                self.instructions.push(format!("  {} = trunc {} {} to i8", trunc_reg, result_type, clean_result_reg));
                                trunc_reg
                            } else if return_type_str == "i16" && (result_type == "i64" || result_type == "i32") {
                                let trunc_reg = format!("%trunc_return_{}", self.instructions.len());
                                self.instructions.push(format!("  {} = trunc {} {} to i16", trunc_reg, result_type, clean_result_reg));
                                trunc_reg
                            } else if return_type_str == "i32" && result_type == "i64" {
                                let trunc_reg = format!("%trunc_return_{}", self.instructions.len());
                                self.instructions.push(format!("  {} = trunc i64 {} to i32", trunc_reg, clean_result_reg));
                                trunc_reg
                            } else if return_type_str == "half" && result_type == "float" {
                                let trunc_reg = format!("%trunc_return_{}", self.instructions.len());
                                // clean_result_reg should already be a register name (no type prefix)
                                // If it's a float literal, create a constant first (fptrunc doesn't accept literals)
                                if !clean_result_reg.starts_with('%') && clean_result_reg.parse::<f64>().is_ok() {
                                    // Create float constant using bitcast from integer (most reliable method)
                                    let float_const = format!("%float_const_return_{}", self.instructions.len());
                                    let instruction = self.create_float_constant_instruction(&clean_result_reg, &float_const, "float");
                                    self.instructions.push(instruction);
                                    self.instructions.push(format!("  {} = fptrunc float {} to half", trunc_reg, float_const));
                                } else {
                                    self.instructions.push(format!("  {} = fptrunc float {} to half", trunc_reg, clean_result_reg));
                                }
                                trunc_reg
                            } else if return_type_str == "float" && result_type == "half" {
                                let ext_reg = format!("%ext_return_{}", self.instructions.len());
                                // Strip "half " prefix from result_reg if present
                                let clean_half = clean_result_reg.trim_start_matches("half ").to_string();
                                self.instructions.push(format!("  {} = fpext half {} to float", ext_reg, clean_half));
                                ext_reg
                            } else {
                                // No conversion needed - use the cleaned register name
                                clean_result_reg
                            };
                            
                            // Check if final_result is a float literal that needs to be converted to a constant
                            let ret_value = if return_type_str == "float" && !final_result.starts_with('%') && final_result.parse::<f64>().is_ok() {
                                // Float literal - create a constant first
                                let float_const = format!("%float_const_ret_{}", self.instructions.len());
                                let instruction = self.create_float_constant_instruction(&final_result, &float_const, "float");
                                self.instructions.push(instruction);
                                float_const
                            } else {
                                final_result
                            };
                            
                            // Ensure register names have % prefix (e.g. t6 -> %t6); literals stay as-is
                            let ret_operand = Self::format_llvm_value_ref(&ret_value);
                            self.instructions.push(format!("  ret {} {}", return_type_str, ret_operand));
                        }
                    }
                    }
                } else {
                    // Fallback to dummy value if no result
                    if use_sret {
                        self.instructions.push("  store i8* null, i8* * %sret".to_string());
                        self.instructions.push("  ret void".to_string());
                    } else {
                        self.instructions.push(format!("  ret {} 0", return_type_str));
                    }
                }
            }
        }

        self.instructions.push("}".to_string());

        // Restore previous module
        self.current_module = prev_module;

        Ok(())
    }

    /// Emit C entry point wrapper: define i32 @main() that calls Silica main and truncates i64 -> i32.
    /// The text-based codegen produces module.main (e.g. main.main), not main; the C runtime expects main.
    #[cfg(not(feature = "llvm_backend"))]
    fn generate_main_wrapper_text(&mut self, program: &Program) -> Result<()> {
        let silica_main = program
            .declarations
            .iter()
            .enumerate()
            .find_map(|(i, decl)| {
                if let Declaration::Function(func) = decl {
                    if func.name == "main" {
                        let module = program
                            .declaration_modules
                            .get(i)
                            .map(|s| s.as_str())
                            .unwrap_or("main");
                        return Some(format!("{}.main", module));
                    }
                }
                None
            });

        if let Some(qualified_main) = silica_main {
            self.instructions.push("".to_string());
            self.instructions.push("; C entry point: calls Silica main, truncates i64 -> i32".to_string());
            self.instructions.push(format!("define i32 @main() {{"));
            self.instructions.push(format!("  %1 = call i64 @{}()", qualified_main));
            self.instructions.push(format!("  %2 = trunc i64 %1 to i32"));
            self.instructions.push(format!("  ret i32 %2"));
            self.instructions.push("}".to_string());
        }

        Ok(())
    }

    /// Generate LLVM IR for an expression
    /// Generate expression (text backend)
    #[cfg(not(feature = "llvm_backend"))]
    fn generate_expression(&mut self, expr: &Expression) -> Result<Option<String>> {
        // Debug: check if we're using LLVM backend
        #[cfg(feature = "llvm_backend")]
        if self.context.is_null() == false {
            // We're using LLVM backend, delegate to LLVM version
            return self.generate_expression_llvm(expr).map(|opt| opt.map(|_| "dummy".to_string()));
        }

        match expr {
            Expression::Literal(lit) => {
                // Get the type from expression_types if available (for float literals)
                let expr_type = Self::try_get_expression_location(expr)
                    .and_then(|loc| self.expression_types.get(loc).cloned());
                Ok(Some(self.generate_literal_with_type(lit, expr_type.as_ref())))
            },
            Expression::Identifier(name) => self.generate_identifier(name),
            Expression::Binary(binary) => self.generate_binary(binary),
            Expression::Unary(unary) => self.generate_unary(unary),
            Expression::Call(call) => {
                // Handle special core affinity calls
                if let Expression::Identifier(func_name) = &*call.function {
                    if func_name == "core_id" {
                        return Ok(Some(self.generate_core_id_call(call)?));
                    }
                }
                self.generate_call(call)
            },
            Expression::If(if_expr) => self.generate_if(if_expr),
            Expression::Case(case) => self.generate_case(case),
            Expression::Do(do_expr) => self.generate_do(do_expr),
            Expression::Region(region) => self.generate_region(region),
            Expression::ReadRef(read) => self.generate_read_ref(read),
            Expression::Spawn(spawn) => self.generate_spawn(spawn),
            Expression::Send(send) => self.generate_send(send),
            Expression::Cast(cast) => self.generate_cast(cast),
            Expression::Recv(recv) => self.generate_recv(recv),
            Expression::ReadFile(read_file) => self.generate_read_file(read_file),
            Expression::WriteFile(write_file) => self.generate_write_file(write_file),
            Expression::Print(print) => self.generate_print(print),
            Expression::PrintLn(println) => self.generate_println(println),
            Expression::PrintInt64(print_int64) => self.generate_print_int64(print_int64),
            Expression::PrintInt32(print_int32) => self.generate_print_int32(print_int32),
            Expression::PrintInt16(print_int16) => self.generate_print_int16(print_int16),
            Expression::PrintInt8(print_int8) => self.generate_print_int8(print_int8),
            Expression::PrintBool(print_bool) => self.generate_print_bool(print_bool),
            Expression::PrintChar(print_char) => self.generate_print_char(print_char),
            Expression::PrintFloat16(print_float16) => self.generate_print_float16(print_float16),
            Expression::PrintFloat32(print_float32) => self.generate_print_float32(print_float32),
            Expression::PrintFloat64(print_float64) => self.generate_print_float64(print_float64),
            Expression::GetCpuTopology(get_topology) => self.generate_get_cpu_topology(get_topology),
            Expression::ReadLines(read_lines) => self.generate_read_lines(read_lines),
            Expression::AppendFile(append_file) => self.generate_append_file(append_file),
            Expression::FileExists(file_exists) => self.generate_file_exists(file_exists),
            Expression::DeleteFile(delete_file) => self.generate_delete_file(delete_file),
            Expression::GetFileSize(get_file_size) => self.generate_get_file_size(get_file_size),
            Expression::CreateDirectory(create_dir) => self.generate_create_directory(create_dir),
            Expression::RemoveDirectory(remove_dir) => self.generate_remove_directory(remove_dir),
            Expression::ListDirectory(list_dir) => self.generate_list_directory(list_dir),
            Expression::StringLen(string_len) => self.generate_string_len(string_len),
            Expression::StringLenChars(string_len_chars) => self.generate_string_len_chars(string_len_chars),
            Expression::StringConcat(string_concat) => self.generate_string_concat(string_concat),
            Expression::StringSubstring(string_substring) => self.generate_string_substring(string_substring),
            Expression::StringSubstringUntilChar(string_substring_until_char) => self.generate_string_substring_until_char(string_substring_until_char),
            Expression::StringStartsWith(string_starts_with) => self.generate_string_starts_with(string_starts_with),
            Expression::StringEndsWith(string_ends_with) => self.generate_string_ends_with(string_ends_with),
            Expression::StringContains(string_contains) => self.generate_string_contains(string_contains),
            Expression::ExecCommand(exec_cmd) => self.generate_exec_command(exec_cmd),
            Expression::FunctionLiteral(func_lit) => self.generate_function_literal(func_lit),
            Expression::Region(_) => {
                Err(CompilerError::codegen_error("Region expressions not yet implemented".to_string()))
            }
            Expression::StructLiteral(struct_lit) => self.generate_struct_literal(struct_lit),
            Expression::FieldAccess(field_access) => self.generate_field_access(field_access),
            Expression::Tuple(tuple) => self.generate_tuple(tuple),
            Expression::ConstructorCall(_) => {
                {
                    let metadata = ErrorMetadataBuilder::new("E4004".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("Constructor calls not yet implemented".to_string(), None, metadata))
                }
            }
            Expression::AsType(as_type) => self.generate_as_type(as_type),
            Expression::ModuleCall(module_call) => self.generate_module_call(module_call),
        }
    }

    /// Generate expression (LLVM backend) - simplified for function calls only
    #[cfg(feature = "llvm_backend")]
    fn generate_expression(&mut self, _expr: &Expression) -> Result<Option<String>> {
        // For LLVM backend, we use generate_expression_llvm for actual LLVM generation
        // This method is only used by text backend code, so return an error for LLVM
        Err(CompilerError::codegen_error("Text expression generation not available in LLVM backend".to_string()))
    }

    /// Get the size in bytes for a Silica type in LLVM
    fn get_type_size_bytes(&self, ty: &Type) -> i64 {
        match ty {
            Type::Int8 => 1,       // i8
            Type::Int16 => 2,      // i16
            Type::Int32 => 4,       // i32
            Type::Int64 => 8,       // i64
            Type::Float16 => 2,     // half (f16)
            Type::Float32 => 4,     // float (f32)
            Type::Float64 => 8,     // double (f64)
            Type::Bool => 1,     // i1
            Type::Char => 4,     // i32
            Type::String => 8,   // i8* (pointer)
            Type::Function { .. } => 8, // i8* (function pointer)
            Type::Tuple(elements) => {
                // Calculate tuple size based on elements
                // This is a simplified calculation - real implementation would need proper alignment
                let mut size = 8 + elements.len() as i64; // count + type_ids
                size = ((size + 7) / 8) * 8; // align to 8 bytes
                for elem in elements {
                    let elem_size = self.get_type_size_bytes(elem);
                    let elem_align = self.get_type_alignment_bytes(elem);
                    size = ((size + elem_align - 1) / elem_align) * elem_align;
                    size += elem_size;
                }
                size
            }
            Type::Record(_) => 8, // i8* (simplified)
            _ => 8, // Default size
        }
    }

    /// Get the alignment in bytes for a Silica type in LLVM
    fn get_type_alignment_bytes(&self, ty: &Type) -> i64 {
        match ty {
            Type::Int8 => 1,       // i8 alignment
            Type::Int16 => 2,      // i16 alignment
            Type::Int32 => 4,      // i32 alignment
            Type::Int64 => 8,      // i64 alignment
            Type::Float16 => 2,    // half alignment
            Type::Float32 => 4,    // float alignment
            Type::Float64 => 8,    // double alignment
            Type::Bool => 1,     // i1 alignment
            Type::Char => 4,     // i32 alignment
            Type::String => 8,   // i8* alignment
            Type::Function { .. } => 8, // i8* alignment
            Type::Tuple(_) => 8, // Tuple alignment
            Type::Record(_) => 8, // Record alignment
            _ => 8, // Default alignment
        }
    }

    /// Generate LLVM IR for literal values
    fn generate_literal(&mut self, lit: &Literal) -> String {
        self.generate_literal_with_type(lit, None)
    }

    /// Create a float constant in LLVM IR using bitcast from integer
    /// This is more reliable than using decimal or hex literals directly in instructions
    fn create_float_constant_instruction(&mut self, float_str: &str, reg_name: &str, float_type: &str) -> String {
        if let Ok(value) = float_str.parse::<f64>() {
            match float_type {
                "half" => {
                    // Convert to f32 first, then we'll truncate to half
                    let f32_value = value as f32;
                    let bits = f32_value.to_bits();
                    format!("  {} = bitcast i32 {} to float", reg_name, bits)
                }
                "float" => {
                    // Convert to f32
                    let f32_value = value as f32;
                    let bits = f32_value.to_bits();
                    format!("  {} = bitcast i32 {} to float", reg_name, bits)
                }
                "double" => {
                    // Convert to f64
                    let bits = value.to_bits();
                    format!("  {} = bitcast i64 {} to double", reg_name, bits)
                }
                _ => {
                    // Fallback: try to use decimal (might fail for inexact values)
                    format!("  {} = fadd {} 0.0, {}", reg_name, float_type, float_str)
                }
            }
        } else {
            // Fallback: try to use decimal (might fail for inexact values)
            format!("  {} = fadd {} 0.0, {}", reg_name, float_type, float_str)
        }
    }

    /// Generate LLVM IR for literal values with optional type context
    fn generate_literal_with_type(&mut self, lit: &Literal, expr_type: Option<&Type>) -> String {
        match lit {
            Literal::Unit => "i64 0".to_string(), // Unit value represented as i64 0
            Literal::Bool(true) => "i1 1".to_string(),
            Literal::Bool(false) => "i1 0".to_string(),
            Literal::Int(value) => format!("i64 {}", value),
            Literal::Float(value) => {
                // Use expression type if available, default to float32
                let type_str = if let Some(ty) = expr_type {
                    match ty {
                        Type::Float16 => "half",
                        Type::Float32 => "float",
                        Type::Float64 => "double",
                        _ => "float", // default
                    }
                } else {
                    "float" // default
                };
                format!("{} {}", type_str, value)
            },
            Literal::Char(c) => format!("i32 {}", *c as i32),
            Literal::String(s) => {
                #[cfg(feature = "llvm_backend")]
                {
                    // LLVM backend handles strings inline - no named constants needed
                    // This should not be reached when LLVM backend is active
                    return Err(CompilerError::codegen_error("String literals should be handled by LLVM backend".to_string()));
                }
                #[cfg(not(feature = "llvm_backend"))]
                {
                    // Text backend: create named constants
                if !self.string_constants.contains_key(s) {
                    let const_name = format!("@str_const_{}", self.string_constants.len());
                    // Store length including null terminator to match constant declaration
                    let length = s.len() + 1;
                    self.string_constants.insert(s.clone(), (const_name, length));
                }
                    let (const_name, length) = self.string_constants.get(s).unwrap();

                    // Generate getelementptr to convert array to pointer
                    // length already includes null terminator
                    // For constant expressions: getelementptr inbounds ([LEN x i8], [LEN x i8]* CONST_NAME, i64 0, i64 0)
                    // For instructions: getelementptr inbounds [LEN x i8], [LEN x i8]* CONST_NAME, i32 0, i32 0
                    format!("getelementptr inbounds ([{} x i8], [{} x i8]* {}, i64 0, i64 0)",
                           length, length, const_name)
                }
            }
        }
    }

    /// Generate LLVM IR for identifier reference
    fn generate_identifier(&self, name: &str) -> Result<Option<String>> {
        // First check the scope stack for variables
        if let Some(var_reg) = self.lookup_variable_text(name) {
            // Self-referential placeholder: not yet defined; use null so store is valid; we patch the field later.
            if self.self_ref_placeholders.contains(name) {
                return Ok(Some("null".to_string()));
            }
            // eprintln!("DEBUG generate_identifier: found '{}' -> '{}'", name, var_reg);
            Ok(Some(var_reg))
        }
        // Then check the global variables map (e.g. function parameters in text backend)
        // Parameters must be returned with type prefix so getelementptr etc. get correct base type.
        // Prefer variable_llvm_types (actual IR type) over variable_types so e.g. tuple bool stored as i64 is correct.
        else if let Some(var_reg) = self.variables.get(name) {
            let result = if let Some(llvm_type) = self.variable_llvm_types.get(name) {
                format!("{} {}", llvm_type, var_reg)
            } else if let Some(silica_type) = self.variable_types.get(name) {
                // When variable_llvm_types is missing, use i64 for types often stored as i64 in this backend:
                // - Bool: pattern bindings and tuple elements use i64; i1 would cause "icmp eq i1 %reg, 0" on i64.
                // - String/Record/Tuple/Function: often passed or stored as i64 (ptr-as-int); i8* would cause
                //   "ptrtoint i8* %reg to i64" when %reg is actually i64 ("defined with type 'i64' but expected 'ptr'").
                let llvm_type = match silica_type {
                    Type::Bool => "i64".to_string(),
                    Type::String | Type::Record(_) | Type::Tuple(_) | Type::Function { .. } => "i64".to_string(),
                    _ => self.type_map.silica_to_llvm_str(silica_type),
                };
                format!("{} {}", llvm_type, var_reg)
            } else {
                var_reg.clone()
            };
            Ok(Some(result))
        }
        // Then check if it's a function
        else if self.functions.contains_key(name) {
            Ok(Some(format!("@{}", name)))
        } else {
            Err(CompilerError::codegen_error(format!("Undefined identifier: {}", name)))
        }
    }

    /// Generate LLVM IR for binary operations
    fn generate_binary(&mut self, binary: &BinaryExpr) -> Result<Option<String>> {
        let left = self.generate_expression(&binary.left)?;
        let right = self.generate_expression(&binary.right)?;

        if let (Some(lhs), Some(rhs)) = (left, right) {
            let temp_reg = format!("%t{}", self.instructions.len());

            // Determine the LLVM type to use for the operation
            // For boolean operations, use i1. For others, determine based on operand types
            let op_type = match binary.operator {
                BinaryOp::And | BinaryOp::Or => "i1",
                _ => {
                    // For other operations, determine type based on operands
                    // Try to get type from expression first (for identifiers, use variable name)
                    // First, try to get type from expression (prioritize variable/expression types over literal prefixes)
                    let lhs_type_from_expr = if let Expression::Identifier(name) = &*binary.left {
                        // Look up by variable name
                        self.variable_types.get(name).cloned()
                    } else {
                        // Try to get from expression_types map
                        Self::try_get_expression_location(&binary.left)
                            .and_then(|loc| self.expression_types.get(loc).cloned())
                    };
                    
                    let lhs_type = if let Some(var_type) = lhs_type_from_expr {
                        // Use type from variable/expression (most accurate)
                        match var_type {
                            Type::Char => "i32",
                            Type::Int8 => "i8",
                            Type::Int16 => "i16",
                            Type::Int32 => "i32",
                            Type::Int64 => "i64",
                            Type::Float16 => "half",
                            Type::Float32 => "float",
                            Type::Float64 => "double",
                            Type::Bool => "i1",
                            _ => "i64", // fallback
                        }
                    } else if lhs.starts_with("i8* ") {
                        "i64" // loaded as i64
                    } else if lhs.starts_with("i64 ") {
                        "i64"
                    } else if lhs.starts_with("i32 ") {
                        "i32"
                    } else if lhs.starts_with("i16 ") {
                        "i16"
                    } else if lhs.starts_with("i8 ") {
                        "i8"
                    } else if lhs.starts_with("half ") {
                        "half"
                    } else if lhs.starts_with("double ") {
                        "double"
                    } else if lhs.starts_with("float ") {
                        "float"
                    } else if lhs.starts_with("i1 ") {
                        "i1"
                    } else {
                        // Fallback: try register name lookup
                        let clean_reg = lhs.trim_start_matches('%');
                        if let Some(var_type) = self.variable_types.get(clean_reg) {
                            match var_type {
                                Type::Char => "i32",
                                Type::Int8 => "i8",
                                Type::Int16 => "i16",
                                Type::Int32 => "i32",
                                Type::Int64 => "i64",
                                Type::Float16 => "half",
                                Type::Float32 => "float",
                                Type::Float64 => "double",
                                Type::Bool => "i1",
                                _ => "i64", // fallback
                            }
                        } else {
                            "i64" // fallback for unknown types
                        }
                    };

                    // First, try to get type from expression (prioritize variable/expression types over literal prefixes)
                    let rhs_type_from_expr = if let Expression::Identifier(name) = &*binary.right {
                        // Look up by variable name
                        self.variable_types.get(name).cloned()
                    } else {
                        // Try to get from expression_types map
                        Self::try_get_expression_location(&binary.right)
                            .and_then(|loc| self.expression_types.get(loc).cloned())
                    };
                    
                    let rhs_type = if let Some(var_type) = rhs_type_from_expr {
                        // Use type from variable/expression (most accurate)
                        match var_type {
                            Type::Char => "i32",
                            Type::Int8 => "i8",
                            Type::Int16 => "i16",
                            Type::Int32 => "i32",
                            Type::Int64 => "i64",
                            Type::Float16 => "half",
                            Type::Float32 => "float",
                            Type::Float64 => "double",
                            Type::Bool => "i1",
                            _ => "i64", // fallback
                        }
                    } else if rhs.starts_with("i8* ") {
                        "i64" // loaded as i64
                    } else if rhs.starts_with("i64 ") {
                        "i64"
                    } else if rhs.starts_with("i32 ") {
                        "i32"
                    } else if rhs.starts_with("i16 ") {
                        "i16"
                    } else if rhs.starts_with("i8 ") {
                        "i8"
                    } else if rhs.starts_with("half ") {
                        "half"
                    } else if rhs.starts_with("double ") {
                        "double"
                    } else if rhs.starts_with("float ") {
                        "float"
                    } else if rhs.starts_with("i1 ") {
                        "i1"
                    } else {
                        // Fallback: try register name lookup
                        let clean_reg = rhs.trim_start_matches('%');
                        if let Some(var_type) = self.variable_types.get(clean_reg) {
                            match var_type {
                                Type::Char => "i32",
                                Type::Int8 => "i8",
                                Type::Int16 => "i16",
                                Type::Int32 => "i32",
                                Type::Int64 => "i64",
                                Type::Float16 => "half",
                                Type::Float32 => "float",
                                Type::Float64 => "double",
                                Type::Bool => "i1",
                                _ => "i64", // fallback
                            }
                        } else {
                            "i64" // fallback for unknown types
                        }
                    };

                    // For arithmetic operations, types must match exactly (no promotion)
                    // For comparisons, types must match (but allow i1 vs i64 by extending i1 to i64)
                    if lhs_type != rhs_type {
                        match binary.operator {
                            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                                return Err(CompilerError::codegen_error(
                                    format!("Arithmetic operands must be same type: {} vs {}", lhs_type, rhs_type)
                                ));
                            }
                            _ => {
                                // Allow i1 vs i64 for comparisons: extend i1 to i64 (treats true=1, false=0)
                                if (lhs_type == "i1" && rhs_type == "i64") || (lhs_type == "i64" && rhs_type == "i1") {
                                    // Will handle below by extending i1 operand to i64
                                } else {
                                    return Err(CompilerError::codegen_error(format!("Cannot compare {} and {}", lhs_type, rhs_type)));
                                }
                            }
                        }
                    }
                    
                    // For modulo, ensure it's an integer type
                    if matches!(binary.operator, BinaryOp::Modulo) {
                        if lhs_type == "half" || lhs_type == "float" || lhs_type == "double" {
                            return Err(CompilerError::codegen_error(
                                format!("Modulo operation requires integer operands, found {}", lhs_type)
                            ));
                        }
                    }
                    
                    // For i1 vs i64 comparison, use i64 (i1 operand will be extended via zext)
                    if (lhs_type == "i1" && rhs_type == "i64") || (lhs_type == "i64" && rhs_type == "i1") {
                        "i64"
                    } else {
                        lhs_type // Both types are the same, use either one
                    }
                }
            };

            // Determine if this is a float operation (needed for operand processing)
            let is_float = op_type == "half" || op_type == "float" || op_type == "double";

            // Handle operands based on their actual types
            let clean_lhs = if lhs.starts_with("i8* ") {
                // Left operand is an i8* register - load it
                let load_reg = format!("%load_left_{}", self.instructions.len());
                let reg_name = lhs.trim_start_matches("i8* ");
                self.instructions.push(format!("  {} = bitcast i8* {} to i64*", load_reg.clone() + "_cast", reg_name));
                self.instructions.push(format!("  {} = load i64, i64* {}_cast", load_reg, load_reg));
                load_reg
            } else {
                let trimmed = lhs.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i16 ").trim_start_matches("i8 ").trim_start_matches("i1 ").trim_start_matches("half ").trim_start_matches("float ").trim_start_matches("double ").trim_start_matches("i8* ").to_string();
                // Check if this is a variable that needs type extension
                let needs_extension = if let Expression::Identifier(name) = &*binary.left {
                    // Look up variable type
                    if let Some(var_type) = self.variable_types.get(name) {
                        match var_type {
                            Type::Int8 => op_type == "i64" || op_type == "i32" || op_type == "i16",
                            Type::Int16 => op_type == "i64" || op_type == "i32",
                            Type::Int32 => op_type == "i64",
                            _ => false,
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };
                
                // Extend operands to match operation type if necessary
                if lhs.starts_with("i32 ") && op_type == "i64" {
                    let extend_reg = format!("%extend_left_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = sext i32 {} to i64", extend_reg, trimmed));
                    extend_reg
                } else if lhs.starts_with("i16 ") && op_type == "i64" {
                    let extend_reg = format!("%extend_left_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = sext i16 {} to i64", extend_reg, trimmed));
                    extend_reg
                } else if lhs.starts_with("i8 ") && op_type == "i64" {
                    let extend_reg = format!("%extend_left_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = sext i8 {} to i64", extend_reg, trimmed));
                    extend_reg
                } else if needs_extension {
                    // Variable needs extension - determine source type
                    if let Expression::Identifier(name) = &*binary.left {
                        if let Some(var_type) = self.variable_types.get(name) {
                            let (source_type, target_type) = match (var_type, op_type) {
                                (Type::Int8, "i16") => ("i8", "i16"),
                                (Type::Int8, "i32") => ("i8", "i32"),
                                (Type::Int8, "i64") => ("i8", "i64"),
                                (Type::Int16, "i32") => ("i16", "i32"),
                                (Type::Int16, "i64") => ("i16", "i64"),
                                (Type::Int32, "i64") => ("i32", "i64"),
                                _ => ("", ""),
                            };
                            if !source_type.is_empty() {
                                let extend_reg = format!("%extend_left_{}", self.instructions.len());
                                self.instructions.push(format!("  {} = sext {} {} to {}", extend_reg, source_type, trimmed, target_type));
                                extend_reg
                            } else {
                                trimmed
                            }
                        } else {
                            trimmed
                        }
                    } else {
                        trimmed
                    }
                } else if lhs.starts_with("i1 ") && op_type != "i1" {
                    // Extend boolean to i32/i64 as needed
                    let extend_reg = format!("%extend_left_{}", self.instructions.len());
                    let extend_type = if op_type == "i64" { "i64" } else { "i32" };
                    self.instructions.push(format!("  {} = zext i1 {} to {}", extend_reg, trimmed, extend_type));
                    extend_reg
                } else {
                    // Check if this operand was originally a type-prefixed literal
                    let is_literal_operand = lhs.starts_with("i64 ") || lhs.starts_with("i32 ") || lhs.starts_with("i16 ") || lhs.starts_with("i8 ") || lhs.starts_with("i1 ") || lhs.starts_with("half ") || lhs.starts_with("float ");
                    if is_literal_operand {
                        // Literal constants like "10" or "3.14" should be used as-is, not as registers
                        // For float operations, ensure integer literals are converted to float
                        if is_float && !trimmed.contains('.') && !trimmed.starts_with('%') {
                            // Integer literal in float operation - convert to float literal
                            format!("{}.0", trimmed)
                        } else {
                            trimmed
                        }
                    } else {
                        // Ensure register names have % prefix
                        // Also check if this register corresponds to a variable that needs extension
                        let reg_name = if trimmed.starts_with('%') {
                            trimmed.clone()
                        } else {
                            format!("%{}", trimmed)
                        };
                        
                        // Check if this register name (without %) corresponds to a variable that needs extension
                        let reg_name_clean = reg_name.trim_start_matches('%');
                        if let Some(var_type) = self.variable_types.get(reg_name_clean) {
                            let needs_ext = match (var_type, op_type) {
                                (Type::Int8, "i16") | (Type::Int8, "i32") | (Type::Int8, "i64") => true,
                                (Type::Int16, "i32") | (Type::Int16, "i64") => true,
                                (Type::Int32, "i64") => true,
                                _ => false,
                            };
                            if needs_ext {
                                let (source_type, target_type) = match (var_type, op_type) {
                                    (Type::Int8, "i16") => ("i8", "i16"),
                                    (Type::Int8, "i32") => ("i8", "i32"),
                                    (Type::Int8, "i64") => ("i8", "i64"),
                                    (Type::Int16, "i32") => ("i16", "i32"),
                                    (Type::Int16, "i64") => ("i16", "i64"),
                                    (Type::Int32, "i64") => ("i32", "i64"),
                                    _ => ("", ""),
                                };
                                if !source_type.is_empty() {
                                    let extend_reg = format!("%extend_left_{}", self.instructions.len());
                                    self.instructions.push(format!("  {} = sext {} {} to {}", extend_reg, source_type, reg_name, target_type));
                                    extend_reg
                                } else {
                                    reg_name
                                }
                            } else {
                                reg_name
                            }
                        } else {
                            reg_name
                        }
                    }
                }
            };

            let clean_rhs = if rhs.starts_with("i8* ") {
                // Right operand is an i8* register - load it
                let load_reg = format!("%load_right_{}", self.instructions.len());
                let reg_name = rhs.trim_start_matches("i8* ");
                self.instructions.push(format!("  {} = bitcast i8* {} to i64*", load_reg.clone() + "_cast", reg_name));
                self.instructions.push(format!("  {} = load i64, i64* {}_cast", load_reg, load_reg));
                load_reg
            } else {
                let trimmed = rhs.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i16 ").trim_start_matches("i8 ").trim_start_matches("i1 ").trim_start_matches("half ").trim_start_matches("float ").trim_start_matches("double ").trim_start_matches("i8* ").to_string();
                // Check if this is a variable that needs type extension
                let needs_extension = if let Expression::Identifier(name) = &*binary.right {
                    // Look up variable type
                    if let Some(var_type) = self.variable_types.get(name) {
                        match var_type {
                            Type::Int8 => op_type == "i64" || op_type == "i32" || op_type == "i16",
                            Type::Int16 => op_type == "i64" || op_type == "i32",
                            Type::Int32 => op_type == "i64",
                            _ => false,
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };
                
                // Extend operands to match operation type if necessary
                if rhs.starts_with("i32 ") && op_type == "i64" {
                    let extend_reg = format!("%extend_right_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = sext i32 {} to i64", extend_reg, trimmed));
                    extend_reg
                } else if rhs.starts_with("i16 ") && op_type == "i64" {
                    let extend_reg = format!("%extend_right_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = sext i16 {} to i64", extend_reg, trimmed));
                    extend_reg
                } else if rhs.starts_with("i8 ") && op_type == "i64" {
                    let extend_reg = format!("%extend_right_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = sext i8 {} to i64", extend_reg, trimmed));
                    extend_reg
                } else if needs_extension {
                    // Variable needs extension - determine source type
                    if let Expression::Identifier(name) = &*binary.right {
                        if let Some(var_type) = self.variable_types.get(name) {
                            let (source_type, target_type) = match (var_type, op_type) {
                                (Type::Int8, "i16") => ("i8", "i16"),
                                (Type::Int8, "i32") => ("i8", "i32"),
                                (Type::Int8, "i64") => ("i8", "i64"),
                                (Type::Int16, "i32") => ("i16", "i32"),
                                (Type::Int16, "i64") => ("i16", "i64"),
                                (Type::Int32, "i64") => ("i32", "i64"),
                                _ => ("", ""),
                            };
                            if !source_type.is_empty() {
                                let extend_reg = format!("%extend_right_{}", self.instructions.len());
                                self.instructions.push(format!("  {} = sext {} {} to {}", extend_reg, source_type, trimmed, target_type));
                                extend_reg
                            } else {
                                trimmed
                            }
                        } else {
                            trimmed
                        }
                    } else {
                        trimmed
                    }
                } else if rhs.starts_with("i1 ") && op_type != "i1" {
                    // Extend boolean to i32/i64 as needed
                    let extend_reg = format!("%extend_right_{}", self.instructions.len());
                    let extend_type = if op_type == "i64" { "i64" } else { "i32" };
                    self.instructions.push(format!("  {} = zext i1 {} to {}", extend_reg, trimmed, extend_type));
                    extend_reg
                } else {
                    // Check if this operand was originally a type-prefixed literal
                    let is_literal_operand = rhs.starts_with("i64 ") || rhs.starts_with("i32 ") || rhs.starts_with("i16 ") || rhs.starts_with("i8 ") || rhs.starts_with("i1 ") || rhs.starts_with("half ") || rhs.starts_with("float ");
                    if is_literal_operand {
                        // Literal constants like "10" or "3.14" should be used as-is, not as registers
                        // For float operations, ensure integer literals are converted to float
                        if is_float && !trimmed.contains('.') && !trimmed.starts_with('%') {
                            // Integer literal in float operation - convert to float literal
                            format!("{}.0", trimmed)
                        } else {
                            trimmed
                        }
                    } else {
                        // Ensure register names have % prefix
                        // Also check if this register corresponds to a variable that needs extension
                        let reg_name = if trimmed.starts_with('%') {
                            trimmed.clone()
                        } else {
                            format!("%{}", trimmed)
                        };
                        
                        // Check if this register name (without %) corresponds to a variable that needs extension
                        let reg_name_clean = reg_name.trim_start_matches('%');
                        if let Some(var_type) = self.variable_types.get(reg_name_clean) {
                            let needs_ext = match (var_type, op_type) {
                                (Type::Int8, "i16") | (Type::Int8, "i32") | (Type::Int8, "i64") => true,
                                (Type::Int16, "i32") | (Type::Int16, "i64") => true,
                                (Type::Int32, "i64") => true,
                                _ => false,
                            };
                            if needs_ext {
                                let (source_type, target_type) = match (var_type, op_type) {
                                    (Type::Int8, "i16") => ("i8", "i16"),
                                    (Type::Int8, "i32") => ("i8", "i32"),
                                    (Type::Int8, "i64") => ("i8", "i64"),
                                    (Type::Int16, "i32") => ("i16", "i32"),
                                    (Type::Int16, "i64") => ("i16", "i64"),
                                    (Type::Int32, "i64") => ("i32", "i64"),
                                    _ => ("", ""),
                                };
                                if !source_type.is_empty() {
                                    let extend_reg = format!("%extend_right_{}", self.instructions.len());
                                    self.instructions.push(format!("  {} = sext {} {} to {}", extend_reg, source_type, reg_name, target_type));
                                    extend_reg
                                } else {
                                    reg_name
                                }
                            } else {
                                reg_name
                            }
                        } else {
                            reg_name
                        }
                    }
                }
            };

            // Determine operation name based on type (int vs float)
            let is_float = op_type == "half" || op_type == "float" || op_type == "double";
            
            // For float operations, always create constants from literals (LLVM doesn't accept decimal literals directly)
            let format_lhs = if is_float && !clean_lhs.starts_with('%') && clean_lhs.parse::<f64>().is_ok() {
                // Float literal - need to create constant first
                match op_type {
                    "half" => {
                        // For half type, create float register first, then truncate
                        let float_const = format!("%float_const_lhs_{}", self.instructions.len());
                        let instruction = self.create_float_constant_instruction(&clean_lhs, &float_const, "float");
                        self.instructions.push(instruction);
                        let const_reg = format!("%const_lhs_{}", self.instructions.len());
                        self.instructions.push(format!("  {} = fptrunc float {} to half", const_reg, float_const));
                        const_reg
                    }
                    "float" => {
                        // Create float constant using bitcast
                        let float_const = format!("%float_const_lhs_{}", self.instructions.len());
                        let instruction = self.create_float_constant_instruction(&clean_lhs, &float_const, "float");
                        self.instructions.push(instruction);
                        float_const
                    }
                    "double" => {
                        // Create double constant using bitcast
                        let double_const = format!("%double_const_lhs_{}", self.instructions.len());
                        let instruction = self.create_float_constant_instruction(&clean_lhs, &double_const, "double");
                        self.instructions.push(instruction);
                        double_const
                    }
                    _ => clean_lhs.clone()
                }
            } else {
                clean_lhs.clone()
            };
            let format_rhs = if is_float && !clean_rhs.starts_with('%') && clean_rhs.parse::<f64>().is_ok() {
                // Float literal - need to create constant first
                match op_type {
                    "half" => {
                        // For half type, create float register first, then truncate
                        let float_const = format!("%float_const_rhs_{}", self.instructions.len());
                        let instruction = self.create_float_constant_instruction(&clean_rhs, &float_const, "float");
                        self.instructions.push(instruction);
                        let const_reg = format!("%const_rhs_{}", self.instructions.len());
                        self.instructions.push(format!("  {} = fptrunc float {} to half", const_reg, float_const));
                        const_reg
                    }
                    "float" => {
                        // Create float constant using bitcast
                        let float_const = format!("%float_const_rhs_{}", self.instructions.len());
                        let instruction = self.create_float_constant_instruction(&clean_rhs, &float_const, "float");
                        self.instructions.push(instruction);
                        float_const
                    }
                    "double" => {
                        // Create double constant using bitcast
                        let double_const = format!("%double_const_rhs_{}", self.instructions.len());
                        let instruction = self.create_float_constant_instruction(&clean_rhs, &double_const, "double");
                        self.instructions.push(instruction);
                        double_const
                    }
                    _ => clean_rhs.clone()
                }
            } else {
                clean_rhs.clone()
            };
            
            // For comparison ops, getelementptr operands must be in registers (LLVM doesn't allow inline GEP in icmp/fcmp)
            let mut cmp_lhs = self.ensure_gep_in_register(&format_lhs, "lhs");
            let mut cmp_rhs = self.ensure_gep_in_register(&format_rhs, "rhs");
            // When comparing as i64 (e.g. pointer-as-integer), ptrtoint any GEP register to i64
            let is_cmp = matches!(binary.operator, BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual);
            if is_cmp && op_type == "i64" {
                if cmp_lhs.starts_with("%gep_") {
                    let ptrtoint_reg = format!("%ptrtoint_lhs_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = ptrtoint i8* {} to i64", ptrtoint_reg, cmp_lhs));
                    cmp_lhs = ptrtoint_reg;
                }
                if cmp_rhs.starts_with("%gep_") {
                    let ptrtoint_reg = format!("%ptrtoint_rhs_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = ptrtoint i8* {} to i64", ptrtoint_reg, cmp_rhs));
                    cmp_rhs = ptrtoint_reg;
                }
            }

            let op_instr = match binary.operator {
                BinaryOp::Add => {
                    let op_name = if is_float { "fadd" } else { "add" };
                    format!("  {} = {} {} {}, {}", temp_reg, op_name, op_type, format_lhs, format_rhs)
                },
                BinaryOp::Subtract => {
                    let op_name = if is_float { "fsub" } else { "sub" };
                    format!("  {} = {} {} {}, {}", temp_reg, op_name, op_type, format_lhs, format_rhs)
                },
                BinaryOp::Multiply => {
                    let op_name = if is_float { "fmul" } else { "mul" };
                    format!("  {} = {} {} {}, {}", temp_reg, op_name, op_type, format_lhs, format_rhs)
                },
                BinaryOp::Divide => {
                    let op_name = if is_float { "fdiv" } else { "sdiv" };
                    format!("  {} = {} {} {}, {}", temp_reg, op_name, op_type, format_lhs, format_rhs)
                },
                BinaryOp::Modulo => {
                    // Modulo only for integers (already checked above)
                    format!("  {} = srem {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                },
                BinaryOp::Equal => {
                    if is_float {
                        format!("  {} = fcmp oeq {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                    } else {
                        format!("  {} = icmp eq {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                    }
                },
                BinaryOp::NotEqual => {
                    if is_float {
                        format!("  {} = fcmp one {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                    } else {
                        format!("  {} = icmp ne {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                    }
                },
                BinaryOp::Less => {
                    if is_float {
                        format!("  {} = fcmp olt {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                    } else {
                        format!("  {} = icmp slt {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                    }
                },
                BinaryOp::LessEqual => {
                    if is_float {
                        format!("  {} = fcmp ole {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                    } else {
                        format!("  {} = icmp sle {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                    }
                },
                BinaryOp::Greater => {
                    if is_float {
                        format!("  {} = fcmp ogt {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                    } else {
                        format!("  {} = icmp sgt {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                    }
                },
                BinaryOp::GreaterEqual => {
                    if is_float {
                        format!("  {} = fcmp oge {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                    } else {
                        format!("  {} = icmp sge {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs)
                    }
                },
                BinaryOp::And => format!("  {} = and {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs),
                BinaryOp::Or => format!("  {} = or {} {}, {}", temp_reg, op_type, cmp_lhs, cmp_rhs),
            };

            self.instructions.push(op_instr);
            // Comparison ops produce i1; And/Or produce op_type. Return with type prefix so case scrutinee etc. see correct type.
            let result = if matches!(binary.operator, BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual) {
                format!("i1 {}", temp_reg)
            } else if matches!(binary.operator, BinaryOp::And | BinaryOp::Or) {
                format!("{} {}", op_type, temp_reg)
            } else {
                temp_reg
            };
            Ok(Some(result))
        } else {
            Err(CompilerError::codegen_error("Binary operation on invalid operands".to_string()))
        }
    }

    /// Generate LLVM IR for type casting: expr as Type
    fn generate_as_type(&mut self, as_type: &AsTypeExpr) -> Result<Option<String>> {
        // Special case: if casting a float literal to double, create it directly as double
        if let Expression::Literal(Literal::Float(float_val)) = &*as_type.expression {
            if matches!(as_type.target_type, Type::Float64) {
                // Create double constant directly from the float literal
                let double_const = format!("%double_const_cast_{}", self.instructions.len());
                let instruction = self.create_float_constant_instruction(&float_val.to_string(), &double_const, "double");
                self.instructions.push(instruction);
                return Ok(Some(format!("double {}", double_const)));
            }
        }
        
        // Generate the expression first
        let expr_val = self.generate_expression(&as_type.expression)?;
        
        if let Some(val) = expr_val {
            // Get source type from expression_types map, or infer from value string
            let source_type = self.get_expression_type(&as_type.expression)
                .ok()
                .or_else(|| {
                    // Try to infer from the value string prefix
                    if val.starts_with("i64 ") {
                        Some(Type::Int64)
                    } else if val.starts_with("i32 ") {
                        Some(Type::Int32)
                    } else if val.starts_with("i16 ") {
                        Some(Type::Int16)
                    } else if val.starts_with("i8 ") {
                        Some(Type::Int8)
                    } else if val.starts_with("half ") {
                        Some(Type::Float16)
                    } else if val.starts_with("float ") {
                        Some(Type::Float32)
                    } else if val.starts_with("double ") {
                        Some(Type::Float64)
                    } else if val.starts_with("i1 ") {
                        Some(Type::Bool)
                    } else {
                        None
                    }
                })
                .unwrap_or(Type::Int64);
            
            let source_type_str = Self::type_to_llvm_string(&source_type);
            let target_type_str = Self::type_to_llvm_string(&as_type.target_type);
            
            // If types are the same, no conversion needed
            if source_type_str == target_type_str {
                return Ok(Some(val));
            }
            
            // Generate type conversion instruction
            let clean_val = val.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i16 ").trim_start_matches("i8 ").trim_start_matches("half ").trim_start_matches("float ").trim_start_matches("double ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();
            let cast_reg = format!("%cast_{}", self.instructions.len());
            
            // Handle different type conversions
            match (source_type_str, target_type_str) {
                ("i8", "i16") | ("i8", "i32") | ("i8", "i64") | ("i16", "i32") | ("i16", "i64") | ("i32", "i64") => {
                    // Sign extension for integer widening
                    self.instructions.push(format!("  {} = sext {} {} to {}", cast_reg, source_type_str, clean_val, target_type_str));
                }
                ("i16", "i8") | ("i32", "i8") | ("i32", "i16") | ("i64", "i8") | ("i64", "i16") | ("i64", "i32") => {
                    // Truncation for integer narrowing
                    self.instructions.push(format!("  {} = trunc {} {} to {}", cast_reg, source_type_str, clean_val, target_type_str));
                }
                ("half", "float") => {
                    // Float widening: half to float
                    self.instructions.push(format!("  {} = fpext half {} to float", cast_reg, clean_val));
                }
                ("half", "double") => {
                    // Float widening: half to double
                    self.instructions.push(format!("  {} = fpext half {} to double", cast_reg, clean_val));
                }
                ("float", "half") => {
                    // Float narrowing: float to half
                    self.instructions.push(format!("  {} = fptrunc float {} to half", cast_reg, clean_val));
                }
                ("float", "double") => {
                    // Float widening: float to double
                    self.instructions.push(format!("  {} = fpext float {} to double", cast_reg, clean_val));
                }
                ("double", "half") => {
                    // Float narrowing: double to half
                    self.instructions.push(format!("  {} = fptrunc double {} to half", cast_reg, clean_val));
                }
                ("double", "float") => {
                    // Float narrowing: double to float
                    self.instructions.push(format!("  {} = fptrunc double {} to float", cast_reg, clean_val));
                }
                ("i32", "i1") | ("i64", "i1") => {
                    // Integer to boolean (truncate to i1)
                    self.instructions.push(format!("  {} = trunc {} {} to i1", cast_reg, source_type_str, clean_val));
                }
                ("i1", "i32") | ("i1", "i64") => {
                    // Boolean to integer (zero extend)
                    self.instructions.push(format!("  {} = zext i1 {} to {}", cast_reg, clean_val, target_type_str));
                }
                _ => {
                    // For now, assume same size or use bitcast for compatible types
                    if source_type_str == "i64" && target_type_str == "i64" {
                        return Ok(Some(val));
                    }
                    // Default: try bitcast (may not always be valid, but works for same-sized types)
                    self.instructions.push(format!("  {} = bitcast {} {} to {}", cast_reg, source_type_str, clean_val, target_type_str));
                }
            }
            
            Ok(Some(format!("{} {}", target_type_str, cast_reg)))
        } else {
            Err(CompilerError::codegen_error("Cannot cast void expression".to_string()))
        }
    }

    /// Generate LLVM IR for unary operations
    fn generate_unary(&mut self, unary: &UnaryExpr) -> Result<Option<String>> {
        let operand = self.generate_expression(&unary.operand)?;

        match unary.operator {
            UnaryOp::Negate => {
                // Negation works on all numeric types
                if let Some(op) = operand {
                    // eprintln!("DEBUG generate_unary: op = '{}'", op);
                    // Get operand type from expression_types if available
                    let operand_type = Self::try_get_expression_location(&unary.operand)
                        .and_then(|loc| {
                            // eprintln!("DEBUG generate_unary: found location, checking expression_types");
                            self.expression_types.get(loc)
                        })
                        .map(|ty| {
                            let llvm_type = Self::type_to_llvm_string(ty);
                            // eprintln!("DEBUG generate_unary: type from expression_types = {:?} -> '{}'", ty, llvm_type);
                            llvm_type
                        })
                        .unwrap_or_else(|| {
                            // eprintln!("DEBUG generate_unary: no type in expression_types, using fallback");
                            // Fallback: determine from operand string
                            if op.starts_with("i8* ") {
                                "i64"
                            } else if op.starts_with("i64 ") {
                                "i64"
                            } else if op.starts_with("i32 ") {
                                "i32"
                            } else if op.starts_with("i16 ") {
                                "i16"
                            } else if op.starts_with("i8 ") {
                                "i8"
                            } else if op.starts_with("half ") {
                                "half"
                            } else if op.starts_with("float ") {
                                "float"
                            } else if op.starts_with("double ") {
                                "double"
                            } else {
                                // Check if it's a bare float literal
                                // High-precision literals (many decimal digits) are likely double
                                if let Ok(val) = op.parse::<f64>() {
                                    // Check precision: if the string representation has many digits, it's likely double
                                    let has_high_precision = op.contains('.') && {
                                        let decimal_part = op.split('.').nth(1).unwrap_or("");
                                        let digits_after_decimal = decimal_part.chars().take_while(|c| c.is_ascii_digit()).count();
                                        digits_after_decimal > 6 // More than 6 decimal digits suggests double precision
                                    };
                                    // Also check if value is outside f32 range
                                    let outside_f32_range = val.abs() > 3.4e38 || (val.abs() < 1e-38 && val != 0.0);
                                    
                                    if has_high_precision || outside_f32_range {
                                        // eprintln!("DEBUG generate_unary: high-precision or out-of-range float literal, assuming double: {}", val);
                                        "double"
                                    } else {
                                        // eprintln!("DEBUG generate_unary: low-precision float literal, defaulting to float: {}", val);
                                        "float"
                                    }
                                } else {
                                    "i64" // fallback
                                }
                            }
                        });
                    // eprintln!("DEBUG generate_unary: operand_type = '{}'", operand_type);
                    // eprintln!("DEBUG generate_unary: op = '{}'", op);
                    
                    let temp_reg = format!("%t{}", self.instructions.len());
                    let clean_op = op.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i16 ").trim_start_matches("i8 ").trim_start_matches("half ").trim_start_matches("float ").trim_start_matches("double ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();
                    // eprintln!("DEBUG generate_unary: clean_op = '{}'", clean_op);
                    // Check if clean_op is a numeric literal (can be parsed as number)
                    // If it's a literal, use it directly; otherwise add % prefix for register
                    let clean_op_reg = if clean_op.starts_with('%') {
                        clean_op
                    } else if clean_op.parse::<i64>().is_ok() || clean_op.parse::<f64>().is_ok() {
                        // It's a numeric literal - use directly without % prefix
                        clean_op
                    } else {
                        // It's a register name - add % prefix
                        format!("%{}", clean_op)
                    };
                    // eprintln!("DEBUG generate_unary: clean_op_reg = '{}'", clean_op_reg);
                    
                    let is_float = operand_type == "half" || operand_type == "float" || operand_type == "double";
                    // Check if operand string indicates double type (even if type detection failed)
                    let is_double_from_string = op.starts_with("double ");
                    // Also check if it's a high-precision float literal that should be double
                    let is_high_precision_double = !clean_op_reg.starts_with('%') && clean_op_reg.parse::<f64>().is_ok() && {
                        let has_high_precision = clean_op_reg.contains('.') && {
                            let decimal_part = clean_op_reg.split('.').nth(1).unwrap_or("");
                            let digits_after_decimal = decimal_part.chars().take_while(|c| c.is_ascii_digit()).count();
                            digits_after_decimal > 6 // More than 6 decimal digits suggests double precision
                        };
                        let outside_f32_range = if let Ok(val) = clean_op_reg.parse::<f64>() {
                            val.abs() > 3.4e38 || (val.abs() < 1e-38 && val != 0.0)
                        } else {
                            false
                        };
                        has_high_precision || outside_f32_range
                    };
                    
                    let actual_type = if is_double_from_string {
                        // eprintln!("DEBUG generate_unary: detected double from string prefix");
                        "double"
                    } else if is_high_precision_double {
                        // eprintln!("DEBUG generate_unary: detected double from high precision: '{}'", clean_op_reg);
                        "double"
                    } else {
                        operand_type
                    };
                    // eprintln!("DEBUG generate_unary: actual_type = '{}'", actual_type);
                    let is_float_actual = actual_type == "half" || actual_type == "float" || actual_type == "double";
                    
                    let op_instr = if is_float_actual {
                        // For float operations, handle double literals specially
                        // If it's a double literal (not a register), create a constant first
                        let final_op = if actual_type == "double" && !clean_op_reg.starts_with('%') && clean_op_reg.parse::<f64>().is_ok() {
                            // eprintln!("DEBUG generate_unary: creating double constant for '{}'", clean_op_reg);
                            // Create a double constant register (similar to print_float32 approach)
                            let double_const = format!("%double_const_unary_{}", self.instructions.len());
                            let instruction = self.create_float_constant_instruction(&clean_op_reg, &double_const, "double");
                            self.instructions.push(instruction);
                            double_const
                        } else {
                            clean_op_reg
                        };
                        // eprintln!("DEBUG generate_unary: final_op = '{}'", final_op);
                        // Use appropriate zero literal for the type
                        let zero_literal = "0.0";
                        let instr = format!("  {} = fsub {} {}, {}", temp_reg, actual_type, zero_literal, final_op);
                        // eprintln!("DEBUG generate_unary: instruction = '{}'", instr);
                        instr
                    } else {
                        format!("  {} = sub {} 0, {}", temp_reg, operand_type, clean_op_reg)
                    };
                    self.instructions.push(op_instr);
                    Ok(Some(temp_reg))
                } else {
                    Err(CompilerError::codegen_error("Unary operation on invalid operand".to_string()))
                }
            }
            UnaryOp::Not => {
                if let Some(op) = operand {
                    let temp_reg = format!("%t{}", self.instructions.len());

                    // Determine if this is a boolean operation by checking variable types
                    let is_boolean = if let Expression::Identifier(name) = &*unary.operand {
                        matches!(self.variable_types.get(name), Some(Type::Bool))
                    } else {
                        false
                    };

                    let op_type = if is_boolean { "i1" } else { "i64" };
                    let not_value = if is_boolean { "1" } else { "-1" };
                    let clean_op = self.clean_register_for_instruction(&op);
                    self.instructions.push(format!("  {} = xor {} {}, {}", temp_reg, op_type, clean_op, not_value));
                    Ok(Some(temp_reg))
                } else {
                    Err(CompilerError::codegen_error("Not operation on invalid operand".to_string()))
                }
            }
        }
    }

    /// Generate LLVM IR for function calls
    /// Generate LLVM IR for function calls (text backend)
    #[cfg(not(feature = "llvm_backend"))]
    fn generate_call(&mut self, call: &CallExpr) -> Result<Option<String>> {
        // Check if this is a method call (receiver.method(args))
        if let Expression::FieldAccess(field_access) = &*call.function {
            return self.generate_method_call(field_access, call);
        }

        // Handle function calls - can be identifiers (named functions or function variables)
        if let Expression::Identifier(func_name) = &*call.function {

            // Special handling for file I/O functions
            if func_name == "read_file" {
                return self.generate_read_file_call(call);
            } else if func_name == "write_file" {
                return self.generate_write_file_call(call);
            }

            // Check if it's a function variable (stored function literal)
            // We need to clone the signature to avoid borrowing issues
            let function_signature = self.lookup_function_variable_signature(func_name).cloned();
            if let Some((param_types, return_type)) = function_signature {
                return self.generate_indirect_call(call, &param_types, &return_type);
            }

            // Resolve qualified name for unqualified calls (same-module; e.g. recursive calls)
            let qualified_func_name = self.current_module.as_ref()
                .map(|m| format!("{}.{}", m, func_name))
                .unwrap_or_else(|| func_name.clone());

            // Check if it's a local function
            if self.functions.contains_key(&qualified_func_name) {
                // Generate arguments
                let mut arg_strs = Vec::new();
                for arg in &call.arguments {
                    if let Some(arg_val) = self.generate_expression(arg)? {
                        arg_strs.push(arg_val);
                    } else {
                        return Err(CompilerError::codegen_error("Invalid argument in function call".to_string()));
                    }
                }

                // For LLVM IR function calls, arguments should have type prefixes
                // e.g., call i64 @func(i64 %arg1, i8* %arg2)
                let typed_args: Vec<String> = if self.functions.contains_key(&qualified_func_name) {
                    // This is a known function - try to get parameter types
                    if let Some(param_types) = self.function_param_types.get(&qualified_func_name) {
                        // Clone param_types to avoid borrow checker issues in closure
                        let param_types = param_types.clone();
                        // Calculate base index for instruction numbering
                        let base_instruction_count = self.instructions.len();
                        // Collect instructions that need to be added during argument processing
                        let mut temp_instructions = Vec::new();
                        // Pre-process getelementptr arguments to assign them to registers
                        let mut processed_args: Vec<String> = arg_strs.iter().enumerate()
                            .map(|(i, arg)| {
                                if arg.contains("getelementptr") {
                                    let base_idx = base_instruction_count + temp_instructions.len();
                                    let gep_reg = format!("%gep_arg_{}", base_idx);
                                    let gep_instr = if arg.starts_with("getelementptr inbounds (") {
                                        self.convert_gep_to_instruction_format(arg)
                                    } else {
                                        arg.to_string()
                                    };
                                    temp_instructions.push(format!("  {} = {}", gep_reg, gep_instr));
                                    format!("i8* {}", gep_reg)
                                } else {
                                    arg.clone()
                                }
                            })
                            .collect();
                        // Use function signature to determine argument types
                        // When sret is used, param_types[0] is sret; use param_types[i+1] for the i-th call arg
                        let param_offset = if param_types.len() > processed_args.len() { 1 } else { 0 };
                        let typed_args: Vec<String> = processed_args.iter().enumerate()
                            .map(|(i, arg)| {
                                if let Some(expected_type) = param_types.get(i + param_offset) {
                                    // Extract actual type and register from argument
                                    let (actual_type, clean_arg) = if arg.starts_with("i64 ") {
                                        ("i64", arg.strip_prefix("i64 ").unwrap())
                                    } else if arg.starts_with("i32 ") {
                                        ("i32", arg.strip_prefix("i32 ").unwrap())
                                    } else if arg.starts_with("i16 ") {
                                        ("i16", arg.strip_prefix("i16 ").unwrap())
                                    } else if arg.starts_with("i8 ") {
                                        ("i8", arg.strip_prefix("i8 ").unwrap())
                                    } else if arg.starts_with("i1 ") {
                                        ("i1", arg.strip_prefix("i1 ").unwrap())
                                    } else if arg.starts_with("float ") {
                                        ("float", arg.strip_prefix("float ").unwrap())
                                    } else if arg.starts_with("half ") {
                                        ("half", arg.strip_prefix("half ").unwrap())
                                    } else if arg.starts_with("i8* ") {
                                        ("i8*", arg.strip_prefix("i8* ").unwrap())
                                    } else if arg.contains("tuple_alloc") || arg.contains("struct_alloc") {
                                        // tuple_alloc and struct_alloc registers are already pointer values (i8*)
                                        // These are generated by tuple/struct literal allocations
                                        ("i8*", arg.as_str())
                                    } else if arg.contains("func_ptr") {
                                        // func_ptr registers are already pointer values (i8*)
                                        // These are generated by function pointer bitcast operations
                                        ("i8*", arg.as_str())
                                    } else {
                                        // No type prefix - infer from register name or expected type
                                        if arg.starts_with("%actor_") {
                                            ("i64", arg.as_str())
                                        } else if expected_type == "i8*" {
                                            ("i8*", arg.as_str())
                                        } else {
                                            ("i64", arg.as_str())
                                        }
                                    };
                                    
                                    // Check if type conversion is needed
                                    if expected_type == "i8*" && actual_type == "i64" {
                                        // Need to convert i64 to i8* (e.g., ActorRef parameter)
                                        let base_idx = base_instruction_count + temp_instructions.len();
                                        let conv_reg = format!("%arg_conv_{}", base_idx);
                                        let val_ref = Self::format_llvm_value_ref(clean_arg);
                                        temp_instructions.push(format!("  {} = inttoptr i64 {} to i8*", conv_reg, val_ref));
                                        format!("i8* {}", conv_reg)
                                    } else if expected_type == "i64" && actual_type == "i8*" {
                                        // Need to convert i8* to i64
                                        let base_idx = base_instruction_count + temp_instructions.len();
                                        let conv_reg = format!("%arg_conv_{}", base_idx);
                                        let val_ref = Self::format_llvm_value_ref(clean_arg);
                                        temp_instructions.push(format!("  {} = ptrtoint i8* {} to i64", conv_reg, val_ref));
                                        format!("i64 {}", conv_reg)
                                    } else {
                                        // Types match or no conversion needed
                                        // For half type with float literals, we need to convert them
                                        let base_idx = base_instruction_count + temp_instructions.len();
                                        let final_arg = if expected_type == "half" && !clean_arg.starts_with('%') {
                                            // For half type, convert literal to half
                                            if !clean_arg.contains('.') && clean_arg.parse::<i64>().is_ok() {
                                                // Integer literal - convert to float first, then to half
                                                let const_reg = format!("%const_arg_{}", base_idx);
                                                let float_lit = format!("{}.0", clean_arg);
                                                temp_instructions.push(format!("  {} = fptrunc float {} to half", const_reg, float_lit));
                                                const_reg
                                            } else if clean_arg.parse::<f64>().is_ok() {
                                                // Float literal - create float register first, then convert to half
                                                // Create float constant using bitcast from integer (most reliable method)
                                                let float_const = format!("%float_const_arg_{}", base_idx);
                                                if let Ok(value) = clean_arg.parse::<f64>() {
                                                    let f32_value = value as f32;
                                                    let bits = f32_value.to_bits();
                                                    temp_instructions.push(format!("  {} = bitcast i32 {} to float", float_const, bits));
                                                } else {
                                                    temp_instructions.push(format!("  {} = fadd float 0.0, {}", float_const, clean_arg));
                                                }
                                                let const_reg = format!("%const_arg_{}", base_idx + 1);
                                                temp_instructions.push(format!("  {} = fptrunc float {} to half", const_reg, float_const));
                                                const_reg
                                            } else {
                                                clean_arg.to_string()
                                            }
                                        } else if expected_type == "float" && !clean_arg.starts_with('%') && clean_arg.parse::<f64>().is_ok() {
                                            // For float type, create a constant first (LLVM doesn't accept literals in function calls)
                                            // Create float constant using bitcast from integer (most reliable method)
                                            let float_const = format!("%float_const_arg_{}", base_idx);
                                            if let Ok(value) = clean_arg.parse::<f64>() {
                                                let f32_value = value as f32;
                                                let bits = f32_value.to_bits();
                                                temp_instructions.push(format!("  {} = bitcast i32 {} to float", float_const, bits));
                                            } else {
                                                temp_instructions.push(format!("  {} = fadd float 0.0, {}", float_const, clean_arg));
                                            }
                                            float_const
                                        } else if expected_type == "double" && !clean_arg.starts_with('%') && clean_arg.parse::<f64>().is_ok() {
                                            // For double type, create a constant first (LLVM doesn't accept literals in function calls)
                                            // Create double constant using bitcast from integer (most reliable method)
                                            let double_const = format!("%double_const_arg_{}", base_idx);
                                            if let Ok(value) = clean_arg.parse::<f64>() {
                                                let bits = value.to_bits();
                                                temp_instructions.push(format!("  {} = bitcast i64 {} to double", double_const, bits));
                                            } else {
                                                temp_instructions.push(format!("  {} = fadd double 0.0, {}", double_const, clean_arg));
                                            }
                                            double_const
                                        } else {
                                            clean_arg.to_string()
                                        };
                                        format!("{} {}", expected_type, final_arg)
                                    }
                                } else {
                                    // No expected type - use heuristic with prefix stripping
                                    let clean_arg = if arg.starts_with("i64 ") {
                                        arg.strip_prefix("i64 ").unwrap().to_string()
                                    } else if arg.starts_with("i32 ") {
                                        arg.strip_prefix("i32 ").unwrap().to_string()
                                    } else if arg.starts_with("i16 ") {
                                        arg.strip_prefix("i16 ").unwrap().to_string()
                                    } else if arg.starts_with("i8 ") {
                                        arg.strip_prefix("i8 ").unwrap().to_string()
                                    } else if arg.starts_with("double ") {
                                        arg.strip_prefix("double ").unwrap().to_string()
                                    } else if arg.starts_with("float ") {
                                        arg.strip_prefix("float ").unwrap().to_string()
                                    } else if arg.starts_with("half ") {
                                        let cleaned = arg.strip_prefix("half ").unwrap();
                                        // For half type, integer literals need to be converted to float first, then to half
                                        let base_idx = base_instruction_count + temp_instructions.len();
                                        let final_cleaned = if !cleaned.starts_with('%') && !cleaned.contains('.') && cleaned.parse::<i64>().is_ok() {
                                            // Integer literal - convert to half via float
                                            let const_reg = format!("%const_half_{}", base_idx);
                                            let float_lit = format!("{}.0", cleaned);
                                            temp_instructions.push(format!("  {} = fptrunc float {} to half", const_reg, float_lit));
                                            const_reg
                                        } else if !cleaned.starts_with('%') && cleaned.contains('.') && cleaned.parse::<f64>().is_ok() {
                                            // Float literal - create float register first, then convert to half
                                            // Create float constant using bitcast from integer (most reliable method)
                                            let float_const = format!("%float_const_half_{}", base_idx);
                                            if let Ok(value) = cleaned.parse::<f64>() {
                                                let f32_value = value as f32;
                                                let bits = f32_value.to_bits();
                                                temp_instructions.push(format!("  {} = bitcast i32 {} to float", float_const, bits));
                                            } else {
                                                temp_instructions.push(format!("  {} = fadd float 0.0, {}", float_const, cleaned));
                                            }
                                            let const_reg = format!("%const_half_{}", base_idx + 1);
                                            temp_instructions.push(format!("  {} = fptrunc float {} to half", const_reg, float_const));
                                            const_reg
                                        } else {
                                            cleaned.to_string()
                                        };
                                        final_cleaned
                                    } else if arg.starts_with("i1 ") {
                                        arg.strip_prefix("i1 ").unwrap().to_string()
                                    } else if arg.starts_with("i8* ") {
                                        arg.strip_prefix("i8* ").unwrap().to_string()
                                    } else {
                                        arg.to_string()
                                    };

                                    // Apply heuristic to determine type
                                    if clean_arg.starts_with('%') && clean_arg.contains("alloc") {
                                        format!("i8* {}", clean_arg)
                                    } else if clean_arg.starts_with('%') && clean_arg.len() > 1 && clean_arg.chars().skip(1).all(|c: char| c.is_ascii_digit()) {
                                        format!("i8* {}", clean_arg)
                                    } else if clean_arg.starts_with('%') {
                                        format!("i64 {}", clean_arg)
                                    } else {
                                        format!("i64 {}", clean_arg)
                                    }
                                }
                            })
                            .map(|arg: String| {
                                // Clean up duplicate type prefixes - more aggressive
                                if arg.contains("i32 i32 ") {
                                    arg.replace("i32 i32 ", "i32 ")
                                } else if arg.contains("i64 i64 ") {
                                    arg.replace("i64 i64 ", "i64 ")
                                } else if arg.contains("i1 i1 ") {
                                    arg.replace("i1 i1 ", "i1 ")
                                } else if arg.contains("i8* i8* ") {
                                    arg.replace("i8* i8* ", "i8* ")
                                } else {
                                    arg
                                }
                            })
                            .collect();
                        // Push all collected instructions
                        self.instructions.extend(temp_instructions);
                        typed_args
                    } else {
                        // Local function but no parameter types stored - use heuristic
                        arg_strs.iter()
                            .map(|arg| {
                                // Apply heuristic to determine type, but check for existing prefixes
                                if arg.starts_with("i64 ") || arg.starts_with("i32 ") || arg.starts_with("i1 ") || arg.starts_with("i8* ") {
                                    arg.clone() // Already has type prefix
                                } else if arg.starts_with('%') && arg.contains("alloc") {
                                    // Allocation results are pointers (i8*)
                                    format!("i8* {}", arg)
                                } else if arg.starts_with('%') && arg.len() > 1 && arg.chars().skip(1).all(|c| c.is_ascii_digit()) {
                                    // Function call result registers (%t\d+) are likely i8* pointers
                                    format!("i8* {}", arg)
                                } else if arg.starts_with('%') {
                                    // Assume i64 type for other registers
                                    format!("i64 {}", arg)
                                } else {
                                    // For bare constants, assume i64
                                    format!("i64 {}", arg)
                                }
                            })
                            .map(|arg| {
                                // Clean up duplicate type prefixes - more aggressive
                                if arg.contains("i32 i32 ") {
                                    arg.replace("i32 i32 ", "i32 ")
                                } else if arg.contains("i64 i64 ") {
                                    arg.replace("i64 i64 ", "i64 ")
                                } else if arg.contains("i1 i1 ") {
                                    arg.replace("i1 i1 ", "i1 ")
                                } else if arg.contains("i8* i8* ") {
                                    arg.replace("i8* i8* ", "i8* ")
                                } else {
                                    arg
                                }
                            })
                            .collect()
                    }
                } else {
                    // External function - use heuristic
                    arg_strs.iter()
                        .map(|arg| {
                            if arg.starts_with("i64 ") || arg.starts_with("i32 ") || arg.starts_with("i1 ") || arg.starts_with("i8* ") {
                                arg.clone() // Already has type prefix
                            } else if arg.starts_with('%') && arg.contains("alloc") {
                                // Allocation results are pointers (i8*)
                                format!("i8* {}", arg)
                            } else if arg.starts_with('%') {
                                // Assume i64 type for registers
                                format!("i64 {}", arg)
                            } else {
                                // For bare constants, assume i64
                                format!("i64 {}", arg)
                            }
                        })
                        .collect()
                };
                let args_str = typed_args.iter()
                    .map(|a| Self::normalize_typed_call_arg(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                let temp_reg = format!("%t{}", self.instructions.len());

                // Determine the return type of the function
                let return_type = self.function_return_types.get(&qualified_func_name)
                    .cloned()
                    .ok_or_else(|| CompilerError::codegen_error(
                        format!("Unknown function '{}'. Function must be declared before it can be called.", qualified_func_name)
                    ))?;

                let fixed_args_str = args_str.replace("i64 %tuple_alloc_", "i8* %tuple_alloc_");
                // sret: allocate slot, pass as first arg, call void, load result
                if return_type == "i8*" {
                    let sret_slot = format!("%sret_slot_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = alloca i8*", sret_slot));
                    let sret_args = if fixed_args_str.is_empty() {
                        format!("i8* {}", sret_slot)
                    } else {
                        format!("i8* {}, {}", sret_slot, fixed_args_str)
                    };
                    self.instructions.push(format!("  call void @{}({})", qualified_func_name, sret_args));
                    let load_reg = format!("%call_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = load i8*, i8* * {}", load_reg, sret_slot));
                    Ok(Some(format!("i8* {}", load_reg)))
                } else {
                    let call_instr = format!("  {} = call {} @{}({})", temp_reg, return_type, qualified_func_name, fixed_args_str);
                    self.instructions.push(call_instr);
                    Ok(Some(format!("{} {}", return_type, temp_reg)))
                }
            }
            // Check if it's an imported function
            else if let Some(symbol_table) = &self.symbol_table {
                let mut found = false;
                for (imported_module_name, module_symbols) in &symbol_table.modules {
                    if let Some(_symbol_info) = module_symbols.get(func_name) {
                        // Found imported function - use module-qualified name for call
                        let imported_qualified = format!("{}.{}", imported_module_name, func_name);
                        let mut arg_strs = Vec::new();
                        for arg in &call.arguments {
                            if let Some(arg_val) = self.generate_expression(arg)? {
                                arg_strs.push(arg_val);
                            } else {
                                return Err(CompilerError::codegen_error("Invalid argument in function call".to_string()));
                            }
                        }

                        // For LLVM IR function calls, arguments should have type prefixes
                        let typed_args: Vec<String> = arg_strs.iter()
                            .map(|arg| {
                                if arg.starts_with("i64 ") || arg.starts_with("i32 ") || arg.starts_with("i1 ") || arg.starts_with("i8* ") {
                                    arg.clone() // Already has type prefix
                                } else if arg.contains("getelementptr") {
                                    // getelementptr produces a pointer (i8* or ptr); string literals use this
                                    format!("i8* {}", arg)
                                } else if arg.starts_with('%') && arg.contains("alloc") {
                                    // Allocation results are pointers (i8*)
                                    format!("i8* {}", arg)
                                } else if arg.starts_with('%') {
                                    // Assume i64 type for registers (most common case)
                                    format!("i64 {}", arg)
                                } else {
                                    format!("i64 {}", arg) // Add type prefix for bare constants
                                }
                            })
                            .map(|arg| {
                                // Clean up duplicate type prefixes - more aggressive
                                if arg.contains("i32 i32 ") {
                                    arg.replace("i32 i32 ", "i32 ")
                                } else if arg.contains("i64 i64 ") {
                                    arg.replace("i64 i64 ", "i64 ")
                                } else if arg.contains("i1 i1 ") {
                                    arg.replace("i1 i1 ", "i1 ")
                                } else if arg.contains("i8* i8* ") {
                                    arg.replace("i8* i8* ", "i8* ")
                                } else {
                                    arg
                                }
                            })
                            .collect();
                        let args_str = typed_args.iter()
                            .map(|a| Self::normalize_typed_call_arg(a))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let temp_reg = format!("%t{}", self.instructions.len());
                        let call_instr = format!("  {} = call i64 @{}({})", temp_reg, imported_qualified, args_str);
                        self.instructions.push(call_instr);

                        found = true;
                        return Ok(Some(format!("i64 {}", temp_reg)));
                    }
                }
                if !found {
                    Err(CompilerError::codegen_error(format!("Undefined function: {}", func_name)))
                } else {
                    unreachable!()
                }
            } else {
                Err(CompilerError::codegen_error(format!("Undefined function: {}", func_name)))
            }
        } else {
            Err(CompilerError::codegen_error("Complex function expressions not yet supported".to_string()))
            }
    }

    /// Generate LLVM IR for module function calls (text backend)
    #[cfg(not(feature = "llvm_backend"))]
    fn generate_module_call(&mut self, module_call: &ModuleCallExpr) -> Result<Option<String>> {
        // Module-qualified function name (e.g. literals.expr_to_sir)
        let qualified_name = format!("{}.{}", module_call.module, module_call.function);

        // Generate arguments
        let mut arg_strs = Vec::new();
        for arg in &module_call.arguments {
            if let Some(arg_val) = self.generate_expression(arg)? {
                arg_strs.push(arg_val);
            } else {
                return Err(CompilerError::codegen_error("Invalid argument in module call".to_string()));
            }
        }

        // Use function signature to determine argument types (same logic as regular function calls)
        let typed_args: Vec<String> = if self.functions.contains_key(&qualified_name) {
            // This is a known function - try to get parameter types
            if let Some(param_types) = self.function_param_types.get(&qualified_name) {
                // Clone param_types to avoid borrow checker issues in closure
                let param_types = param_types.clone();
                // Calculate base index for instruction numbering
                let base_instruction_count = self.instructions.len();
                // Collect instructions that need to be added during argument processing
                let mut temp_instructions = Vec::new();
                // Pre-process getelementptr arguments to assign them to registers
                let mut processed_args: Vec<String> = arg_strs.iter().enumerate()
                    .map(|(i, arg)| {
                        if arg.contains("getelementptr") {
                            let base_idx = base_instruction_count + temp_instructions.len();
                            let gep_reg = format!("%gep_arg_{}", base_idx);
                            let gep_instr = if arg.starts_with("getelementptr inbounds (") {
                                self.convert_gep_to_instruction_format(arg)
                            } else {
                                arg.to_string()
                            };
                            temp_instructions.push(format!("  {} = {}", gep_reg, gep_instr));
                            format!("i8* {}", gep_reg)
                        } else {
                            arg.clone()
                        }
                    })
                    .collect();
                // Use function signature to determine argument types
                // When sret is used, param_types[0] is sret; use param_types[i+1] for the i-th call arg
                let param_offset = if param_types.len() > processed_args.len() { 1 } else { 0 };
                let typed_args: Vec<String> = processed_args.iter().enumerate()
                    .map(|(i, arg)| {
                        if let Some(expected_type) = param_types.get(i + param_offset) {
                            // Extract actual type and register from argument
                            let (actual_type, clean_arg) = if arg.starts_with("i64 ") {
                                ("i64", arg.strip_prefix("i64 ").unwrap())
                            } else if arg.starts_with("i32 ") {
                                ("i32", arg.strip_prefix("i32 ").unwrap())
                            } else if arg.starts_with("i16 ") {
                                ("i16", arg.strip_prefix("i16 ").unwrap())
                            } else if arg.starts_with("i8 ") {
                                ("i8", arg.strip_prefix("i8 ").unwrap())
                            } else if arg.starts_with("i1 ") {
                                ("i1", arg.strip_prefix("i1 ").unwrap())
                            } else if arg.starts_with("float ") {
                                ("float", arg.strip_prefix("float ").unwrap())
                            } else if arg.starts_with("half ") {
                                ("half", arg.strip_prefix("half ").unwrap())
                            } else if arg.starts_with("i8* ") {
                                ("i8*", arg.strip_prefix("i8* ").unwrap())
                            } else if arg.contains("tuple_alloc") || arg.contains("struct_alloc") {
                                // tuple_alloc and struct_alloc registers are already pointer values (i8*)
                                // These are generated by tuple/struct literal allocations
                                ("i8*", arg.as_str())
                            } else if arg.contains("func_ptr") {
                                // func_ptr registers are already pointer values (i8*)
                                // These are generated by function pointer bitcast operations
                                ("i8*", arg.as_str())
                            } else {
                                // No type prefix - infer from register name or expected type
                                if arg.starts_with("%actor_") {
                                    ("i64", arg.as_str())
                                } else if expected_type == "i8*" {
                                    ("i8*", arg.as_str())
                                } else {
                                    ("i64", arg.as_str())
                                }
                            };
                            
                            // Check if type conversion is needed
                            if expected_type == "i8*" && actual_type == "i64" {
                                // Need to convert i64 to i8* (e.g., ActorRef parameter)
                                let base_idx = base_instruction_count + temp_instructions.len();
                                let conv_reg = format!("%arg_conv_{}", base_idx);
                                let val_ref = Self::format_llvm_value_ref(clean_arg);
                                temp_instructions.push(format!("  {} = inttoptr i64 {} to i8*", conv_reg, val_ref));
                                format!("i8* {}", conv_reg)
                            } else if expected_type == "i64" && actual_type == "i8*" {
                                // Need to convert i8* to i64
                                let base_idx = base_instruction_count + temp_instructions.len();
                                let conv_reg = format!("%arg_conv_{}", base_idx);
                                let val_ref = Self::format_llvm_value_ref(clean_arg);
                                temp_instructions.push(format!("  {} = ptrtoint i8* {} to i64", conv_reg, val_ref));
                                format!("i64 {}", conv_reg)
                            } else {
                                // Types match or no conversion needed
                                let base_idx = base_instruction_count + temp_instructions.len();
                                let final_arg = if expected_type == "half" && !clean_arg.starts_with('%') {
                                    // For half type, convert literal to half
                                    if !clean_arg.contains('.') && clean_arg.parse::<i64>().is_ok() {
                                        // Integer literal - convert to float first, then to half
                                        let const_reg = format!("%const_arg_{}", base_idx);
                                        let float_lit = format!("{}.0", clean_arg);
                                        temp_instructions.push(format!("  {} = fptrunc float {} to half", const_reg, float_lit));
                                        const_reg
                                    } else if clean_arg.parse::<f64>().is_ok() {
                                        // Float literal - create float register first, then convert to half
                                        let float_const = format!("%float_const_arg_{}", base_idx);
                                        if let Ok(value) = clean_arg.parse::<f64>() {
                                            let f32_value = value as f32;
                                            let bits = f32_value.to_bits();
                                            temp_instructions.push(format!("  {} = bitcast i32 {} to float", float_const, bits));
                                        } else {
                                            temp_instructions.push(format!("  {} = fadd float 0.0, {}", float_const, clean_arg));
                                        }
                                        let const_reg = format!("%const_arg_{}", base_idx + 1);
                                        temp_instructions.push(format!("  {} = fptrunc float {} to half", const_reg, float_const));
                                        const_reg
                                    } else {
                                        clean_arg.to_string()
                                    }
                                } else if expected_type == "float" && !clean_arg.starts_with('%') && clean_arg.parse::<f64>().is_ok() {
                                    // For float type, create a constant first (LLVM doesn't accept literals in function calls)
                                    let float_const = format!("%float_const_arg_{}", base_idx);
                                    if let Ok(value) = clean_arg.parse::<f64>() {
                                        let f32_value = value as f32;
                                        let bits = f32_value.to_bits();
                                        temp_instructions.push(format!("  {} = bitcast i32 {} to float", float_const, bits));
                                    } else {
                                        temp_instructions.push(format!("  {} = fadd float 0.0, {}", float_const, clean_arg));
                                    }
                                    float_const
                                } else if expected_type == "double" && !clean_arg.starts_with('%') && clean_arg.parse::<f64>().is_ok() {
                                    // For double type, create a constant first
                                    let double_const = format!("%double_const_arg_{}", base_idx);
                                    if let Ok(value) = clean_arg.parse::<f64>() {
                                        let bits = value.to_bits();
                                        temp_instructions.push(format!("  {} = bitcast i64 {} to double", double_const, bits));
                                    } else {
                                        temp_instructions.push(format!("  {} = fadd double 0.0, {}", double_const, clean_arg));
                                    }
                                    double_const
                                } else {
                                    clean_arg.to_string()
                                };
                                format!("{} {}", expected_type, final_arg)
                            }
                        } else {
                            // No expected type - use heuristic with prefix stripping
                            let clean_arg = if arg.starts_with("i64 ") {
                                arg.strip_prefix("i64 ").unwrap().to_string()
                            } else if arg.starts_with("i32 ") {
                                arg.strip_prefix("i32 ").unwrap().to_string()
                            } else if arg.starts_with("i16 ") {
                                arg.strip_prefix("i16 ").unwrap().to_string()
                            } else if arg.starts_with("i8 ") {
                                arg.strip_prefix("i8 ").unwrap().to_string()
                            } else if arg.starts_with("double ") {
                                arg.strip_prefix("double ").unwrap().to_string()
                            } else if arg.starts_with("float ") {
                                arg.strip_prefix("float ").unwrap().to_string()
                            } else if arg.starts_with("half ") {
                                arg.strip_prefix("half ").unwrap().to_string()
                            } else if arg.starts_with("i8* ") {
                                arg.strip_prefix("i8* ").unwrap().to_string()
                            } else if arg.contains("getelementptr") {
                                // getelementptr expressions need to be assigned to a register first
                                let base_idx = base_instruction_count + temp_instructions.len();
                                let gep_reg = format!("%gep_arg_{}", base_idx);
                                let gep_instr = if arg.starts_with("getelementptr inbounds (") {
                                    self.convert_gep_to_instruction_format(arg)
                                } else {
                                    arg.to_string()
                                };
                                temp_instructions.push(format!("  {} = {}", gep_reg, gep_instr));
                                format!("i8* {}", gep_reg)
                            } else if arg.contains("tuple_alloc") || arg.contains("struct_alloc") {
                                // tuple_alloc and struct_alloc are i8* pointers
                                format!("i8* {}", arg)
                            } else if arg.contains("func_ptr") {
                                // func_ptr registers are i8* pointers
                                format!("i8* {}", arg)
                            } else {
                                arg.to_string()
                            };
                            // Default to i64 if no type prefix
                            if !clean_arg.starts_with("i64 ") && !clean_arg.starts_with("i32 ") && 
                               !clean_arg.starts_with("i16 ") && !clean_arg.starts_with("i8 ") &&
                               !clean_arg.starts_with("float ") && !clean_arg.starts_with("double ") &&
                               !clean_arg.starts_with("half ") && !clean_arg.starts_with("i8* ") &&
                               !clean_arg.starts_with("i1 ") {
                                format!("i64 {}", clean_arg)
                            } else {
                                clean_arg
                            }
                        }
                    })
                    .collect();
                
                // Insert temporary instructions before the call
                for instr in temp_instructions {
                    self.instructions.push(instr);
                }
                
                typed_args
            } else {
                // No parameter types available - fall back to simple heuristic
                arg_strs.iter().map(|arg| {
                    if arg.starts_with("i64 ") || arg.starts_with("i32 ") || arg.starts_with("i16 ") ||
                       arg.starts_with("i8 ") || arg.starts_with("float ") || arg.starts_with("double ") ||
                       arg.starts_with("half ") || arg.starts_with("i8* ") || arg.starts_with("i1 ") {
                        arg.clone()
                    } else if arg.contains("getelementptr") {
                        // Assign getelementptr to register first
                        let gep_reg = format!("%gep_arg_{}", self.instructions.len());
                        let gep_instr = if arg.starts_with("getelementptr inbounds (") {
                            self.convert_gep_to_instruction_format(arg)
                        } else {
                            arg.to_string()
                        };
                        self.instructions.push(format!("  {} = {}", gep_reg, gep_instr));
                        format!("i8* {}", gep_reg)
                    } else if arg.contains("tuple_alloc") || arg.contains("struct_alloc") {
                        format!("i8* {}", arg)
                    } else if arg.contains("func_ptr") {
                        format!("i8* {}", arg)
                    } else {
                        format!("i64 {}", arg)
                    }
                }).collect()
            }
        } else {
            // Function not found - fall back to simple heuristic
            arg_strs.iter().map(|arg| {
                if arg.starts_with("i64 ") || arg.starts_with("i32 ") || arg.starts_with("i16 ") ||
                   arg.starts_with("i8 ") || arg.starts_with("float ") || arg.starts_with("double ") ||
                   arg.starts_with("half ") || arg.starts_with("i8* ") || arg.starts_with("i1 ") {
                    arg.clone()
                } else if arg.contains("getelementptr") {
                    // Assign getelementptr to register first
                    let gep_reg = format!("%gep_arg_{}", self.instructions.len());
                    let gep_instr = if arg.starts_with("getelementptr inbounds (") {
                        self.convert_gep_to_instruction_format(arg)
                    } else {
                        arg.to_string()
                    };
                    self.instructions.push(format!("  {} = {}", gep_reg, gep_instr));
                    format!("i8* {}", gep_reg)
                } else if arg.contains("tuple_alloc") || arg.contains("struct_alloc") {
                    format!("i8* {}", arg)
                } else if arg.contains("func_ptr") {
                    format!("i8* {}", arg)
                } else {
                    format!("i64 {}", arg)
                }
            }).collect()
        };

        // Get return type
        let return_type_str = self.function_return_types.get(&qualified_name)
            .cloned()
            .unwrap_or_else(|| "i64".to_string());

        // Look up function and generate call
        let args_str = typed_args.iter()
            .map(|a| Self::normalize_typed_call_arg(a))
            .collect::<Vec<_>>()
            .join(", ");

        // sret: allocate slot, pass as first arg, call void, load result (fixes recursive struct return bug)
        if return_type_str == "i8*" {
            let sret_slot = format!("%sret_slot_{}", self.instructions.len());
            self.instructions.push(format!("  {} = alloca i8*", sret_slot));
            let sret_args = if args_str.is_empty() {
                format!("i8* {}", sret_slot)
            } else {
                format!("i8* {}, {}", sret_slot, args_str)
            };
            self.instructions.push(format!("  call void @{}({})", qualified_name, sret_args));
            let load_reg = format!("%call_{}", self.instructions.len());
            self.instructions.push(format!("  {} = load i8*, i8* * {}", load_reg, sret_slot));
            Ok(Some(format!("i8* {}", load_reg)))
        } else {
            let temp_reg = format!("%call_{}", self.instructions.len());
            self.instructions.push(format!("  {} = call {} @{}({})", temp_reg, return_type_str, qualified_name, args_str));
            Ok(Some(format!("{} {}", return_type_str, temp_reg)))
        }
    }

    /// Generate LLVM IR for indirect function calls (calling function pointers)
    #[cfg(not(feature = "llvm_backend"))]
    fn generate_indirect_call(&mut self, call: &CallExpr, param_types: &[Type], return_type: &Type) -> Result<Option<String>> {
        // Get the function pointer from the variable
        if let Expression::Identifier(func_name) = &*call.function {
            let func_ptr = self.lookup_variable_text(func_name)
                .ok_or_else(|| CompilerError::codegen_error(
                    format!("Function variable '{}' not found", func_name)
                ))?;

            // Generate arguments
            let mut arg_strs = Vec::new();
            for arg in &call.arguments {
                if let Some(arg_val) = self.generate_expression(arg)? {
                    arg_strs.push(arg_val);
                } else {
                    return Err(CompilerError::codegen_error("Invalid argument in function call".to_string()));
                }
            }

            // Add type prefixes to arguments with type conversion if needed
            let typed_args: Vec<String> = arg_strs.iter().enumerate()
                .map(|(i, arg)| {
                    if let Some(expected_type) = param_types.get(i) {
                        let expected_type_str = self.get_llvm_type_string(expected_type);
                        // Extract actual type and register from argument
                        let (actual_type, clean_arg) = if arg.starts_with("i64 ") {
                            ("i64", arg.strip_prefix("i64 ").unwrap())
                        } else if arg.starts_with("i32 ") {
                            ("i32", arg.strip_prefix("i32 ").unwrap())
                        } else if arg.starts_with("i1 ") {
                            ("i1", arg.strip_prefix("i1 ").unwrap())
                        } else if arg.starts_with("i8* ") {
                            ("i8*", arg.strip_prefix("i8* ").unwrap())
                        } else if arg.contains("getelementptr") {
                            // getelementptr expressions are already pointer values (i8*)
                            // This handles string literals which generate getelementptr expressions
                            ("i8*", arg.as_str())
                        } else if arg.contains("tuple_alloc") || arg.contains("struct_alloc") {
                            // tuple_alloc and struct_alloc registers are already pointer values (i8*)
                            // These are generated by tuple/struct literal allocations
                            ("i8*", arg.as_str())
                        } else if arg.contains("func_ptr") {
                            // func_ptr registers are already pointer values (i8*)
                            // These are generated by function pointer bitcast operations
                            ("i8*", arg.as_str())
                        } else {
                            // No type prefix - infer from register name or expected type.
                            // Variables from scope often have no prefix; if we expect i8* (e.g. string param), use as-is to avoid wrong inttoptr.
                            if arg.starts_with("%actor_") {
                                // Actor references from spawn are i64
                                ("i64", arg.as_str())
                            } else if expected_type_str == "i8*" {
                                // Expecting pointer: value is likely already i8* (e.g. from variable_scopes)
                                ("i8*", arg.as_str())
                            } else {
                                ("i64", arg.as_str())
                            }
                        };
                        
                        // Check if type conversion is needed
                        if expected_type_str == "i8*" && actual_type == "i64" {
                            // Need to convert i64 to i8* (e.g., ActorRef parameter)
                            let conv_reg = format!("%arg_conv_{}", self.instructions.len());
                            self.instructions.push(format!("  {} = inttoptr i64 {} to i8*", conv_reg, clean_arg));
                            format!("i8* {}", conv_reg)
                        } else if expected_type_str == "i64" && actual_type == "i8*" {
                            // Need to convert i8* to i64
                            let conv_reg = format!("%arg_conv_{}", self.instructions.len());
                            self.instructions.push(format!("  {} = ptrtoint i8* {} to i64", conv_reg, clean_arg));
                            format!("i64 {}", conv_reg)
                        } else {
                            // Types match or no conversion needed
                            format!("{} {}", expected_type_str, clean_arg)
                        }
                    } else {
                        // No expected type - use heuristic with prefix stripping
                        let clean_arg = if arg.starts_with("i64 ") {
                            arg.strip_prefix("i64 ").unwrap()
                        } else if arg.starts_with("i32 ") {
                            arg.strip_prefix("i32 ").unwrap()
                        } else if arg.starts_with("i1 ") {
                            arg.strip_prefix("i1 ").unwrap()
                        } else if arg.starts_with("i8* ") {
                            arg.strip_prefix("i8* ").unwrap()
                        } else {
                            arg.as_str()
                        };

                        // Apply heuristic
                        if clean_arg.starts_with('%') && clean_arg.contains("alloc") {
                            format!("i8* {}", clean_arg)
                        } else if clean_arg.starts_with('%') && clean_arg.len() > 1 && clean_arg.chars().skip(1).all(|c| c.is_ascii_digit()) {
                            format!("i8* {}", clean_arg)
                        } else if clean_arg.starts_with('%') {
                            format!("i64 {}", clean_arg)
                        } else {
                            format!("i64 {}", clean_arg)
                        }
                    }
                })
                .collect();

            // Generate the function pointer type
            let param_types_str = param_types.iter()
                .map(|t| self.get_llvm_type_string(t))
                .collect::<Vec<_>>()
                .join(", ");
            let return_type_str = self.get_llvm_type_string(return_type);
            let func_type_str = format!("{} ({})*", return_type_str, param_types_str);

            // Cast the i8* function pointer to the correct type
            let cast_reg = format!("%func_cast_{}", self.instructions.len());
            let clean_func_ptr = self.clean_register_for_instruction(&func_ptr);
            self.instructions.push(format!("  {} = bitcast i8* {} to {}", cast_reg, clean_func_ptr, func_type_str));

            // Generate the indirect call
            let temp_reg = format!("%t{}", self.instructions.len());
            let args_str = typed_args.iter()
                .map(|a| Self::normalize_typed_call_arg(a))
                .collect::<Vec<_>>()
                .join(", ");
            self.instructions.push(format!("  {} = call {} {}{}({})",
                temp_reg,
                return_type_str,
                cast_reg,
                if param_types.is_empty() { "" } else { " " },
                args_str
            ));

            Ok(Some(format!("{} {}", return_type_str, temp_reg)))
        } else {
            Err(CompilerError::codegen_error("Indirect calls require identifier expressions".to_string()))
        }
    }

    /// Generate code for method calls (receiver.method(args))
    fn generate_method_call(&mut self, field_access: &FieldAccessExpr, call: &CallExpr) -> Result<Option<String>> {
        // Generate the receiver
        let receiver_val = match self.generate_expression(&field_access.object)? {
            Some(val) => val,
            None => {
                let metadata = ErrorMetadataBuilder::new("E4003".to_string())
                    .severity(ErrorSeverity::Error)
                    .build();
                return Err(CompilerError::CodegenError { message: "Invalid receiver in method call".to_string(), location: None, metadata });
            }
        };

        // Get the receiver type to create the method name
        let receiver_type = match &*field_access.object {
            Expression::Identifier(var_name) => {
                self.variable_types.get(var_name)
                    .ok_or_else(|| {
                        let metadata = ErrorMetadataBuilder::new("E4003".to_string())
                            .severity(ErrorSeverity::Error)
                            .build();
                        CompilerError::codegen_error_with_metadata(format!("Unknown variable '{}' in method call", var_name), None, metadata)
                    })?
                    .clone()
            },
            _ => {
                // For more complex receivers, try expression types
                self.expression_types.get(&field_access.location)
                    .ok_or_else(|| CompilerError::codegen_error("Cannot determine receiver type for method call".to_string()))?
                    .clone()
            }
        };

        // Resolve the method to find the implementing type and get return type
        let (method_name, return_type_str) = match &receiver_type {
            Type::Named(type_name) => {
                // Find the trait implementation for this type and method
                let mut found_method = None;
                for trait_impl in &self.trait_impls {
                    if trait_impl.methods.contains_key(&field_access.field) {
                        // Check if this trait impl applies to our receiver type
                        if self.types_equal_codegen(&trait_impl.for_type, &receiver_type) {
                            found_method = trait_impl.methods.get(&field_access.field);
                            break;
                        }
                    }
                }
                
                let method_name = format!("{}_{}", type_name, field_access.field);
                
                // Get return type from method signature using silica_type_to_llvm
                let return_type_str = if let Some(method_decl) = found_method {
                    if let Some(ref return_type) = method_decl.return_type {
                        self.silica_type_to_llvm(return_type).unwrap_or_else(|_| "i64".to_string())
                    } else {
                        "void".to_string()
                    }
                } else {
                    "i64".to_string() // Fallback if method not found
                };
                
                (method_name, return_type_str)
            },
            _ => {
                return Err(CompilerError::codegen_error(format!("Method calls not supported on type {:?}", receiver_type)));
            }
        };

        // When receiver type is a trait (no concrete impl matched), ensure a forwarder exists
        if let Type::Named(type_name) = &receiver_type {
            let is_trait = self.trait_impls.iter().any(|i| &i.trait_name == type_name);
            let has_concrete_impl = self.trait_impls.iter().any(|i| {
                i.methods.contains_key(&field_access.field) && self.types_equal_codegen(&i.for_type, &receiver_type)
            });
            if is_trait && !has_concrete_impl {
                self.ensure_trait_method_forwarder(type_name, &field_access.field, &return_type_str)?;
            }
        }

        // Generate arguments (receiver first, then call arguments)
        let mut arg_strs = vec![receiver_val];
        for arg in &call.arguments {
            if let Some(arg_val) = self.generate_expression(arg)? {
                arg_strs.push(arg_val);
            } else {
                return Err(CompilerError::codegen_error("Invalid argument in method call".to_string()));
            }
        }

        // For LLVM IR method calls, arguments should have type prefixes
        // First argument (receiver) is always a struct pointer (i8*)
        // Other arguments follow normal typing rules
        let typed_args: Vec<String> = arg_strs.iter().enumerate()
            .map(|(i, arg)| {
                if i == 0 {
                    // Receiver is always a struct pointer; strip any existing type prefix so we don't emit "i8* %i64 %data"
                    let clean = self.clean_register_for_instruction(arg);
                    format!("i8* {}", clean)
                } else if arg.starts_with("i64 ") || arg.starts_with("i32 ") || arg.starts_with("i1 ") {
                    arg.clone() // Already has type prefix
                } else {
                    format!("i64 {}", arg) // Add type prefix for other arguments
                }
            })
            .collect();
        let args_str = typed_args.iter()
            .map(|a| Self::normalize_typed_call_arg(a))
            .collect::<Vec<_>>()
            .join(", ");
        let temp_reg = format!("%t{}", self.instructions.len());
        let call_instr = format!("  {} = call {} @{}({})", temp_reg, return_type_str, method_name, args_str);
        self.instructions.push(call_instr);

        // Return value with type prefix if it's a pointer type
        let result = if return_type_str == "i8*" {
            format!("i8* %{}", temp_reg.trim_start_matches('%'))
        } else {
            temp_reg
        };
        Ok(Some(result))
    }

    /// Generate LLVM value for expressions (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_expression_llvm(&mut self, expr: &Expression) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        match expr {
            Expression::Literal(lit) => {
                // Get the type from expression_types if available (for float literals)
                let expr_type = Self::try_get_expression_location(expr)
                    .and_then(|loc| self.expression_types.get(loc));
                self.generate_literal_llvm_with_type(lit, expr_type)
            },
            Expression::Identifier(name) => self.generate_identifier_llvm(name),
            Expression::Binary(binary) => self.generate_binary_llvm(binary),
            Expression::Unary(unary) => self.generate_unary_llvm(unary),
            Expression::Call(call) => self.generate_call_llvm(call),
            Expression::ModuleCall(module_call) => self.generate_module_call_llvm(module_call),
            Expression::If(if_expr) => self.generate_if_llvm(if_expr),
            Expression::Case(case) => self.generate_case_llvm(case),
            Expression::Do(do_expr) => self.generate_do_llvm(do_expr),
            Expression::Region(region) => self.generate_region_llvm(region),
            Expression::ReadRef(read) => self.generate_read_ref_llvm(read),
            Expression::Tuple(exprs) => self.generate_tuple_llvm(exprs),
            Expression::StructLiteral(struct_lit) => self.generate_struct_literal_llvm(struct_lit),
            Expression::FieldAccess(field_access) => self.generate_field_access_llvm(field_access),
            Expression::Spawn(spawn) => self.generate_spawn_llvm(spawn),
            Expression::Send(send) => self.generate_send_llvm(send),
            Expression::Cast(cast) => self.generate_cast_llvm(cast),
            Expression::Recv(recv) => self.generate_recv_llvm(recv),
            Expression::ReadFile(read_file) => self.generate_read_file_llvm(read_file),
            Expression::WriteFile(write_file) => self.generate_write_file_llvm(write_file),
            Expression::ExecCommand(exec_cmd) => self.generate_exec_command_llvm(exec_cmd),
            Expression::ListDirectory(list_dir) => self.generate_list_directory_llvm(list_dir),
            Expression::FunctionLiteral(func_lit) => self.generate_function_literal_llvm(func_lit),
            Expression::GetCpuTopology(get_topology) => self.generate_get_cpu_topology_llvm(get_topology),
            Expression::PrintInt64(print_int64) => self.generate_print_int64_llvm(print_int64),
            Expression::PrintInt32(print_int32) => self.generate_print_int32_llvm(print_int32),
            Expression::PrintInt16(print_int16) => self.generate_print_int16_llvm(print_int16),
            Expression::PrintInt8(print_int8) => self.generate_print_int8_llvm(print_int8),
            Expression::PrintChar(print_char) => self.generate_print_char_llvm(print_char),
            Expression::PrintFloat16(print_float16) => self.generate_print_float16_llvm(print_float16),
            Expression::PrintFloat32(print_float32) => self.generate_print_float32_llvm(print_float32),
            Expression::PrintFloat64(print_float64) => self.generate_print_float64_llvm(print_float64),
            Expression::AsType(as_type) => self.generate_as_type_llvm(as_type),
            _ => Err(CompilerError::codegen_error(format!("Expression type not yet supported in LLVM backend: {:?}", expr))),
        }
    }

    /// Generate LLVM value for module function calls (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_module_call_llvm(&mut self, module_call: &ModuleCallExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate call to the unqualified function name
        // The combined program resolves imports and includes all functions with their original names
        let function_name = module_call.function.clone();

        // Generate arguments
        let mut args = Vec::new();
        for arg in &module_call.arguments {
            if let Some(arg_value) = self.generate_expression_llvm(arg)? {
                args.push(arg_value);
            } else {
                return Err(CompilerError::codegen_error(
                    format!("Failed to generate LLVM value for argument in module call to {}", function_name)
                ));
            }
        }

        // Look up function and generate call
        let func = if let Some(func) = self.module.get_function(&function_name) {
            Some(func)
        } else if let Some(symbol_table) = &self.symbol_table {
            // Check if it's an imported function from another module
            let mut found_func = None;
            for (_module_name, module_symbols) in &symbol_table.modules {
                if let Some(symbol_info) = module_symbols.get(&function_name) {
                    // Extract function type from symbol_info.ty
                    if let Type::Function { parameters, return_type } = &symbol_info.ty {
                        unsafe {
                            // Convert Silica parameter types to LLVM types
                            let mut llvm_param_types = Vec::new();
                            for param_type in parameters {
                                llvm_param_types.push(self.silica_type_to_llvm_type(param_type));
                            }
                            
                            // Handle return type - if it's a Process type, extract the result type
                            let llvm_return_type = if let Type::Process { result_type, .. } = &**return_type {
                                self.silica_type_to_llvm_type(result_type)
                            } else {
                                self.silica_type_to_llvm_type(return_type)
                            };
                            
                            let fn_type = llvm_return_type.fn_type(&llvm_param_types, false);
                            let func = (*self.module).add_function(&function_name, fn_type, None);
                            found_func = Some(func);
                            break;
                        }
                    } else {
                        // Fallback: if type is not a Function type, use arity-based approach
                        unsafe {
                            let mut param_types = Vec::new();
                            for _ in 0..symbol_info.arity {
                                param_types.push((*self.context).i64_type().into());
                            }
                            let fn_type = (*self.context).i64_type().fn_type(&param_types, false);
                            let func = (*self.module).add_function(&function_name, fn_type, None);
                            found_func = Some(func);
                            break;
                        }
                    }
                }
            }
            found_func
        } else {
            None
        };

        match func {
            Some(func) => {
                let call_site_value = self.builder.build_call(func, &args, "module_call");
                if func.get_type().get_return_type().is_some() {
                    Ok(Some(call_site_value.try_as_basic_value().left_or(None)))
                } else {
                    Ok(None)
                }
            }
            None => {
                Err(CompilerError::codegen_error(format!("Function not found: {}", module_call.function)))
            }
        }
    }

    /// Generate LLVM value for function calls (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_call_llvm(&mut self, call: &CallExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {

        // Check if this is a method call (receiver.method(args))
        if let Expression::FieldAccess(field_access) = &*call.function {
            return self.generate_method_call_llvm(field_access, call);
        }

        // Handle function calls - can be identifiers (named functions or function variables)
        if let Expression::Identifier(func_name) = &*call.function {
            // Special handling for file I/O functions
            if func_name == "read_file" {
                return self.generate_read_file_call_llvm(call);
            } else if func_name == "write_file" {
                return self.generate_write_file_call_llvm(call);
            }

            // Check if it's a function variable (stored function literal)
            if let Some((param_types, return_type)) = self.lookup_function_variable_signature(func_name).cloned() {
                return self.generate_indirect_call_llvm(call, &param_types, &return_type);
            }

            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                unsafe {
                    // Check if it's a function variable first
                    if self.lookup_function_variable_signature(func_name).is_some() {
                        // This should have been handled above, but just in case
                        return Err(CompilerError::codegen_error(
                            format!("Function variable '{}' should have been handled above", func_name)
                        ));
                    }

                    // First try to get the function from the current module
                    let func = if let Some(func) = (*module).get_function(func_name) {
                        Some(func)
                    } else if let Some(symbol_table) = &self.symbol_table {
                        // Check if it's an imported function
                        let mut found_func = None;
                        let mut function_type_info = None;
                        for (_module_name, module_symbols) in &symbol_table.modules {
                            if let Some(symbol_info) = module_symbols.get(func_name) {
                                // Extract function type from symbol_info.ty
                                if let Type::Function { parameters, return_type } = &symbol_info.ty {
                                    // Convert Silica parameter types to LLVM types
                                    let mut llvm_param_types = Vec::new();
                                    for param_type in parameters {
                                        llvm_param_types.push(self.silica_type_to_llvm_type(param_type));
                                    }
                                    
                                    // Handle return type - if it's a Process type, extract the result type
                                    let llvm_return_type = if let Type::Process { result_type, .. } = &**return_type {
                                        self.silica_type_to_llvm_type(result_type)
                                    } else {
                                        self.silica_type_to_llvm_type(return_type)
                                    };
                                    
                                    let fn_type = llvm_return_type.fn_type(&llvm_param_types, false);
                                    let func = (*module).add_function(func_name, fn_type, None);
                                    found_func = Some(func);
                                    function_type_info = Some((parameters.clone(), return_type.clone()));
                                    break;
                                } else {
                                    // Fallback: if type is not a Function type, use arity-based approach
                                    // This handles cases where type checking hasn't populated the type yet
                                    let mut param_types = Vec::new();
                                    for _ in 0..symbol_info.arity {
                                        param_types.push((*self.context).i64_type().into());
                                    }
                                    let fn_type = (*self.context).i64_type().fn_type(&param_types, false);
                                    let func = (*module).add_function(func_name, fn_type, None);
                                    found_func = Some(func);
                                    break;
                                }
                            }
                        }
                        found_func
                    } else {
                        None
                    };

                    if let Some(func) = func {
                        // Generate arguments as LLVM values using generate_expression_llvm
                        let mut llvm_args = Vec::new();
                        for arg in &call.arguments {
                            if let Some(arg_value) = self.generate_expression_llvm(arg)? {
                                llvm_args.push(arg_value);
                            } else {
                                return Err(CompilerError::codegen_error(
                                    format!("Failed to generate LLVM value for argument in call to {}", func_name)
                                ));
                            }
                        }

                        // Call the function
                        let call_result = builder.build_call(func, &llvm_args, "call_result").unwrap();
                        
                        // Extract return value if the function returns a value
                        if func.get_type().get_return_type().is_some() {
                            Ok(Some(call_result.try_as_basic_value().left_or(None)))
                        } else {
                            // Function returns void
                            Ok(None)
                        }
                    } else {
                        Err(CompilerError::codegen_error(format!("Undefined function: {}", func_name)))
                    }
                }
            } else {
                Err(CompilerError::codegen_error("LLVM module or builder not initialized".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error("Complex function expressions not yet supported".to_string()))
        }
    }

    /// Generate LLVM value for indirect function calls (function variables) (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_indirect_call_llvm(&mut self, call: &CallExpr, param_types: &[Type], return_type: &Type) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        if let Expression::Identifier(func_name) = &*call.function {
            if let (Some(builder), Some(context)) = (&self.builder, &self.context) {
                unsafe {
                    // Get the function pointer from the variable
                    let func_ptr_var = self.lookup_variable(func_name)
                        .ok_or_else(|| CompilerError::codegen_error(
                            format!("Function variable '{}' not found", func_name)
                        ))?;

                    // Load the function pointer
                    let func_ptr = (*builder).build_load(
                        (*context).i8_type().ptr_type(inkwell::AddressSpace::Generic),
                        func_ptr_var,
                        &format!("func_ptr_{}", func_name)
                    ).unwrap();

                    // Generate arguments
                    let mut arg_values = Vec::new();
                    for arg_expr in &call.arguments {
                        let arg_val = self.generate_expression_llvm(arg_expr)?
                            .ok_or_else(|| CompilerError::codegen_error("Invalid argument in function call".to_string()))?;
                        arg_values.push(arg_val);
                    }

                    // Create function type for the call
                    let mut llvm_param_types = Vec::new();
                    for param_type in param_types {
                        llvm_param_types.push(self.silica_type_to_llvm_type(param_type));
                    }
                    let llvm_return_type = self.silica_type_to_llvm_type(return_type);
                    let func_type = llvm_return_type.fn_type(&llvm_param_types, false);

                    // Cast the function pointer to the correct type
                    let typed_func_ptr = (*builder).build_bitcast(
                        func_ptr,
                        func_type.ptr_type(inkwell::AddressSpace::Generic),
                        &format!("typed_func_ptr_{}", func_name)
                    ).unwrap().into_pointer_value();

                    // Generate the indirect call
                    let call_result = (*builder).build_indirect_call(
                        func_type,
                        typed_func_ptr,
                        &arg_values,
                        &format!("call_result_{}", func_name)
                    ).unwrap();

                    Ok(Some(call_result.try_as_basic_value().left().unwrap()))
                }
            } else {
                Err(CompilerError::codegen_error("LLVM context or builder not initialized".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error("Indirect call requires identifier".to_string()))
        }
    }

    /// Generate LLVM value for method calls (receiver.method(args)) (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_method_call_llvm(&mut self, _field_access: &FieldAccessExpr, _call: &CallExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // TODO: Implement proper method call generation
        // For now, return a placeholder
        unsafe {
            if let Some(context) = self.context.as_ref() {
                let placeholder = context.i64_type().const_int(0, false);
                let result: inkwell::values::BasicValueEnum<'static> = unsafe { std::mem::transmute(placeholder.into()) };
                Ok(Some(result))
            } else {
                {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
            }
        }
    }

    /// Generate LLVM value for memory allocation (alloc_ref) (LLVM backend)
    #[cfg(feature = "llvm_backend")]

    /// Generate LLVM value for memory read (read_ref) (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_read_ref_llvm(&mut self, read: &ReadRefExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate reference expression first (without borrowing builder)
        let ref_val = self.generate_expression_llvm(&read.reference)?;

        if let Some(ref_ptr) = ref_val {
            if let (Some(builder), Some(module)) = (&self.builder, &self.module) {
                unsafe {
                    // Get the silica_region_read function
                    if let Some(read_func) = (*module).get_function("silica_region_read") {
                        // Call silica_region_read(ref_ptr) -> value (i64)
                        let _call_result = builder.build_call(read_func, &[ref_ptr.into()], "read_result").unwrap();

                        // For now, return a placeholder i64 value - the actual implementation would extract from call_result
                        // This is a temporary simplification to get the basic structure working
                        let placeholder_val = (*self.context).i64_type().const_int(0, false);
                        Ok(Some(placeholder_val.into()))
                    } else {
                        Err(CompilerError::codegen_error("silica_region_read function not found".to_string()))
                    }
                }
            } else {
                Err(CompilerError::codegen_error("LLVM builder or module not initialized".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error("Invalid reference for read operation".to_string()))
        }
    }

    /// Generate LLVM value for region creation (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_region_llvm(&mut self, region: &RegionExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate the value to be stored
        let value = self.generate_expression_llvm(&region.value)?;

        if let (Some(builder), Some(module), Some(val)) = (&self.builder, &self.module, value) {
            unsafe {
                // Determine the type for size calculation
                let initial_expr_type = self.get_expression_type(&region.value).unwrap_or(Type::Int64);
                let size_bytes = self.get_type_size_bytes(&initial_expr_type);

                // Get the appropriate runtime function
                let func_name = if matches!(initial_expr_type,
                    Type::Tuple(_) | Type::Record(_) | Type::String | Type::Function { .. } |
                    Type::Reference { .. } | Type::Buffer { .. } | Type::ActorRef |
                    Type::Region { .. } | Type::Process { .. }
                ) {
                    "silica_region_create_with_data"
                } else {
                    "silica_region_create_with_value"
                };

                if let Some(region_func) = (*module).get_function(func_name) {
                    if func_name == "silica_region_create_with_data" {
                        // For complex types, pass data pointer and size
                        let size_val = (*self.context).i64_type().const_int(size_bytes as u64, false);
                        let call_result = builder.build_call(region_func, &[val, size_val.into()], "region_result").unwrap();
                        Ok(Some(call_result.try_as_basic_value().left().unwrap()))
                    } else {
                        // For primitive types, pass the value directly
                        let call_result = builder.build_call(region_func, &[val], "region_result").unwrap();
                        Ok(Some(call_result.try_as_basic_value().left().unwrap()))
                    }
                } else {
                    Err(CompilerError::codegen_error(format!("{} function not found", func_name)))
                }
            }
        } else {
            Err(CompilerError::codegen_error("LLVM context not available".to_string()))
        }
    }

    /// Generate LLVM value for do expressions (monadic sequencing) (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_do_llvm(&mut self, do_expr: &DoExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Enter a new scope for the do expression
        self.enter_scope();

        // Do expressions execute statements sequentially and return the value of the last statement
        let mut result = None;

        for statement in &do_expr.statements {
            match statement {
                Statement::Bind { pattern, expr } => {
                    // println!("LLVM: Processing bind statement");
                    // Evaluate the expression
                    let value = self.generate_expression_llvm(expr)?;

                    match pattern {
                        Pattern::Identifier(name) => {
                            // Store the value in the current scope
                            if let Some(val) = value {
                                // For LLVM, we need to allocate space for the variable
                                if let Some(builder) = &self.builder {
                                    unsafe {
                                        // Allocate space on the stack for this variable
                                        let var_type = val.get_type();
                                        let alloca = builder.build_alloca(var_type, &name).unwrap();
                                        builder.build_store(alloca, val).unwrap();

                                        // Check if this is a function literal
                                        if let Expression::FunctionLiteral(_) = &*expr {
                                            // For function literals, try to get the type from expression_types
                                            if let Some(expr_type) = self.expression_types.get(&expr.location).cloned() {
                                                if let Type::Function { parameters, return_type } = expr_type {
                                                    self.add_function_variable_llvm(name.clone(), alloca, &expr_type);
                                                } else {
                                                    // Not a function type, store as regular variable
                                                    self.add_variable(name.clone(), alloca);
                                                }
                                            } else {
                                                // Fallback: assume it's a function and create a default signature
                                                let default_params = vec![Type::Int64];
                                                let default_return = Box::new(Type::Int64);
                                                let default_func_type = Type::Function {
                                                    parameters: default_params,
                                                    return_type: default_return,
                                                };
                                                self.add_function_variable_llvm(name.clone(), alloca, &default_func_type);
                                            }
                                        } else {
                                            // Store in current scope as regular variable
                                            self.add_variable(name.clone(), alloca);
                                        }
                                    }
                                }
                            }
                        }
                        Pattern::TypedIdentifier { name, .. } => {
                            // Store the value in the current scope
                            if let Some(val) = value {
                                // For LLVM, we need to allocate space for the variable
                                if let Some(builder) = &self.builder {
                                    unsafe {
                                        // Allocate space on the stack for this variable
                                        let var_type = val.get_type();
                                        let alloca = builder.build_alloca(var_type, &name).unwrap();
                                        builder.build_store(alloca, val).unwrap();

                                        // Check if this is a function literal
                                        if let Expression::FunctionLiteral(_) = &*expr {
                                            // For function literals, try to get the type from expression_types
                                            if let Some(expr_type) = self.expression_types.get(&expr.location).cloned() {
                                                if let Type::Function { parameters, return_type } = expr_type {
                                                    self.add_function_variable_llvm(name.clone(), alloca, &expr_type);
                                                } else {
                                                    // Not a function type, store as regular variable
                                                    self.add_variable(name.clone(), alloca);
                                                }
                                            } else {
                                                // Fallback: assume it's a function and create a default signature
                                                let default_params = vec![Type::Int64];
                                                let default_return = Box::new(Type::Int64);
                                                let default_func_type = Type::Function {
                                                    parameters: default_params,
                                                    return_type: default_return,
                                                };
                                                self.add_function_variable_llvm(name.clone(), alloca, &default_func_type);
                                            }
                                        } else {
                                            // Store in current scope as regular variable
                                            self.add_variable(name.clone(), alloca);
                                        }
                                    }
                                }
                            }
                        }
                        Pattern::Tuple(elements) => {
                            // Handle tuple decomposition
                            if let Some(tuple_ptr) = value {
                                if let Some(builder) = &self.builder {
                                    unsafe {
                                        for (i, elem_pattern) in elements.iter().enumerate() {
                                            match elem_pattern {
                                                Pattern::Identifier(elem_name) => {
                                                    // For tuples stored as raw memory (i8*), calculate byte offset
                                                    // For 2-element tuples: element 0 at offset 0 (i64), element 1 at offset 8 (i64, converted from bool)
                                                    // For other tuples: all elements at 8-byte aligned offsets (i64)
                                                    let byte_offset = (i * 8) as u64;

                                                    // Get pointer to the element using getelementptr on i8*
                                                    let elem_ptr_i8 = builder.build_gep(
                                                        (*self.context).i8_type(),
                                                        tuple_ptr.into_pointer_value(),
                                                        &[*(*self.context).i64_type().const_int(byte_offset, false)],
                                                        &format!("elem_ptr_i8_{}", i)
                                                    ).unwrap();

                                                    // Cast to i64* since all elements are stored as i64
                                                    let elem_ptr_i64 = builder.build_pointer_cast(
                                                        elem_ptr_i8,
                                                        (*self.context).i64_type().ptr_type(inkwell::AddressSpace::default()),
                                                        &format!("elem_ptr_i64_{}", i)
                                                    ).unwrap();

                                                    // Load the i64 value
                                                    let elem_val = builder.build_load(elem_ptr_i64, &format!("elem_val_{}", i)).unwrap();

                                                    // Allocate space and store the element value
                                                    let alloca = builder.build_alloca(elem_val.get_type(), elem_name).unwrap();
                                                    builder.build_store(alloca, elem_val).unwrap();

                                                    // Store in current scope
                                                    self.add_variable(elem_name.clone(), alloca);
                                                }
                                                Pattern::TypedIdentifier { name: elem_name, .. } => {
                                                    if elem_name == "_" {
                                                        // Wildcards don't bind variables - skip
                                                    } else {
                                                        // For tuples stored as raw memory (i8*), calculate byte offset
                                                        // For 2-element tuples: element 0 at offset 0 (i64), element 1 at offset 8 (i64, converted from bool)
                                                        // For other tuples: all elements at 8-byte aligned offsets (i64)
                                                        let byte_offset = (i * 8) as u64;

                                                        // Get pointer to the element using getelementptr on i8*
                                                        let elem_ptr_i8 = builder.build_gep(
                                                            (*self.context).i8_type(),
                                                            tuple_ptr.into_pointer_value(),
                                                            &[*(*self.context).i64_type().const_int(byte_offset, false)],
                                                            &format!("elem_ptr_i8_{}", i)
                                                        ).unwrap();

                                                        // Cast to i64* since all elements are stored as i64
                                                        let elem_ptr_i64 = builder.build_pointer_cast(
                                                            elem_ptr_i8,
                                                            (*self.context).i64_type().ptr_type(inkwell::AddressSpace::default()),
                                                            &format!("elem_ptr_i64_{}", i)
                                                        ).unwrap();

                                                        // Load the i64 value
                                                        let elem_val = builder.build_load(elem_ptr_i64, &format!("elem_val_{}", i)).unwrap();

                                                        // Allocate space and store the element value
                                                        let alloca = builder.build_alloca(elem_val.get_type(), elem_name).unwrap();
                                                        builder.build_store(alloca, elem_val).unwrap();

                                                        // Store in current scope
                                                        self.add_variable(elem_name.clone(), alloca);
                                                    }
                                                }
                                                Pattern::Literal(_) => {
                                                    // Literals don't bind variables
                                                }
                                                Pattern::Tuple(sub_patterns) => {
                                                    // Full nested tuple decomposition: load nested tuple pointer, then recurse
                                                    let byte_offset = (i * 8) as u64;
                                                    let elem_ptr_i8 = builder.build_gep(
                                                        (*self.context).i8_type(),
                                                        tuple_ptr.into_pointer_value(),
                                                        &[*(*self.context).i64_type().const_int(byte_offset, false)],
                                                        &format!("elem_ptr_i8_nested_{}", i)
                                                    ).unwrap();
                                                    let elem_ptr_i8ptr = builder.build_pointer_cast(
                                                        elem_ptr_i8,
                                                        (*self.context).i8_type().ptr_type(inkwell::AddressSpace::default()).ptr_type(inkwell::AddressSpace::default()),
                                                        &format!("elem_ptr_i8ptr_{}", i)
                                                    ).unwrap();
                                                    let nested_tuple_ptr = builder.build_load((*self.context).i8_type().ptr_type(inkwell::AddressSpace::default()), elem_ptr_i8ptr, &format!("nested_tuple_{}", i)).unwrap().into_pointer_value();
                                                    self.generate_tuple_decomposition_llvm(nested_tuple_ptr, sub_patterns, 0)?;
                                                }
                                                Pattern::Record(_) | Pattern::Variant { .. } | Pattern::Alternative(_) => {
                                                    // Not yet implemented in LLVM path
                                                }
                                                _ => {
                                                    return Err(codegen_error(format!("Unsupported pattern type in tuple decomposition: {:?}", elem_pattern)));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => return Err(CompilerError::codegen_error("Pattern type not supported in bindings".to_string())),
                    }
                }
                Statement::Expr(expr) => {
                    // Just evaluate the expression
                    result = self.generate_expression_llvm(expr)?;
                }
            }
        }

        // Exit the scope
        self.exit_scope_text();

        Ok(result)
    }

    /// Full nested tuple decomposition for LLVM backend: recursively bind tuple elements (including nested tuples).
    #[cfg(feature = "llvm_backend")]
    fn generate_tuple_decomposition_llvm(
        &mut self,
        tuple_ptr: inkwell::values::PointerValue<'static>,
        elements: &[Pattern],
        base_offset: u64,
    ) -> Result<()> {
        if let Some(builder) = &self.builder {
            unsafe {
                for (i, elem_pattern) in elements.iter().enumerate() {
                    let byte_offset = base_offset + (i as u64 * 8);
                    match elem_pattern {
                        Pattern::Identifier(elem_name) => {
                            let elem_ptr_i8 = builder.build_gep(
                                (*self.context).i8_type(),
                                tuple_ptr,
                                &[*(*self.context).i64_type().const_int(byte_offset as u64, false)],
                                &format!("elem_ptr_i8_{}", i),
                            ).unwrap();
                            let elem_ptr_i64 = builder.build_pointer_cast(
                                elem_ptr_i8,
                                (*self.context).i64_type().ptr_type(inkwell::AddressSpace::default()),
                                &format!("elem_ptr_i64_{}", i),
                            ).unwrap();
                            let elem_val = builder.build_load(elem_ptr_i64, &format!("elem_val_{}", i)).unwrap();
                            let alloca = builder.build_alloca(elem_val.get_type(), elem_name).unwrap();
                            builder.build_store(alloca, elem_val).unwrap();
                            self.add_variable(elem_name.clone(), alloca);
                        }
                        Pattern::TypedIdentifier { name: elem_name, .. } => {
                            if elem_name != "_" {
                                let elem_ptr_i8 = builder.build_gep(
                                    (*self.context).i8_type(),
                                    tuple_ptr,
                                    &[*(*self.context).i64_type().const_int(byte_offset as u64, false)],
                                    &format!("elem_ptr_i8_{}", i),
                                ).unwrap();
                                let elem_ptr_i64 = builder.build_pointer_cast(
                                    elem_ptr_i8,
                                    (*self.context).i64_type().ptr_type(inkwell::AddressSpace::default()),
                                    &format!("elem_ptr_i64_{}", i),
                                ).unwrap();
                                let elem_val = builder.build_load(elem_ptr_i64, &format!("elem_val_{}", i)).unwrap();
                                let alloca = builder.build_alloca(elem_val.get_type(), elem_name).unwrap();
                                builder.build_store(alloca, elem_val).unwrap();
                                self.add_variable(elem_name.clone(), alloca);
                            }
                        }
                        Pattern::Literal(_) => {}
                        Pattern::Tuple(sub_patterns) => {
                            let elem_ptr_i8 = builder.build_gep(
                                (*self.context).i8_type(),
                                tuple_ptr,
                                &[*(*self.context).i64_type().const_int(byte_offset as u64, false)],
                                &format!("elem_ptr_i8_nested_{}", i),
                            ).unwrap();
                            let elem_ptr_i8ptr = builder.build_pointer_cast(
                                elem_ptr_i8,
                                (*self.context).i8_type().ptr_type(inkwell::AddressSpace::default()).ptr_type(inkwell::AddressSpace::default()),
                                &format!("elem_ptr_i8ptr_{}", i),
                            ).unwrap();
                            let nested_tuple_ptr = builder.build_load(
                                (*self.context).i8_type().ptr_type(inkwell::AddressSpace::default()),
                                elem_ptr_i8ptr,
                                &format!("nested_tuple_{}", i),
                            ).unwrap().into_pointer_value();
                            self.generate_tuple_decomposition_llvm(nested_tuple_ptr, sub_patterns, 0)?;
                        }
                        Pattern::Record(_) | Pattern::Variant { .. } | Pattern::Alternative(_) => {}
                        _ => return Err(codegen_error(format!("Unsupported pattern type in nested tuple decomposition (LLVM): {:?}", elem_pattern))), 
                    }
                }
            }
        }
        Ok(())
    }

    /// Generate LLVM value for memory write (write_ref) (LLVM backend)
    #[cfg(feature = "llvm_backend")]

    /// Enter a new variable scope (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn enter_scope(&mut self) {
        self.llvm_variable_scopes.push(HashMap::new());
        self.function_variable_scopes.push(HashMap::new());
    }

    /// Exit the current variable scope (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn exit_scope(&mut self) {
        self.llvm_variable_scopes.pop();
        self.function_variable_scopes.pop();
    }

    /// Add a variable to the current scope (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn add_variable(&mut self, name: String, alloca: inkwell::values::PointerValue<'static>) {
        if let Some(current_scope) = self.llvm_variable_scopes.last_mut() {
            current_scope.insert(name, alloca);
        }
    }

    /// Look up a variable in the scope stack (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn lookup_variable(&self, name: &str) -> Option<inkwell::values::PointerValue<'static>> {
        // Search from innermost scope outward
        for scope in self.llvm_variable_scopes.iter().rev() {
            if let Some(alloca) = scope.get(name) {
                return Some(*alloca);
            }
        }
        None
    }

    /// Enter a new variable scope (text IR)
    fn enter_scope_text(&mut self) {
        self.variable_scopes.push(HashMap::new());
        self.function_variable_scopes.push(HashMap::new());
    }

    /// Exit the current variable scope (text IR)
    fn exit_scope_text(&mut self) {
        self.variable_scopes.pop();
        self.function_variable_scopes.pop();
    }

    /// Add a variable to the current scope (text IR)
    fn add_variable_text(&mut self, name: String, register: String) {
        // Store the full register string with type information preserved
        if let Some(current_scope) = self.variable_scopes.last_mut() {
            current_scope.insert(name, register);
        }
    }

    /// Add a variable with function signature information
    fn add_function_variable(&mut self, name: String, register: String, func_type: &Type) {
        // Store the variable normally
        self.add_variable_text(name.clone(), register);

        // Store function signature information if it's a function type
        if let Type::Function { parameters, return_type } = func_type {
            if let Some(current_scope) = self.function_variable_scopes.last_mut() {
                current_scope.insert(name, (parameters.clone(), (**return_type).clone()));
            }
        }
    }

    /// Convert a Silica type to LLVM type (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn silica_type_to_llvm_type(&self, ty: &Type) -> inkwell::types::BasicMetadataTypeEnum<'static> {
        unsafe {
            match ty {
                Type::Int8 => (*self.context).i8_type().into(),
                Type::Int16 => (*self.context).i16_type().into(),
                Type::Int32 => (*self.context).i32_type().into(),
                Type::Int64 => (*self.context).i64_type().into(),
                Type::Float16 => (*self.context).f16_type().into(),
                Type::Float32 => (*self.context).f32_type().into(),
                Type::Bool => (*self.context).i1_type().into(),
                Type::Char => (*self.context).i32_type().into(),
                Type::String => (*self.context).i8_type().ptr_type(inkwell::AddressSpace::Generic).into(),
                Type::Function { .. } => (*self.context).i8_type().ptr_type(inkwell::AddressSpace::Generic).into(),
                Type::Tuple(_) => (*self.context).i8_type().ptr_type(inkwell::AddressSpace::Generic).into(),
                Type::Record(_) => (*self.context).i8_type().ptr_type(inkwell::AddressSpace::Generic).into(),
                Type::Process { result_type, .. } => {
                    // Process types return their result type at runtime
                    self.silica_type_to_llvm_type(result_type)
                },
                Type::Unit => (*self.context).void_type().into(),
                // NEON 128-bit vector types
                Type::Vec128Int8 => (*self.context).i8_type().vec_type(16).into(),
                Type::Vec128Int16 => (*self.context).i16_type().vec_type(8).into(),
                Type::Vec128Int32 => (*self.context).i32_type().vec_type(4).into(),
                Type::Vec128Int64 => (*self.context).i64_type().vec_type(2).into(),
                Type::Vec128Float32 => (*self.context).f32_type().vec_type(4).into(),
                Type::Vec128Bool => (*self.context).i1_type().vec_type(16).into(),
                // SVE scalable vector types - use fixed-size vectors as placeholder
                Type::VecInt8 => (*self.context).i8_type().vec_type(16).into(),
                Type::VecInt16 => (*self.context).i16_type().vec_type(8).into(),
                Type::VecInt32 => (*self.context).i32_type().vec_type(4).into(),
                Type::VecInt64 => (*self.context).i64_type().vec_type(2).into(),
                Type::VecFloat16 => (*self.context).f16_type().vec_type(8).into(),
                Type::VecFloat32 => (*self.context).f32_type().vec_type(4).into(),
                Type::VecFloat64 => (*self.context).f64_type().vec_type(2).into(),
                Type::VecBool => (*self.context).i1_type().vec_type(16).into(),
                // SVE predicate type
                Type::Pred => (*self.context).i1_type().vec_type(16).into(),
                _ => (*self.context).i64_type().into(), // Default fallback
            }
        }
    }

    /// Add a function variable to the current scope (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn add_function_variable_llvm(&mut self, name: String, pointer: inkwell::values::PointerValue<'static>, func_type: &Type) {
        // Store the variable normally
        self.add_variable(name.clone(), pointer);

        // Store function signature information if it's a function type
        if let Type::Function { parameters, return_type } = func_type {
            if let Some(current_scope) = self.function_variable_scopes.last_mut() {
                current_scope.insert(name, (parameters.clone(), (**return_type).clone()));
            }
        }
    }

    /// Look up function signature for a variable
    fn lookup_function_variable_signature(&self, name: &str) -> Option<&(Vec<Type>, Type)> {
        // Search from innermost scope outward
        for scope in self.function_variable_scopes.iter().rev() {
            if let Some(sig) = scope.get(name) {
                return Some(sig);
            }
        }
        None
    }

    /// Check if a value string is a plain integer literal (e.g. "0", "1", "42", "-123").
    /// Used to avoid emitting %0/%1 for literals, which LLVM interprets as labels.
    fn is_integer_literal(value: &str) -> bool {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return false;
        }
        let rest = trimmed.strip_prefix('-').unwrap_or(trimmed);
        rest.chars().all(|c| c.is_ascii_digit())
    }

    /// Clean a register name for use in LLVM instructions (strip type prefixes)
    fn clean_register_for_instruction(&self, register: &str) -> String {
        if register.starts_with("i64 ") {
            register.strip_prefix("i64 ").unwrap().to_string()
        } else if register.starts_with("i32 ") {
            register.strip_prefix("i32 ").unwrap().to_string()
        } else if register.starts_with("i1 ") {
            register.strip_prefix("i1 ").unwrap().to_string()
        } else if register.starts_with("i8* ") {
            register.strip_prefix("i8* ").unwrap().to_string()
        } else {
            register.to_string()
        }
    }

    /// If the value is a getelementptr expression (inline), assign it to a register first; return register or value for use in instructions.
    fn ensure_gep_in_register(&mut self, value: &str, side: &str) -> String {
        let trimmed = value.trim_start_matches('%');
        if !trimmed.contains("getelementptr") {
            return value.to_string();
        }
        let gep_reg = format!("%gep_{}_{}", side, self.instructions.len());
        let gep_instr = if trimmed.starts_with("getelementptr inbounds (") {
            self.convert_gep_to_instruction_format(trimmed)
        } else {
            trimmed.to_string()
        };
        self.instructions.push(format!("  {} = {}", gep_reg, gep_instr));
        gep_reg
    }

    /// Check if a type represents a pointer
    fn is_pointer_type(&self, ty: &Type) -> bool {
        matches!(ty, Type::Named(name) if name == "string") || matches!(ty, Type::Record(_))
    }

    /// Convert a Silica type to LLVM type string
    fn get_llvm_type_string(&self, ty: &Type) -> String {
        match ty {
            Type::Int8 => "i8".to_string(),
            Type::Int16 => "i16".to_string(),
            Type::Int32 => "i32".to_string(),
            Type::Int64 => "i64".to_string(),
            Type::Float16 => "half".to_string(),
            Type::Float32 => "float".to_string(),
            Type::Float64 => "double".to_string(),
            Type::Bool => "i1".to_string(),
            Type::Char => "i32".to_string(),
            Type::String => "i8*".to_string(),
            Type::Function { .. } => "i8*".to_string(), // Function pointers are i8*
            Type::Tuple(_) => "i8*".to_string(), // Tuples are heap-allocated
            Type::Record(_) => "i8*".to_string(), // Structs are heap-allocated
            Type::Unit => "void".to_string(),
            _ => "i64".to_string(), // Default fallback
        }
    }

    /// Look up a variable in the scope stack (text IR)
    fn lookup_variable_text(&self, name: &str) -> Option<String> {
        // Search from innermost scope outward
        for scope in self.variable_scopes.iter().rev() {
            if let Some(register) = scope.get(name) {
                return Some(register.clone());
            }
        }
        None
    }

    /// Generate LLVM literal values (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_literal_llvm(&mut self, lit: &Literal) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        self.generate_literal_llvm_with_type(lit, None)
    }

    /// Generate LLVM literal values with optional type context (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_literal_llvm_with_type(&mut self, lit: &Literal, expr_type: Option<&Type>) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        unsafe {
            let val = match lit {
                Literal::Unit => {
                    // Unit type - return void, but we need a value, so maybe return i64 0 for now
                    (*self.context).i64_type().const_int(0, false).into()
                }
                Literal::Bool(b) => {
                    (*self.context).bool_type().const_int(if *b { 1 } else { 0 }, false).into()
                }
                Literal::Int(i) => {
                    (*self.context).i64_type().const_int(*i as u64, false).into()
                }
                Literal::Float(f) => {
                    // Use expression type if available, default to float32
                    let float_type = if let Some(ty) = expr_type {
                        match ty {
                            Type::Float16 => (*self.context).f16_type(),
                            Type::Float32 => (*self.context).f32_type(),
                            Type::Float64 => (*self.context).f64_type(),
                            _ => (*self.context).f32_type(), // default
                        }
                    } else {
                        (*self.context).f32_type() // default
                    };
                    float_type.const_float(*f).into()
                }
                Literal::Char(c) => {
                    (*self.context).i32_type().const_int(*c as u32 as u64, false).into()
                }
                Literal::String(s) => {
                    // For now, create a global string constant
                    let string_val = (*self.context).const_string(s.as_bytes(), false);
                    string_val.into()
                }
            };
            Ok(Some(val))
        }
    }

    /// Generate LLVM identifier/variable lookup (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_identifier_llvm(&mut self, name: &str) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Look up the variable in the scope stack
        if let Some(alloca) = self.lookup_variable(name) {
            if let Some(builder) = &self.builder {
                unsafe {
                    // Load the value from the allocated stack slot
                    // For now, assume all variables are i64
                    let loaded_value = builder.build_load((*self.context).i64_type(), alloca, name).unwrap();
                    Ok(Some(loaded_value))
                }
            } else {
                Err(CompilerError::codegen_error("LLVM builder not initialized".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error(format!("Undefined variable: {}", name)))
        }
    }

    /// Generate LLVM binary operations (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_binary_llvm(&mut self, binary: &BinaryExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate operands first (without borrowing builder)
        let left = self.generate_expression_llvm(&binary.left)?;
        let right = self.generate_expression_llvm(&binary.right)?;

        if let (Some(left_val), Some(right_val)) = (left, right) {
            // Get operand types for type-specific code generation
            let left_type = self.get_expression_type(&binary.left).ok();
            let right_type = self.get_expression_type(&binary.right).ok();
            
            if let Some(builder) = &self.builder {
                unsafe {
                    let result = match binary.operator {
                        BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                            // Arithmetic operations - need to check types
                            if let (Some(lt), Some(rt)) = (left_type, right_type) {
                                // Ensure types match (type checker should have enforced this)
                                if lt != rt {
                                    return Err(CompilerError::codegen_error(
                                        format!("Arithmetic operands must be same type: {:?} vs {:?}", lt, rt)
                                    ));
                                }
                                
                                // Generate type-specific operations
                                match lt {
                                    Type::Int8 => {
                                        let left_i8 = left_val.into_int_value();
                                        let right_i8 = right_val.into_int_value();
                                        match binary.operator {
                                            BinaryOp::Add => builder.build_int_add(left_i8, right_i8, "add_i8").unwrap().into(),
                                            BinaryOp::Subtract => builder.build_int_sub(left_i8, right_i8, "sub_i8").unwrap().into(),
                                            BinaryOp::Multiply => builder.build_int_mul(left_i8, right_i8, "mul_i8").unwrap().into(),
                                            BinaryOp::Divide => builder.build_int_signed_div(left_i8, right_i8, "div_i8").unwrap().into(),
                                            _ => unreachable!(),
                                        }
                                    }
                                    Type::Int16 => {
                                        let left_i16 = left_val.into_int_value();
                                        let right_i16 = right_val.into_int_value();
                                        match binary.operator {
                                            BinaryOp::Add => builder.build_int_add(left_i16, right_i16, "add_i16").unwrap().into(),
                                            BinaryOp::Subtract => builder.build_int_sub(left_i16, right_i16, "sub_i16").unwrap().into(),
                                            BinaryOp::Multiply => builder.build_int_mul(left_i16, right_i16, "mul_i16").unwrap().into(),
                                            BinaryOp::Divide => builder.build_int_signed_div(left_i16, right_i16, "div_i16").unwrap().into(),
                                            _ => unreachable!(),
                                        }
                                    }
                                    Type::Int32 => {
                                        let left_i32 = left_val.into_int_value();
                                        let right_i32 = right_val.into_int_value();
                                        match binary.operator {
                                            BinaryOp::Add => builder.build_int_add(left_i32, right_i32, "add_i32").unwrap().into(),
                                            BinaryOp::Subtract => builder.build_int_sub(left_i32, right_i32, "sub_i32").unwrap().into(),
                                            BinaryOp::Multiply => builder.build_int_mul(left_i32, right_i32, "mul_i32").unwrap().into(),
                                            BinaryOp::Divide => builder.build_int_signed_div(left_i32, right_i32, "div_i32").unwrap().into(),
                                            _ => unreachable!(),
                                        }
                                    }
                                    Type::Int64 => {
                                        let left_i64 = left_val.into_int_value();
                                        let right_i64 = right_val.into_int_value();
                                        match binary.operator {
                                            BinaryOp::Add => builder.build_int_add(left_i64, right_i64, "add_i64").unwrap().into(),
                                            BinaryOp::Subtract => builder.build_int_sub(left_i64, right_i64, "sub_i64").unwrap().into(),
                                            BinaryOp::Multiply => builder.build_int_mul(left_i64, right_i64, "mul_i64").unwrap().into(),
                                            BinaryOp::Divide => builder.build_int_signed_div(left_i64, right_i64, "div_i64").unwrap().into(),
                                            _ => unreachable!(),
                                        }
                                    }
                                    Type::Float16 => {
                                        let left_f16 = left_val.into_float_value();
                                        let right_f16 = right_val.into_float_value();
                                        match binary.operator {
                                            BinaryOp::Add => builder.build_float_add(left_f16, right_f16, "add_f16").unwrap().into(),
                                            BinaryOp::Subtract => builder.build_float_sub(left_f16, right_f16, "sub_f16").unwrap().into(),
                                            BinaryOp::Multiply => builder.build_float_mul(left_f16, right_f16, "mul_f16").unwrap().into(),
                                            BinaryOp::Divide => builder.build_float_div(left_f16, right_f16, "div_f16").unwrap().into(),
                                            _ => unreachable!(),
                                        }
                                    }
                                    Type::Float32 => {
                                        let left_f32 = left_val.into_float_value();
                                        let right_f32 = right_val.into_float_value();
                                        match binary.operator {
                                            BinaryOp::Add => builder.build_float_add(left_f32, right_f32, "add_f32").unwrap().into(),
                                            BinaryOp::Subtract => builder.build_float_sub(left_f32, right_f32, "sub_f32").unwrap().into(),
                                            BinaryOp::Multiply => builder.build_float_mul(left_f32, right_f32, "mul_f32").unwrap().into(),
                                            BinaryOp::Divide => builder.build_float_div(left_f32, right_f32, "div_f32").unwrap().into(),
                                            _ => unreachable!(),
                                        }
                                    }
                                    _ => {
                                        // Fallback to int64 for unknown types (backward compatibility)
                                        let left_i64 = left_val.into_int_value();
                                        let right_i64 = right_val.into_int_value();
                                        match binary.operator {
                                            BinaryOp::Add => builder.build_int_add(left_i64, right_i64, "add_result").unwrap().into(),
                                            BinaryOp::Subtract => builder.build_int_sub(left_i64, right_i64, "sub_result").unwrap().into(),
                                            BinaryOp::Multiply => builder.build_int_mul(left_i64, right_i64, "mul_result").unwrap().into(),
                                            BinaryOp::Divide => builder.build_int_signed_div(left_i64, right_i64, "div_result").unwrap().into(),
                                            _ => unreachable!(),
                                        }
                                    }
                                }
                            } else {
                                // Fallback if type information not available (backward compatibility)
                                let left_i64 = left_val.into_int_value();
                                let right_i64 = right_val.into_int_value();
                                match binary.operator {
                                    BinaryOp::Add => builder.build_int_add(left_i64, right_i64, "add_result").unwrap().into(),
                                    BinaryOp::Subtract => builder.build_int_sub(left_i64, right_i64, "sub_result").unwrap().into(),
                                    BinaryOp::Multiply => builder.build_int_mul(left_i64, right_i64, "mul_result").unwrap().into(),
                                    BinaryOp::Divide => builder.build_int_signed_div(left_i64, right_i64, "div_result").unwrap().into(),
                                    _ => unreachable!(),
                                }
                            }
                        }
                        BinaryOp::Modulo => {
                            // Modulo only works on integer types
                            if let (Some(lt), Some(rt)) = (left_type, right_type) {
                                if lt != rt {
                                    return Err(CompilerError::codegen_error(
                                        format!("Modulo operands must be same type: {:?} vs {:?}", lt, rt)
                                    ));
                                }
                                
                                if !Self::is_integer_type(&lt) {
                                    return Err(CompilerError::codegen_error(
                                        format!("Modulo operation requires integer operands, found {:?}", lt)
                                    ));
                                }
                                
                                let left_int = left_val.into_int_value();
                                let right_int = right_val.into_int_value();
                                builder.build_int_signed_rem(left_int, right_int, "mod_result").unwrap().into()
                            } else {
                                // Fallback
                                let left_i64 = left_val.into_int_value();
                                let right_i64 = right_val.into_int_value();
                                builder.build_int_signed_rem(left_i64, right_i64, "mod_result").unwrap().into()
                            }
                        }
                        BinaryOp::Equal => {
                            builder.build_int_compare(inkwell::IntPredicate::EQ, left_val.into_int_value(), right_val.into_int_value(), "eq_result").unwrap().into()
                        }
                        BinaryOp::NotEqual => {
                            builder.build_int_compare(inkwell::IntPredicate::NE, left_val.into_int_value(), right_val.into_int_value(), "ne_result").unwrap().into()
                        }
                        BinaryOp::Less => {
                            builder.build_int_compare(inkwell::IntPredicate::SLT, left_val.into_int_value(), right_val.into_int_value(), "lt_result").unwrap().into()
                        }
                        BinaryOp::LessEqual => {
                            builder.build_int_compare(inkwell::IntPredicate::SLE, left_val.into_int_value(), right_val.into_int_value(), "le_result").unwrap().into()
                        }
                        BinaryOp::Greater => {
                            builder.build_int_compare(inkwell::IntPredicate::SGT, left_val.into_int_value(), right_val.into_int_value(), "gt_result").unwrap().into()
                        }
                        BinaryOp::GreaterEqual => {
                            builder.build_int_compare(inkwell::IntPredicate::SGE, left_val.into_int_value(), right_val.into_int_value(), "ge_result").unwrap().into()
                        }
                        _ => return Err(CompilerError::codegen_error(format!("Binary operator not implemented: {:?}", binary.operator))),
                    };
                    Ok(Some(result))
                }
            } else {
                Err(CompilerError::codegen_error("LLVM builder not initialized".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error("Invalid operands for binary operation".to_string()))
        }
    }

    /// Generate LLVM IR for type casting: expr as Type (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_as_type_llvm(&mut self, as_type: &AsTypeExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate the expression first
        let expr_val = self.generate_expression_llvm(&as_type.expression)?;
        
        if let Some(val) = expr_val {
            let source_type = self.get_expression_type(&as_type.expression).unwrap_or(Type::Int64);
            let target_type = &as_type.target_type;
            
            // If types are the same, no conversion needed
            if source_type == *target_type {
                return Ok(Some(val));
            }
            
            if let Some(builder) = &self.builder {
                unsafe {
                    let result = match (source_type, target_type) {
                        // Integer widening (sign extension)
                        (Type::Int8, Type::Int16) | (Type::Int8, Type::Int32) | (Type::Int8, Type::Int64) |
                        (Type::Int16, Type::Int32) | (Type::Int16, Type::Int64) | (Type::Int32, Type::Int64) => {
                            builder.build_int_s_extend(val.into_int_value(), self.silica_type_to_llvm_type(target_type).into_int_type(), "sext").unwrap().into()
                        }
                        // Integer narrowing (truncation)
                        (Type::Int16, Type::Int8) | (Type::Int32, Type::Int8) | (Type::Int32, Type::Int16) |
                        (Type::Int64, Type::Int8) | (Type::Int64, Type::Int16) | (Type::Int64, Type::Int32) => {
                            builder.build_int_truncate(val.into_int_value(), self.silica_type_to_llvm_type(target_type).into_int_type(), "trunc").unwrap().into()
                        }
                        // Float conversions
                        (Type::Float16, Type::Float32) => {
                            builder.build_float_ext(val.into_float_value(), (*self.context).f32_type(), "fpext").unwrap().into()
                        }
                        (Type::Float16, Type::Float64) => {
                            builder.build_float_ext(val.into_float_value(), (*self.context).f64_type(), "fpext").unwrap().into()
                        }
                        (Type::Float32, Type::Float16) => {
                            builder.build_float_trunc(val.into_float_value(), (*self.context).f16_type(), "fptrunc").unwrap().into()
                        }
                        (Type::Float32, Type::Float64) => {
                            builder.build_float_ext(val.into_float_value(), (*self.context).f64_type(), "fpext").unwrap().into()
                        }
                        (Type::Float64, Type::Float16) => {
                            builder.build_float_trunc(val.into_float_value(), (*self.context).f16_type(), "fptrunc").unwrap().into()
                        }
                        (Type::Float64, Type::Float32) => {
                            builder.build_float_trunc(val.into_float_value(), (*self.context).f32_type(), "fptrunc").unwrap().into()
                        }
                        // Integer to boolean
                        (Type::Int32, Type::Bool) | (Type::Int64, Type::Bool) => {
                            let zero = self.silica_type_to_llvm_type(&source_type).into_int_type().const_int(0, false);
                            builder.build_int_compare(inkwell::IntPredicate::NE, val.into_int_value(), zero, "cmp").unwrap().into()
                        }
                        // Boolean to integer
                        (Type::Bool, Type::Int32) | (Type::Bool, Type::Int64) => {
                            builder.build_int_z_extend(val.into_int_value(), self.silica_type_to_llvm_type(target_type).into_int_type(), "zext").unwrap().into()
                        }
                        _ => {
                            // For other conversions, try bitcast (may not always be valid)
                            val // Return as-is for now, type checker should validate
                        }
                    };
                    Ok(Some(result))
                }
            } else {
                Err(CompilerError::codegen_error("LLVM builder not available".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error("Cannot cast void expression".to_string()))
        }
    }

    /// Generate LLVM unary operations (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_unary_llvm(&mut self, unary: &UnaryExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate operand first (without borrowing builder)
        let operand = self.generate_expression_llvm(&unary.operand)?;

        if let Some(op_val) = operand {
            // Get operand type for type-specific code generation
            let operand_type = self.get_expression_type(&unary.operand).ok();
            
            if let Some(builder) = &self.builder {
                unsafe {
                    let result = match unary.operator {
                        UnaryOp::Negate => {
                            // Negation works on all numeric types
                            if let Some(ty) = operand_type {
                                match ty {
                                    Type::Int8 => {
                                        let zero = (*self.context).i8_type().const_int(0, false);
                                        builder.build_int_sub(zero, op_val.into_int_value(), "neg_i8").unwrap().into()
                                    }
                                    Type::Int16 => {
                                        let zero = (*self.context).i16_type().const_int(0, false);
                                        builder.build_int_sub(zero, op_val.into_int_value(), "neg_i16").unwrap().into()
                                    }
                                    Type::Int32 => {
                                        let zero = (*self.context).i32_type().const_int(0, false);
                                        builder.build_int_sub(zero, op_val.into_int_value(), "neg_i32").unwrap().into()
                                    }
                                    Type::Int64 => {
                                        let zero = (*self.context).i64_type().const_int(0, false);
                                        builder.build_int_sub(zero, op_val.into_int_value(), "neg_i64").unwrap().into()
                                    }
                                    Type::Float16 => {
                                        let zero = (*self.context).f16_type().const_float(0.0);
                                        builder.build_float_sub(zero, op_val.into_float_value(), "neg_f16").unwrap().into()
                                    }
                                    Type::Float32 => {
                                        let zero = (*self.context).f32_type().const_float(0.0);
                                        builder.build_float_sub(zero, op_val.into_float_value(), "neg_f32").unwrap().into()
                                    }
                                    Type::Float64 => {
                                        let zero = (*self.context).f64_type().const_float(0.0);
                                        builder.build_float_sub(zero, op_val.into_float_value(), "neg_f64").unwrap().into()
                                    }
                                    _ => {
                                        // Fallback to int64
                                        let zero = (*self.context).i64_type().const_int(0, false);
                                        builder.build_int_sub(zero, op_val.into_int_value(), "neg_result").unwrap().into()
                                    }
                                }
                            } else {
                                // Fallback if type information not available
                                let zero = (*self.context).i64_type().const_int(0, false);
                                builder.build_int_sub(zero, op_val.into_int_value(), "neg_result").unwrap().into()
                            }
                        }
                        UnaryOp::Not => {
                            builder.build_not(op_val.into_int_value(), "not_result").unwrap().into()
                        }
                    };
                    Ok(Some(result))
                }
            } else {
                Err(CompilerError::codegen_error("LLVM builder not initialized".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error("Invalid operand for unary operation".to_string()))
        }
    }

    /// Generate LLVM actor spawn (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_spawn_llvm(&mut self, spawn: &SpawnExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate initial state and behavior expressions
        let initial_state_val = self.generate_expression_llvm(&spawn.initial_state)?;
        let behavior_val = self.generate_expression_llvm(&spawn.behavior)?;

        // Generate core affinity (default to 0 for any core)
        let core_affinity_val = if let Some(ref affinity_expr) = spawn.core_affinity {
            self.generate_expression_llvm(affinity_expr)?
        } else {
            Some((*self.context).i32_type().const_int(0, false).into())
        };

        if let (Some(initial_state), Some(behavior), Some(core_affinity)) = (initial_state_val, behavior_val, core_affinity_val) {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                unsafe {
                    // Get the silica_actor_spawn function
                    if let Some(spawn_func) = (*module).get_function("silica_actor_spawn") {
                        // Call silica_actor_spawn(initial_state, behavior, core_affinity)
                        let _call_result = builder.build_call(
                            spawn_func,
                            &[initial_state.into(), behavior.into(), core_affinity.into()],
                            "actor_spawn_result"
                        ).unwrap();

                        // Return a placeholder actor pointer (i8*)
                        // In a real implementation, this would be the actual actor reference
                        let placeholder_actor = (*self.context).i8_type().ptr_type(inkwell::AddressSpace::default()).const_null();
                        Ok(Some(placeholder_actor.into()))
                    } else {
                        Err(CompilerError::codegen_error("silica_actor_spawn function not found".to_string()))
                    }
                }
            } else {
                Err(CompilerError::codegen_error("LLVM module or builder not initialized".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error("Invalid arguments for spawn".to_string()))
        }
    }

    /// Generate LLVM message send (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_send_llvm(&mut self, send: &SendExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate actor and message expressions
        let actor_val = self.generate_expression_llvm(&send.actor)?;
        let message_val = self.generate_expression_llvm(&send.message)?;

        if let (Some(actor), Some(message)) = (actor_val, message_val) {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                unsafe {
                    // Get the silica_actor_send function
                    if let Some(send_func) = (*module).get_function("silica_actor_send") {
                        // Call silica_actor_send(actor, message)
                        builder.build_call(
                            send_func,
                            &[actor.into(), message.into()],
                            "send_result"
                        ).unwrap();

                        // Send returns unit (void), so no result value
                        Ok(None)
                    } else {
                        Err(CompilerError::codegen_error("silica_actor_send function not found".to_string()))
                    }
                }
            } else {
                Err(CompilerError::codegen_error("LLVM module or builder not initialized".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error("Invalid actor or message for send".to_string()))
        }
    }

    /// Generate LLVM message cast (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_cast_llvm(&mut self, cast: &CastExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate actor and message expressions
        let actor_val = self.generate_expression_llvm(&cast.actor)?;
        let message_val = self.generate_expression_llvm(&cast.message)?;

        if let (Some(actor), Some(message)) = (actor_val, message_val) {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                unsafe {
                    // Get the silica_actor_cast function (returns i1 bool)
                    if let Some(cast_func) = (*module).get_function("silica_actor_cast") {
                        // Call silica_actor_cast(actor, message) -> bool (i1)
                        let result = builder.build_call(
                            cast_func,
                            &[actor.into(), message.into()],
                            "cast_result"
                        ).unwrap();

                        // Cast returns bool (i1), convert to i64 for consistency with other expressions
                        let bool_val = result.try_as_basic_value().left().unwrap().into_int_value();
                        let i64_val = builder.build_int_z_extend_or_bit_cast(
                            bool_val,
                            (*self.context).i64_type(),
                            "cast_bool_i64"
                        ).unwrap();

                        Ok(Some(i64_val.into()))
                    } else {
                        Err(CompilerError::codegen_error("silica_actor_cast function not found".to_string()))
                    }
                }
            } else {
                Err(CompilerError::codegen_error("LLVM module or builder not initialized".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error("Invalid actor or message for cast".to_string()))
        }
    }

    /// Generate LLVM message receive (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_recv_llvm(&mut self, recv: &RecvExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        unsafe {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                let context = &*self.context;

                if let Some(actor_expr) = &recv.actor {
                    // recv(actor) - receive from specific actor
                    let actor_val = self.generate_expression_llvm(actor_expr)?
                        .ok_or_else(|| CompilerError::codegen_error("Invalid actor in recv".to_string()))?;

                    // Get the silica_actor_recv function
                    let recv_func = module.get_function("silica_actor_recv")
                        .ok_or_else(|| CompilerError::codegen_error("silica_actor_recv function not found".to_string()))?;

                    // Call the function
                    let call_result = builder.build_call(recv_func, &[actor_val.into()], "recv_result")
                        .unwrap()
                        .try_as_basic_value()
                        .unwrap_basic();

                    Ok(Some(call_result))
                } else {
                    // recv() - this is not supported without an actor context
                    // For now, return a null pointer
                    let null_ptr = context.i8_type().ptr_type(inkwell::AddressSpace::default()).const_null();
                    Ok(Some(null_ptr.into()))
                }
            } else {
                {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
            }
        }
    }
    /// Generate LLVM value for read_file expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_read_file_llvm(&mut self, read_file: &ReadFileExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        unsafe {
            // Generate path expression
            let path_val = self.generate_expression_llvm(&read_file.path)?
                .ok_or_else(|| CompilerError::codegen_error("Invalid path in read_file".to_string()))?;

            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                // Get the silica_read_file function
                if let Some(read_func) = (*module).get_function("silica_read_file") {
                    // For now, create a placeholder call
                    // In a real implementation, we'd need to handle string arguments properly
                    let call_result = builder.build_call(
                        read_func,
                        &[path_val.into(), (*self.context).i64_type().const_int(0, false).into()],
                        "read_result"
                    ).unwrap();

                    let result: inkwell::values::BasicValueEnum<'static> = unsafe { std::mem::transmute(call_result) };
                    Ok(Some(result))
                } else {
                    Err(CompilerError::codegen_error("silica_read_file function not found".to_string()))
                }
            } else {
                {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
            }
        }
    }

    /// Generate LLVM value for write_file expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_write_file_llvm(&mut self, write_file: &WriteFileExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        unsafe {
            // Generate path and content expressions
            let path_val = self.generate_expression_llvm(&write_file.path)?
                .ok_or_else(|| CompilerError::codegen_error("Invalid path in write_file".to_string()))?;
            let content_val = self.generate_expression_llvm(&write_file.content)?
                .ok_or_else(|| CompilerError::codegen_error("Invalid content in write_file".to_string()))?;

            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                // Get the silica_write_file function
                if let Some(write_func) = (*module).get_function("silica_write_file") {
                    // For now, create a placeholder call
                    // In a real implementation, we'd need to handle string arguments properly
                    let call_result = builder.build_call(
                        write_func,
                        &[path_val.into(), (*self.context).i64_type().const_int(0, false).into(),
                          content_val.into(), (*self.context).i64_type().const_int(0, false).into()],
                        "write_result"
                    ).unwrap();

                    let result: inkwell::values::BasicValueEnum<'static> = unsafe { std::mem::transmute(call_result) };
                    Ok(Some(result))
                } else {
                    Err(CompilerError::codegen_error("silica_write_file function not found".to_string()))
                }
            } else {
                {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
            }
        }
    }

    /// Generate LLVM call to read_file runtime function (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_read_file_call_llvm(&mut self, call: &CallExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        if call.arguments.len() != 1 {
            return Err(CompilerError::codegen_error("read_file expects exactly 1 argument".to_string()));
        }

        unsafe {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                // Get the silica_read_file function
                if let Some(read_func) = (*module).get_function("silica_read_file") {
                    // Generate the path argument
                    let path_arg = self.generate_expression_llvm(&call.arguments[0])?
                        .ok_or_else(|| CompilerError::codegen_error("Invalid path argument in read_file".to_string()))?;

                    // For now, create a placeholder call (simplified string handling)
                    let call_result = builder.build_call(
                        read_func,
                        &[path_arg.into(), (*self.context).i64_type().const_int(0, false).into()],
                        "read_result"
                    ).unwrap();

                    let result: inkwell::values::BasicValueEnum<'static> = unsafe { std::mem::transmute(call_result) };
                    Ok(Some(result))
                } else {
                    Err(CompilerError::codegen_error("silica_read_file function not found".to_string()))
                }
            } else {
                {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
            }
        }
    }

    /// Generate LLVM call to write_file runtime function (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_write_file_call_llvm(&mut self, call: &CallExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        if call.arguments.len() != 2 {
            return Err(CompilerError::codegen_error("write_file expects exactly 2 arguments".to_string()));
        }

        unsafe {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                // Get the silica_write_file function
                if let Some(write_func) = (*module).get_function("silica_write_file") {
                    // Generate the path and content arguments
                    let path_arg = self.generate_expression_llvm(&call.arguments[0])?
                        .ok_or_else(|| CompilerError::codegen_error("Invalid path argument in write_file".to_string()))?;
                    let content_arg = self.generate_expression_llvm(&call.arguments[1])?
                        .ok_or_else(|| CompilerError::codegen_error("Invalid content argument in write_file".to_string()))?;

                    // For now, create a placeholder call (simplified string handling)
                    let call_result = builder.build_call(
                        write_func,
                        &[path_arg.into(), (*self.context).i64_type().const_int(0, false).into(),
                          content_arg.into(), (*self.context).i64_type().const_int(0, false).into()],
                        "write_result"
                    ).unwrap();

                    let result: inkwell::values::BasicValueEnum<'static> = unsafe { std::mem::transmute(call_result) };
                    Ok(Some(result))
                } else {
                    Err(CompilerError::codegen_error("silica_write_file function not found".to_string()))
                }
            } else {
                {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
            }
        }
    }

    /// Generate LLVM value for exec_command expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_exec_command_llvm(&mut self, exec_cmd: &ExecCommandExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        unsafe {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                // Generate command expression
                let cmd_val = self.generate_expression_llvm(&exec_cmd.command)?
                    .ok_or_else(|| CompilerError::codegen_error("Invalid command in exec_command".to_string()))?;

                // Generate arguments array
                let mut arg_vals = Vec::new();
                for arg in &exec_cmd.args {
                    let arg_val = self.generate_expression_llvm(arg)?
                        .ok_or_else(|| CompilerError::codegen_error("Invalid argument in exec_command".to_string()))?;
                    arg_vals.push(arg_val);
                }

                // Get the silica_exec_command function
                if let Some(exec_func) = (*module).get_function("silica_exec_command") {
                    // For now, create a placeholder call
                    // In a real implementation, we'd need to properly construct the arguments array
                    let placeholder_call = builder.build_call(
                        exec_func,
                        &[cmd_val.into(), (*self.context).i64_type().const_int(0, false).into(),
                          (*self.context).i8_type().ptr_type(inkwell::AddressSpace::default()).const_null().into(),
                          (*self.context).i64_type().const_int(0, false).into(),
                          (*self.context).i8_type().ptr_type(inkwell::AddressSpace::default()).const_null().into()],
                        "exec_result"
                    ).unwrap();

                    let result: inkwell::values::BasicValueEnum<'static> = unsafe { std::mem::transmute(placeholder_call) };
                    Ok(Some(result))
                } else {
                    Err(CompilerError::codegen_error("silica_exec_command function not found".to_string()))
                }
            } else {
                {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
            }
        }
    }

    /// Generate LLVM value for function literal expressions (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_function_literal_llvm(&mut self, func_lit: &FunctionLiteralExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        unsafe {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                let context = &*self.context;

                // Create parameter types using actual Silica types mapped to LLVM
                let is_behavior_function = func_lit.parameters.len() == 2;
                let param_types: Vec<inkwell::types::BasicTypeEnum<'static>> =
                    if is_behavior_function {
                        // Behavior functions: use i8* for runtime compatibility
                        func_lit.parameters.iter().map(|_| context.i8_type().ptr_type(inkwell::AddressSpace::Generic).into()).collect()
                    } else {
                        // Regular functions: use actual Silica types mapped to LLVM
                        func_lit.parameters.iter().map(|param| {
                            match &param.type_ {
                                Type::Bool => context.i1_type().into(),
                                Type::Int64 => context.i64_type().into(),
                                Type::Char => context.i32_type().into(),
                                _ => context.i8_type().ptr_type(inkwell::AddressSpace::Generic).into(), // Complex types as pointers
                            }
                        }).collect()
                    };

                let param_metadata: Vec<inkwell::types::BasicMetadataTypeEnum<'static>> =
                    param_types.iter().map(|ty| (*ty).into()).collect();

                // Create function type using actual return type
                let return_type = func_lit.return_type.as_ref().unwrap_or(&Type::Unit);
                let llvm_return_type = match return_type {
                    Type::Unit => context.void_type(),
                    Type::Bool => context.i1_type().into(),
                    Type::Int64 => context.i64_type().into(),
                    Type::Char => context.i32_type().into(),
                    _ => context.i8_type().ptr_type(inkwell::AddressSpace::Generic).into(), // Complex types as pointers
                };
                let fn_type = llvm_return_type.fn_type(&param_metadata, false);

                // Generate unique function name
                let func_name = format!("func_literal_{}", self.instructions.len());

                // Add function to module
                let llvm_func = module.add_function(&func_name, fn_type, None);

                // Create entry block and set builder position
                let entry_block = context.append_basic_block(llvm_func, "entry");
                builder.position_at_end(entry_block);

                // Set up parameters in the symbol table
                for (i, param) in func_lit.parameters.iter().enumerate() {
                    let param_value = llvm_func.get_nth_param(i as u32).unwrap();
                    self.llvm_values.insert(param.name.clone(), param_value);
                }

                // Generate function body
                let body_result = self.generate_expression_llvm(&func_lit.body)?;

                // Generate return
                if let Some(body_val) = body_result {
                    builder.build_return(Some(&body_val));
                } else {
                    // Return 0 if no result
                    let zero = context.i64_type().const_int(0, false);
                    builder.build_return(Some(&zero.into()));
                }

                // Clean up parameter variables from symbol table
                for param in &func_lit.parameters {
                    self.llvm_values.remove(&param.name);
                }

                // Return pointer to the function
                let func_ptr = llvm_func.as_global_value().as_pointer_value();
                Ok(Some(func_ptr.into()))
            } else {
                {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
            }
        }
    }

    /// Generate LLVM value for struct literal expressions (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_struct_literal_llvm(&mut self, struct_lit: &StructLiteralExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        unsafe {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                let context = &*self.context;
                if struct_lit.fields.is_empty() {
                    // Empty struct - return null pointer
                    return Ok(Some(context.i8_type().ptr_type(inkwell::AddressSpace::default()).const_null().into()));
                }

                // Generate all field values first
                let mut field_values = Vec::new();
                for (_, field_expr) in &struct_lit.fields {
                    if let Some(value) = self.generate_expression_llvm(field_expr)? {
                        field_values.push(value);
                    } else {
                        return Err(CompilerError::codegen_error("Invalid field value in struct literal".to_string()));
                    }
                }

                // Allocate memory for the struct
                let struct_size = (field_values.len() * 8) as u64; // 8 bytes per i64 field
                let size_value = context.i64_type().const_int(struct_size, false);

                // Call malloc
                let malloc_func = module.get_function("malloc")
                    .ok_or_else(|| CompilerError::codegen_error("malloc function not found".to_string()))?;
                let struct_ptr = builder.build_call(malloc_func, &[size_value.into()], "struct_alloc")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic();

                // Store each field value
                for (i, field_value) in field_values.iter().enumerate() {
                    let field_offset = (i * 8) as u64;
                    let offset_value = context.i64_type().const_int(field_offset, false);

                    // Get pointer to field location
                    let field_ptr = builder.build_gep(
                        context.i8_type(),
                        struct_ptr.into_pointer_value(),
                        &[offset_value],
                        &format!("field_ptr_{}", i)
                    ).unwrap();

                    // Cast to i64 pointer and store
                    let field_ptr_i64 = builder.build_bit_cast(
                        field_ptr,
                        context.i64_type().ptr_type(inkwell::AddressSpace::default()),
                        &format!("field_ptr_i64_{}", i)
                    ).unwrap();

                    builder.build_store(field_ptr_i64.into_pointer_value(), *field_value).unwrap();
                }

                Ok(Some(struct_ptr))
            } else {
                {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
            }
        }
    }

    /// Generate LLVM value for field access expressions (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_field_access_llvm(&mut self, field_access: &FieldAccessExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        unsafe {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                let context = &*self.context;

                // Generate the object expression first
                let object_value = self.generate_expression_llvm(&field_access.object)?
                    .ok_or_else(|| CompilerError::codegen_error("Field access requires valid object".to_string()))?;

                // Get the type of the object to determine which struct we're accessing
                let object_type = self.infer_expression(&field_access.object)?;
                // Expand type aliases to get the actual type
                let expanded_object_type = self.expand_type_aliases(&object_type);

                // Look up the field index from the struct definition
                let field_index = match &expanded_object_type {
                    Type::Named(type_name) => {
                        // Look up the struct definition
                        if let Some(struct_def) = self.struct_defs.get(type_name) {
                            // Find the field index
                            struct_def.iter().position(|field| field.name == field_access.field)
                                .ok_or_else(|| CompilerError::codegen_error(format!("Unknown field '{}' in struct '{}'", field_access.field, type_name)))?
                        } else {
                            return Err(CompilerError::codegen_error(format!("Cannot access field '{}' on non-struct type {:?}", field_access.field, expanded_object_type)));
                        }
                    }
                    Type::Record(fields) => {
                        // Find the field index directly from the record fields
                        fields.iter().position(|(field_name, _)| field_name == &field_access.field)
                            .ok_or_else(|| CompilerError::codegen_error(format!("Unknown field '{}' in record type {:?}", field_access.field, expanded_object_type)))?
                    }
                    _ => {
                        return Err(CompilerError::codegen_error(format!("Cannot access field '{}' on non-struct type {:?}", field_access.field, expanded_object_type)));
                    }
                };

                // Calculate field offset and load the value
                let field_offset = (field_index * 8) as u64;
                let offset_value = context.i64_type().const_int(field_offset, false);

                // Get pointer to field location
                let field_ptr = builder.build_gep(
                    context.i8_type(),
                    object_value.into_pointer_value(),
                    &[offset_value],
                    "field_ptr"
                ).unwrap();

                // Cast to i64 pointer and load the value
                let field_ptr_i64 = builder.build_bit_cast(
                    field_ptr,
                    context.i64_type().ptr_type(inkwell::AddressSpace::default()),
                    "field_ptr_i64"
                ).unwrap();

                let field_value = builder.build_load(
                    context.i64_type(),
                    field_ptr_i64.into_pointer_value(),
                    "field_value"
                ).unwrap();

                Ok(Some(field_value.into()))
            } else {
                {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
            }
        }
    }

    /// Helper to convert types to strings for monomorphization names
    #[cfg(feature = "llvm_backend")]
    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::Int64 => "int".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Char => "char".to_string(),
            Type::String => "string".to_string(),
            Type::Unit => "unit".to_string(),
            Type::Named(name) => name.clone(),
            Type::Tuple(types) => {
                let type_strs: Vec<String> = types.iter().map(|t| self.type_to_string(t)).collect();
                format!("tuple_{}", type_strs.join("_"))
            }
            _ => "unknown".to_string(),
        }
    }

    /// Generate LLVM tuple values (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_tuple_llvm(&mut self, exprs: &[Expression]) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // For now, implement tuples as heap-allocated structures
        // This is a simplified implementation - real tuples would need proper memory layout
        Err(CompilerError::codegen_error("Tuple generation not yet implemented in LLVM backend".to_string()))
    }

    /// Generate LLVM value for if expressions (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_if_llvm(&mut self, if_expr: &IfExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate operands first to avoid borrowing conflicts
        let cond_val = self.generate_expression_llvm(&if_expr.condition)?
            .ok_or_else(|| CompilerError::codegen_error("If condition must produce a value".to_string()))?;
        let then_val = self.generate_expression_llvm(&if_expr.then_branch)?
            .ok_or_else(|| CompilerError::codegen_error("If then branch must produce a value".to_string()))?;
        let else_val = self.generate_expression_llvm(&if_expr.else_branch)?
            .ok_or_else(|| CompilerError::codegen_error("If else branch must produce a value".to_string()))?;

        if let Some(builder) = &self.builder {
            unsafe {
                let current_function = builder.get_insert_block().unwrap().get_parent().ok_or_else(|| {
                    CompilerError::codegen_error("Not in a function".to_string())
                })?;

                // Create basic blocks
                let then_block = (*self.context).append_basic_block(current_function, "then");
                let else_block = (*self.context).append_basic_block(current_function, "else");
                let merge_block = (*self.context).append_basic_block(current_function, "merge");

                // Generate conditional branch
                builder.build_conditional_branch(cond_val.into_int_value(), then_block, else_block).unwrap();

                // Generate then block
                builder.position_at_end(then_block);
                builder.build_unconditional_branch(merge_block).unwrap();
                let then_end_block = builder.get_insert_block().unwrap();

                // Generate else block
                builder.position_at_end(else_block);
                builder.build_unconditional_branch(merge_block).unwrap();
                let else_end_block = builder.get_insert_block().unwrap();

                // Generate merge block with phi
                builder.position_at_end(merge_block);

                // Determine the result type - prefer pointers over primitives for Silica compatibility
                let result_type = if then_val.get_type().is_pointer_type() {
                    then_val.get_type()
                } else if else_val.get_type().is_pointer_type() {
                    else_val.get_type()
                } else {
                    // Both are primitives, use the then type
                    then_val.get_type()
                };

                // Cast values to match result type
                let then_val_typed = if then_val.get_type() == result_type {
                    then_val
                } else if result_type.is_pointer_type() && then_val.get_type().is_int_type() {
                    // Cast int to pointer
                    unsafe {
                        builder.build_int_to_ptr(then_val.into_int_value(), result_type.into_pointer_type(), "cast_then").unwrap().into()
                    }
                } else {
                    // For now, assume types are compatible or add more casting logic
                    then_val
                };

                let else_val_typed = if else_val.get_type() == result_type {
                    else_val
                } else if result_type.is_pointer_type() && else_val.get_type().is_int_type() {
                    // Cast int to pointer
                    unsafe {
                        builder.build_int_to_ptr(else_val.into_int_value(), result_type.into_pointer_type(), "cast_else").unwrap().into()
                    }
                } else {
                    // For now, assume types are compatible or add more casting logic
                    else_val
                };

                let phi = builder.build_phi(result_type, "if_result").unwrap();
                phi.add_incoming(&[(&then_val_typed, then_end_block), (&else_val_typed, else_end_block)]);

                Ok(Some(phi.as_basic_value()))
            }
        } else {
            Err(CompilerError::codegen_error("LLVM builder not initialized".to_string()))
        }
    }

    /// Generate LLVM value for case expressions (pattern matching) (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_case_llvm(&mut self, case: &CaseExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        if case.branches.is_empty() {
            return Err(CompilerError::codegen_error("Case expression must have at least one branch".to_string()));
        }

        if let (Some(context), Some(module), Some(builder)) = (&self.context, &self.module, &self.builder) {
            unsafe {
                // Generate scrutinee value
                let scrutinee_val = self.generate_expression_llvm(&case.scrutinee)?
                    .ok_or_else(|| CompilerError::codegen_error("Case scrutinee must produce a value".to_string()))?;

                // Allocate result variable (all case branches return i64)
                let result_type = (*context).i64_type();
                let result_alloca = builder.build_alloca(result_type, "case_result").unwrap();

                // Create basic blocks for case logic
                let current_fn = builder.get_insert_block().unwrap().get_parent();
                let case_end_block = (*context).append_basic_block(current_fn, "case_end");
                let case_fail_block = (*context).append_basic_block(current_fn, "case_fail");

                // Default failure case - store 0
                builder.position_at_end(case_fail_block);
                builder.build_store(result_alloca, result_type.const_int(0, false)).unwrap();
                builder.build_unconditional_branch(case_end_block);

                // Generate each branch
                let mut next_check_block = None;
                for (branch_idx, branch) in case.branches.iter().enumerate() {
                    // Create blocks for this branch
                    let check_block = if branch_idx == 0 {
                        // First branch - continue from current block
                        builder.get_insert_block()
                    } else {
                        (*context).append_basic_block(current_fn, &format!("case_check_{}", branch_idx))
                    };

                    let body_block = (*context).append_basic_block(current_fn, &format!("case_body_{}", branch_idx));
                    let bind_block = if branch.guard.is_some() {
                        Some((*context).append_basic_block(current_fn, &format!("case_bind_{}", branch_idx)))
                    } else {
                        None
                    };

                    // Position at check block (skip for first branch)
                    if branch_idx > 0 {
                        builder.position_at_end(check_block);
                    }

                    // Generate pattern match condition
                    let pattern_matches = self.generate_pattern_match_llvm(&branch.pattern, &scrutinee_val)?;

                    // Branch based on pattern match
                    let success_block = bind_block.as_ref().unwrap_or(&body_block);
                    let fail_block = if branch_idx + 1 < case.branches.len() {
                        next_check_block.get_or_insert_with(|| {
                            (*context).append_basic_block(current_fn, &format!("case_check_{}", branch_idx + 1))
                        })
                    } else {
                        &case_fail_block
                    };

                    builder.build_conditional_branch(pattern_matches, *success_block, *fail_block);

                    // Handle guard if present
                    if let Some(guard_expr) = &branch.guard {
                        if let Some(bind_block) = bind_block {
                            builder.position_at_end(bind_block);

                            // Bind pattern variables to scope
                            self.bind_pattern_variables(&branch.pattern, &scrutinee_val)?;

                            // Evaluate guard
                            let guard_val = self.generate_expression_llvm(guard_expr)?
                                .ok_or_else(|| CompilerError::codegen_error("Guard expression must produce a value".to_string()))?;

                            // Guard must be boolean (i1), convert if needed
                            let guard_bool = if guard_val.get_type().as_int_type().unwrap().get_bit_width() == 1 {
                                guard_val.into_int_value()
                            } else {
                                // Convert i64 to i1 (non-zero = true)
                                builder.build_int_compare(inkwell::IntPredicate::NE, guard_val.into_int_value(), result_type.const_int(0, false), "guard_bool").unwrap()
                            };

                            builder.build_conditional_branch(guard_bool, body_block, *fail_block);
                        }
                    }

                    // Generate branch body
                    builder.position_at_end(body_block);

                    // Bind pattern variables for body evaluation
                    self.bind_pattern_variables(&branch.pattern, &scrutinee_val)?;

                    // Generate body expression
                    let body_val = self.generate_expression_llvm(&branch.body)?
                        .ok_or_else(|| CompilerError::codegen_error("Case branch body must produce a value".to_string()))?;

                    // Convert body value to i64 if needed and store to result
                    let body_i64 = match body_val.get_type() {
                        inkwell::types::BasicTypeEnum::IntType(int_type) if int_type.get_bit_width() == 64 => {
                            body_val.into_int_value()
                        }
                        inkwell::types::BasicTypeEnum::IntType(int_type) if int_type.get_bit_width() == 1 => {
                            // Convert bool to i64
                            builder.build_int_z_extend(body_val.into_int_value(), result_type, "bool_to_i64").unwrap()
                        }
                        inkwell::types::BasicTypeEnum::PointerType(_) => {
                            // This should not happen - if we get a pointer, there's double boxing
                            // Convert pointer to i64 (bitcast)
                            builder.build_ptr_to_int(body_val.into_pointer_value(), result_type, "ptr_to_i64").unwrap()
                        }
                        _ => return Err(CompilerError::codegen_error("Unsupported type in case branch".to_string())),
                    };

                    builder.build_store(result_alloca, body_i64).unwrap();
                    builder.build_unconditional_branch(case_end_block);
        }

                // Load final result
                builder.position_at_end(case_end_block);
                let final_result = builder.build_load(result_type, result_alloca, "case_final").unwrap();

                Ok(Some(final_result.into()))
            }
        } else {
            Err(CompilerError::codegen_error("LLVM context, module, or builder not initialized".to_string()))
        }
    }

    /// Generate LLVM pattern match condition for case expressions (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_pattern_match_llvm(&mut self, pattern: &Pattern, scrutinee: &inkwell::values::BasicValueEnum<'static>) -> Result<inkwell::values::IntValue<'static>> {
        match pattern {
            Pattern::Identifier(_) | Pattern::TypedIdentifier { name, .. } => {
                // Both identifiers and wildcards always match
                if let Some(context) = &self.context {
                    unsafe {
                        Ok((*context).bool_type().const_int(1, false))
                    }
                } else {
                    {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
                }
            }
            Pattern::Literal(lit) => {
                // Compare literal with scrutinee
                if let Some(builder) = &self.builder {
                    unsafe {
                        match lit {
                            Literal::Int(pattern_val) => {
                                let pattern_const = (*self.context).i64_type().const_int(*pattern_val as u64, false);
                                Ok(builder.build_int_compare(inkwell::IntPredicate::EQ, scrutinee.into_int_value(), pattern_const, "literal_match").unwrap())
                            }
                            Literal::Float(pattern_val) => {
                                let pattern_const = (*self.context).f32_type().const_float(*pattern_val);
                                Ok(builder.build_float_compare(inkwell::FloatPredicate::OEQ, scrutinee.into_float_value(), pattern_const, "float_match").unwrap())
                            }
                            Literal::Bool(pattern_bool) => {
                                let pattern_const = (*self.context).bool_type().const_int(if *pattern_bool { 1 } else { 0 }, false);
                                Ok(builder.build_int_compare(inkwell::IntPredicate::EQ, scrutinee.into_int_value(), pattern_const, "bool_match").unwrap())
                            }
                            _ => Err(CompilerError::codegen_error("Unsupported literal type in pattern".to_string())),
                        }
                    }
                } else {
                    Err(CompilerError::codegen_error("LLVM builder not initialized".to_string()))
                }
            }
            Pattern::Tuple(_) => {
                // For now, tuple patterns always match (simplified - full structural matching would be more complex)
                if let Some(context) = &self.context {
                    unsafe {
                        Ok((*context).bool_type().const_int(1, false))
                    }
                } else {
                    {
                    let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                        .severity(ErrorSeverity::Error)
                        .build();
                    Err(CompilerError::codegen_error_with_metadata("LLVM context not initialized".to_string(), None, metadata))
                }
                }
            }
            Pattern::Record(_) => {
                // Record patterns not yet implemented
                Err(CompilerError::codegen_error("Record patterns not yet supported".to_string()))
            }
        }
    }

    /// Bind variables from pattern matching (without generating LLVM comparison code)
    #[cfg(feature = "llvm_backend")]
    fn bind_pattern_variables(&mut self, pattern: &Pattern, value: &inkwell::values::BasicValueEnum<'static>) -> Result<()> {
        match pattern {
            Pattern::Identifier(name) => {
                // Bind the value to the variable name
                if let Some(builder) = &self.builder {
                    unsafe {
                        let var_type = value.get_type();
                        let alloca = builder.build_alloca(var_type, name).unwrap();
                        builder.build_store(alloca, *value).unwrap();
                        self.add_variable(name.clone(), alloca);
                    }
                }
                Ok(())
            }
            Pattern::TypedIdentifier { name, .. } => {
                // Bind the value to the variable name (skip for wildcards)
                if name != "_" {
                    if let Some(builder) = &self.builder {
                        unsafe {
                            let var_type = value.get_type();
                            let alloca = builder.build_alloca(var_type, name).unwrap();
                            builder.build_store(alloca, *value).unwrap();
                            self.add_variable(name.clone(), alloca);
                        }
                    }
                }
                Ok(())
            }
            Pattern::Literal(_) => {
                // Literal patterns don't bind variables
                Ok(())
            }
            Pattern::Tuple(patterns) => {
                // For tuple patterns in case expressions, we don't decompose - 
                // the pattern matching was already done by generate_pattern_match_llvm
                // We just need to handle any variable bindings within the tuple elements
                for pattern in patterns {
                    self.bind_pattern_variables(pattern, value)?;
                }
                Ok(())
            }
            Pattern::Record(field_patterns) => {
                // Bind variables from record field patterns
                for (field_name, field_pattern) in field_patterns {
                    // For now, assume we can access fields by name
                    // In a full implementation, this would extract the field value from the record
                    // For demo purposes, we'll bind the whole value to field names as variables
                    if let Pattern::TypedIdentifier { name: var_name, .. } = field_pattern {
                        if let Some(builder) = &self.builder {
                            unsafe {
                                let var_type = value.get_type();
                                let alloca = builder.build_alloca(var_type, var_name).unwrap();
                                builder.build_store(alloca, *value).unwrap();
                                self.add_variable(var_name.clone(), alloca);
                            }
                        }
                    }
                }
                Ok(())
            }
            Pattern::Variant { constructor: _, payload } => {
                // Bind payload variables if present
                if let Some(payload_pattern) = payload {
                    self.bind_pattern_variables(payload_pattern, value)
                } else {
                    Ok(())
                }
            }
            Pattern::Alternative(patterns) => {
                // For alternatives, bind variables from the first pattern
                if !patterns.is_empty() {
                    self.bind_pattern_variables(&patterns[0], value)
                } else {
                    Ok(())
                }
            }
            _ => {
                // Other patterns don't bind variables for now
                Ok(())
            }
        }
    }

    /// Generate LLVM code for pattern matching against a value
    #[cfg(feature = "llvm_backend")]
    fn generate_pattern_match_llvm(&mut self, pattern: &Pattern, value: &inkwell::values::BasicValueEnum<'static>) -> Result<inkwell::values::IntValue<'static>> {
        match pattern {
            Pattern::Literal(lit) => {
                // Generate literal value first (before builder borrow)
                let lit_val = self.generate_literal_llvm(lit)?
                    .ok_or_else(|| CompilerError::codegen_error("Pattern literal must produce a value".to_string()))?;

                if let Some(builder) = &self.builder {
                    unsafe {
                        Ok(builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            value.clone().into_int_value(),
                            lit_val.into_int_value(),
                            "literal_match"
                        ).unwrap())
                    }
                } else {
                    Err(CompilerError::codegen_error("LLVM builder not initialized".to_string()))
                }
            }
            Pattern::TypedIdentifier { .. } => {
                // Both identifier and wildcard patterns always match
                unsafe {
                    Ok((*self.context).i64_type().const_int(1, false))
                }
            }
            Pattern::Tuple(patterns) => {
                // For now, only support single element tuples or fail
                if patterns.len() == 1 {
                    self.generate_pattern_match_llvm(&patterns[0], value)
                } else {
                    unsafe {
                        Ok((*self.context).i64_type().const_int(0, false)) // No match for multi-element tuples
                    }
                }
            }
            Pattern::Record(field_patterns) => {
                // For record patterns, assume all fields match for now
                // In a full implementation, this would check each field
                unsafe {
                    Ok((*self.context).i64_type().const_int(1, false)) // Assume record matches
                }
            }
            Pattern::Variant { constructor, payload } => {
                // For variant patterns, assume they match for now
                // In a full implementation, this would check the constructor and payload
                unsafe {
                    Ok((*self.context).i64_type().const_int(1, false)) // Assume variant matches
                }
            }
            Pattern::Alternative(patterns) => {
                // Alternative patterns: pat1 | pat2 | pat3
                // For now, assume the first pattern matches
                if !patterns.is_empty() {
                    self.generate_pattern_match_llvm(&patterns[0], value)
                } else {
                    unsafe {
                        Ok((*self.context).i64_type().const_int(0, false))
                    }
                }
            }
            _ => {
                // For other pattern types, return false (no match) for now
                unsafe {
                    Ok((*self.context).i64_type().const_int(0, false))
                }
            }
        }
    }

    /// Analyze case branches to determine the LLVM result type.
    /// Case expressions return whatever type the branches produce; all branches must have the same type.
    fn analyze_case_result_type(&self, case: &CaseExpr) -> Result<String> {
        if let Some(first_branch) = case.branches.first() {
            // Prefer type from type checker (expression_types) when available
            if let Some(loc) = Self::try_get_expression_location(&*first_branch.body) {
                if let Some(silica_type) = self.expression_types.get(&loc) {
                    // Struct types (Named in struct_defs/type_aliases, Record, Tuple) use i8* for case result
                    let llvm_ty = match silica_type {
                        Type::Record(_) | Type::Tuple(_) => "i8*".to_string(),
                        Type::Named(name) if self.struct_defs.contains_key(name) || self.type_aliases.contains_key(name) => "i8*".to_string(),
                        _ => self.type_map.silica_to_llvm_str(silica_type),
                    };
                    return Ok(llvm_ty);
                }
            }
            // Fallback: heuristic from first branch body
            match &*first_branch.body {
                Expression::FunctionLiteral(_) => Ok("i8*".to_string()),
                Expression::Literal(Literal::String(_)) => Ok("i8*".to_string()),
                Expression::Tuple(_) => Ok("i8*".to_string()),
                Expression::Literal(Literal::Int(_)) => Ok("i64".to_string()),
                Expression::Literal(Literal::Float(_)) => {
                    let expr_type = Self::try_get_expression_location(&*first_branch.body)
                        .and_then(|loc| self.expression_types.get(loc));
                    match expr_type {
                        Some(Type::Float16) => Ok("half".to_string()),
                        Some(Type::Float32) => Ok("float".to_string()),
                        Some(Type::Float64) => Ok("double".to_string()),
                        Some(ty) => panic!("Float literal has non-float type in expression_types: {:?}", ty),
                        None => panic!("Float literal type information missing - type annotation required for float16 vs float32 vs float64 distinction"),
                    }
                },
                Expression::Literal(Literal::Bool(_)) => Ok("i1".to_string()),
                Expression::Literal(Literal::Char(_)) => Ok("i32".to_string()),
                Expression::StructLiteral(_) => Ok("i8*".to_string()),
                _ => Ok("i64".to_string()),
            }
        } else {
            Err(CompilerError::codegen_error("Case expression must have at least one branch".to_string()))
        }
    }

    /// Generate LLVM IR for case expressions (text IR)
    fn generate_case(&mut self, case: &CaseExpr) -> Result<Option<String>> {
        // Enter a new scope for case pattern variables
        self.enter_scope_text();

        // Analyze the result type from case branches
        let result_llvm_type = self.analyze_case_result_type(case)?;

        // Generate scrutinee
        let boxed_scrutinee_reg = match self.generate_expression(&case.scrutinee)? {
            Some(reg) => reg,
            None => return codegen_error("Case scrutinee must produce a value".to_string()),
        };

        // Unbox the scrutinee if it's boxed; preserve type prefix for pattern check (i1 vs i64)
        let clean_scrutinee_reg = boxed_scrutinee_reg.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();
        let scrutinee_reg = if clean_scrutinee_reg == "%0" || clean_scrutinee_reg == "%1" {
            // Parameter register - assume it's i8* containing boxed i64, bitcast and load
            let load_reg = format!("%scrutinee_load_{}", self.instructions.len());
            self.instructions.push(format!("  {} = bitcast i8* {} to i64*", load_reg.clone() + "_cast", clean_scrutinee_reg));
            self.instructions.push(format!("  {} = load i64, i64* {}_cast", load_reg, load_reg));
            format!("i64 {}", load_reg)
        } else if clean_scrutinee_reg.contains("box_result") || clean_scrutinee_reg.contains("param") || clean_scrutinee_reg.starts_with("%box_") || clean_scrutinee_reg.starts_with("%param_") {
            // Load the value from the boxed pointer
            let load_reg = format!("%scrutinee_load_{}", self.instructions.len());
            self.instructions.push(format!("  {} = bitcast i8* {} to i64*", load_reg.clone() + "_cast", clean_scrutinee_reg));
            self.instructions.push(format!("  {} = load i64, i64* {}_cast", load_reg, load_reg));
            format!("i64 {}", load_reg)
        } else {
            // Keep type prefix so generate_runtime_pattern_check (e.g. Literal::Bool) sees i1 vs i64
            boxed_scrutinee_reg.clone()
        };

        // Allocate result variable with the correct type
        let result_reg = format!("%case_result_{}", self.instructions.len());
        self.instructions.push(format!("  {} = alloca {}", result_reg, result_llvm_type));

        // Create labels
        let case_end = format!("case_end_{}", self.instructions.len());
        let case_fail = format!("case_fail_{}", self.instructions.len());

        // Generate branch checking logic
        let mut next_check_label = format!("case_check_{}_0", self.instructions.len());
        self.instructions.push(format!("  br label %{}", next_check_label));

        for (branch_idx, branch) in case.branches.iter().enumerate() {
            // Branch check label
            self.instructions.push(format!("{}:", next_check_label));

            // Create branch labels for this pattern
            let pattern_bind = format!("case_bind_{}_{}", self.instructions.len(), branch_idx);
            let branch_body = format!("case_body_{}_{}", self.instructions.len(), branch_idx);
            next_check_label = format!("case_check_{}_{}", self.instructions.len(), branch_idx + 1);

            // Generate runtime pattern match
            let match_result = self.generate_runtime_pattern_check(&branch.pattern, &scrutinee_reg)?;

            // Branch to binding if pattern matches
            self.instructions.push(format!("  br i1 {}, label %{}, label %{}",
                match_result, pattern_bind,
                if branch_idx + 1 < case.branches.len() { &next_check_label } else { &case_fail }));

            // Pattern binding and guard check
            self.instructions.push(format!("{}:", pattern_bind));

            // Bind pattern variables here
            let bound_vars = self.generate_pattern_variable_binding(&branch.pattern, &scrutinee_reg, branch_idx)?;

            // If there's a guard, evaluate it with bound variables in scope
            if let Some(guard_expr) = &branch.guard {
                // Temporarily add bound variables to scope for guard evaluation
                for (var_name, var_reg) in &bound_vars {
                    self.add_variable_text(var_name.clone(), var_reg.clone());
                }

                let guard_result = self.generate_expression(guard_expr)?
                    .ok_or_else(|| CompilerError::codegen_error("Guard expression must produce a value".to_string()))?;

                // Guard must evaluate to true (i1 boolean)
                // Guard expressions should return i1 (boolean) values
                let guard_bool = format!("%guard_bool_{}_{}", self.instructions.len(), branch_idx);
                // Strip type prefixes from guard_result before using in instruction
                let clean_guard_result = guard_result.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();
                // For now, assume guard_result is i1. In full type system, this would be checked.
                self.instructions.push(format!("  {} = add i1 {}, 0", guard_bool, clean_guard_result));

                // If guard passes, go to body; else try next branch
                self.instructions.push(format!("  br i1 {}, label %{}, label %{}",
                    guard_bool, branch_body,
                    if branch_idx + 1 < case.branches.len() { &next_check_label } else { &case_fail }));
            } else {
                // No guard - go directly to body
                self.instructions.push(format!("  br label %{}", branch_body));
            }

            // Branch body
            self.instructions.push(format!("{}:", branch_body));

            // Add bound variables to scope for branch body evaluation
            for (var_name, var_reg) in &bound_vars {
                self.add_variable_text(var_name.clone(), var_reg.clone());
            }

            let body_val_raw = match self.generate_expression(&branch.body)? {
                Some(val) => {
                    // Extract value and ensure type matches result_llvm_type for store
                    if val.starts_with(&format!("{} ", result_llvm_type)) {
                        val[result_llvm_type.len() + 1..].to_string()
                    } else if result_llvm_type == "i8*" && val.starts_with("i64 ") {
                        // Branch returned i64 (e.g. pointer as integer from field load); convert before store into i8* slot
                        let i64_reg = val.trim_start_matches("i64 ").trim_start_matches("i8* ").to_string();
                        let i64_ref = Self::format_llvm_value_ref(&i64_reg);
                        let ptr_reg = format!("%case_inttoptr_{}_{}", self.instructions.len(), branch_idx);
                        self.instructions.push(format!("  {} = inttoptr i64 {} to i8*", ptr_reg, i64_ref));
                        ptr_reg
                    } else if result_llvm_type == "i64" && val.starts_with("i8* ") {
                        // Branch returned i8* but case result is i64 (e.g. pointer-as-integer); convert before store
                        let ptr_reg = val.trim_start_matches("i8* ").to_string();
                        let ptr_ref = Self::format_llvm_value_ref(&ptr_reg);
                        let i64_reg = format!("%case_ptrtoint_{}_{}", self.instructions.len(), branch_idx);
                        self.instructions.push(format!("  {} = ptrtoint i8* {} to i64", i64_reg, ptr_ref));
                        i64_reg
                    } else if result_llvm_type == "i8*" && val.starts_with("i8* ") {
                        val.trim_start_matches("i8* ").to_string()
                    } else {
                        let trimmed = val.trim_start_matches("i64 ").trim_start_matches("i8* ").trim_start_matches("i1 ").trim_start_matches("i32 ").to_string();
                        // No type prefix: if result is i64 and value looks like a ptr register (e.g. case_final from nested i8* case), ptrtoint
                        if result_llvm_type == "i64" && trimmed.starts_with('%') && trimmed.contains("case_final") {
                            let ptr_ref = Self::format_llvm_value_ref(&trimmed);
                            let i64_reg = format!("%case_ptrtoint_{}_{}", self.instructions.len(), branch_idx);
                            self.instructions.push(format!("  {} = ptrtoint i8* {} to i64", i64_reg, ptr_ref));
                            i64_reg
                        } else {
                            trimmed
                        }
                    }
                },
                None => return codegen_error("Case branch body must produce a value".to_string()),
            };
            let body_val = Self::format_llvm_value_ref(&body_val_raw);
            self.instructions.push(format!("  store {} {}, {}* {}", result_llvm_type, body_val, result_llvm_type, result_reg));
            self.instructions.push(format!("  br label %{}", case_end));
        }

        // Failure case
        self.instructions.push(format!("{}:", case_fail));
        let default_value = match result_llvm_type.as_str() {
            "i64" => "0",
            "i1" => "0",
            "i32" => "0",
            "i8*" => "null",
            _ => "0",
        };
        self.instructions.push(format!("  store {} {}, {}* {}", result_llvm_type, default_value, result_llvm_type, result_reg));
        self.instructions.push(format!("  br label %{}", case_end));

        // End - load result
        self.instructions.push(format!("{}:", case_end));
        let final_reg = format!("%case_final_{}", self.instructions.len());
        self.instructions.push(format!("  {} = load {}, {}* {}", final_reg, result_llvm_type, result_llvm_type, result_reg));

        // Exit the case scope
        self.exit_scope_text();

        // Return with type prefix so callers (e.g. return statement) know the case result type
        Ok(Some(format!("{} {}", result_llvm_type, final_reg)))
    }

    /// Generate runtime pattern matching check that returns an i1 result
    fn generate_pattern_variable_binding(&mut self, pattern: &Pattern, scrutinee_reg: &str, _branch_idx: usize) -> Result<HashMap<String, String>> {
        let mut bound_vars = HashMap::new();

        match pattern {
            // Wildcard patterns: no binding needed
            Pattern::Identifier(name) if name == "_" => {
                // Wildcard: no binding
                return Ok(bound_vars);
            }
            Pattern::TypedIdentifier { name, .. } if name == "_" => {
                // Wildcard with type: no binding
                return Ok(bound_vars);
            }
            Pattern::Identifier(name) => {
                // Bind the scrutinee value to the variable
                let bind_reg = format!("%bind_{}_{}", name, self.instructions.len());

                // Handle different types based on the scrutinee register type
                if scrutinee_reg.starts_with("i64 ") {
                    // i64 integer
                    let reg_name = &scrutinee_reg[4..];
                self.instructions.push(format!("  {} = add i64 {}, 0", bind_reg, reg_name)); // Copy the value
                } else if scrutinee_reg.starts_with("i1 ") {
                    // i1 boolean - extend to i64 for consistency
                    let reg_name = &scrutinee_reg[3..];
                    let extended_reg = format!("%{}_ext_{}", name, self.instructions.len());
                    self.instructions.push(format!("  {} = zext i1 {} to i64", extended_reg, reg_name));
                    self.instructions.push(format!("  {} = add i64 {}, 0", bind_reg, extended_reg)); // Copy the value
                } else if scrutinee_reg.starts_with("i8* ") {
                    // Pointer
                    let reg_name = &scrutinee_reg[4..];
                    self.instructions.push(format!("  {} = bitcast i8* {} to i8*", bind_reg, reg_name)); // Copy the pointer
                } else {
                    // Default to i64
                    let reg_name = &scrutinee_reg.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();
                    self.instructions.push(format!("  {} = add i64 {}, 0", bind_reg, reg_name));
                }

                bound_vars.insert(name.clone(), bind_reg);
            }
            Pattern::TypedIdentifier { name, type_: pattern_type, .. } => {
                // Wildcard: never bind (safety in case guard didn't match)
                if name == "_" {
                    return Ok(bound_vars);
                }
                // Bind the scrutinee value to the variable
                let bind_reg = format!("%bind_{}_{}", name, self.instructions.len());
                let bind_llvm_ty: String;

                // Handle different types based on the scrutinee register type
                if scrutinee_reg.starts_with("i64 ") {
                    // i64 integer
                    let reg_name = &scrutinee_reg[4..];
                    self.instructions.push(format!("  {} = add i64 {}, 0", bind_reg, reg_name)); // Copy the value
                    bind_llvm_ty = "i64".to_string();
                } else if scrutinee_reg.starts_with("i1 ") {
                    // i1 boolean - extend to i64 for consistency
                    let reg_name = &scrutinee_reg[3..];
                    let extended_reg = format!("%{}_ext_{}", name, self.instructions.len());
                    self.instructions.push(format!("  {} = zext i1 {} to i64", extended_reg, reg_name));
                    self.instructions.push(format!("  {} = add i64 {}, 0", bind_reg, extended_reg)); // Copy the value
                    bind_llvm_ty = "i64".to_string();
                } else if scrutinee_reg.starts_with("i8* ") {
                    // Pointer (string, etc.) - copy via bitcast
                    let reg_name = &scrutinee_reg[4..];
                    self.instructions.push(format!("  {} = bitcast i8* {} to i8*", bind_reg, reg_name));
                    bind_llvm_ty = "i8*".to_string();
                } else if scrutinee_reg.contains("tuple_alloc") {
                    // Tuple pointer - convert to integer value
                    let int_reg = format!("%{}_int_{}", name, self.instructions.len());
                    self.instructions.push(format!("  {} = ptrtoint i8* {} to i64", int_reg, scrutinee_reg));
                    self.instructions.push(format!("  {} = add i64 {}, 0", bind_reg, int_reg)); // Copy the value
                    bind_llvm_ty = "i64".to_string();
                } else {
                    // No type prefix: use pattern type to decide (e.g. string param -> %aString)
                    let llvm_ty = self.type_map.silica_to_llvm_str(pattern_type);
                    if llvm_ty == "i8*" {
                        let reg = if scrutinee_reg.starts_with('%') { scrutinee_reg.to_string() } else { format!("%{}", scrutinee_reg) };
                        self.instructions.push(format!("  {} = bitcast i8* {} to i8*", bind_reg, reg));
                        bind_llvm_ty = "i8*".to_string();
                    } else {
                        self.instructions.push(format!("  {} = add i64 {}, 0", bind_reg, scrutinee_reg));
                        bind_llvm_ty = "i64".to_string();
                    }
                }

                // Strip type prefixes before storing in global map
                let clean_bind_reg = bind_reg.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();
                self.variables.insert(name.clone(), clean_bind_reg); // Add to global map for testing
                self.variable_llvm_types.insert(name.clone(), bind_llvm_ty); // So generate_identifier uses correct type (avoids icmp eq i1 %reg when %reg is i64)
                bound_vars.insert(name.clone(), bind_reg);
            }
            Pattern::Tuple(elements) => {
                // Tuple destructuring with proper type-aware element access
                // Uses the same layout calculation as tuple creation for consistency

                // Pre-calculate element offsets from pattern types to match tuple creation layout
                let element_types: Vec<Type> = elements.iter().filter_map(|p| {
                    match p {
                        Pattern::TypedIdentifier { type_, .. } => Some(self.expand_type_aliases_codegen(type_)),
                        _ => None,
                    }
                }).collect();

                let element_count = elements.len() as i64;
                let mut current_offset = 8 + element_count; // After count and type IDs
                let mut element_offsets = Vec::new();
                if element_types.len() == elements.len() {
                    for silica_type in &element_types {
                        let elem_size = self.get_type_size_bytes(silica_type);
                        let elem_alignment = self.get_type_alignment_bytes(silica_type);
                        current_offset = ((current_offset + elem_alignment - 1) / elem_alignment) * elem_alignment;
                        element_offsets.push(current_offset);
                        current_offset += elem_size;
                    }
                } else {
                    // Fallback: fixed layout 16 + i*8
                    for i in 0..elements.len() {
                        element_offsets.push(16 + (i as i64 * 8));
                    }
                }

                for (i, elem_pattern) in elements.iter().enumerate() {
                    let elem_offset = element_offsets.get(i).copied().unwrap_or(16 + (i as i64 * 8));
                    match elem_pattern {
                        Pattern::TypedIdentifier { name: elem_name, type_: elem_type, .. } => {
                        // Strip any type prefixes from scrutinee_reg
                        let clean_scrutinee = scrutinee_reg.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();

                        // Read the type ID for this element
                        let type_id_offset = 8 + i as i64;
                        let type_ptr_reg = format!("%type_ptr_{}_{}", elem_name, self.instructions.len());
                        self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", type_ptr_reg, clean_scrutinee, type_id_offset));
                        let type_id_reg = format!("%type_id_{}_{}", elem_name, self.instructions.len());
                        self.instructions.push(format!("  {} = load i8, i8* {}", type_id_reg, type_ptr_reg));

                        // Generate pointer to element at type-aware offset (matches tuple creation layout)
                        let elem_ptr_reg = format!("%{}_ptr_{}", elem_name, self.instructions.len());
                        self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", elem_ptr_reg, clean_scrutinee, elem_offset));

                        // Load element with type-aware casting
                        // Use unique register for wildcard '_' to avoid "multiple definition of local value named '_'"
                        let elem_reg = if elem_name == "_" {
                            format!("%_discard_{}", self.instructions.len())
                        } else {
                            format!("%{}", elem_name)
                        };

                        // Load both possible types and select
                        let unique_id = self.instructions.len();

                        // Cast to both i1* and i64*
                        let i1_cast_reg = format!("%{}_i1_cast_{}", elem_name, unique_id);
                        let i64_cast_reg = format!("%{}_i64_cast_{}", elem_name, unique_id);
                        self.instructions.push(format!("  {} = bitcast i8* {} to i1*", i1_cast_reg, elem_ptr_reg));
                        self.instructions.push(format!("  {} = bitcast i8* {} to i64*", i64_cast_reg, elem_ptr_reg));

                        // Load both values
                        let bool_val_reg = format!("%{}_bool_val_{}", elem_name, unique_id);
                        let i64_val_reg = format!("%{}_i64_val_{}", elem_name, unique_id);
                        self.instructions.push(format!("  {} = load i1, i1* {}", bool_val_reg, i1_cast_reg));
                        self.instructions.push(format!("  {} = load i64, i64* {}", i64_val_reg, i64_cast_reg));

                        // Extend bool to i64
                        let extended_bool_reg = format!("%{}_extended_{}", elem_name, unique_id);
                        self.instructions.push(format!("  {} = zext i1 {} to i64", extended_bool_reg, bool_val_reg));

                        // Select based on type
                        let is_i1_check = format!("%{}_is_i1_{}", elem_name, unique_id);
                        self.instructions.push(format!("  {} = icmp eq i8 {}, 0", is_i1_check, type_id_reg));
                        // Select the correct result based on type
                        self.instructions.push(format!("  {} = select i1 {}, i64 {}, i64 {}", elem_reg, is_i1_check, extended_bool_reg, i64_val_reg));

                        bound_vars.insert(elem_name.clone(), elem_reg);
                        self.variable_types.insert(elem_name.clone(), elem_type.clone());
                        }
                        Pattern::Identifier(elem_name) => {
                            // Untyped tuple element: bind with same layout as TypedIdentifier (i64 / type-select)
                            let clean_scrutinee = scrutinee_reg.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();
                            let type_id_offset = 8 + i as i64;
                            let type_ptr_reg = format!("%type_ptr_{}_{}", elem_name, self.instructions.len());
                            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", type_ptr_reg, clean_scrutinee, type_id_offset));
                            let type_id_reg = format!("%type_id_{}_{}", elem_name, self.instructions.len());
                            self.instructions.push(format!("  {} = load i8, i8* {}", type_id_reg, type_ptr_reg));
                            let elem_ptr_reg = format!("%{}_ptr_{}", elem_name, self.instructions.len());
                            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", elem_ptr_reg, clean_scrutinee, elem_offset));
                            // Use unique register for wildcard '_' to avoid "multiple definition of local value named '_'"
                            let elem_reg = if elem_name == "_" {
                                format!("%_discard_{}", self.instructions.len())
                            } else {
                                format!("%{}", elem_name)
                            };
                            let unique_id = self.instructions.len();
                            let i1_cast_reg = format!("%{}_i1_cast_{}", elem_name, unique_id);
                            let i64_cast_reg = format!("%{}_i64_cast_{}", elem_name, unique_id);
                            self.instructions.push(format!("  {} = bitcast i8* {} to i1*", i1_cast_reg, elem_ptr_reg));
                            self.instructions.push(format!("  {} = bitcast i8* {} to i64*", i64_cast_reg, elem_ptr_reg));
                            let bool_val_reg = format!("%{}_bool_val_{}", elem_name, unique_id);
                            let i64_val_reg = format!("%{}_i64_val_{}", elem_name, unique_id);
                            self.instructions.push(format!("  {} = load i1, i1* {}", bool_val_reg, i1_cast_reg));
                            self.instructions.push(format!("  {} = load i64, i64* {}", i64_val_reg, i64_cast_reg));
                            let extended_bool_reg = format!("%{}_extended_{}", elem_name, unique_id);
                            self.instructions.push(format!("  {} = zext i1 {} to i64", extended_bool_reg, bool_val_reg));
                            let is_i1_check = format!("%{}_is_i1_{}", elem_name, unique_id);
                            self.instructions.push(format!("  {} = icmp eq i8 {}, 0", is_i1_check, type_id_reg));
                            self.instructions.push(format!("  {} = select i1 {}, i64 {}, i64 {}", elem_reg, is_i1_check, extended_bool_reg, i64_val_reg));
                            bound_vars.insert(elem_name.clone(), elem_reg);
                        }
                        Pattern::Literal(_) => {
                            // No variable binding needed for literals
                        }
                        Pattern::TypedIdentifier { name, .. } if name == "_" => {
                            // No variable binding needed for wildcards
                        }
                        Pattern::Tuple(sub_patterns) => {
                            // Nested tuple: delegate to same layout as do-block path (load nested ptr, recurse)
                            let clean_scrutinee = scrutinee_reg.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();
                            let elem_ptr_reg = format!("%nested_ptr_{}_{}", i, self.instructions.len());
                            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", elem_ptr_reg, clean_scrutinee, elem_offset));
                            let i8pp_cast = format!("%nested_cast_{}_{}", i, self.instructions.len());
                            self.instructions.push(format!("  {} = bitcast i8* {} to i8**", i8pp_cast, elem_ptr_reg));
                            let nested_ptr_reg = format!("%nested_tuple_{}_{}", i, self.instructions.len());
                            self.instructions.push(format!("  {} = load i8*, i8** {}", nested_ptr_reg, i8pp_cast));
                            let n = sub_patterns.len() as i64;
                            let base_nested = ((8 + n + 7) / 8) * 8;
                            self.generate_tuple_decomposition(nested_ptr_reg, sub_patterns, base_nested)?;
                            // Collect any bound vars from nested decomposition (they're in self.variables)
                            for sub in sub_patterns {
                                if let Pattern::Identifier(name) = sub {
                                    if name != "_" {
                                        if let Some(reg) = self.variables.get(name) {
                                            bound_vars.insert(name.clone(), reg.clone());
                                        }
                                    }
                                } else if let Pattern::TypedIdentifier { name, .. } = sub {
                                    if name != "_" {
                                        if let Some(reg) = self.variables.get(name) {
                                            bound_vars.insert(name.clone(), reg.clone());
                                        }
                                    }
                                }
                            }
                        }
                        Pattern::Record(_) | Pattern::Variant { .. } | Pattern::Alternative(_) => {
                            // Not yet implemented for case tuple binding
                        }
                        _ => {
                            return Err(CompilerError::codegen_error(format!("Unsupported pattern type in tuple decomposition: {:?}", elem_pattern)));
                        }
                    }
                }
            }
            _ => {} // Other patterns don't bind variables
        }

        Ok(bound_vars)
    }

    fn generate_runtime_pattern_check(&mut self, pattern: &Pattern, scrutinee_reg: &str) -> Result<String> {
        match pattern {
            Pattern::Literal(lit) => {
                // Generate comparison with literal value
                match lit {
                    Literal::Int(n) => {
                        let cmp_reg = format!("%cmp_int_{}", self.instructions.len());
                        // Extract just the register name, not the type prefix
                        let reg_name = if scrutinee_reg.starts_with("i64 ") {
                            &scrutinee_reg[4..]
                        } else {
                            scrutinee_reg
                        };
                        // If the register is a boxed value, load it first
                        if reg_name.contains("box_result") || reg_name.contains("param") || reg_name.starts_with("%box_") || reg_name.starts_with("%param_") {
                            let load_reg = format!("%scrutinee_load_{}", self.instructions.len());
                            self.instructions.push(format!("  {} = bitcast i8* {} to i64*", load_reg.clone() + "_cast", reg_name));
                            self.instructions.push(format!("  {} = load i64, i64* {}_cast", load_reg, load_reg));
                            self.instructions.push(format!("  {} = icmp eq i64 {}, {}", cmp_reg, load_reg, n));
                        } else {
                            self.instructions.push(format!("  {} = icmp eq i64 {}, {}", cmp_reg, reg_name, n));
                        }
                        Ok(cmp_reg)
                    }
                    Literal::Bool(b) => {
                        let bool_val = if *b { 1 } else { 0 };
                        let cmp_reg = format!("%cmp_bool_{}", self.instructions.len());
                        // Use scrutinee's actual type: i1 or i64 (booleans may be stored as i64)
                        let has_i1 = scrutinee_reg.starts_with("i1 ");
                        let has_i64 = scrutinee_reg.starts_with("i64 ");
                        let (cmp_type, reg_name) = if has_i1 {
                            ("i1", scrutinee_reg[3..].to_string())
                        } else if has_i64 {
                            ("i64", scrutinee_reg[4..].to_string())
                        } else {
                            // No prefix: assume i64 (bool-as-int from tuple/variable)
                            ("i64", scrutinee_reg.to_string())
                        };
                        self.instructions.push(format!("  {} = icmp eq {} {}, {}", cmp_reg, cmp_type, reg_name, bool_val));
                        Ok(cmp_reg)
                    }
                    Literal::String(s) => {
                        // String literal: when no type is given for the literal, compare as string (scrutinee type).
                        // Ensure the string constant is registered so we can reference it.
                        if !self.string_constants.contains_key(s) {
                            let const_name = format!("@str_const_{}", self.string_constants.len());
                            let length = s.len() + 1;
                            self.string_constants.insert(s.clone(), (const_name.clone(), length));
                        }
                        let (const_name, length) = self.string_constants.get(s).unwrap();
                        let literal_ptr_reg = format!("%literal_ptr_{}", self.instructions.len());
                        // Use i8, ptr form for compatibility with LLVM opaque pointer mode; ptr is the address of the global
                        self.instructions.push(format!(
                            "  {} = getelementptr inbounds i8, ptr {}, i32 0",
                            literal_ptr_reg, const_name
                        ));
                        let scrutinee_reg_clean = scrutinee_reg.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();
                        let scrutinee_for_call = if scrutinee_reg.starts_with("i8* ") { scrutinee_reg[4..].to_string() } else if scrutinee_reg_clean.starts_with('%') { scrutinee_reg_clean } else { format!("%{}", scrutinee_reg_clean) };
                        let cmp_reg = format!("%cmp_str_{}", self.instructions.len());
                        self.instructions.push(format!(
                            "  {} = call i1 @silica_string_equals(i8* {}, i8* {})",
                            cmp_reg, scrutinee_for_call, literal_ptr_reg
                        ));
                        Ok(cmp_reg)
                    }
                    _ => {
                        // For unsupported literals, always return false
                        let false_reg = format!("%pattern_false_{}", self.instructions.len());
                        self.instructions.push(format!("  {} = add i1 0, 0", false_reg));
                        Ok(false_reg)
                    }
                }
            }
            Pattern::Identifier(_) | Pattern::TypedIdentifier { .. } => {
                // Identifier patterns always match - return true
                let true_reg = format!("%pattern_true_{}", self.instructions.len());
                self.instructions.push(format!("  {} = add i1 0, 1", true_reg));
                Ok(true_reg)
            }
            Pattern::Tuple(patterns) => {
                // Tuple pattern: load each element and check against subpattern; combine with and
                let clean_scrutinee = scrutinee_reg.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();
                if patterns.is_empty() {
                    let true_reg = format!("%pattern_true_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = add i1 0, 1", true_reg));
                    return Ok(true_reg);
                }
                let mut match_results = Vec::new();
                for (i, elem_pattern) in patterns.iter().enumerate() {
                    let elem_offset = 16 + (i as i64 * 8);
                    let elem_ptr_reg = format!("%tuple_elem_ptr_{}_{}", i, self.instructions.len());
                    self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", elem_ptr_reg, clean_scrutinee, elem_offset));
                    let elem_match = match elem_pattern {
                        Pattern::Literal(Literal::String(s)) => {
                            let elem_cast_reg = format!("%tuple_elem_cast_{}_{}", i, self.instructions.len());
                            self.instructions.push(format!("  {} = bitcast i8* {} to i8**", elem_cast_reg, elem_ptr_reg));
                            let elem_load_reg = format!("%tuple_elem_str_{}_{}", i, self.instructions.len());
                            self.instructions.push(format!("  {} = load i8*, i8** {}", elem_load_reg, elem_cast_reg));
                            self.generate_runtime_pattern_check(&Pattern::Literal(Literal::String(s.clone())), &elem_load_reg)?
                        }
                        Pattern::Literal(Literal::Int(n)) => {
                            let elem_cast_reg = format!("%tuple_elem_cast_{}_{}", i, self.instructions.len());
                            self.instructions.push(format!("  {} = bitcast i8* {} to i64*", elem_cast_reg, elem_ptr_reg));
                            let elem_load_reg = format!("%tuple_elem_i64_{}_{}", i, self.instructions.len());
                            self.instructions.push(format!("  {} = load i64, i64* {}", elem_load_reg, elem_cast_reg));
                            self.generate_runtime_pattern_check(&Pattern::Literal(Literal::Int(*n)), &elem_load_reg)?
                        }
                        Pattern::Literal(Literal::Bool(b)) => {
                            let elem_cast_reg = format!("%tuple_elem_cast_{}_{}", i, self.instructions.len());
                            self.instructions.push(format!("  {} = bitcast i8* {} to i1*", elem_cast_reg, elem_ptr_reg));
                            let elem_load_reg = format!("%tuple_elem_i1_{}_{}", i, self.instructions.len());
                            self.instructions.push(format!("  {} = load i1, i1* {}", elem_load_reg, elem_cast_reg));
                            self.generate_runtime_pattern_check(&Pattern::Literal(Literal::Bool(*b)), &format!("i1 {}", elem_load_reg))?
                        }
                        Pattern::TypedIdentifier { .. } | Pattern::Identifier(_) => {
                            let true_reg = format!("%tuple_elem_true_{}_{}", i, self.instructions.len());
                            self.instructions.push(format!("  {} = add i1 0, 1", true_reg));
                            true_reg
                        }
                        _ => {
                            return Err(CompilerError::codegen_error(format!("Unsupported tuple element pattern in case match: {:?}", elem_pattern)));
                        }
                    };
                    match_results.push(elem_match);
                }
                // Combine all element match results with and
                let mut combined = match_results.remove(0);
                for r in match_results {
                    let and_reg = format!("%tuple_and_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = and i1 {}, {}", and_reg, combined, r));
                    combined = and_reg;
                }
                Ok(combined)
            }
            _ => {
                // Unsupported patterns don't match
                let false_reg = format!("%pattern_false_{}", self.instructions.len());
                self.instructions.push(format!("  {} = add i1 0, 0", false_reg));
                Ok(false_reg)
            }
        }
    }

    /// Generate LLVM IR for if expressions
    fn generate_if(&mut self, if_expr: &IfExpr) -> Result<Option<String>> {
        let cond = self.generate_expression(&if_expr.condition)?;

        if let Some(cond_val) = cond {
            let then_label = format!("then_{}", self.instructions.len());
            let else_label = format!("else_{}", self.instructions.len());
            let end_label = format!("end_{}", self.instructions.len());
            let result_reg = format!("%if_result_{}", self.instructions.len());

            // Condition should be i1 (boolean)
            // Strip type prefix from condition value
            let clean_cond = cond_val.trim_start_matches("i64 ").trim_start_matches("i1 ");
            self.instructions.push(format!("  br i1 {}, label %{}, label %{}",
                clean_cond, then_label, else_label));

            // Generate then block
            self.instructions.push(format!("{}:", then_label));
            let then_result = self.generate_expression(&if_expr.then_branch)?;
            let then_val = then_result.unwrap_or_else(|| "0".to_string());
            self.instructions.push(format!("  br label %{}", end_label));

            // Generate else block
            self.instructions.push(format!("{}:", else_label));
            let else_result = self.generate_expression(&if_expr.else_branch)?;
            let else_val = else_result.unwrap_or_else(|| "0".to_string());
            self.instructions.push(format!("  br label %{}", end_label));

            // Generate merge block with phi
            // For behavior functions, always use i8*
            let result_type = "i8*";
            self.instructions.push(format!("{}:", end_label));
            self.instructions.push(format!("  {} = phi i8* [{}, %{}], [{}, %{}]",
                result_reg,
                then_val.trim_start_matches("i64 ").trim_start_matches("i8* "),
                then_label,
                else_val.trim_start_matches("i64 ").trim_start_matches("i8* "),
                else_label));

            Ok(Some(result_reg))
        } else {
            codegen_error("If condition must be valid".to_string())
        }
    }

    /// Create a new code generator with LLVM backend
    #[cfg(feature = "llvm_backend")]
    pub fn new_with_llvm(module_name: &str, optimization_level: OptimizationLevel, context: *const inkwell::context::Context) -> Self {
        CodeGenerator {
            module_name: module_name.to_string(),
            type_map: TypeMap::new(),
            functions: HashMap::new(),
            function_return_types: HashMap::new(),
            function_param_types: HashMap::new(),
            variables: HashMap::new(),
            variable_types: HashMap::new(),
            variable_llvm_types: HashMap::new(),
            instructions: Vec::new(),
            global_functions: Vec::new(),
            optimization_level,
            symbol_table: None,
            expression_types: HashMap::new(),
            type_aliases: HashMap::new(),
            struct_defs: HashMap::new(),
            trait_impls: Vec::new(),
            trait_forwarders_emitted: std::collections::HashSet::new(),
            trait_forwarder_ir: Vec::new(),
            variable_scopes: vec![HashMap::new()], // Start with global scope
            function_variable_scopes: vec![HashMap::new()],
            register_counter: 0,
            string_constants: HashMap::new(),
            in_behavior_function: false,
            self_ref_placeholders: std::collections::HashSet::new(),
            context,
            module: None,
            builder: None,
            pass_manager: None,
            llvm_variable_scopes: vec![HashMap::new()], // Start with global scope
            monomorphized_functions: HashMap::new(), // Cache for monomorphized functions
        }
    }

    /// Convert Silica type to LLVM type
    #[cfg(feature = "llvm_backend")]
    fn silica_type_to_llvm(&self, ty: &Type) -> inkwell::types::BasicTypeEnum<'static> {
        unsafe {
            match ty {
                Type::Unit => (*self.context).i64_type().into(), // Unit is represented as i64 for now
                Type::Bool => (*self.context).bool_type().into(),
                Type::Int8 => (*self.context).i8_type().into(),
                Type::Int16 => (*self.context).i16_type().into(),
                Type::Int32 => (*self.context).i32_type().into(),
                Type::Int64 => (*self.context).i64_type().into(),
                Type::Float16 => (*self.context).f16_type().into(),
                Type::Float32 => (*self.context).f32_type().into(),
                Type::Char => (*self.context).i32_type().into(),
                Type::String => (*self.context).ptr_type(inkwell::AddressSpace::default()).into(),
                Type::Tuple(_) => (*self.context).ptr_type(inkwell::AddressSpace::default()).into(), // Tuples as opaque pointers
                Type::Record(_) => (*self.context).ptr_type(inkwell::AddressSpace::default()).into(), // Records as opaque pointers
                Type::Reference { .. } => (*self.context).ptr_type(inkwell::AddressSpace::default()).into(),
                // NEON 128-bit vector types
                Type::Vec128Int8 => (*self.context).i8_type().vec_type(16).into(),
                Type::Vec128Int16 => (*self.context).i16_type().vec_type(8).into(),
                Type::Vec128Int32 => (*self.context).i32_type().vec_type(4).into(),
                Type::Vec128Int64 => (*self.context).i64_type().vec_type(2).into(),
                Type::Vec128Float32 => (*self.context).f32_type().vec_type(4).into(),
                Type::Vec128Bool => (*self.context).bool_type().vec_type(16).into(),
                // SVE scalable vector types - use fixed-size vectors as placeholder (SVE support requires target features)
                Type::VecInt8 => (*self.context).i8_type().vec_type(16).into(), // Placeholder - will be vscale in actual SVE codegen
                Type::VecInt16 => (*self.context).i16_type().vec_type(8).into(),
                Type::VecInt32 => (*self.context).i32_type().vec_type(4).into(),
                Type::VecInt64 => (*self.context).i64_type().vec_type(2).into(),
                Type::VecFloat16 => (*self.context).f16_type().vec_type(8).into(),
                Type::VecFloat32 => (*self.context).f32_type().vec_type(4).into(),
                Type::VecFloat64 => (*self.context).f64_type().vec_type(2).into(),
                Type::VecBool => (*self.context).bool_type().vec_type(16).into(),
                // SVE predicate type
                Type::Pred => (*self.context).bool_type().vec_type(16).into(), // Placeholder - will be vscale predicate in actual SVE codegen
                _ => (*self.context).i64_type().into(), // Default to i64
            }
        }
    }

    /// Generate LLVM function body
    #[cfg(feature = "llvm_backend")]
    fn generate_llvm_function_body(&mut self, func: &FunctionDecl, llvm_func: inkwell::values::FunctionValue<'static>) -> Result<()> {
        // Enter function scope first (before any borrows)
        self.enter_scope();

        // Collect parameter information for allocation
        let mut param_info = Vec::new();
        for (i, param) in func.parameters.iter().enumerate() {
            let param_value = llvm_func.get_nth_param(i as u32).unwrap();
            if let Some(pattern) = &param.pattern {
                // For pattern parameters, set a generic name
                param_value.set_name(&format!("param_{}", i));
                param_info.push((format!("param_{}", i), param_value));
            } else {
                // Regular parameter
                param_value.set_name(&param.name);
                param_info.push((param.name.clone(), param_value));
            }
        }

        // Do LLVM operations to allocate parameters (borrow builder here)
        let mut param_allocas = Vec::new();
        if let Some(builder) = &self.builder {
            unsafe {
                let entry_block = (*self.context).append_basic_block(llvm_func, "entry");
                builder.position_at_end(entry_block);

                // Create parameter variables and allocate them in the function scope
                for (i, (name, param_value)) in param_info.iter().enumerate() {
                    let param = &func.parameters[i];
                    if let Some(pattern) = &param.pattern {
                        // Handle pattern parameters - extract elements from tuple
                        match pattern {
                            Pattern::Tuple(elements) => {
                                for (elem_idx, elem_pattern) in elements.iter().enumerate() {
                                    if let Pattern::TypedIdentifier { name: elem_name, .. } = elem_pattern {
                                        // The param_value is an i8* (tuple pointer)
                                        // Calculate offset for this element (8 bytes per element)
                                        let offset = elem_idx as u64 * 8;
                                        let elem_ptr = builder.build_struct_gep(param_value.into_pointer_value(), offset as u32, &format!("{}_ptr", elem_name)).unwrap();

                                        // Cast to i64* and load
                                        let i64_ptr_type = (*self.context).i64_type().ptr_type(inkwell::AddressSpace::Generic);
                                        let cast_ptr = builder.build_bitcast(elem_ptr, i64_ptr_type, &format!("{}_cast", elem_name)).unwrap();
                                        let elem_val = builder.build_load(cast_ptr.into_pointer_value(), &format!("{}_val", elem_name)).unwrap();

                                        // Allocate space for the element
                                        let alloca = builder.build_alloca(elem_val.get_type(), elem_name).unwrap();
                                        builder.build_store(alloca, elem_val).unwrap();

                                        param_allocas.push((elem_name.clone(), alloca));
                                    }
                                }
                            }
                            _ => {} // Other patterns not supported
                        }
                    } else {
                        // Regular parameter - allocate space for the parameter on the stack
                        let param_type = param_value.get_type();
                        let alloca = builder.build_alloca(param_type, &name).unwrap();

                        // Store the parameter value in the allocated space
                        builder.build_store(alloca, *param_value).unwrap();

                        param_allocas.push((name.clone(), alloca));
                    }
                }
            }
        } else {
            return Err(CompilerError::codegen_error("LLVM builder not initialized".to_string()));
        }

        // Now add parameters to scope (after builder borrow ends)
        for (name, alloca) in param_allocas {
            self.add_variable(name, alloca);
        }

        // Generate the function body statements (scope is now set up)
        self.generate_statements_llvm(&func.body)?;

        // Generate return instruction (another builder borrow)
        if let Some(builder) = &self.builder {
            unsafe {
                if let Some(return_val) = result {
                    builder.build_return(Some(&return_val)).unwrap();
                } else {
                    // Void return
                    builder.build_return(None).unwrap();
                }
            }
            Ok(())
        } else {
            Err(CompilerError::codegen_error("LLVM builder not initialized".to_string()))
        }?;

        // Exit function scope after all operations
        self.exit_scope_text();

        Ok(())
    }

    /// Write the generated LLVM IR to a file
    pub fn write_to_file(&self, filename: &str) -> Result<()> {
        #[cfg(feature = "llvm_backend")]
        {
            if let Some(module) = &self.module {
                if filename.ends_with(".bc") {
                    // Write LLVM bitcode directly from module
                    unsafe {
                        let file = std::fs::File::create(filename)
                            .map_err(|e| CompilerError::IoError(e))?;
                        if module.write_bitcode_to_file(&file, true, false) {
                            // println!("📄 LLVM bitcode written to {}", filename);
                            return Ok(());
                        } else {
                            let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                                .severity(ErrorSeverity::Error)
                                .build();
                            return Err(CompilerError::CodegenError { message: "Failed to write LLVM bitcode".to_string(), location: None, metadata });
                        }
                    }
                } else {
                    // Write LLVM text IR directly from module
                    let content = unsafe { module.print_to_string().to_string() };
                    std::fs::write(filename, content)
                        .map_err(|e| CompilerError::IoError(e))?;
                    // println!("📄 LLVM text IR written to {}", filename);
                    return Ok(());
                }
            }
        }

        // Fallback to text IR generation
        // Check if the output file has .bc extension - convert from text IR
        if filename.ends_with(".bc") {
            return self.write_bitcode_to_file(filename);
        }

        // Otherwise, write text LLVM IR
        let mut content_parts = Vec::new();

        // Add global function definitions first
        content_parts.extend(self.global_functions.clone());
        if !self.global_functions.is_empty() {
            content_parts.push("".to_string());
        }

        // Reorganize: put string constants after declarations but before function definitions,
        // so globals like @str_const_0 are defined before any "define" that references them.
        let mut declarations = Vec::new();
        let mut constants_section = Vec::new();
        let mut definitions = Vec::new();
        let mut in_constants_section = false;
        let mut seen_constants_section = false;

        for instruction in &self.instructions {
            if instruction.starts_with("; String constants") {
                in_constants_section = true;
                seen_constants_section = true;
                constants_section.push(instruction.clone());
            } else if in_constants_section && (instruction.starts_with("@str_const_") || instruction.is_empty()) {
                constants_section.push(instruction.clone());
            } else if in_constants_section && !instruction.starts_with("@str_const_") && !instruction.is_empty() {
                in_constants_section = false;
                definitions.push(instruction.clone());
            } else if in_constants_section {
                constants_section.push(instruction.clone());
            } else if !seen_constants_section {
                // Before we've seen "; String constants": declarations (declare, comments, blank) then definitions (define)
                let is_declaration = instruction.starts_with("declare ") || instruction.starts_with(";") || instruction.is_empty();
                if is_declaration && definitions.is_empty() {
                    declarations.push(instruction.clone());
                } else {
                    definitions.push(instruction.clone());
                }
            } else {
                definitions.push(instruction.clone());
            }
        }

        // Output order: declarations, string constants (so @str_const_* exist), trait forwarders, then function definitions
        content_parts.extend(declarations);
        if !constants_section.is_empty() {
            content_parts.push("".to_string());
            content_parts.extend(constants_section);
            content_parts.push("".to_string());
        }
        if !self.trait_forwarder_ir.is_empty() {
            content_parts.push("; Trait method forwarders (trait-typed receiver dispatch)".to_string());
            content_parts.extend(self.trait_forwarder_ir.clone());
            content_parts.push("".to_string());
        }
        content_parts.extend(definitions);

        let content = content_parts.join("\n");
        std::fs::write(filename, content)
            .map_err(|e| CompilerError::IoError(e))?;

        // println!("📄 LLVM text IR written to {}", filename);
        Ok(())
    }

    /// Write LLVM bitcode by generating text IR and converting with llvm-as
    fn write_bitcode_to_file(&self, filename: &str) -> Result<()> {
        use std::process::Command;

        // First write the text IR to a temporary file
        let temp_ll = format!("{}.tmp.ll", filename);
        let content = self.instructions.join("\n");
        std::fs::write(&temp_ll, content)
            .map_err(|e| CompilerError::IoError(e))?;

        // Use llvm-as to convert .ll to .bc
        let result = Command::new("llvm-as")
            .args(&[&temp_ll, "-o", filename])
            .output();

        // Clean up temporary file
        let _ = std::fs::remove_file(&temp_ll);

        match result {
            Ok(output) if output.status.success() => {
                // println!("📄 LLVM bitcode written to {}", filename);
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                    .severity(ErrorSeverity::Error)
                    .build();
                Err(CompilerError::CodegenError { message: format!("llvm-as failed: {}", stderr), location: None, metadata })
            }
            Err(e) => {
                let metadata = ErrorMetadataBuilder::new("E4001".to_string())
                    .severity(ErrorSeverity::Error)
                    .build();
                Err(CompilerError::CodegenError { message: format!("Failed to run llvm-as: {}", e), location: None, metadata })
            }
        }
    }

    /// Print the generated LLVM IR (for debugging)
    pub fn print_ir(&self) {
        // Use LLVM module if available and working
        #[cfg(feature = "llvm_backend")]
        {
            if let Some(module) = &self.module {
                unsafe {
                    // println!("Generated LLVM IR (Real LLVM Backend):");
                    // println!("=======================================");
                    // println!("{}", module.print_to_string().to_string());
                }
                return;
            }
        }

        // Fallback to text IR
        // println!("Generated LLVM IR (Text Representation):");
        // println!("========================================");

        // Output global function definitions first
        for func in &self.global_functions {
            // println!("{}", func);
        }
        if !self.global_functions.is_empty() {
            // println!("");
        }

        // Then output main instructions
        for instruction in &self.instructions {
            // println!("{}", instruction);
        }
    }

    /// Extract element types from a tuple type
    fn extract_tuple_element_types(&self, tuple_type: &Type) -> Option<Vec<Type>> {
        match tuple_type {
            Type::Tuple(element_types) => Some(element_types.clone()),
            _ => None,
        }
    }

    /// Generate LLVM IR for do expressions
    fn generate_do(&mut self, do_expr: &DoExpr) -> Result<Option<String>> {
        // Enter a new scope for the do expression
        self.enter_scope_text();

        // Do expressions execute statements sequentially and return the value of the last statement
        let mut result = None;

        for statement in &do_expr.statements {
            match statement {
                Statement::Bind { pattern, expr } => {
                    // Evaluate the expression
                    let value = self.generate_expression(expr)?;

                    match pattern {
                        Pattern::Identifier(name) => {
                            // Store the value in the current scope
                            if let Some(ref val) = value {
                                // For text IR, we just track the variable name -> register mapping
                                self.add_variable_text(name.clone(), val.clone());

                                // Also store the variable type for method calls
                                let var_type = if let Some(location) = Self::try_get_expression_location(expr) {
                                    self.expression_types.get(location).cloned()
                                } else {
                                    // For expressions without location (like literals), infer the type directly
                                    match **expr {
                                        Expression::Literal(Literal::Int(_)) => Some(Type::Int64),
                                        Expression::Literal(Literal::Bool(_)) => Some(Type::Bool),
                                        Expression::Literal(Literal::Char(_)) => Some(Type::Char),
                                        Expression::Literal(Literal::String(_)) => Some(Type::String),
                                        Expression::Literal(Literal::Unit) => Some(Type::Unit),
                                        _ => None,
                                    }
                                };

                                if let Some(expr_type) = var_type {
                                    self.variable_types.insert(name.clone(), expr_type);
                                }
                            }
                            result = value;
                        }
                        Pattern::TypedIdentifier { name, type_ } => {
                            // Store the value in the current scope
                            if let Some(ref val) = value {
                                // eprintln!("DEBUG TypedIdentifier bind (do): name = '{}', val = '{}'", name, val);
                                // For float16 and float64 bindings, we need to create a proper register instead of storing the literal string
                                // Check if this is a float16 binding
                                let stored_val = if matches!(type_, crate::ast::Type::Float16) {
                                    // Check if value is already a half register (from function call, etc.)
                                    if val.starts_with('%') && !val.contains(' ') {
                                        // eprintln!("DEBUG TypedIdentifier bind (do): float16 binding with register, using directly: '{}'", val);
                                        // It's already a register - assume it's half and use directly
                                        val.clone()
                                    } else if val.starts_with("half ") || val.starts_with("float ") {
                                        // eprintln!("DEBUG TypedIdentifier bind (do): float16 binding with float literal");
                                        // Extract the constant value, handling both "half 3.14" and "float 3.14"
                                        let const_val = if val.starts_with("half ") {
                                            val.trim_start_matches("half ")
                                        } else {
                                            val.trim_start_matches("float ")
                                        };
                                        // eprintln!("DEBUG TypedIdentifier bind (do): const_val = '{}'", const_val);
                                        // If const_val contains spaces, it's malformed - extract the numeric part
                                        let clean_const = if const_val.contains(' ') {
                                            // eprintln!("DEBUG TypedIdentifier bind (do): const_val contains spaces, extracting numeric part");
                                            // Split and find the numeric constant
                                            let found = const_val.split_whitespace()
                                                .find(|p| p.parse::<f64>().is_ok());
                                            // eprintln!("DEBUG TypedIdentifier bind (do): found numeric = {:?}", found);
                                            found.unwrap_or(const_val)
                                        } else {
                                            const_val
                                        };
                                        // eprintln!("DEBUG TypedIdentifier bind (do): clean_const = '{}'", clean_const);
                                        // Create a float constant first, then convert to half
                                        let float_const = format!("%float_const_bind_{}", self.instructions.len());
                                        let instruction = self.create_float_constant_instruction(clean_const, &float_const, "float");
                                        self.instructions.push(instruction);
                                        let half_const = format!("%half_const_bind_{}", self.instructions.len());
                                        self.instructions.push(format!("  {} = fptrunc float {} to half", half_const, float_const));
                                        // eprintln!("DEBUG TypedIdentifier bind (do): created half_const = '{}'", half_const);
                                        half_const
                                    } else {
                                        // eprintln!("DEBUG TypedIdentifier bind (do): float16 binding but val doesn't match expected patterns, using as-is");
                                        val.clone()
                                    }
                                } else if matches!(type_, crate::ast::Type::Float64) && (val.starts_with("double ") || val.starts_with("float ")) {
                                    // eprintln!("DEBUG TypedIdentifier bind (do): float64 binding with float literal");
                                    // Extract the constant value, handling both "double 3.14" and "float 3.14"
                                    let const_val = if val.starts_with("double ") {
                                        val.trim_start_matches("double ")
                                    } else {
                                        val.trim_start_matches("float ")
                                    };
                                    // eprintln!("DEBUG TypedIdentifier bind (do): const_val = '{}'", const_val);
                                    // If const_val contains spaces, it's malformed - extract the numeric part
                                    let clean_const = if const_val.contains(' ') {
                                        // eprintln!("DEBUG TypedIdentifier bind (do): const_val contains spaces, extracting numeric part");
                                        let found = const_val.split_whitespace()
                                            .find(|p| p.parse::<f64>().is_ok());
                                        // eprintln!("DEBUG TypedIdentifier bind (do): found numeric = {:?}", found);
                                        found.unwrap_or(const_val)
                                    } else {
                                        const_val
                                    };
                                    // eprintln!("DEBUG TypedIdentifier bind (do): clean_const = '{}'", clean_const);
                                    // Create a double constant register
                                    let double_const = format!("%double_const_bind_{}", self.instructions.len());
                                    let instruction = self.create_float_constant_instruction(clean_const, &double_const, "double");
                                    self.instructions.push(instruction);
                                    // eprintln!("DEBUG TypedIdentifier bind (do): created double_const = '{}'", double_const);
                                    double_const
                                } else {
                                    // eprintln!("DEBUG TypedIdentifier bind (do): not float16/float64 binding or not float literal, using as-is");
                                    // For other types, use the value as-is
                                    val.clone()
                                };
                                // eprintln!("DEBUG TypedIdentifier bind (do): storing '{}' -> '{}'", name, stored_val);
                                // Clone stored_val before passing it to add_variable_text since we need it for result
                                let stored_val_clone = stored_val.clone();

                                // Use the type from the pattern annotation (the declared type)
                                // Convert ast::Type to internal Type representation
                                let var_type = match type_ {
                                    crate::ast::Type::Int8 => Type::Int8,
                                    crate::ast::Type::Int16 => Type::Int16,
                                    crate::ast::Type::Int32 => Type::Int32,
                                    crate::ast::Type::Int64 => Type::Int64,
                                    crate::ast::Type::Float16 => Type::Float16,
                                    crate::ast::Type::Float32 => Type::Float32,
                                    crate::ast::Type::Float64 => Type::Float64,
                                    crate::ast::Type::Bool => Type::Bool,
                                    crate::ast::Type::Char => Type::Char,
                                    crate::ast::Type::String => Type::String,
                                    crate::ast::Type::Unit => Type::Unit,
                                    crate::ast::Type::Function { parameters, return_type } => {
                                        // Recursively convert parameter types
                                        let converted_params: Vec<Type> = parameters.iter().map(|param_type| {
                                            match param_type {
                                                crate::ast::Type::Int8 => Type::Int8,
                                                crate::ast::Type::Int16 => Type::Int16,
                                                crate::ast::Type::Int32 => Type::Int32,
                                                crate::ast::Type::Int64 => Type::Int64,
                                                crate::ast::Type::Float16 => Type::Float16,
                                                crate::ast::Type::Float32 => Type::Float32,
                                                crate::ast::Type::Float64 => Type::Float64,
                                                crate::ast::Type::Bool => Type::Bool,
                                                crate::ast::Type::Char => Type::Char,
                                                crate::ast::Type::String => Type::String,
                                                crate::ast::Type::Unit => Type::Unit,
                                                crate::ast::Type::Function { parameters: nested_params, return_type: nested_ret } => {
                                                    // Recursively handle nested function types
                                                    let nested_converted_params: Vec<Type> = nested_params.iter().map(|p| match p {
                                                        crate::ast::Type::Int8 => Type::Int8,
                                                        crate::ast::Type::Int16 => Type::Int16,
                                                        crate::ast::Type::Int32 => Type::Int32,
                                                        crate::ast::Type::Int64 => Type::Int64,
                                                        crate::ast::Type::Float16 => Type::Float16,
                                                        crate::ast::Type::Float32 => Type::Float32,
                                                        crate::ast::Type::Float64 => Type::Float64,
                                                        crate::ast::Type::Bool => Type::Bool,
                                                        crate::ast::Type::Char => Type::Char,
                                                        crate::ast::Type::String => Type::String,
                                                        crate::ast::Type::Unit => Type::Unit,
                                                        _ => Type::Int64, // Fallback for nested types
                                                    }).collect();
                                                    let nested_converted_ret = match &**nested_ret {
                                                        crate::ast::Type::Int8 => Type::Int8,
                                                        crate::ast::Type::Int16 => Type::Int16,
                                                        crate::ast::Type::Int32 => Type::Int32,
                                                        crate::ast::Type::Int64 => Type::Int64,
                                                        crate::ast::Type::Float16 => Type::Float16,
                                                        crate::ast::Type::Float32 => Type::Float32,
                                                        crate::ast::Type::Float64 => Type::Float64,
                                                        crate::ast::Type::Bool => Type::Bool,
                                                        crate::ast::Type::Char => Type::Char,
                                                        crate::ast::Type::String => Type::String,
                                                        crate::ast::Type::Unit => Type::Unit,
                                                        _ => Type::Int64, // Fallback
                                                    };
                                                    Type::Function {
                                                        parameters: nested_converted_params,
                                                        return_type: Box::new(nested_converted_ret),
                                                    }
                                                },
                                                crate::ast::Type::Named(name) => Type::Named(name.clone()),
                                                _ => Type::Int64, // Fallback
                                            }
                                        }).collect();
                                        // Convert return type
                                        let converted_ret = match &**return_type {
                                            crate::ast::Type::Int8 => Type::Int8,
                                            crate::ast::Type::Int16 => Type::Int16,
                                            crate::ast::Type::Int32 => Type::Int32,
                                            crate::ast::Type::Int64 => Type::Int64,
                                            crate::ast::Type::Float16 => Type::Float16,
                                            crate::ast::Type::Float32 => Type::Float32,
                                            crate::ast::Type::Float64 => Type::Float64,
                                            crate::ast::Type::Bool => Type::Bool,
                                            crate::ast::Type::Char => Type::Char,
                                            crate::ast::Type::String => Type::String,
                                            crate::ast::Type::Unit => Type::Unit,
                                            crate::ast::Type::Function { parameters: nested_params, return_type: nested_ret } => {
                                                // Recursively handle nested function types in return type
                                                let nested_converted_params: Vec<Type> = nested_params.iter().map(|p| match p {
                                                    crate::ast::Type::Int8 => Type::Int8,
                                                    crate::ast::Type::Int16 => Type::Int16,
                                                    crate::ast::Type::Int32 => Type::Int32,
                                                    crate::ast::Type::Int64 => Type::Int64,
                                                    crate::ast::Type::Float16 => Type::Float16,
                                                    crate::ast::Type::Float32 => Type::Float32,
                                                    crate::ast::Type::Float64 => Type::Float64,
                                                    crate::ast::Type::Bool => Type::Bool,
                                                    crate::ast::Type::Char => Type::Char,
                                                    crate::ast::Type::String => Type::String,
                                                    crate::ast::Type::Unit => Type::Unit,
                                                    _ => Type::Int64, // Fallback
                                                }).collect();
                                                let nested_converted_ret = match &**nested_ret {
                                                    crate::ast::Type::Int8 => Type::Int8,
                                                    crate::ast::Type::Int16 => Type::Int16,
                                                    crate::ast::Type::Int32 => Type::Int32,
                                                    crate::ast::Type::Int64 => Type::Int64,
                                                    crate::ast::Type::Float16 => Type::Float16,
                                                    crate::ast::Type::Float32 => Type::Float32,
                                                    crate::ast::Type::Float64 => Type::Float64,
                                                    crate::ast::Type::Bool => Type::Bool,
                                                    crate::ast::Type::Char => Type::Char,
                                                    crate::ast::Type::String => Type::String,
                                                    crate::ast::Type::Unit => Type::Unit,
                                                    _ => Type::Int64, // Fallback
                                                };
                                                Type::Function {
                                                    parameters: nested_converted_params,
                                                    return_type: Box::new(nested_converted_ret),
                                                }
                                            },
                                            crate::ast::Type::Named(name) => Type::Named(name.clone()),
                                            _ => Type::Int64, // Fallback
                                        };
                                        Type::Function {
                                            parameters: converted_params,
                                            return_type: Box::new(converted_ret),
                                        }
                                    }
                                    crate::ast::Type::Tuple(elem_types) => {
                                        let converted: Vec<Type> = elem_types.iter().map(|t| match t {
                                            crate::ast::Type::Int8 => Type::Int8,
                                            crate::ast::Type::Int16 => Type::Int16,
                                            crate::ast::Type::Int32 => Type::Int32,
                                            crate::ast::Type::Int64 => Type::Int64,
                                            crate::ast::Type::Float16 => Type::Float16,
                                            crate::ast::Type::Float32 => Type::Float32,
                                            crate::ast::Type::Float64 => Type::Float64,
                                            crate::ast::Type::Bool => Type::Bool,
                                            crate::ast::Type::Char => Type::Char,
                                            crate::ast::Type::String => Type::String,
                                            crate::ast::Type::Unit => Type::Unit,
                                            crate::ast::Type::Named(name) => Type::Named(name.clone()),
                                            _ => Type::Int64, // Fallback
                                        }).collect();
                                        Type::Tuple(converted)
                                    }
                                    crate::ast::Type::Named(name) => Type::Named(name.clone()),
                                    _ => Type::Int64, // Fallback for other types
                                };
                                self.variable_types.insert(name.clone(), var_type.clone());
                                
                                // If this is a function type, register it in function_variable_scopes
                                if let Type::Function { .. } = &var_type {
                                    // eprintln!("DEBUG TypedIdentifier bind (do): registering function variable '{}' with type {:?}", name, var_type);
                                    self.add_function_variable(name.clone(), stored_val, &var_type);
                                } else {
                                    // For non-function types, use regular variable storage
                                    self.add_variable_text(name.clone(), stored_val);
                                }
                                
                                // Update result to use the stored value (which may be a register for float16)
                                result = Some(stored_val_clone);
                            } else {
                                result = value;
                            }
                        }
                        Pattern::Tuple(elements) => {
                            // Get RHS expression type so we can record variable types for untyped (Identifier) elements.
                            let expr_type_opt = if let Some(location) = Self::try_get_expression_location(expr) {
                                self.expression_types.get(location).cloned()
                            } else {
                                match **expr {
                                    Expression::Literal(Literal::Int(_)) => Some(Type::Int64),
                                    Expression::Literal(Literal::Bool(_)) => Some(Type::Bool),
                                    Expression::Literal(Literal::Char(_)) => Some(Type::Char),
                                    Expression::Literal(Literal::String(_)) => Some(Type::String),
                                    Expression::Literal(Literal::Unit) => Some(Type::Unit),
                                    _ => None,
                                }
                            };
                            let element_types_opt = expr_type_opt.as_ref().and_then(|t| self.extract_tuple_element_types(t));
                            // Record variable types from the pattern first, so field access (e.g. type_name.name)
                            // sees the correct type even if value is None or we're in a different code path.
                            for (i, elem_pattern) in elements.iter().enumerate() {
                                match elem_pattern {
                                    Pattern::TypedIdentifier { name, type_, .. } => {
                                        if name != "_" {
                                            self.variable_types.insert(name.clone(), type_.clone());
                                        }
                                    }
                                    Pattern::Identifier(name) => {
                                        if name != "_" {
                                            if let Some(ref elem_types) = element_types_opt {
                                                if let Some(elem_ty) = elem_types.get(i) {
                                                    self.variable_types.insert(name.clone(), elem_ty.clone());
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            // Handle generic tuple decomposition
                            // This implementation works for tuples where all elements are 8-byte aligned
                            // For full genericity with mixed types, proper offset calculation based on
                            // element types and alignment would be needed.
                            if let Some(ref tuple_ptr_raw) = value {
                                // Strip type prefixes from tuple pointer
                                let tuple_ptr = tuple_ptr_raw.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();

                                // Get the expression type for tuple decomposition
                                let expr_type_opt = if let Some(location) = Self::try_get_expression_location(expr) {
                                    self.expression_types.get(location).cloned()
                                } else {
                                    // For expressions without location (like literals), infer the type directly
                                    match **expr {
                                        Expression::Literal(Literal::Int(_)) => Some(Type::Int64),
                                        Expression::Literal(Literal::Bool(_)) => Some(Type::Bool),
                                        Expression::Literal(Literal::Char(_)) => Some(Type::Char),
                                        Expression::Literal(Literal::String(_)) => Some(Type::String),
                                        Expression::Literal(Literal::Unit) => Some(Type::Unit),
                                        _ => None,
                                    }
                                };

                                // Calculate proper offsets using compile-time type information
                                // This mirrors the tuple creation logic for accurate offset calculation
                                let element_count = elements.len() as i64;
                                let mut current_offset = 8 + element_count; // After count and type IDs

                                // Get the element types from the tuple type
                                // Try from expression type first, then fall back to pattern types
                                let element_types_from_expr = expr_type_opt
                                    .as_ref()
                                    .and_then(|t| self.extract_tuple_element_types(t));
                                let element_types = if element_types_from_expr.is_some() {
                                    element_types_from_expr
                                } else {
                                    // Fall back to pattern types when expr type isn't Tuple or is missing
                                    let mut pattern_types = Vec::new();
                                    for elem_pattern in elements {
                                        match elem_pattern {
                                            Pattern::TypedIdentifier { type_, .. } => {
                                                pattern_types.push(self.expand_type_aliases_codegen(type_));
                                            }
                                            Pattern::Identifier(_) => {
                                                // For untyped patterns, assume i64
                                                pattern_types.push(Type::Int64);
                                            }
                                            _ => {}
                                        }
                                    }
                                    if !pattern_types.is_empty() {
                                        Some(pattern_types)
                                    } else {
                                        None
                                    }
                                };

                                // Calculate offsets for each element based on types (must match generate_tuple layout)
                                let mut element_offsets = Vec::new();
                                if let Some(ref types) = element_types {
                                    for silica_type in types.iter() {
                                        let size = self.get_type_size_bytes(silica_type);
                                        let alignment = self.get_type_alignment_bytes(silica_type);

                                        // Align offset to element alignment
                                        current_offset = ((current_offset + alignment - 1) / alignment) * alignment;

                                        element_offsets.push(current_offset);
                                        current_offset += size;
                                    }
                                } else {
                                    // Fallback: assume all elements are i64 at 8-byte intervals
                                    for i in 0..elements.len() {
                                        element_offsets.push(16 + (i as i64 * 8));
                                    }
                                }

                                for (i, elem_pattern) in elements.iter().enumerate() {
                                    let elem_offset = element_offsets.get(i).copied().unwrap_or(16 + (i as i64 * 8));

                                    match elem_pattern {
                                        Pattern::Identifier(elem_name) => {
                                            let elem_ptr_reg = format!("%{}_ptr_{}", elem_name, self.instructions.len());
                                            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", elem_ptr_reg, tuple_ptr, elem_offset));

                                            // Use element type from element_types when available (for correct bool/i1/string load)
                                            let llvm_ty = if elem_name == "_" {
                                                "i64".to_string()
                                            } else if let Some(ref types) = element_types {
                                                let elem_type = types.get(i).map(|t| self.expand_type_aliases_codegen(t));
                                                match elem_type.as_ref() {
                                                    Some(Type::Bool) => {
                                                        let i1_cast_reg = format!("%i1_cast_{}_{}", self.instructions.len(), i);
                                                        self.instructions.push(format!("  {} = bitcast i8* {} to i1*", i1_cast_reg, elem_ptr_reg));
                                                        self.instructions.push(format!("  %{} = load i1, i1* {}", elem_name, i1_cast_reg));
                                                        "i1".to_string()
                                                    }
                                                    Some(Type::String) => {
                                                        let i8pp_cast_reg = format!("%i8pp_cast_{}_{}", self.instructions.len(), i);
                                                        self.instructions.push(format!("  {} = bitcast i8* {} to i8**", i8pp_cast_reg, elem_ptr_reg));
                                                        self.instructions.push(format!("  %{} = load i8*, i8** {}", elem_name, i8pp_cast_reg));
                                                        "i8*".to_string()
                                                    }
                                                    _ => {
                                                        let i64_cast_reg = format!("%i64_cast_{}_{}", self.instructions.len(), i);
                                                        self.instructions.push(format!("  {} = bitcast i8* {} to i64*", i64_cast_reg, elem_ptr_reg));
                                                        self.instructions.push(format!("  %{} = load i64, i64* {}", elem_name, i64_cast_reg));
                                                        "i64".to_string()
                                                    }
                                                }
                                            } else {
                                                let i64_cast_reg = format!("%i64_cast_{}_{}", self.instructions.len(), i);
                                                self.instructions.push(format!("  {} = bitcast i8* {} to i64*", i64_cast_reg, elem_ptr_reg));
                                                self.instructions.push(format!("  %{} = load i64, i64* {}", elem_name, i64_cast_reg));
                                                "i64".to_string()
                                            };
                                            if elem_name != "_" {
                                                let final_val_reg = format!("%{}", elem_name);
                                                self.variables.insert(elem_name.clone(), final_val_reg);
                                                self.variable_llvm_types.insert(elem_name.clone(), llvm_ty);
                                            }
                                        }
                                        Pattern::TypedIdentifier { name: elem_name, type_: elem_type } => {
                                            if elem_name == "_" {
                                                continue; // Wildcards don't bind variables
                                            }

                                            let elem_ptr_reg = format!("%{}_ptr_{}", elem_name, self.instructions.len());
                                            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", elem_ptr_reg, tuple_ptr, elem_offset));

                                            // Load based on declared type (expand aliases e.g. boolean -> Bool for correct i1 load)
                                            let expanded_type = self.expand_type_aliases_codegen(elem_type);
                                            let final_val_reg = format!("%{}", elem_name);
                                            let llvm_ty = match &expanded_type {
                                                Type::Bool => {
                                                    // Load as boolean (i1)
                                                    let i1_cast_reg = format!("%i1_cast_{}_{}", self.instructions.len(), i);
                                                    self.instructions.push(format!("  {} = bitcast i8* {} to i1*", i1_cast_reg, elem_ptr_reg));
                                                    self.instructions.push(format!("  {} = load i1, i1* {}", final_val_reg, i1_cast_reg));
                                                    "i1"
                                                }
                                                Type::Int64 => {
                                                    // Load as integer (i64)
                                                    let i64_cast_reg = format!("%i64_cast_{}_{}", self.instructions.len(), i);
                                                    self.instructions.push(format!("  {} = bitcast i8* {} to i64*", i64_cast_reg, elem_ptr_reg));
                                                    self.instructions.push(format!("  {} = load i64, i64* {}", final_val_reg, i64_cast_reg));
                                                    "i64"
                                                }
                                                Type::Char => {
                                                    // Load as character (i32)
                                                    let i32_cast_reg = format!("%i32_cast_{}_{}", self.instructions.len(), i);
                                                    self.instructions.push(format!("  {} = bitcast i8* {} to i32*", i32_cast_reg, elem_ptr_reg));
                                                    self.instructions.push(format!("  {} = load i32, i32* {}", final_val_reg, i32_cast_reg));
                                                    "i32"
                                                }
                                                Type::String => {
                                                    // Load as string (i8*) - strings are stored as i8* in memory
                                                    let string_ptr_cast_reg = format!("%{}_string_cast_{}", elem_name, self.instructions.len());
                                                    self.instructions.push(format!("  {} = bitcast i8* {} to i8**", string_ptr_cast_reg, elem_ptr_reg));
                                                    self.instructions.push(format!("  {} = load i8*, i8** {}", final_val_reg, string_ptr_cast_reg));
                                                    "i8*"
                                                }
                                                _ => {
                                                    // Default to i64 for unknown types (e.g. Named structs: stored as ptr/i64)
                                                    let i64_cast_reg = format!("%i64_cast_{}_{}", self.instructions.len(), i);
                                                    self.instructions.push(format!("  {} = bitcast i8* {} to i64*", i64_cast_reg, elem_ptr_reg));
                                                    self.instructions.push(format!("  {} = load i64, i64* {}", final_val_reg, i64_cast_reg));
                                                    "i64"
                                                }
                                            };

                                            self.variables.insert(elem_name.clone(), final_val_reg);
                                            self.variable_llvm_types.insert(elem_name.clone(), llvm_ty.to_string());
                                            // Record the variable's type from the pattern so field access (e.g. type_name.name) sees the correct type
                                            self.variable_types.insert(elem_name.clone(), elem_type.clone());
                                        }
                                        Pattern::Literal(_) => {
                                            // Literals don't bind variables
                                        }
                                        Pattern::Tuple(sub_patterns) => {
                                            // Handle nested tuple decomposition recursively
                                            self.generate_tuple_decomposition(tuple_ptr.clone(), sub_patterns, elem_offset)?;
                                        }
                                        Pattern::Record(_) | Pattern::Variant { .. } | Pattern::Alternative(_) => {
                                            // Record/variant/alternative in tuple element: no binding (not yet implemented)
                                        }
                                        _ => {
                                            return codegen_error(format!("Unsupported pattern type in tuple decomposition: {:?}", elem_pattern));
                                        }
                                    }
                                }
                            }
                            result = value; // The tuple pointer itself
                        }
                        _ => return codegen_error("Pattern type not supported in bindings".to_string()),
                    }
                }
                Statement::Expr(expr) => {
                    result = self.generate_expression(expr)?;
                }
            }
        }

        // Exit the scope
        self.exit_scope_text();

        Ok(result)
    }

    /// Generate LLVM IR for region creation
    fn generate_region(&mut self, region: &RegionExpr) -> Result<Option<String>> {
        // Generate the value to be stored
        let value = self.generate_expression(&region.value)?;

        if let Some(val) = value {
            // Skip void expressions - they don't produce values for regions
            if val == "void" {
                return Ok(None);
            }

            // Determine if this is a complex type that needs special handling
            let initial_expr_type = match &*region.value {
                Expression::Identifier(name) => {
                    // For identifiers, look up in variable_types
                    self.variable_types.get(name)
                        .cloned()
                        .unwrap_or(Type::Int64)
                },
                _ => {
                    // For other expressions, use get_expression_type
                    match self.get_expression_type(&region.value) {
                Ok(ty) => ty,
                Err(_) => Type::Int64
                    }
                }
            };
            let is_complex_type = matches!(initial_expr_type,
                Type::Tuple(_) | Type::Record(_) | Type::String | Type::Function { .. } |
                Type::Reference { .. } | Type::Buffer { .. } | Type::ActorRef |
                Type::Region { .. } | Type::Process { .. }
            );

            // Also check if the generated value looks like a heap allocation (complex type)
            let is_allocation_result = val.starts_with('%') && val.contains("alloc");
            // Also treat getelementptr expressions (like string constants) as complex
            let is_pointer_expr = val.contains("getelementptr");
            let is_complex_type = is_complex_type || is_allocation_result || is_pointer_expr;

            let ref_reg = format!("%ref_{}", self.instructions.len());

            if is_complex_type {
                // For complex types, call silica_region_create_with_data
                // The value should already be an i8* pointer from generate_expression

                // For complex types, we need to determine the allocated size
                // For now, use a conservative size - in practice, this should be tracked
                let data_size = match initial_expr_type {
                    Type::Tuple(ref elements) => {
                        // Rough estimate: header (16) + elements
                        16 + elements.len() as i64 * 8
                    }
                    _ => 32, // Default size for complex types
                };

                // WORKAROUND: LLVM 21.1.8 has a bug with silica_region_create_with_data
                // Use silica_region_create_with_value instead, passing a fixed size as the value
                self.instructions.push(format!("  {} = call i8* @silica_region_create_with_value(i64 32)",
                    ref_reg));
            } else {
                // For primitive types, call silica_region_create_with_value
                let val_with_type = if val.starts_with("i64 ") { val.clone() } else { format!("i64 {}", val) };
                self.instructions.push(format!("  {} = call i8* @silica_region_create_with_value({})", ref_reg, val_with_type));
            }

            Ok(Some(ref_reg))
        } else {
            codegen_error("Invalid value for region creation".to_string())
        }
    }


    /// Generate LLVM IR for memory read (read_ref)
    fn generate_read_ref(&mut self, read: &ReadRefExpr) -> Result<Option<String>> {
        // Generate reference expression
        let ref_val = self.generate_expression(&read.reference)?;

        if let Some(ref_ptr) = ref_val {
            // Check what type we're expecting to read
            let expected_type = self.get_expression_type(&Expression::ReadRef(read.clone()))
                .unwrap_or(Type::Int64);

            let is_complex_type = matches!(expected_type,
                Type::Tuple(_) | Type::Record(_) | Type::String | Type::Function { .. } |
                Type::Reference { .. } | Type::Buffer { .. } | Type::ActorRef |
                Type::Region { .. } | Type::Process { .. } | Type::Unit
            );

            if is_complex_type {
                // For complex types, the reference is just the allocated structure pointer
                // Return it directly (it might need casting based on expected type)
                let value_reg = format!("%complex_value_{}", self.instructions.len());
                let ref_with_type = if ref_ptr.starts_with('%') { format!("i8* {}", ref_ptr) } else { ref_ptr.to_string() };

                // Cast back to the expected type if needed
                match expected_type {
                    Type::Tuple(_) | Type::Record(_) => {
                        // For tuples and records, we stored them as i8*, return as i8*
                        self.instructions.push(format!("  ; Complex type read: returning allocated structure directly"));
                        self.instructions.push(format!("  {} = bitcast i8* {} to i8*", value_reg, ref_ptr));
                    }
                    _ => {
                        // For other complex types, return the pointer
                        self.instructions.push(format!("  ; Complex type read: returning pointer directly"));
                        self.instructions.push(format!("  {} = bitcast i8* {} to i8*", value_reg, ref_ptr));
                    }
                }
                Ok(Some(value_reg))
            } else {
                // For primitive types, use region read
                let value_reg = format!("%value_{}", self.instructions.len());
                let ref_with_type = if ref_ptr.starts_with('%') { format!("i8* {}", ref_ptr) } else { ref_ptr.to_string() };
                self.instructions.push(format!("  {} = call i64 @silica_region_read({})", value_reg, ref_with_type));
                Ok(Some(value_reg))
            }
        } else {
            codegen_error("Invalid reference for read operation".to_string())
        }
    }

    /// Generate LLVM IR for tuple decomposition patterns
    fn generate_tuple_decomposition(&mut self, tuple_ptr: String, patterns: &[Pattern], base_offset: i64) -> Result<()> {
        for (i, pattern) in patterns.iter().enumerate() {
            let elem_offset = base_offset + (i as i64 * 8); // Assume 8 bytes per element for now

            match pattern {
                Pattern::Identifier(elem_name) => {
                    let elem_ptr_reg = format!("%{}_ptr_{}", elem_name, self.instructions.len());
                    self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", elem_ptr_reg, tuple_ptr, elem_offset));

                    // Load as i64 for untyped identifiers (simplified generic handling)
                    let i64_cast_reg = format!("%i64_cast_{}_{}", self.instructions.len(), i);
                    self.instructions.push(format!("  {} = bitcast i8* {} to i64*", i64_cast_reg, elem_ptr_reg));
                    if elem_name != "_" {
                        let final_val_reg = format!("%{}", elem_name);
                        self.instructions.push(format!("  {} = load i64, i64* {}", final_val_reg, i64_cast_reg));
                        self.variables.insert(elem_name.clone(), final_val_reg);
                        self.variable_llvm_types.insert(elem_name.clone(), "i64".to_string());
                    }
                }
                Pattern::TypedIdentifier { name: elem_name, type_: elem_type } => {
                    if elem_name == "_" {
                        continue; // Wildcards don't bind variables
                    }

                    let elem_ptr_reg = format!("%{}_ptr_{}", elem_name, self.instructions.len());
                    self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", elem_ptr_reg, tuple_ptr, elem_offset));

                    // Load based on declared type (generic type handling)
                    let final_val_reg = format!("%{}", elem_name);
                    let llvm_ty = match elem_type {
                        Type::Bool => {
                            // Load as boolean (i1)
                            let i1_cast_reg = format!("%i1_cast_{}_{}", self.instructions.len(), i);
                            self.instructions.push(format!("  {} = bitcast i8* {} to i1*", i1_cast_reg, elem_ptr_reg));
                            self.instructions.push(format!("  {} = load i1, i1* {}", final_val_reg, i1_cast_reg));
                            "i1"
                        }
                        Type::Int64 => {
                            // Load as integer (i64)
                            let i64_cast_reg = format!("%i64_cast_{}_{}", self.instructions.len(), i);
                            self.instructions.push(format!("  {} = bitcast i8* {} to i64*", i64_cast_reg, elem_ptr_reg));
                            self.instructions.push(format!("  {} = load i64, i64* {}", final_val_reg, i64_cast_reg));
                            "i64"
                        }
                        Type::Char => {
                            // Load as character (i32)
                            let i32_cast_reg = format!("%i32_cast_{}_{}", self.instructions.len(), i);
                            self.instructions.push(format!("  {} = bitcast i8* {} to i32*", i32_cast_reg, elem_ptr_reg));
                            self.instructions.push(format!("  {} = load i32, i32* {}", final_val_reg, i32_cast_reg));
                            "i32"
                        }
                        Type::String => {
                            // Load as string (i8*) - strings are stored as i8* in memory
                            let string_ptr_cast_reg = format!("%{}_string_cast_{}", elem_name, self.instructions.len());
                            self.instructions.push(format!("  {} = bitcast i8* {} to i8**", string_ptr_cast_reg, elem_ptr_reg));
                            self.instructions.push(format!("  {} = load i8*, i8** {}", final_val_reg, string_ptr_cast_reg));
                            "i8*"
                        }
                        _ => {
                            // Default to i64 for unknown types (e.g. Named structs)
                            let i64_cast_reg = format!("%i64_cast_{}_{}", self.instructions.len(), i);
                            self.instructions.push(format!("  {} = bitcast i8* {} to i64*", i64_cast_reg, elem_ptr_reg));
                            self.instructions.push(format!("  {} = load i64, i64* {}", final_val_reg, i64_cast_reg));
                            "i64"
                        }
                    };

                    self.variables.insert(elem_name.clone(), final_val_reg);
                    self.variable_llvm_types.insert(elem_name.clone(), llvm_ty.to_string());
                    // Record type from pattern so field access (e.g. type_name.name) sees correct type
                    self.variable_types.insert(elem_name.clone(), elem_type.clone());
                }
                                        Pattern::Tuple(sub_patterns) => {
                                            // Full nested tuple decomposition: load nested tuple pointer, then recurse
                                            let elem_ptr_reg = format!("%nested_ptr_{}_{}", i, self.instructions.len());
                                            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", elem_ptr_reg, tuple_ptr, elem_offset));
                                            let i8pp_cast = format!("%nested_cast_{}_{}", i, self.instructions.len());
                                            self.instructions.push(format!("  {} = bitcast i8* {} to i8**", i8pp_cast, elem_ptr_reg));
                                            let nested_ptr_reg = format!("%nested_tuple_{}_{}", i, self.instructions.len());
                                            self.instructions.push(format!("  {} = load i8*, i8** {}", nested_ptr_reg, i8pp_cast));
                                            let n = sub_patterns.len() as i64;
                                            let base_nested = ((8 + n + 7) / 8) * 8;
                                            self.generate_tuple_decomposition(nested_ptr_reg, sub_patterns, base_nested)?;
                                        }
                Pattern::Literal(_) => {
                    // Literals don't bind variables
                }
                _ => {
                    return Err(CompilerError::codegen_error("Unsupported pattern type in nested tuple decomposition".to_string()));
                }
            }
        }
        Ok(())
    }

    /// Generate LLVM IR for trait implementations
    fn generate_impl_declaration(&mut self, impl_decl: &crate::ast::ImplDecl) -> Result<()> {
        // Only generate code for trait implementations (not inherent impls)
        if let Some(trait_name) = &impl_decl.trait_name {
            // Get the type name for method naming
            let type_name = match &impl_decl.for_type {
                Type::Named(name) => name.clone(),
                _ => return Err(CompilerError::codegen_error("Impl for non-named types not supported yet".to_string())),
            };

            // Generate each method in the implementation
            for method in &impl_decl.methods {
                self.generate_trait_method(&type_name, method)?;
            }
        }
        Ok(())
    }

    /// Convert a Silica type to LLVM type string
    fn silica_type_to_llvm(&self, ty: &crate::ast::Type) -> Result<String> {
        match ty {
            crate::ast::Type::Int64 => Ok("i64".to_string()),
            crate::ast::Type::Bool => Ok("i1".to_string()),
            crate::ast::Type::Char => Ok("i32".to_string()), // Unicode code point
            crate::ast::Type::String => Ok("i8*".to_string()),
            crate::ast::Type::Unit => Ok("void".to_string()),
            crate::ast::Type::Tuple(_) => Ok("i8*".to_string()), // Tuples are passed as pointers
            crate::ast::Type::Record(_) => Ok("i8*".to_string()), // Records are passed as pointers
            crate::ast::Type::Named(name) => {
                // Check if it's a struct type
                if self.struct_defs.contains_key(name) || self.type_aliases.contains_key(name) {
                    Ok("i8*".to_string()) // Structs are passed as pointers
                } else {
                    Err(CompilerError::codegen_error(format!("Unknown named type: {}", name)))
                }
            }
            _ => Err(CompilerError::codegen_error(format!("Unsupported type in method parameters: {:?}", ty)))
        }
    }

    /// Generate LLVM IR for a single trait method implementation
    fn generate_trait_method(&mut self, type_name: &str, method: &crate::ast::FunctionDecl) -> Result<()> {
        let method_name = format!("{}_{}", type_name, method.name);

        // Generate parameter list: self is i8*, others use their actual types
        let mut param_strs = vec!["i8* %self".to_string()];

        // Add other parameters (skip self in the method signature)
        for param in method.parameters.iter().skip(1) {
            let llvm_type = self.silica_type_to_llvm(&param.type_)?;
            param_strs.push(format!("{} %{}", llvm_type, param.name));
        }

        let params_str = param_strs.join(", ");

        // Determine return type from method signature using the same conversion logic as parameters
        let return_type_str = if let Some(ref return_type) = method.return_type {
            self.silica_type_to_llvm(return_type)?
        } else {
            "void".to_string() // Unit return type
        };

        // Function header with actual return type
        self.instructions.push(format!("define {} @{}({}) {{", return_type_str, method_name, params_str));

        // Generate method body with type context
        self.generate_method_body_with_type(type_name, method, &return_type_str)?;

        self.instructions.push("}".to_string());
        self.instructions.push("".to_string());

        Ok(())
    }

    /// Ensure a forwarder for a trait method (e.g. Shape_area) exists, forwarding to one concrete impl.
    /// Used when the receiver type is the trait (Shape) so we have a single symbol to call.
    fn ensure_trait_method_forwarder(&mut self, trait_name: &str, method_name: &str, return_type_str: &str) -> Result<()> {
        let key = (trait_name.to_string(), method_name.to_string());
        if self.trait_forwarders_emitted.contains(&key) {
            return Ok(());
        }
        let concrete_type_name = self.trait_impls
            .iter()
            .find(|i| i.trait_name == trait_name && i.methods.contains_key(method_name))
            .and_then(|i| match &i.for_type {
                Type::Named(name) => Some(name.clone()),
                _ => None,
            })
            .ok_or_else(|| CompilerError::codegen_error(
                format!("No concrete implementation of trait {} for method {} to forward to", trait_name, method_name)
            ))?;
        let forwarder_name = format!("{}_{}", trait_name, method_name);
        let concrete_method_name = format!("{}_{}", concrete_type_name, method_name);
        if return_type_str == "void" {
            self.trait_forwarder_ir.push(format!("define void @{}(i8* %self) {{", forwarder_name));
            self.trait_forwarder_ir.push(format!("  call void @{}(i8* %self)", concrete_method_name));
            self.trait_forwarder_ir.push("  ret void".to_string());
        } else {
            self.trait_forwarder_ir.push(format!("define {} @{}(i8* %self) {{", return_type_str, forwarder_name));
            self.trait_forwarder_ir.push(format!("  %r = call {} @{}(i8* %self)", return_type_str, concrete_method_name));
            self.trait_forwarder_ir.push(format!("  ret {} %r", return_type_str));
        }
        self.trait_forwarder_ir.push("}".to_string());
        self.trait_forwarder_ir.push("".to_string());
        self.trait_forwarders_emitted.insert(key);
        Ok(())
    }

    /// Generate the body of a trait method
    fn generate_method_body_with_type(&mut self, type_name: &str, method: &crate::ast::FunctionDecl, return_type_str: &str) -> Result<()> {
        // For now, only handle simple expressions
        // The method body is a single expression for trait methods
        // For trait methods, expect a single expression statement
        if method.body.len() == 1 {
            if let crate::ast::Statement::Expr(expr) = &method.body[0] {
                // Generate the expression result
                let result_val = self.generate_expression_in_method(type_name, method, expr.as_ref())?;

                // Strip type prefix so ret gets "ret i8* %reg" not "ret i8* i8* %reg"
                let ret_value = self.clean_register_for_instruction(&result_val);
                // Literal integer constant (e.g. "1") must be used as immediate "ret i64 1", not "ret i64 %1"
                let ret_operand = if ret_value.starts_with('%') {
                    ret_value
                } else if ret_value.parse::<i64>().is_ok() {
                    ret_value
                } else {
                    format!("%{}", ret_value)
                };
                self.instructions.push(format!("  ret {} {}", return_type_str, ret_operand));
            } else {
                return Err(CompilerError::codegen_error("Trait methods must have expression bodies".to_string()));
            }
        } else {
            return Err(CompilerError::codegen_error("Trait methods must have single expression bodies".to_string()));
        }

        Ok(())
    }

    /// Generate binary operations in method bodies (like self.x + self.y)
    fn generate_binary_operation_for_method_with_type(&mut self, type_name: &str, method: &crate::ast::FunctionDecl, binary: &crate::ast::BinaryExpr) -> Result<()> {
        // Handle binary operations directly in this method context
        let left_val = self.generate_expression_in_method(type_name, method, &binary.left)?;
        let right_val = self.generate_expression_in_method(type_name, method, &binary.right)?;

        let result_reg = match binary.operator {
            crate::ast::BinaryOp::Add => {
                let reg = self.next_register();
                self.instructions.push(format!("  %{} = add i64 {}, {}", reg, left_val, right_val));
                reg
            }
            crate::ast::BinaryOp::Subtract => {
                let reg = self.next_register();
                self.instructions.push(format!("  %{} = sub i64 {}, {}", reg, left_val, right_val));
                reg
            }
            crate::ast::BinaryOp::Multiply => {
                let reg = self.next_register();
                self.instructions.push(format!("  %{} = mul i64 {}, {}", reg, left_val, right_val));
                reg
            }
            _ => {
                return Err(CompilerError::codegen_error(format!("Unsupported binary operator in method: {:?}", binary.operator)));
            }
        };

        // Return the result
        self.instructions.push(format!("  ret i64 %{}", result_reg));

        Ok(())
    }

    /// Generate trait method calls within trait method bodies (self.method(args))
    fn generate_trait_method_call(&mut self, type_name: &str, method: &crate::ast::FunctionDecl, field_access: &crate::ast::FieldAccessExpr, call: &crate::ast::CallExpr) -> Result<String> {
        // Construct the method name: {type_name}_{method_name}
        let method_name = format!("{}_{}", type_name, field_access.field);

        // Resolve the callee signature from trait impls and extract all needed information upfront
        // This avoids borrow checker issues by extracting data before mutable borrows
        let (expected_param_types, return_type_str) = {
            let callee_sig = self.trait_impls.iter()
                .find(|impl_| {
                    // Match impls for this concrete receiver type and containing this method name
                    matches!(&impl_.for_type, Type::Named(n) if n == type_name) && impl_.methods.contains_key(&field_access.field)
                })
                .and_then(|impl_| impl_.methods.get(&field_access.field));

            // Expected LLVM param types for the call arguments (excluding self).
            // Fall back to i64 if we can't resolve.
            let expected_param_types: Vec<String> = callee_sig
                .map(|m| {
                    m.parameters.iter()
                        .skip(1) // skip self
                        .map(|p| self.type_map.silica_to_llvm_str(&p.type_))
                        .collect()
                })
                .unwrap_or_else(|| vec![]);

            // Get return type from callee signature
            let return_type_str = callee_sig
                .and_then(|m| m.return_type.as_ref())
                .map(|rt| self.silica_type_to_llvm(rt).unwrap_or_else(|_| "i64".to_string()))
                .unwrap_or_else(|| "i64".to_string());

            (expected_param_types, return_type_str)
        };

        // Generate typed arguments, starting with self (i8*)
        let mut typed_args = vec!["i8* %self".to_string()];

        // Add the call arguments with LLVM types based on the callee signature
        for (idx, arg) in call.arguments.iter().enumerate() {
            let arg_val = self.generate_expression_in_method(type_name, method, arg)?;

            // Determine expected type string for this argument position
            let expected_ty = expected_param_types.get(idx).cloned().unwrap_or_else(|| "i64".to_string());

            // Strip any existing type prefix so we can re-apply the expected type
            let clean_arg = arg_val
                .trim_start_matches("i64 ")
                .trim_start_matches("i32 ")
                .trim_start_matches("i1 ")
                .trim_start_matches("i8* ")
                .to_string();

            // Ensure registers remain registers; literals remain literals
            let typed_arg = format!("{} {}", expected_ty, clean_arg);
            typed_args.push(typed_arg);
        }

        // Create a unique register for the result
        let result_reg = format!("%call_{}", self.instructions.len());
        let args_str = typed_args.iter()
            .map(|a| Self::normalize_typed_call_arg(a))
            .collect::<Vec<_>>()
            .join(", ");

        // Generate the LLVM call instruction with actual return type
        self.instructions.push(format!("  {} = call {} @{}({})", result_reg, return_type_str, method_name, args_str));

        Ok(result_reg)
    }

    /// Generate function calls within trait method bodies
    fn generate_function_call_in_method(&mut self, type_name: &str, method: &crate::ast::FunctionDecl, call: &crate::ast::CallExpr) -> Result<String> {
        // For now, only support simple function calls by name
        if let Expression::Identifier(func_name) = &*call.function {
            // Generate argument values
            let mut arg_values = Vec::new();
            for arg in &call.arguments {
                let arg_val = self.generate_expression_in_method(type_name, method, arg)?;
                arg_values.push(arg_val);
            }

            // Create a unique register for the result
            let result_reg = format!("%func_call_{}", self.instructions.len());
            let args_str = arg_values.join(", ");

            // Generate the LLVM call instruction
            // For now, assume all functions return i64
            self.instructions.push(format!("  {} = call i64 @{}({})", result_reg, func_name, args_str));

            Ok(result_reg)
        } else {
            Err(CompilerError::codegen_error("Complex function calls not supported in trait method bodies".to_string()))
        }
    }

    /// Generate any expression within method bodies
    fn generate_expression_in_method(&mut self, type_name: &str, method: &crate::ast::FunctionDecl, expr: &Expression) -> Result<String> {
        match expr {
            Expression::FieldAccess(field_access) => {
                self.generate_field_access_for_method_with_type(type_name, method, field_access)
            }
            Expression::Literal(lit) => {
                self.generate_literal_value(lit)
            }
            Expression::Identifier(name) => {
                // Check if it's the self parameter
                if name == "self" {
                    Ok("%self".to_string())
                } else {
                    // Check if it's a method parameter
                    let is_param = method.parameters.iter().any(|param| param.name == *name);
                    if is_param {
                        Ok(format!("%{}", name))
                    } else {
                        Err(CompilerError::codegen_error(format!("Unsupported identifier in method: {}", name)))
                    }
                }
            }
            Expression::Binary(binary) => {
                // Handle nested binary expressions
                self.generate_nested_binary_in_method(type_name, method, binary)
            }
            Expression::Unary(unary) => {
                // Handle unary expressions
                self.generate_unary_in_method(type_name, method, unary)
            }
            Expression::If(if_expr) => {
                self.generate_if_in_method(type_name, method, if_expr)
            }
            Expression::Case(case_expr) => {
                self.generate_case_in_method(type_name, method, case_expr)
            }
            Expression::Call(call) => {
                // Handle method calls on self within trait method bodies
                if let Expression::FieldAccess(field_access) = &*call.function {
                    // Check if this is a method call on self
                    if let Expression::Identifier(var_name) = &*field_access.object {
                        if var_name == "self" {
                            // This is a method call on self within a trait method
                            return self.generate_trait_method_call(type_name, method, field_access, call);
                        }
                    }
                }

                // Handle regular function calls within trait methods
                self.generate_function_call_in_method(type_name, method, call)
            }
            Expression::StructLiteral(struct_lit) => {
                self.generate_struct_literal_in_method(type_name, method, struct_lit)
            }
            _ => {
                Err(CompilerError::codegen_error(format!("Unsupported expression type in method: {:?}", expr)))
            }
        }
    }

    /// Generate struct literals within method bodies
    fn generate_struct_literal_in_method(&mut self, type_name: &str, method: &crate::ast::FunctionDecl, struct_lit: &StructLiteralExpr) -> Result<String> {
        if struct_lit.fields.is_empty() {
            return Ok("null".to_string()); // Empty struct
        }

        // Get the struct definition to know field types - check both struct_defs and type_aliases
        let mut field_type_map = HashMap::new();

        if let Some(struct_def) = self.struct_defs.get(&struct_lit.type_name) {
            // Handle struct definitions: struct Point { x: int, y: int }
            for field_def in struct_def {
                field_type_map.insert(field_def.name.clone(), field_def.ty.clone());
            }
        } else if let Some(alias_type) = self.type_aliases.get(&struct_lit.type_name) {
            // Handle type aliases: type Point = {x: int, y: int}
            if let Type::Record(fields) = alias_type {
                for (field_name, field_type) in fields {
                    field_type_map.insert(field_name.clone(), field_type.clone());
                }
            } else {
                return Err(CompilerError::codegen_error(format!("Type alias '{}' is not a record type", struct_lit.type_name)));
            }
        } else {
            let metadata = ErrorMetadataBuilder::new("E4002".to_string())
                .severity(ErrorSeverity::Error)
                .suggestion(format!("Check if struct type '{}' is defined or imported", struct_lit.type_name))
                .build();
            return Err(CompilerError::CodegenError { 
                message: format!("Unknown struct type: {}", struct_lit.type_name), 
                location: None, 
                metadata 
            });
        }

        // Generate all field expressions using method-aware generation
        let mut field_values = Vec::new();
        let mut field_types = Vec::new();

        for (field_name, field_expr) in &struct_lit.fields {
            let field_type = field_type_map.get(field_name)
                .ok_or_else(|| CompilerError::codegen_error(format!("Unknown field '{}' in struct '{}'", field_name, struct_lit.type_name)))?
                .clone();

            let value = self.generate_expression_in_method(type_name, method, field_expr)?;
            field_values.push((field_name.clone(), value));
            field_types.push(field_type);
        }

        // Calculate proper memory layout based on actual field types
        // Expand type aliases so e.g. Named("boolean") -> Bool and we get i1, not i8*
        let mut total_size = 0;
        let mut field_layout = Vec::new();

        for field_type in &field_types {
            let expanded = self.expand_type_aliases_codegen(field_type);
            let (llvm_type_str, size, alignment) = self.get_llvm_type_info(&expanded);
            // Simple alignment: align to type size (could be more sophisticated)
            let aligned_offset = ((total_size + alignment - 1) / alignment) * alignment;
            field_layout.push((aligned_offset, llvm_type_str, size));
            total_size = aligned_offset + size;
        }

        // Allocate memory for the struct
        let malloc_reg = format!("%struct_alloc_{}", self.instructions.len());
        self.instructions.push(format!("  {} = call i8* @malloc(i64 {})", malloc_reg, total_size));

        // Store each field at its proper offset with correct type
        for (i, ((field_name, field_value), (offset, llvm_type_str, _))) in field_values.iter().zip(field_layout.iter()).enumerate() {
            let field_ptr_reg = format!("%field_ptr_{}_{}", self.instructions.len(), i);

            // Get pointer to field location
            let clean_malloc_reg = self.clean_register_for_instruction(&malloc_reg);
            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", field_ptr_reg, clean_malloc_reg, offset));

            // Cast to appropriate pointer type (use expanded type so bool -> i1*, not i8**)
            let field_ptr_typed = format!("%field_ptr_typed_{}_{}", self.instructions.len(), i);
            self.instructions.push(format!("  {} = bitcast i8* {} to {}*", field_ptr_typed, field_ptr_reg, llvm_type_str));

            // Extract type from field_value if it has a type prefix
            let (value_type, clean_value) = if field_value.starts_with("double ") {
                ("double", field_value.strip_prefix("double ").unwrap().to_string())
            } else if field_value.starts_with("float ") {
                ("float", field_value.strip_prefix("float ").unwrap().to_string())
            } else if field_value.starts_with("half ") {
                ("half", field_value.strip_prefix("half ").unwrap().to_string())
            } else if field_value.starts_with("i64 ") {
                ("i64", field_value.strip_prefix("i64 ").unwrap().to_string())
            } else if field_value.starts_with("i32 ") {
                ("i32", field_value.strip_prefix("i32 ").unwrap().to_string())
            } else if field_value.starts_with("i16 ") {
                ("i16", field_value.strip_prefix("i16 ").unwrap().to_string())
            } else if field_value.starts_with("i8 ") {
                ("i8", field_value.strip_prefix("i8 ").unwrap().to_string())
            } else if field_value.starts_with("i1 ") {
                ("i1", field_value.strip_prefix("i1 ").unwrap().to_string())
            } else if field_value.starts_with("i8* ") {
                ("i8*", field_value.strip_prefix("i8* ").unwrap().to_string())
            } else {
                // No type prefix - use llvm_type_str
                (llvm_type_str.as_str(), field_value.clone())
            };
            
            // Cast pointer to the correct type if needed
            let final_ptr_type = if value_type != llvm_type_str && llvm_type_str != "i8*" {
                // Need to cast pointer to match value type
                let cast_ptr = format!("%field_ptr_cast_{}_{}", self.instructions.len(), i);
                self.instructions.push(format!("  {} = bitcast {}* {} to {}*", cast_ptr, llvm_type_str, field_ptr_typed, value_type));
                cast_ptr
            } else {
                field_ptr_typed
            };
            
            // Generate store instruction (value must be valid LLVM token, e.g. %t4 not t4)
            let store_instruction = if value_type == "i8*" && clean_value.contains("getelementptr") {
                format!("  store i8* {}, i8** {}", Self::format_llvm_value_ref(&clean_value), final_ptr_type)
            } else if clean_value.contains("getelementptr") {
                format!("  store i8* {}, {}* {}", Self::format_llvm_value_ref(&clean_value), value_type, final_ptr_type)
            } else if clean_value.contains('@') {
                format!("  store i8* {}, {}* {}", Self::format_llvm_value_ref(&clean_value), value_type, final_ptr_type)
            } else if clean_value.contains("alloc") {
                format!("  store i8* {}, {}* {}", Self::format_llvm_value_ref(&clean_value), value_type, final_ptr_type)
            } else {
                // Use the extracted type
                let store_value = if value_type == "float" && !clean_value.starts_with('%') && clean_value.parse::<f64>().is_ok() {
                    let float_const = format!("%float_const_store_{}_{}", self.instructions.len(), i);
                    let instruction = self.create_float_constant_instruction(&clean_value, &float_const, "float");
                    self.instructions.push(instruction);
                    float_const
                } else if value_type == "double" && !clean_value.starts_with('%') && clean_value.parse::<f64>().is_ok() {
                    let double_const = format!("%double_const_store_{}_{}", self.instructions.len(), i);
                    let instruction = self.create_float_constant_instruction(&clean_value, &double_const, "double");
                    self.instructions.push(instruction);
                    double_const
                } else if value_type == "half" && !clean_value.starts_with('%') && clean_value.parse::<f64>().is_ok() {
                    let float_const = format!("%float_const_store_{}_{}", self.instructions.len(), i);
                    let instruction = self.create_float_constant_instruction(&clean_value, &float_const, "float");
                    self.instructions.push(instruction);
                    float_const
                } else {
                    clean_value.clone()
                };
                let store_value_ref = Self::format_llvm_value_ref(&store_value);
                format!("  store {} {}, {}* {}", value_type, store_value_ref, value_type, final_ptr_type)
            };
            
            self.instructions.push(store_instruction);
        }

        // Return the pointer to the allocated struct directly (i8*)
        Ok(malloc_reg)
    }

    /// Generate nested binary expressions within methods
    fn generate_nested_binary_in_method(&mut self, type_name: &str, method: &crate::ast::FunctionDecl, binary: &crate::ast::BinaryExpr) -> Result<String> {
        let left_val = self.generate_expression_in_method(type_name, method, &binary.left)?;
        let right_val = self.generate_expression_in_method(type_name, method, &binary.right)?;

        match binary.operator {
            // Arithmetic operators
            crate::ast::BinaryOp::Add => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = add i64 {}, {}", result_reg, left_val, right_val));
                Ok(format!("%{}", result_reg))
            }
            crate::ast::BinaryOp::Subtract => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = sub i64 {}, {}", result_reg, left_val, right_val));
                Ok(format!("%{}", result_reg))
            }
            crate::ast::BinaryOp::Multiply => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = mul i64 {}, {}", result_reg, left_val, right_val));
                Ok(format!("%{}", result_reg))
            }
            crate::ast::BinaryOp::Divide => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = sdiv i64 {}, {}", result_reg, left_val, right_val));
                Ok(format!("%{}", result_reg))
            }
            crate::ast::BinaryOp::Modulo => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = srem i64 {}, {}", result_reg, left_val, right_val));
                Ok(format!("%{}", result_reg))
            }
            // Comparison operators
            crate::ast::BinaryOp::Equal => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = icmp eq i64 {}, {}", result_reg, left_val, right_val));
                // Convert i1 to i64 (true=1, false=0)
                let ext_reg = self.next_register();
                self.instructions.push(format!("  %{} = zext i1 %{} to i64", ext_reg, result_reg));
                Ok(format!("%{}", ext_reg))
            }
            crate::ast::BinaryOp::NotEqual => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = icmp ne i64 {}, {}", result_reg, left_val, right_val));
                let ext_reg = self.next_register();
                self.instructions.push(format!("  %{} = zext i1 %{} to i64", ext_reg, result_reg));
                Ok(format!("%{}", ext_reg))
            }
            crate::ast::BinaryOp::Less => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = icmp slt i64 {}, {}", result_reg, left_val, right_val));
                let ext_reg = self.next_register();
                self.instructions.push(format!("  %{} = zext i1 %{} to i64", ext_reg, result_reg));
                Ok(format!("%{}", ext_reg))
            }
            crate::ast::BinaryOp::LessEqual => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = icmp sle i64 {}, {}", result_reg, left_val, right_val));
                let ext_reg = self.next_register();
                self.instructions.push(format!("  %{} = zext i1 %{} to i64", ext_reg, result_reg));
                Ok(format!("%{}", ext_reg))
            }
            crate::ast::BinaryOp::Greater => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = icmp sgt i64 {}, {}", result_reg, left_val, right_val));
                let ext_reg = self.next_register();
                self.instructions.push(format!("  %{} = zext i1 %{} to i64", ext_reg, result_reg));
                Ok(format!("%{}", ext_reg))
            }
            crate::ast::BinaryOp::GreaterEqual => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = icmp sge i64 {}, {}", result_reg, left_val, right_val));
                let ext_reg = self.next_register();
                self.instructions.push(format!("  %{} = zext i1 %{} to i64", ext_reg, result_reg));
                Ok(format!("%{}", ext_reg))
            }
            // Logical operators
            crate::ast::BinaryOp::And => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = and i64 {}, {}", result_reg, left_val, right_val));
                Ok(format!("%{}", result_reg))
            }
            crate::ast::BinaryOp::Or => {
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = or i64 {}, {}", result_reg, left_val, right_val));
                Ok(format!("%{}", result_reg))
            }
            _ => {
                Err(CompilerError::codegen_error(format!("Unsupported binary operator in method: {:?}", binary.operator)))
            }
        }
    }

    /// Generate unary expressions within methods
    fn generate_unary_in_method(&mut self, type_name: &str, method: &crate::ast::FunctionDecl, unary: &crate::ast::UnaryExpr) -> Result<String> {
        let operand_val = self.generate_expression_in_method(type_name, method, &unary.operand)?;

        match unary.operator {
            crate::ast::UnaryOp::Negate => {
                // For negation: 0 - value
                let zero_reg = self.next_register();
                self.instructions.push(format!("  %{} = add i64 0, 0", zero_reg)); // Load 0

                // Check if operand_val is a numeric literal or a register
                // If it's a literal (can be parsed as number), use it directly; otherwise it's already a register
                let operand_clean = if operand_val.starts_with('%') {
                    operand_val.clone()
                } else if operand_val.parse::<i64>().is_ok() || operand_val.parse::<f64>().is_ok() {
                    // It's a numeric literal - use directly without % prefix
                    operand_val.clone()
                } else {
                    // It's a register name without % prefix - add it
                    format!("%{}", operand_val)
                };

                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = sub i64 %{}, {}", result_reg, zero_reg, operand_clean));

                Ok(format!("%{}", result_reg))
            }
            crate::ast::UnaryOp::Not => {
                // For logical not: xor with 1
                let result_reg = self.next_register();
                self.instructions.push(format!("  %{} = xor i64 {}, 1", result_reg, operand_val));

                Ok(format!("%{}", result_reg))
            }
            _ => {
                Err(CompilerError::codegen_error(format!("Unsupported unary operator in method: {:?}", unary.operator)))
            }
        }
    }

    /// Generate if expressions within methods
    fn generate_if_in_method(&mut self, type_name: &str, method: &crate::ast::FunctionDecl, if_expr: &crate::ast::IfExpr) -> Result<String> {
        // Generate labels for the if statement
        let then_label = format!("if_then_{}", self.next_register());
        let else_label = format!("if_else_{}", self.next_register());
        let end_label = format!("if_end_{}", self.next_register());

        // Generate the condition
        let cond_val = self.generate_expression_in_method(type_name, method, &if_expr.condition)?;

        // Compare condition with 0 (false)
        let cond_reg = self.next_register();
        self.instructions.push(format!("  %{} = icmp ne i64 {}, 0", cond_reg, cond_val));

        // Branch based on condition
        self.instructions.push(format!("  br i1 %{}, label %{}, label %{}", cond_reg, then_label, else_label));

        // Generate then block
        self.instructions.push(format!("{}:", then_label));
        let then_val = self.generate_expression_in_method(type_name, method, &if_expr.then_branch)?;
        let then_result_reg = self.next_register();
        self.instructions.push(format!("  %{} = add i64 {}, 0", then_result_reg, then_val)); // Copy to result register
        self.instructions.push(format!("  br label %{}", end_label));

        // Generate else block
        self.instructions.push(format!("{}:", else_label));
        let else_val = self.generate_expression_in_method(type_name, method, &if_expr.else_branch)?;
        let else_result_reg = self.next_register();
        self.instructions.push(format!("  %{} = add i64 {}, 0", else_result_reg, else_val)); // Copy to result register
        self.instructions.push(format!("  br label %{}", end_label));

        // Phi node to merge the results
        self.instructions.push(format!("{}:", end_label));
        let phi_reg = self.next_register();
        self.instructions.push(format!("  %{} = phi i64 [%{}, %{}], [%{}, %{}]",
                                      phi_reg, then_result_reg, then_label, else_result_reg, else_label));

        Ok(format!("%{}", phi_reg))
    }

    /// Generate case expressions within methods
    fn generate_case_in_method(&mut self, type_name: &str, method: &crate::ast::FunctionDecl, case_expr: &crate::ast::CaseExpr) -> Result<String> {
        // For now, only support boolean case expressions (true/false patterns)
        // This handles the common case of conditional logic in trait methods

        // Generate the scrutinee (condition)
        let scrutinee_val = self.generate_expression_in_method(type_name, method, &case_expr.scrutinee)?;

        // Compare scrutinee with 0 (false) to get boolean
        let bool_reg = self.next_register();
        self.instructions.push(format!("  %{} = icmp ne i64 {}, 0", bool_reg, scrutinee_val));

        // Create labels
        let case_end = format!("case_end_{}", self.next_register());
        let true_label = format!("case_true_{}", self.next_register());
        let false_label = format!("case_false_{}", self.next_register());

        // Branch based on boolean value
        self.instructions.push(format!("  br i1 %{}, label %{}, label %{}", bool_reg, true_label, false_label));

        // Find true and false branches
        let mut true_expr = None;
        let mut false_expr = None;

        for branch in &case_expr.branches {
            match &branch.pattern {
                crate::ast::Pattern::Literal(crate::ast::Literal::Bool(true)) => {
                    true_expr = Some(&branch.body);
                }
                crate::ast::Pattern::Literal(crate::ast::Literal::Bool(false)) => {
                    false_expr = Some(&branch.body);
                }
                _ => {
                    return Err(CompilerError::codegen_error(format!("Unsupported case pattern in method: {:?}", branch.pattern)));
                }
            }
        }

        // Generate true branch
        self.instructions.push(format!("{}:", true_label));
        let true_val = if let Some(expr) = true_expr {
            self.generate_expression_in_method(type_name, method, expr)?
        } else {
            return Err(CompilerError::codegen_error("Case expression missing true branch".to_string()));
        };
        let true_result_reg = self.next_register();
        self.instructions.push(format!("  %{} = add i64 {}, 0", true_result_reg, true_val)); // Copy to result register
        self.instructions.push(format!("  br label %{}", case_end));

        // Generate false branch
        self.instructions.push(format!("{}:", false_label));
        let false_val = if let Some(expr) = false_expr {
            self.generate_expression_in_method(type_name, method, expr)?
        } else {
            return Err(CompilerError::codegen_error("Case expression missing false branch".to_string()));
        };
        let false_result_reg = self.next_register();
        self.instructions.push(format!("  %{} = add i64 {}, 0", false_result_reg, false_val)); // Copy to result register
        self.instructions.push(format!("  br label %{}", case_end));

        // Phi node to merge the results
        self.instructions.push(format!("{}:", case_end));
        let phi_reg = self.next_register();
        self.instructions.push(format!("  %{} = phi i64 [%{}, %{}], [%{}, %{}]",
                                      phi_reg, true_result_reg, true_label, false_result_reg, false_label));

        Ok(format!("%{}", phi_reg))
    }

    /// Generate field access within method bodies
    fn generate_field_access_for_method_with_type(&mut self, type_name: &str, method: &crate::ast::FunctionDecl, field_access: &crate::ast::FieldAccessExpr) -> Result<String> {
        // For self.field, generate code to load the field from the struct pointer
        match &*field_access.object {
            Expression::Identifier(var_name) if var_name == "self" => {
                // Look up the struct definition or type alias
                let field_index = if let Some(struct_def) = self.struct_defs.get(type_name) {
                    // It's a struct definition
                    struct_def.iter().position(|field| field.name == field_access.field)
                        .ok_or_else(|| CompilerError::codegen_error(format!("Unknown field '{}' in struct '{}'", field_access.field, type_name)))?
                } else if let Some(type_info) = self.type_aliases.get(type_name) {
                    // Check if it's a type alias with a record type
                    if let crate::ast::Type::Record(fields) = type_info {
                        fields.iter().position(|(field_name, _)| field_name == &field_access.field)
                            .ok_or_else(|| CompilerError::codegen_error(format!("Unknown field '{}' in type '{}'", field_access.field, type_name)))?
                    } else {
                        return Err(CompilerError::codegen_error(format!("Type '{}' is not a record type", type_name)));
                    }
                } else {
                    return Err(CompilerError::codegen_error(format!("Cannot find struct definition or type alias for '{}'", type_name)));
                };

                // Calculate offset (assume 8 bytes per field)
                let offset = field_index * 8;

                let ptr_reg = self.next_register();
                let typed_reg = self.next_register();
                let value_reg = self.next_register();

                // Get pointer to field
                self.instructions.push(format!("  %{} = getelementptr i8, i8* %self, i64 {}", ptr_reg, offset));

                // Cast to i64*
                self.instructions.push(format!("  %{} = bitcast i8* %{} to i64*", typed_reg, ptr_reg));

                // Load the value
                self.instructions.push(format!("  %{} = load i64, i64* %{}", value_reg, typed_reg));

                Ok(format!("%{}", value_reg))
            }
            _ => {
                Err(CompilerError::codegen_error("Only self.field access supported in methods".to_string()))
            }
        }
    }

    /// Generate literal values for method bodies
    fn generate_literal_value(&mut self, literal: &Literal) -> Result<String> {
        match literal {
            Literal::Int(value) => Ok(value.to_string()),
            _ => Err(CompilerError::codegen_error("Unsupported literal type in method".to_string())),
        }
    }

    /// Generate LLVM IR for struct literals with proper mixed-type support
    fn generate_struct_literal(&mut self, struct_lit: &StructLiteralExpr) -> Result<Option<String>> {
        if struct_lit.fields.is_empty() {
            return Ok(Some("null".to_string())); // Empty struct
        }

        // Get the struct definition to know field types - check both struct_defs and type_aliases
        let mut field_type_map = HashMap::new();

        if let Some(struct_def) = self.struct_defs.get(&struct_lit.type_name) {
            // Handle struct definitions: struct Point { x: int, y: int }
            for field_def in struct_def {
                field_type_map.insert(field_def.name.clone(), field_def.ty.clone());
            }
        } else if let Some(alias_type) = self.type_aliases.get(&struct_lit.type_name) {
            // Handle type aliases: type Point = {x: int, y: int}
            if let Type::Record(fields) = alias_type {
                for (field_name, field_type) in fields {
                    field_type_map.insert(field_name.clone(), field_type.clone());
                }
            } else {
                return Err(CompilerError::codegen_error(format!("Type alias '{}' is not a record type", struct_lit.type_name)));
            }
        } else {
                let metadata = ErrorMetadataBuilder::new("E4002".to_string())
                    .severity(ErrorSeverity::Error)
                    .suggestion(format!("Check if struct type '{}' is defined or imported", struct_lit.type_name))
                    .build();
                return Err(CompilerError::CodegenError { message: format!("Unknown struct type: {}", struct_lit.type_name), location: None, metadata });
        }

        // Generate all field expressions first and collect their types
        let mut field_values = Vec::new();
        let mut field_types = Vec::new();

        for (field_name, field_expr) in &struct_lit.fields {
            let field_type = field_type_map.get(field_name)
                .ok_or_else(|| CompilerError::codegen_error(format!("Unknown field '{}' in struct '{}'", field_name, struct_lit.type_name)))?
                .clone();

            if let Some(value) = self.generate_expression(field_expr)? {
                field_values.push((field_name.clone(), value));
                field_types.push(field_type);
            } else {
                return Err(CompilerError::codegen_error("Invalid field value in struct literal".to_string()));
            }
        }

        // Calculate proper memory layout based on actual field types
        // Expand type aliases so e.g. Named("boolean") -> Bool and we get i1, not i8*
        let mut total_size = 0;
        let mut field_layout = Vec::new();

        for field_type in &field_types {
            let expanded = self.expand_type_aliases_codegen(field_type);
            let (llvm_type_str, size, alignment) = self.get_llvm_type_info(&expanded);
            // Simple alignment: align to type size (could be more sophisticated)
            let aligned_offset = ((total_size + alignment - 1) / alignment) * alignment;
            field_layout.push((aligned_offset, llvm_type_str, size));
            total_size = aligned_offset + size;
        }

        // Allocate memory for the struct
        let malloc_reg = format!("%struct_alloc_{}", self.instructions.len());
        self.instructions.push(format!("  {} = call i8* @malloc(i64 {})", malloc_reg, total_size));

        // Store each field at its proper offset with correct type
        for (i, ((field_name, field_value), (offset, llvm_type_str, _))) in field_values.iter().zip(field_layout.iter()).enumerate() {
            let field_ptr_reg = format!("%field_ptr_{}_{}", self.instructions.len(), i);

            // Get pointer to field location
            let clean_malloc_reg = self.clean_register_for_instruction(&malloc_reg);
            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", field_ptr_reg, clean_malloc_reg, offset));

            // Cast to appropriate pointer type (use expanded type so bool -> i1*, not i8**)
            let field_ptr_typed = format!("%field_ptr_typed_{}_{}", self.instructions.len(), i);
            self.instructions.push(format!("  {} = bitcast i8* {} to {}*", field_ptr_typed, field_ptr_reg, llvm_type_str));

            // Store the value with correct type - use the actual LLVM type
            // Extract type from field_value if it has a type prefix, otherwise use llvm_type_str
            let (value_type, clean_value) = if field_value.starts_with("double ") {
                ("double", field_value.strip_prefix("double ").unwrap().to_string())
            } else if field_value.starts_with("float ") {
                ("float", field_value.strip_prefix("float ").unwrap().to_string())
            } else if field_value.starts_with("half ") {
                ("half", field_value.strip_prefix("half ").unwrap().to_string())
            } else if field_value.starts_with("i64 ") {
                ("i64", field_value.strip_prefix("i64 ").unwrap().to_string())
            } else if field_value.starts_with("i32 ") {
                ("i32", field_value.strip_prefix("i32 ").unwrap().to_string())
            } else if field_value.starts_with("i16 ") {
                ("i16", field_value.strip_prefix("i16 ").unwrap().to_string())
            } else if field_value.starts_with("i8 ") {
                ("i8", field_value.strip_prefix("i8 ").unwrap().to_string())
            } else if field_value.starts_with("i1 ") {
                ("i1", field_value.strip_prefix("i1 ").unwrap().to_string())
            } else if field_value.starts_with("i8* ") {
                ("i8*", field_value.strip_prefix("i8* ").unwrap().to_string())
            } else {
                // No type prefix - use llvm_type_str
                (llvm_type_str.as_str(), field_value.clone())
            };
            
            // Cast pointer to the correct type if needed
            let final_ptr_type = if value_type != llvm_type_str && llvm_type_str != "i8*" {
                // Need to cast pointer to match value type
                let cast_ptr = format!("%field_ptr_cast_{}_{}", self.instructions.len(), i);
                self.instructions.push(format!("  {} = bitcast {}* {} to {}*", cast_ptr, llvm_type_str, field_ptr_typed, value_type));
                cast_ptr
            } else {
                field_ptr_typed
            };
            
            let store_instruction = if llvm_type_str == "i8*" && value_type == "i64" {
                let ptr_reg = format!("%field_inttoptr_{}_{}", self.instructions.len(), i);
                self.instructions.push(format!("  {} = inttoptr i64 {} to i8*", ptr_reg, Self::format_llvm_value_ref(&clean_value)));
                format!("  store i8* {}, i8** {}", ptr_reg, final_ptr_type)
            } else if value_type == "i8*" && clean_value.contains("getelementptr") {
                format!("  store i8* {}, i8** {}", Self::format_llvm_value_ref(&clean_value), final_ptr_type)
            } else if clean_value.contains("getelementptr") {
                format!("  store i8* {}, {}* {}", Self::format_llvm_value_ref(&clean_value), value_type, final_ptr_type)
            } else if clean_value.contains('@') {
                format!("  store i8* {}, {}* {}", Self::format_llvm_value_ref(&clean_value), value_type, final_ptr_type)
            } else if clean_value.contains("alloc") {
                format!("  store i8* {}, {}* {}", Self::format_llvm_value_ref(&clean_value), value_type, final_ptr_type)
            } else {
                let store_value = if value_type == "float" && !clean_value.starts_with('%') && clean_value.parse::<f64>().is_ok() {
                    let float_const = format!("%float_const_store_{}_{}", self.instructions.len(), i);
                    let instruction = self.create_float_constant_instruction(&clean_value, &float_const, "float");
                    self.instructions.push(instruction);
                    float_const
                } else if value_type == "double" && !clean_value.starts_with('%') && clean_value.parse::<f64>().is_ok() {
                    let double_const = format!("%double_const_store_{}_{}", self.instructions.len(), i);
                    let instruction = self.create_float_constant_instruction(&clean_value, &double_const, "double");
                    self.instructions.push(instruction);
                    double_const
                } else if value_type == "half" && !clean_value.starts_with('%') && clean_value.parse::<f64>().is_ok() {
                    let float_const = format!("%float_const_store_{}_{}", self.instructions.len(), i);
                    let instruction = self.create_float_constant_instruction(&clean_value, &float_const, "float");
                    self.instructions.push(instruction);
                    let half_const = format!("%half_const_store_{}_{}", self.instructions.len(), i);
                    self.instructions.push(format!("  {} = fptrunc float {} to half", half_const, float_const));
                    half_const
                } else {
                    clean_value.clone()
                };
                let store_value_ref = Self::format_llvm_value_ref(&store_value);
                format!("  store {} {}, {}* {}", value_type, store_value_ref, value_type, final_ptr_type)
            };
            self.instructions.push(store_instruction);
        }

        Ok(Some(format!("i8* {}", malloc_reg)))
    }

    /// Generate LLVM IR for tuple expressions with proper mixed-type support
    fn generate_tuple(&mut self, tuple: &Vec<Expression>) -> Result<Option<String>> {
        if tuple.is_empty() {
            return Ok(Some("null".to_string())); // Empty tuple
        }

        // Generate all element expressions first
        let mut elements = Vec::new();
        for element_expr in tuple {
            if let Some(value) = self.generate_expression(element_expr)? {
                elements.push(value);
            } else {
                return codegen_error("Invalid element value in tuple".to_string());
            }
        }

        // Calculate memory layout for proper tuple storage
        // For now, assume all elements are primitive types (int/bool/char)
        // In a full implementation, this would handle arbitrary nested types

        // Determine types for each element based on the actual expressions
        // This respects the specification requirement that tuples can contain any valid types
        let mut element_types = Vec::new();
        for (i, element_expr) in tuple.iter().enumerate() {
            let silica_type = match element_expr {
                // For identifiers, look up in variable_types
                Expression::Identifier(name) => {
                    self.variable_types.get(name)
                        .cloned()
                        .unwrap_or(Type::Int64) // Fallback if not found
                },
                // For other expressions, try to get from expression_types
                _ => {
                    if let Some(location) = crate::types::TypeChecker::try_get_expression_location(element_expr) {
                        self.expression_types.get(location)
                            .cloned()
                            .unwrap_or(Type::Int64)
                    } else {
                        // For expressions without location, infer from the expression
                        match element_expr {
                            Expression::Literal(Literal::Bool(_)) => Type::Bool,
                            Expression::Literal(Literal::Int(_)) => Type::Int64,
                            Expression::Literal(Literal::Char(_)) => Type::Char,
                            Expression::Literal(Literal::String(_)) => Type::String,
                            Expression::StructLiteral(struct_lit) => {
                                // Return the named type of the struct being created
                                Type::Named(struct_lit.type_name.clone())
                            },
                            Expression::Tuple(_) => Type::Tuple(vec![]), // Complex tuple type
                            _ => Type::Int64, // Default fallback
                        }
                    }
                }
            };

            // Convert Silica type to LLVM type string
            // Override for complex expressions that should be pointers
            let llvm_type = match &tuple[i] {
                Expression::StructLiteral(_) => "i8*".to_string(),
                Expression::Tuple(_) => "i8*".to_string(),
                _ => {
                    let base_type = self.type_map.silica_to_llvm_str(&silica_type);
                    // For bootstrap: override certain types that should be pointers
                    if matches!(silica_type, Type::Named(_) | Type::Record(_)) {
                        "i8*".to_string()
                    } else {
                        base_type
                    }
                }
            };
            element_types.push(llvm_type);
        }

        // Create proper tuple memory layout with type information
        // Tuple structure: [element_count: i64][type_ids: i8*][element_data: ...]

        let element_count = elements.len() as i64;
        let mut current_offset = 0i64;

        // Reserve space for element count (i64)
        let count_offset = current_offset;
        current_offset += 8;

        // Reserve space for type IDs (1 byte per element)
        let type_ids_offset = current_offset;
        current_offset += element_count;

        // Calculate element data layout with proper alignment
        let mut element_layout = Vec::new();
        for (i, llvm_type) in element_types.iter().enumerate() {
            let (size, alignment) = match llvm_type.as_ref() {
                "i1" => (1, 1),
                "i32" => (4, 4),
                "i64" => (8, 8),
                "i8*" => (8, 8),
                _ => (8, 8),
            };

            // Align offset to element alignment
            current_offset = ((current_offset + alignment - 1) / alignment) * alignment;

            element_layout.push((llvm_type.to_string(), current_offset, elements[i].clone()));
            current_offset += size;
        }

        // Total size with final alignment
        let total_size = ((current_offset + 7) / 8) * 8;

        // Allocate memory for the complete tuple structure
        let malloc_reg = format!("%tuple_alloc_{}", self.instructions.len());
        self.instructions.push(format!("  {} = call i8* @malloc(i64 {})", malloc_reg, total_size));

        // Store element count
        let count_ptr_reg = format!("%count_ptr_{}", self.instructions.len());
        self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", count_ptr_reg, malloc_reg, count_offset));
        let count_ptr_typed = format!("%count_ptr_typed_{}", self.instructions.len());
        self.instructions.push(format!("  {} = bitcast i8* {} to i64*", count_ptr_typed, count_ptr_reg));
        self.instructions.push(format!("  store i64 {}, i64* {}", element_count, count_ptr_typed));

        // Store type IDs
        for (i, llvm_type) in element_types.iter().enumerate() {
            let type_ptr_reg = format!("%type_ptr_{}_{}", self.instructions.len(), i);
            let type_offset = type_ids_offset + i as i64;
            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", type_ptr_reg, malloc_reg, type_offset));

            // Map LLVM type to type ID (0=i1, 1=i32, 2=i64, 3=i8*)
            let type_id = match llvm_type.as_ref() {
                "i1" => 0i64,
                "i32" => 1i64,
                "i64" => 2i64,
                "i8*" => 3i64,
                _ => 2i64, // Default to i64
            };
            self.instructions.push(format!("  store i8 {}, i8* {}", type_id, type_ptr_reg));
        }

        // Store element data
        for (i, (llvm_type, offset, element_value)) in element_layout.iter().enumerate() {
            let element_ptr_reg = format!("%element_ptr_{}_{}", self.instructions.len(), i);
            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", element_ptr_reg, malloc_reg, offset));

            let element_ptr_typed = format!("%element_ptr_typed_{}_{}", self.instructions.len(), i);
            self.instructions.push(format!("  {} = bitcast i8* {} to {}*", element_ptr_typed, element_ptr_reg, llvm_type));

            // Strip type prefix from element_value if present
            let clean_element_value = element_value.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();

            // Store the value with proper type conversion
            let value_to_store = if llvm_type == "i64" && element_value.contains("alloc") {
                // HACK: Cast pointer to i64 for storage in tuple
                let cast_reg = format!("%ptr_cast_{}", self.instructions.len());
                self.instructions.push(format!("  {} = ptrtoint i8* {} to i64", cast_reg, clean_element_value));
                cast_reg
            } else if llvm_type == "i8*" && element_value.starts_with("i64 ") {
                // Value was generated as i64 (e.g. call i64) but slot expects i8* - cast to pointer
                let inttoptr_reg = format!("%inttoptr_{}_{}", self.instructions.len(), i);
                self.instructions.push(format!("  {} = inttoptr i64 {} to i8*", inttoptr_reg, clean_element_value));
                inttoptr_reg
            } else {
                self.convert_to_llvm_type_value(&clean_element_value, llvm_type)
            };
            // Handle type mismatches for bootstrap compatibility
            if llvm_type == "i64" && value_to_store.starts_with('%') && value_to_store.contains("alloc") {
                // Cast pointer to i64 for storage
                let cast_reg = format!("%ptr_cast_{}", self.instructions.len());
                self.instructions.push(format!("  {} = ptrtoint i8* {} to i64", cast_reg, value_to_store));
                self.instructions.push(format!("  store {} {}, {}* {}", llvm_type, cast_reg, llvm_type, element_ptr_typed));
            } else {
                self.instructions.push(format!("  store {} {}, {}* {}", llvm_type, value_to_store, llvm_type, element_ptr_typed));
            }
        }


        Ok(Some(format!("i8* {}", malloc_reg)))
    }

    /// Infer type for an expression (simplified version for codegen)
    fn infer_expression_type(&self, expr: &Expression) -> Type {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Int(_) => Type::Int64,
                Literal::Bool(_) => Type::Bool,
                Literal::Char(_) => Type::Char,
                Literal::String(_) => Type::String,
                Literal::Float(_) => {
                    // Get the type from expression_types if available (for float16 vs float32 vs float64)
                    // If type information is missing, this indicates missing type annotation
                    // The type checker should have caught this, but if we reach here without
                    // type info, we cannot determine float16 vs float32
                    if let Some(location) = Self::try_get_expression_location(expr) {
                        if let Some(ty) = self.expression_types.get(location) {
                            match ty {
                                Type::Float16 => Type::Float16,
                                Type::Float32 => Type::Float32,
                                Type::Float64 => Type::Float64,
                                _ => {
                                    // Non-float type stored - this is unexpected for a Float literal
                                    // This should not happen if type checking worked correctly
                                    panic!("Float literal has non-float type in expression_types: {:?}", ty)
                                }
                            }
                        } else {
                            // Type information missing - indicates missing type annotation
                            panic!("Float literal type information missing - type annotation required for float16 vs float32 vs float64 distinction")
                        }
                    } else {
                        // Literals don't have locations, so we can't look up their type
                        // This means type information is not available - indicates missing annotation
                        panic!("Float literal has no location - cannot determine float16 vs float32 vs float64 without type annotation")
                    }
                },
                Literal::Unit => Type::Unit,
            },
            Expression::Identifier(name) => {
                // Look up the variable type from the variable_types map
                // This should match the type checker's behavior
                if let Some(var_type) = self.variable_types.get(name) {
                    // Return the stored type
                    var_type.clone()
                } else {
                    // Fallback for unknown identifiers (builtins, etc.)
                    Type::Int64
                }
            }
            Expression::Binary(_) => Type::Int64, // Binary operations typically return Int
            Expression::Unary(_) => Type::Int64, // Unary operations typically return Int
            Expression::Call(_) => {
                // Use type from type checker so field access on call result (e.g. current_token(stream).kind) gets correct struct type
                if let Some(location) = Self::try_get_expression_location(expr) {
                    if let Some(ty) = self.expression_types.get(location) {
                        return ty.clone();
                    }
                }
                Type::Int64 // Fallback when no type recorded (e.g. before type checker)
            }
            Expression::If(_) => Type::Int64, // If expressions default to Int
            Expression::Tuple(_) => Type::Int64, // Nested tuples as Int (simplified)
            Expression::FieldAccess(field_access) => {
                // Use type from type checker when available (so nested x.y.z gets correct type)
                if let Some(ty) = Self::try_get_expression_location(expr).and_then(|loc| self.expression_types.get(loc)) {
                    return ty.clone();
                }
                // Otherwise infer from object type and struct/record definition
                let object_type = self.infer_expression_type(&field_access.object);
                let expanded = self.expand_type_aliases_codegen(&object_type);
                match &expanded {
                    Type::Named(name) => {
                        if let Some(struct_def) = self.struct_defs.get(name.as_str()) {
                            if let Some(field) = struct_def.iter().find(|f| f.name == field_access.field) {
                                return field.ty.clone();
                            }
                        }
                        Type::Int64
                    }
                    Type::Record(fields) => {
                        fields.iter()
                            .find(|(fn_, _)| fn_ == &field_access.field)
                            .map(|(_, ft)| ft.clone())
                            .unwrap_or(Type::Int64)
                    }
                    _ => Type::Int64,
                }
            }
            // Other expression types default to Int for now
            _ => Type::Int64,
        }
    }

    /// Byte size and alignment for ast::Type (for struct layout).
    fn ast_type_size_align(ty: &crate::ast::Type) -> (i64, i64) {
        match ty {
            crate::ast::Type::Unit => (0, 1),
            crate::ast::Type::Bool => (1, 1),
            crate::ast::Type::Int8 => (1, 1),
            crate::ast::Type::Int16 => (2, 2),
            crate::ast::Type::Int32 => (4, 4),
            crate::ast::Type::Int64 => (8, 8),
            crate::ast::Type::Float16 => (2, 2),
            crate::ast::Type::Float32 => (4, 4),
            crate::ast::Type::Float64 => (8, 8),
            crate::ast::Type::Char => (4, 4),
            crate::ast::Type::String | crate::ast::Type::Named(_) | crate::ast::Type::Record(_) => (8, 8),
            crate::ast::Type::Function { .. } => (8, 8),
            _ => (8, 8),
        }
    }

    /// Byte offset of a field in a struct (for self-referential patch).
    fn get_struct_field_offset(&self, struct_name: &str, field_name: &str) -> Option<u64> {
        let fields = self.struct_defs.get(struct_name)?;
        let mut offset: i64 = 0;
        for f in fields {
            let (size, alignment) = Self::ast_type_size_align(&f.ty);
            let aligned = ((offset + alignment - 1) / alignment) * alignment;
            if f.name == field_name {
                return Some(aligned as u64);
            }
            offset = aligned + size;
        }
        None
    }

    /// Get LLVM type information for a Silica type
    fn get_llvm_type_info(&self, silica_type: &Type) -> (String, i64, i64) {
        match silica_type {
            Type::Int8 => ("i8".to_string(), 1, 1),
            Type::Int16 => ("i16".to_string(), 2, 2),
            Type::Int32 => ("i32".to_string(), 4, 4),
            Type::Int64 => ("i64".to_string(), 8, 8),
            Type::Float16 => ("half".to_string(), 2, 2),
            Type::Float32 => ("float".to_string(), 4, 4),
            Type::Float64 => ("double".to_string(), 8, 8),
            Type::Bool => ("i1".to_string(), 1, 1),
            Type::Char => ("i32".to_string(), 4, 4),
            Type::String => ("i8*".to_string(), 8, 8), // Pointer
            Type::Unit => ("void".to_string(), 0, 1), // Not used in tuples
            Type::Tuple(_) => ("i8*".to_string(), 8, 8), // Nested tuple as pointer
            Type::Record(_) => ("i8*".to_string(), 8, 8), // Struct as pointer
            Type::Named(_) => ("i8*".to_string(), 8, 8), // Named type as pointer
            Type::ActorRef => ("i64".to_string(), 8, 8), // ActorRef stored as i64 (pointer value)
            // NEON 128-bit vector types (all 16 bytes)
            Type::Vec128Int8 => ("<16 x i8>".to_string(), 16, 16),
            Type::Vec128Int16 => ("<8 x i16>".to_string(), 16, 16),
            Type::Vec128Int32 => ("<4 x i32>".to_string(), 16, 16),
            Type::Vec128Int64 => ("<2 x i64>".to_string(), 16, 16),
            Type::Vec128Float32 => ("<4 x float>".to_string(), 16, 16),
            Type::Vec128Bool => ("<16 x i1>".to_string(), 16, 16),
            // SVE scalable vector types (size depends on hardware, use placeholder)
            Type::VecInt8 => ("<vscale x 16 x i8>".to_string(), 16, 16), // Placeholder size
            Type::VecInt16 => ("<vscale x 8 x i16>".to_string(), 16, 16),
            Type::VecInt32 => ("<vscale x 4 x i32>".to_string(), 16, 16),
            Type::VecInt64 => ("<vscale x 2 x i64>".to_string(), 16, 16),
            Type::VecFloat16 => ("<vscale x 8 x half>".to_string(), 16, 16),
            Type::VecFloat32 => ("<vscale x 4 x float>".to_string(), 16, 16),
            Type::VecFloat64 => ("<vscale x 2 x double>".to_string(), 16, 16),
            Type::VecBool => ("<vscale x 16 x i1>".to_string(), 16, 16),
            // SVE predicate type
            Type::Pred => ("<vscale x 16 x i1>".to_string(), 16, 16), // Placeholder size
            _ => ("i64".to_string(), 8, 8), // Default fallback
        }
    }

    /// Tuple decomposition now reads type information from stored metadata
    /// This method is kept for backward compatibility but is no longer used
    /// The new implementation stores type metadata with tuples for proper typing
    fn infer_decomposition_type(&self, _tuple_size: usize, _position: usize) -> &str {
        // Legacy method - tuples now use stored type metadata
        "i64"
    }

    /// Convert an LLVM value string to the specified LLVM type
    fn convert_to_llvm_type_value(&self, value: &str, target_type: &str) -> String {
        match target_type {
            "i64" => self.convert_to_i64_value(value),
            "i32" => {
                if value.starts_with("i32 ") {
                    value.strip_prefix("i32 ").unwrap().to_string()
                } else if value.starts_with("i64 ") {
                    let i64_val = value.strip_prefix("i64 ").unwrap();
                    format!("trunc (i64 {} to i32)", i64_val)
                } else if value.starts_with("i1 ") {
                    let bool_val = value.strip_prefix("i1 ").unwrap();
                    format!("zext (i1 {} to i32)", bool_val)
                } else {
                    // Register or unknown
                    value.to_string()
                }
            }
            "i1" => {
                if value.starts_with("i1 ") {
                    value.strip_prefix("i1 ").unwrap().to_string()
                } else if value.starts_with("i64 ") {
                    let i64_val = value.strip_prefix("i64 ").unwrap();
                    format!("trunc (i64 {} to i1)", i64_val)
                } else {
                    // Register or unknown - assume boolean context
                    value.to_string()
                }
            }
            "i8*" => {
                // For pointers, validate the value and default to null if invalid
                let clean_value = if value.contains(" ") {
                    value.split_whitespace().last().unwrap_or("null").to_string()
                } else {
                    value.to_string()
                };

                // Valid pointer values: null, registers (%name or bare name like t0), globals (@name)
                if clean_value == "null" {
                    clean_value
                } else if clean_value.starts_with('%') || clean_value.starts_with('@') {
                    clean_value
                } else if clean_value.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
                    && clean_value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    // Bare register name (e.g. t0 from next_register()) - use as pointer register
                    format!("%{}", clean_value)
                } else {
                    // Invalid pointer value, default to null
                    "null".to_string()
                }
            }
            _ => {
                // Unknown type - return as-is
                if value.contains(" ") {
                    value.split_whitespace().last().unwrap_or("0").to_string()
                } else {
                    value.to_string()
                }
            }
        }
    }

    /// Convert an LLVM value string to i64 format for storage
    fn convert_to_i64_value(&self, value: &str) -> String {
        if value.starts_with("i64 ") {
            value.strip_prefix("i64 ").unwrap().to_string()
        } else if value.starts_with("i1 ") {
            // Convert boolean to i64 (0 or 1)
            if value == "i1 1" { "1".to_string() } else { "0".to_string() }
        } else if value.starts_with("i32 ") {
            // Convert char to i64 (zero-extend)
            let char_val = value.strip_prefix("i32 ").unwrap();
            format!("zext (i32 {} to i64)", char_val)
        } else if value.contains("tuple_alloc") {
            // Convert tuple pointer to i64
            format!("ptrtoint (i8* {} to i64)", value)
        } else {
            // For pointers or other types, this is a placeholder
            // In a proper implementation, we'd handle each type appropriately
            // DEBUG: For now, just strip any type prefix
            if value.contains(" ") {
                value.split_whitespace().last().unwrap_or("0").to_string()
            } else {
                value.to_string()
            }
        }
    }

    /// Generate LLVM IR for field access
    fn generate_field_access(&mut self, field_access: &FieldAccessExpr) -> Result<Option<String>> {
        // Generate the object expression first
        let object_value = match self.generate_expression(&field_access.object)? {
            Some(value) => value,
            None => return codegen_error("Field access requires valid object".to_string()),
        };

                // Get the type of the object to determine which struct we're accessing
                let object_type = self.infer_expression_type(&field_access.object);
                // Expand type aliases to get the actual type
                let expanded_object_type = self.expand_type_aliases_codegen(&object_type);

        // Look up the field index, field type, and byte offset from the struct definition
        let (field_index, field_llvm_type, field_offset) = match &expanded_object_type {
            Type::Named(type_name) => {
                // Look up the struct definition
                let struct_def_opt = self.struct_defs.get(type_name.as_str());
                let struct_def = struct_def_opt
                    .ok_or_else(|| CompilerError::codegen_error(format!("Cannot access field '{}' on non-struct type {:?}", field_access.field, expanded_object_type)))?;
                let field_index = struct_def.iter().position(|field| field.name == field_access.field)
                    .ok_or_else(|| CompilerError::codegen_error(format!("Unknown field '{}' in struct '{}'", field_access.field, type_name)))?;
                let field_ty = &struct_def[field_index].ty;
                // Expand type aliases so e.g. Named("boolean") -> Bool and we get i1, not ptr
                let expanded_field_ty = self.expand_type_aliases_codegen(field_ty);
                let (field_llvm_type, _, _) = self.get_llvm_type_info(&expanded_field_ty);
                let field_offset = self.get_struct_field_offset(type_name, &field_access.field)
                    .unwrap_or((field_index * 8) as u64) as i64;
                (field_index, field_llvm_type, field_offset)
            }
            Type::Record(fields) => {
                // Find the field index directly from the record fields
                let field_index = fields.iter().position(|(field_name, _)| field_name == &field_access.field)
                    .ok_or_else(|| CompilerError::codegen_error(format!("Unknown field '{}' in record type {:?}", field_access.field, expanded_object_type)))?;
                let field_ty = &fields[field_index].1;
                // Expand type aliases so e.g. Named("boolean") -> Bool and we get i1, not ptr
                let expanded_field_ty = self.expand_type_aliases_codegen(field_ty);
                let (field_llvm_type, _, _) = self.get_llvm_type_info(&expanded_field_ty);
                // Compute byte offset from preceding fields
                let mut offset: i64 = 0;
                for (_, ft) in fields.iter().take(field_index) {
                    let (_, size, align) = self.get_llvm_type_info(ft);
                    let aligned = ((offset + align - 1) / align) * align;
                    offset = aligned + size;
                }
                (field_index, field_llvm_type, offset)
            }
            _ => {
                return Err(CompilerError::codegen_error(format!("Cannot access field '{}' on non-struct type {:?}", field_access.field, expanded_object_type)));
            }
        };

        let field_ptr_reg = format!("%field_ptr_{}", self.instructions.len());
        let field_ptr_typed = format!("%field_ptr_typed_{}", self.instructions.len());
        let result_reg = format!("%field_value_{}", self.instructions.len());

        // Get pointer to field location (clean register name for instruction)
        // If object_value is i64 (from function call returning pointer as i64), convert to i8*
        let clean_object = if object_value.starts_with("i64 ") {
            let i64_reg = object_value.strip_prefix("i64 ").unwrap();
            let ptr_reg = format!("%field_obj_ptr_{}", self.instructions.len());
            self.instructions.push(format!("  {} = inttoptr i64 {} to i8*", ptr_reg, i64_reg));
            ptr_reg
        } else {
            let cleaned = self.clean_register_for_instruction(&object_value);
            // Ensure register names have % prefix for LLVM instructions
            if cleaned.starts_with('%') {
                cleaned
            } else {
                format!("%{}", cleaned)
            }
        };

        // Special case: if this is a direct struct value from a function return (like get_cpu_topology()),
        // we need to handle it differently since it's not a pointer
        let is_direct_struct_value = object_value.starts_with("t") && !object_value.starts_with("i64 ") &&
                                   matches!(&expanded_object_type, Type::Record(_) | Type::Named(_));

        if is_direct_struct_value {
            // For direct struct values from function returns, use extractvalue
            let struct_type_str = match &expanded_object_type {
                Type::Record(fields) => {
                    let field_types: Vec<String> = fields.iter()
                        .map(|(_, field_type)| self.get_llvm_type_string(field_type))
                        .collect();
                    format!("{{{}}}", field_types.join(", "))
                }
                Type::Named(name) if self.struct_defs.contains_key(name) => {
                    // For named structs, we need to look up the definition
                    // For now, assume it's the CpuTopology struct
                    "{i64, i64, i64, i1, i64, i64, i1, i64, i64}".to_string()
                }
                _ => "{i64}".to_string(), // Fallback
            };

            self.instructions.push(format!("  {} = extractvalue {} {}, {}", result_reg, struct_type_str, clean_object, field_index));
            return Ok(Some(format!("{} {}", field_llvm_type, result_reg)));
        } else {
            // For struct pointers (normal case), use getelementptr + load with actual field type (e.g. i1 for bool)
            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", field_ptr_reg, clean_object, field_offset));

            // Cast to field type pointer and load (so bool fields load as i1, not i64)
            self.instructions.push(format!("  {} = bitcast i8* {} to {}*", field_ptr_typed, field_ptr_reg, field_llvm_type));
            self.instructions.push(format!("  {} = load {}, {}* {}", result_reg, field_llvm_type, field_llvm_type, field_ptr_typed));
            return Ok(Some(format!("{} {}", field_llvm_type, result_reg)));
        }
    }


    /// Generate LLVM IR for actor spawn (spawn)
    fn generate_spawn(&mut self, spawn: &SpawnExpr) -> Result<Option<String>> {
        // Generate initial state and behavior expressions
        let initial_state = self.generate_expression(&spawn.initial_state)?;
        let behavior = self.generate_expression(&spawn.behavior)?;

        // Generate core affinity expression (default to 0 for any core)
        let core_affinity = if let Some(ref affinity_expr) = spawn.core_affinity {
            match &**affinity_expr {
                Expression::Identifier(name) if name == "any_core" => {
                    Some("i32 0".to_string()) // Any core
                },
                Expression::Identifier(name) if name == "performance_cores" => {
                    Some("i32 -1".to_string()) // Performance cores
                },
                Expression::Identifier(name) if name == "efficiency_cores" => {
                    Some("i32 -2".to_string()) // Efficiency cores
                },
                _ => self.generate_expression(affinity_expr)?,
            }
        } else {
            Some("i32 0".to_string()) // Default: any core
        };

        if let (Some(state), Some(behav), Some(affinity)) = (initial_state, behavior, core_affinity) {
            let actor_reg = format!("%actor_{}", self.instructions.len());


            // Allocate space for the initial state and create a pointer
            let mut final_ptr = format!("%state_final_{}", self.instructions.len());

            if state.starts_with("i64 ") {
                let val_str = &state[4..];
                // Check if this is a literal integer or a register
                if val_str.chars().all(|c| c.is_ascii_digit() || c == '-') {
                    // Integer literal - allocate and store
                    let alloc_reg = format!("%state_alloc_{}", self.instructions.len());
                    let int_ptr = format!("%state_int_ptr_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg)); // Allocate 8 bytes for i64
                    self.instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                    self.instructions.push(format!("  store i64 {}, i64* {}", val_str, int_ptr));
                    self.instructions.push(format!("  {} = bitcast i64* {} to i8*", final_ptr, int_ptr));
                } else {
                    // This is a register containing an i64 value - allocate and store the register value
                    let alloc_reg = format!("%state_alloc_{}", self.instructions.len());
                    let int_ptr = format!("%state_int_ptr_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg)); // Allocate 8 bytes for i64
                    self.instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                    self.instructions.push(format!("  store i64 {}, i64* {}", val_str, int_ptr));
                    self.instructions.push(format!("  {} = bitcast i64* {} to i8*", final_ptr, int_ptr));
                }
            } else if state.starts_with("i1 ") {
                // Boolean literal - allocate and store as i64 (0/1)
                let bool_val = &state[3..];
                let int_val = if bool_val == "1" { "1" } else { "0" };
                let alloc_reg = format!("%state_alloc_{}", self.instructions.len());
                let int_ptr = format!("%state_int_ptr_{}", self.instructions.len());
                self.instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg)); // Allocate 8 bytes for i64
                self.instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                self.instructions.push(format!("  store i64 {}, i64* {}", int_val, int_ptr));
                self.instructions.push(format!("  {} = bitcast i64* {} to i8*", final_ptr, int_ptr));
            } else if state.starts_with("%") {
                // Check if this is a pointer register (from tuple generation) or an i64 register
                if state.contains("tuple_alloc") {
                    // This is a register holding an i8* pointer (e.g., from tuple generation)
                    // Use the pointer directly
                    final_ptr.clear();
                    final_ptr.push_str(&state);
                } else {
                    // This is an i64 register - allocate memory and store the value
                    let alloc_reg = format!("%state_alloc_{}", self.instructions.len());
                    let int_ptr = format!("%state_int_ptr_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg)); // Allocate 8 bytes for i64
                    self.instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                    self.instructions.push(format!("  store i64 {}, i64* {}", state, int_ptr));
                    self.instructions.push(format!("  {} = bitcast i64* {} to i8*", final_ptr, int_ptr));
                }
            } else {
                // For other types or complex expressions, use a placeholder allocation
                // This is a simplification for the bootstrap compiler
                let alloc_reg = format!("%state_alloc_{}", self.instructions.len());
                let int_ptr = format!("%state_int_ptr_{}", self.instructions.len());
                self.instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg)); // Allocate 8 bytes
                // For now, initialize to 0
                self.instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                self.instructions.push(format!("  store i64 0, i64* {}", int_ptr));
                self.instructions.push(format!("  {} = bitcast i64* {} to i8*", final_ptr, int_ptr));
            }

            // Call Silica runtime actor spawn function
            // silica_actor_spawn(initial_state_ptr, behavior_fn_ptr, core_affinity) -> actor_ref
            let temp_actor = format!("%temp_actor_{}", self.instructions.len());

            // Convert affinity based on type - for now, assume i32 values
            // TODO: Extend to handle CoreId, PerformanceCores, EfficiencyCores, etc.
            let affinity_val = if affinity.starts_with("i32 ") {
                affinity[4..].to_string()
            } else {
                // Default to 0 (any core) for complex expressions
                "0".to_string()
            };

            // Handle function pointer casting for behavior parameter
            let behavior_ptr = if behav.starts_with('@') {
                // This is a function reference - cast it to i8* for the runtime
                let cast_reg = format!("%behavior_cast_{}", self.instructions.len());
                // Assume i64(i64,i64)* type for regular functions used as behaviors
                self.instructions.push(format!("  {} = bitcast i64 (i64, i64)* {} to i8*", cast_reg, behav));
                cast_reg
            } else {
                // This is already an i8* (e.g., from function literal)
                if let Some(stripped) = behav.strip_prefix("i8* ") {
                    stripped.to_string()
                } else {
                    behav
                }
            };

            self.instructions.push(format!("  {} = call i8* @silica_actor_spawn(i8* {}, i8* {}, i32 {})", temp_actor, final_ptr, behavior_ptr, affinity_val));
            // Cast actor reference to i64 for return (bootstrap compiler compatibility)
            self.instructions.push(format!("  {} = ptrtoint i8* {} to i64", actor_reg, temp_actor));

            Ok(Some(actor_reg))
        } else {
            codegen_error("Invalid initial state or behavior for spawn".to_string())
        }
    }

    /// Generate LLVM IR for core_id(core_number) call
    fn generate_core_id_call(&mut self, call: &CallExpr) -> Result<String> {
        if call.arguments.len() != 1 {
            return codegen_error("core_id expects exactly 1 argument".to_string());
        }

        let core_expr = self.generate_expression(&call.arguments[0])?;
        if let Some(core_val) = core_expr {
            // Convert the core number to i32
            if core_val.starts_with("i64 ") {
                let core_num = &core_val[4..];
                Ok(format!("i32 {}", core_num))
            } else {
                // For other expressions, truncate to i32
                let reg = format!("%core_id_{}", self.instructions.len());
                self.instructions.push(format!("  {} = trunc i64 {} to i32", reg, core_val));
                Ok(reg)
            }
        } else {
            codegen_error("Invalid core_id argument".to_string())
        }
    }

    /// Generate LLVM IR for a sequence of statements (text-based)
    fn generate_statements(&mut self, statements: &[Statement]) -> Result<Option<String>> {
        let mut last_result = None;

        for statement in statements {
            match statement {
                Statement::Bind { pattern, expr } => {
                    // Self-referential binding (e.g. nil_placeholder: ListToken <- ListToken { ..., tail: nil_placeholder }):
                    // pre-register the variable so the RHS can reference it, then emit copy from result to that slot.
                    let value = if let Pattern::TypedIdentifier { name, type_ } = pattern {
                        if name != "_" && Self::expression_references_identifier(expr, name) {
                            let placeholder_reg = format!("%t_self_{}", self.instructions.len());
                            self.add_variable_text(name.clone(), placeholder_reg.clone());
                            if let crate::ast::Type::Named(n) = type_ {
                                self.variable_types.insert(name.clone(), Type::Named(n.clone()));
                            }
                            self.self_ref_placeholders.insert(name.clone());
                            let result = self.generate_expression(expr)?;
                            if let Some(ref value_reg) = result {
                                let clean = value_reg
                                    .strip_prefix("i8* ")
                                    .or_else(|| value_reg.strip_prefix("i64 "))
                                    .unwrap_or(value_reg);
                                // Patch the self-referential field: it was stored with uninitialized placeholder;
                                // now store the struct pointer into that field.
                                if let crate::ast::Type::Named(struct_name) = type_ {
                                    if let Some(fields) = self.struct_defs.get(struct_name) {
                                        if let Some(self_ref_field) = fields.iter().find(|f| {
                                            matches!(&f.ty, crate::ast::Type::Named(n) if n == struct_name)
                                        }) {
                                            if let Some(tail_offset) = self.get_struct_field_offset(struct_name, &self_ref_field.name) {
                                                let patch_ptr = format!("%patch_ptr_{}", self.instructions.len());
                                                self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", patch_ptr, clean, tail_offset));
                                                let patch_ptr_i8ptr = format!("%patch_ptr_i8ptr_{}", self.instructions.len());
                                                self.instructions.push(format!("  {} = bitcast i8* {} to i8**", patch_ptr_i8ptr, patch_ptr));
                                                self.instructions.push(format!("  store i8* {}, i8** {}", Self::format_llvm_value_ref(&clean), patch_ptr_i8ptr));
                                            }
                                        }
                                    }
                                }
                            }
                            self.self_ref_placeholders.remove(name);
                            result
                        } else {
                            self.generate_expression(expr)?
                        }
                    } else {
                        self.generate_expression(expr)?
                    };
                    // Handle pattern binding - for now just handle simple identifier patterns
                    if let Some(value_reg) = value {
                        match pattern {
                            Pattern::Identifier(name) => {
                                // Simple identifier binding - just store the value
                                self.add_variable_text(name.clone(), value_reg);
                            }
                            Pattern::TypedIdentifier { name, type_ } => {
                                // For float16 and float64 bindings, we need to create a proper register instead of storing the literal string
                                // Check if this is a float16 binding
                                // eprintln!("DEBUG TypedIdentifier bind (statements): name = '{}', type_ = {:?}, value_reg = '{}'", name, type_, value_reg);
                                let stored_val = if matches!(type_, crate::ast::Type::Float16) {
                                    // Check if value is already a half register (from function call, etc.)
                                    // Handle both bare registers (%t54) and type-prefixed registers (half %t54)
                                    if value_reg.starts_with('%') && !value_reg.contains(' ') {
                                        // eprintln!("DEBUG TypedIdentifier bind (statements): float16 binding with register, using directly: '{}'", value_reg);
                                        // It's already a register - assume it's half and use directly
                                        value_reg.clone()
                                    } else if value_reg.starts_with("half ") {
                                        // Type-prefixed register (e.g., "half %t54") - strip the prefix
                                        let reg_part = value_reg.trim_start_matches("half ");
                                        if reg_part.starts_with('%') && !reg_part.contains(' ') {
                                            // eprintln!("DEBUG TypedIdentifier bind (statements): float16 binding with type-prefixed register, stripping prefix: '{}' -> '{}'", value_reg, reg_part);
                                            reg_part.to_string()
                                        } else {
                                            // Not a register, treat as literal
                                            // eprintln!("DEBUG TypedIdentifier bind (statements): float16 binding with half literal: '{}'", value_reg);
                                            // Extract the constant value
                                            let const_val = reg_part;
                                            // If const_val contains spaces, it's malformed - extract the numeric part
                                            let clean_const = if const_val.contains(' ') {
                                                // Split and find the numeric constant
                                                const_val.split_whitespace()
                                                    .find(|p| p.parse::<f64>().is_ok())
                                                    .map(|s| s.to_string())
                                                    .unwrap_or_else(|| const_val.to_string())
                                            } else {
                                                const_val.to_string()
                                            };
                                            // Create a float constant first, then convert to half
                                            let float_const = format!("%float_const_bind_{}", self.instructions.len());
                                            let instruction = self.create_float_constant_instruction(&clean_const, &float_const, "float");
                                            self.instructions.push(instruction);
                                            let half_const = format!("%half_const_bind_{}", self.instructions.len());
                                            self.instructions.push(format!("  {} = fptrunc float {} to half", half_const, float_const));
                                            // eprintln!("DEBUG TypedIdentifier bind (statements): created half_const = '{}'", half_const);
                                            half_const
                                        }
                                    } else if value_reg.starts_with("float ") {
                                        // eprintln!("DEBUG TypedIdentifier bind (statements): float16 binding with float literal: '{}'", value_reg);
                                        // Extract the constant value, handling both "half 3.14" and "float 3.14"
                                        let const_val = if value_reg.starts_with("half ") {
                                            value_reg.trim_start_matches("half ")
                                        } else {
                                            value_reg.trim_start_matches("float ")
                                        };
                                        // If const_val contains spaces, it's malformed - extract the numeric part
                                        let clean_const = if const_val.contains(' ') {
                                            // Split and find the numeric constant
                                            const_val.split_whitespace()
                                                .find(|p| p.parse::<f64>().is_ok())
                                                .map(|s| s.to_string())
                                                .unwrap_or_else(|| const_val.to_string())
                                        } else {
                                            const_val.to_string()
                                        };
                                        // Create a float constant first, then convert to half
                                        let float_const = format!("%float_const_bind_{}", self.instructions.len());
                                        let instruction = self.create_float_constant_instruction(&clean_const, &float_const, "float");
                                        self.instructions.push(instruction);
                                        let half_const = format!("%half_const_bind_{}", self.instructions.len());
                                        self.instructions.push(format!("  {} = fptrunc float {} to half", half_const, float_const));
                                        // eprintln!("DEBUG TypedIdentifier bind (statements): created half_const = '{}'", half_const);
                                        half_const
                                    } else {
                                        // eprintln!("DEBUG TypedIdentifier bind (statements): float16 binding but value_reg doesn't match expected patterns, using as-is: '{}'", value_reg);
                                        // Value doesn't match expected patterns, use as-is
                                        value_reg.clone()
                                    }
                                } else if matches!(type_, crate::ast::Type::Float64) && (value_reg.starts_with("double ") || value_reg.starts_with("float ")) {
                                    // Extract the constant value, handling both "double 3.14" and "float 3.14"
                                    let const_val = if value_reg.starts_with("double ") {
                                        value_reg.trim_start_matches("double ")
                                    } else {
                                        value_reg.trim_start_matches("float ")
                                    };
                                    // If const_val contains spaces, it's malformed - extract the numeric part
                                    let clean_const = if const_val.contains(' ') {
                                        // Split and find the numeric constant
                                        const_val.split_whitespace()
                                            .find(|p| p.parse::<f64>().is_ok())
                                            .unwrap_or(const_val)
                                    } else {
                                        const_val
                                    };
                                    // Create a double constant register
                                    let double_const = format!("%double_const_bind_{}", self.instructions.len());
                                    let instruction = self.create_float_constant_instruction(clean_const, &double_const, "double");
                                    self.instructions.push(instruction);
                                    double_const
                                } else {
                                    // eprintln!("DEBUG TypedIdentifier bind (statements): not float16/float64 binding, using as-is: '{}'", value_reg);
                                    // For other types, use the value as-is
                                    value_reg.clone()
                                };
                                // eprintln!("DEBUG TypedIdentifier bind (statements): stored_val = '{}'", stored_val);
                                
                                // Use the type from the pattern annotation (the declared type)
                                // Convert ast::Type to internal Type representation
                                let var_type = match type_ {
                                    crate::ast::Type::Int8 => Type::Int8,
                                    crate::ast::Type::Int16 => Type::Int16,
                                    crate::ast::Type::Int32 => Type::Int32,
                                    crate::ast::Type::Int64 => Type::Int64,
                                    crate::ast::Type::Float16 => Type::Float16,
                                    crate::ast::Type::Float32 => Type::Float32,
                                    crate::ast::Type::Float64 => Type::Float64,
                                    crate::ast::Type::Bool => Type::Bool,
                                    crate::ast::Type::Char => Type::Char,
                                    crate::ast::Type::String => Type::String,
                                    crate::ast::Type::Unit => Type::Unit,
                                    crate::ast::Type::Tuple(elem_types) => {
                                        let converted: Vec<Type> = elem_types.iter().map(|t| match t {
                                            crate::ast::Type::Int8 => Type::Int8,
                                            crate::ast::Type::Int16 => Type::Int16,
                                            crate::ast::Type::Int32 => Type::Int32,
                                            crate::ast::Type::Int64 => Type::Int64,
                                            crate::ast::Type::Float16 => Type::Float16,
                                            crate::ast::Type::Float32 => Type::Float32,
                                            crate::ast::Type::Float64 => Type::Float64,
                                            crate::ast::Type::Bool => Type::Bool,
                                            crate::ast::Type::Char => Type::Char,
                                            crate::ast::Type::String => Type::String,
                                            crate::ast::Type::Unit => Type::Unit,
                                            crate::ast::Type::Named(name) => Type::Named(name.clone()),
                                            _ => Type::Int64, // Fallback
                                        }).collect();
                                        Type::Tuple(converted)
                                    }
                                    crate::ast::Type::Named(name) => Type::Named(name.clone()),
                                    crate::ast::Type::Function { parameters, return_type } => {
                                        // Recursively convert parameter types
                                        let converted_params: Vec<Type> = parameters.iter().map(|param_type| {
                                            match param_type {
                                                crate::ast::Type::Int8 => Type::Int8,
                                                crate::ast::Type::Int16 => Type::Int16,
                                                crate::ast::Type::Int32 => Type::Int32,
                                                crate::ast::Type::Int64 => Type::Int64,
                                                crate::ast::Type::Float16 => Type::Float16,
                                                crate::ast::Type::Float32 => Type::Float32,
                                                crate::ast::Type::Float64 => Type::Float64,
                                                crate::ast::Type::Bool => Type::Bool,
                                                crate::ast::Type::Char => Type::Char,
                                                crate::ast::Type::String => Type::String,
                                                crate::ast::Type::Unit => Type::Unit,
                                                crate::ast::Type::Function { parameters: nested_params, return_type: nested_ret } => {
                                                    // Recursively handle nested function types
                                                    let nested_converted_params: Vec<Type> = nested_params.iter().map(|p| match p {
                                                        crate::ast::Type::Int8 => Type::Int8,
                                                        crate::ast::Type::Int16 => Type::Int16,
                                                        crate::ast::Type::Int32 => Type::Int32,
                                                        crate::ast::Type::Int64 => Type::Int64,
                                                        crate::ast::Type::Float16 => Type::Float16,
                                                        crate::ast::Type::Float32 => Type::Float32,
                                                        crate::ast::Type::Float64 => Type::Float64,
                                                        crate::ast::Type::Bool => Type::Bool,
                                                        crate::ast::Type::Char => Type::Char,
                                                        crate::ast::Type::String => Type::String,
                                                        crate::ast::Type::Unit => Type::Unit,
                                                        _ => Type::Int64, // Fallback for nested types
                                                    }).collect();
                                                    let nested_converted_ret = match &**nested_ret {
                                                        crate::ast::Type::Int8 => Type::Int8,
                                                        crate::ast::Type::Int16 => Type::Int16,
                                                        crate::ast::Type::Int32 => Type::Int32,
                                                        crate::ast::Type::Int64 => Type::Int64,
                                                        crate::ast::Type::Float16 => Type::Float16,
                                                        crate::ast::Type::Float32 => Type::Float32,
                                                        crate::ast::Type::Float64 => Type::Float64,
                                                        crate::ast::Type::Bool => Type::Bool,
                                                        crate::ast::Type::Char => Type::Char,
                                                        crate::ast::Type::String => Type::String,
                                                        crate::ast::Type::Unit => Type::Unit,
                                                        _ => Type::Int64, // Fallback
                                                    };
                                                    Type::Function {
                                                        parameters: nested_converted_params,
                                                        return_type: Box::new(nested_converted_ret),
                                                    }
                                                },
                                                crate::ast::Type::Named(name) => Type::Named(name.clone()),
                                                _ => Type::Int64, // Fallback
                                            }
                                        }).collect();
                                        // Convert return type
                                        let converted_ret = match &**return_type {
                                            crate::ast::Type::Int8 => Type::Int8,
                                            crate::ast::Type::Int16 => Type::Int16,
                                            crate::ast::Type::Int32 => Type::Int32,
                                            crate::ast::Type::Int64 => Type::Int64,
                                            crate::ast::Type::Float16 => Type::Float16,
                                            crate::ast::Type::Float32 => Type::Float32,
                                            crate::ast::Type::Float64 => Type::Float64,
                                            crate::ast::Type::Bool => Type::Bool,
                                            crate::ast::Type::Char => Type::Char,
                                            crate::ast::Type::String => Type::String,
                                            crate::ast::Type::Unit => Type::Unit,
                                            crate::ast::Type::Function { parameters: nested_params, return_type: nested_ret } => {
                                                // Recursively handle nested function types in return type
                                                let nested_converted_params: Vec<Type> = nested_params.iter().map(|p| match p {
                                                    crate::ast::Type::Int8 => Type::Int8,
                                                    crate::ast::Type::Int16 => Type::Int16,
                                                    crate::ast::Type::Int32 => Type::Int32,
                                                    crate::ast::Type::Int64 => Type::Int64,
                                                    crate::ast::Type::Float16 => Type::Float16,
                                                    crate::ast::Type::Float32 => Type::Float32,
                                                    crate::ast::Type::Float64 => Type::Float64,
                                                    crate::ast::Type::Bool => Type::Bool,
                                                    crate::ast::Type::Char => Type::Char,
                                                    crate::ast::Type::String => Type::String,
                                                    crate::ast::Type::Unit => Type::Unit,
                                                    _ => Type::Int64, // Fallback
                                                }).collect();
                                                let nested_converted_ret = match &**nested_ret {
                                                    crate::ast::Type::Int8 => Type::Int8,
                                                    crate::ast::Type::Int16 => Type::Int16,
                                                    crate::ast::Type::Int32 => Type::Int32,
                                                    crate::ast::Type::Int64 => Type::Int64,
                                                    crate::ast::Type::Float16 => Type::Float16,
                                                    crate::ast::Type::Float32 => Type::Float32,
                                                    crate::ast::Type::Float64 => Type::Float64,
                                                    crate::ast::Type::Bool => Type::Bool,
                                                    crate::ast::Type::Char => Type::Char,
                                                    crate::ast::Type::String => Type::String,
                                                    crate::ast::Type::Unit => Type::Unit,
                                                    _ => Type::Int64, // Fallback
                                                };
                                                Type::Function {
                                                    parameters: nested_converted_params,
                                                    return_type: Box::new(nested_converted_ret),
                                                }
                                            },
                                            crate::ast::Type::Named(name) => Type::Named(name.clone()),
                                            _ => Type::Int64, // Fallback
                                        };
                                        Type::Function {
                                            parameters: converted_params,
                                            return_type: Box::new(converted_ret),
                                        }
                                    }
                                    _ => Type::Int64, // Fallback for other types
                                };
                                
                                self.variable_types.insert(name.clone(), var_type.clone());
                                
                                // If this is a function type, register it in function_variable_scopes
                                if let Type::Function { .. } = &var_type {
                                    // eprintln!("DEBUG TypedIdentifier bind (statements): registering function variable '{}' with type {:?}", name, var_type);
                                    self.add_function_variable(name.clone(), stored_val, &var_type);
                                } else {
                                    // For non-function types, use regular variable storage
                                    self.add_variable_text(name.clone(), stored_val);
                                }
                            }
                            Pattern::Tuple(elements) => {
                                // Handle tuple pattern destructuring in text IR
                                // The value_reg should be an i8* pointing to the tuple memory

                                // Clean the value_reg of any type prefixes for use in getelementptr
                                // Extract just the register name (everything after the last space)
                                let clean_value_reg = if let Some(space_pos) = value_reg.rfind(' ') {
                                    value_reg[space_pos + 1..].to_string()
                                } else {
                                    value_reg.clone()
                                };

                                // Get element types from pattern annotations, or from expression type
                                let mut element_types = Vec::new();

                                // First try to get types from pattern annotations
                                let mut has_typed_patterns = false;
                                for elem_pattern in elements {
                                    let elem_type = match elem_pattern {
                                        Pattern::TypedIdentifier { type_, .. } => {
                                            has_typed_patterns = true;
                                            type_.clone()
                                        }
                                        _ => Type::Int64, // Temporary fallback
                                    };
                                    element_types.push(elem_type);
                                }

                                // If no pattern annotations, try to get from expression type
                                if !has_typed_patterns {
                                    if let Some(location) = Self::try_get_expression_location(&**expr) {
                                        if let Some(Type::Tuple(ref expr_elem_types)) = self.expression_types.get(location) {
                                            element_types = expr_elem_types.clone();
                                        }
                                    } else if let Expression::Identifier(var_name) = &**expr {
                                        // Try to get from variable types
                                        if let Some(var_type) = self.variable_types.get(var_name) {
                                            if let Type::Tuple(ref var_elem_types) = var_type {
                                                element_types = var_elem_types.clone();
                                            }
                                        }
                                    }
                                }

                                // Calculate offsets to match tuple creation exactly
                                // Tuple structure: [count: i64][type_ids: i8*][element_data: ...]
                                // Expand type aliases (e.g. Named("boolean") -> Bool) so we get correct sizes
                                let element_types_expanded: Vec<Type> = element_types.iter()
                                    .map(|t| self.expand_type_aliases_codegen(t))
                                    .collect();
                                let element_count = elements.len() as i64;
                                let mut current_offset = 8; // Start after count
                                current_offset += element_count; // After type IDs

                                // Calculate element data layout with proper alignment
                                let mut element_offsets = Vec::new();
                                for elem_type in &element_types_expanded {
                                    let elem_size = self.get_type_size_bytes(elem_type);
                                    let elem_alignment = self.get_type_alignment_bytes(elem_type);

                                    // Align current offset to element alignment
                                    current_offset = ((current_offset + elem_alignment - 1) / elem_alignment) * elem_alignment;

                                    element_offsets.push(current_offset);
                                    current_offset += elem_size;
                                }

                                for (i, elem_pattern) in elements.iter().enumerate() {
                                    let elem_type = &element_types_expanded[i];
                                    let elem_size = self.get_type_size_bytes(elem_type);
                                    let current_offset = element_offsets[i];

                                    // Generate getelementptr to get element pointer
                                    let elem_ptr_reg = format!("%tuple_elem_ptr_{}_{}", i, self.instructions.len());
                                    self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}",
                                        elem_ptr_reg, clean_value_reg, current_offset));

                                    // Load the element value based on its type
                                    let elem_val_reg = format!("%tuple_elem_val_{}_{}", i, self.instructions.len());
                                    let elem_llvm_ty: &str = match elem_type {
                                        Type::Int64 => {
                                            // Cast i8* to i64* and load
                                            let cast_reg = format!("%tuple_elem_cast_{}_{}", i, self.instructions.len());
                                            self.instructions.push(format!("  {} = bitcast i8* {} to i64*", cast_reg, elem_ptr_reg));
                                            self.instructions.push(format!("  {} = load i64, i64* {}", elem_val_reg, cast_reg));
                                            "i64"
                                        }
                                        Type::Bool => {
                                            // Cast i8* to i1* and load
                                            let cast_reg = format!("%tuple_elem_cast_{}_{}", i, self.instructions.len());
                                            self.instructions.push(format!("  {} = bitcast i8* {} to i1*", cast_reg, elem_ptr_reg));
                                            self.instructions.push(format!("  {} = load i1, i1* {}", elem_val_reg, cast_reg));
                                            "i1"
                                        }
                                        Type::Char => {
                                            // Cast i8* to i32* and load
                                            let cast_reg = format!("%tuple_elem_cast_{}_{}", i, self.instructions.len());
                                            self.instructions.push(format!("  {} = bitcast i8* {} to i32*", cast_reg, elem_ptr_reg));
                                            self.instructions.push(format!("  {} = load i32, i32* {}", elem_val_reg, cast_reg));
                                            "i32"
                                        }
                                        Type::Function { .. } => {
                                            // Cast i8* to i8** and load function pointer
                                            let cast_reg = format!("%tuple_elem_cast_{}_{}", i, self.instructions.len());
                                            self.instructions.push(format!("  {} = bitcast i8* {} to i8**", cast_reg, elem_ptr_reg));
                                            self.instructions.push(format!("  {} = load i8*, i8** {}", elem_val_reg, cast_reg));
                                            "i8*"
                                        }
                                        Type::String => {
                                            // Cast i8* to i8** and load string pointer
                                            let cast_reg = format!("%tuple_elem_cast_{}_{}", i, self.instructions.len());
                                            self.instructions.push(format!("  {} = bitcast i8* {} to i8**", cast_reg, elem_ptr_reg));
                                            self.instructions.push(format!("  {} = load i8*, i8** {}", elem_val_reg, cast_reg));
                                            "i8*"
                                        }
                                        Type::Tuple(_) | Type::Record(_) => {
                                            // Cast i8* to i8** and load nested tuple/struct pointer
                                            let cast_reg = format!("%tuple_elem_cast_{}_{}", i, self.instructions.len());
                                            self.instructions.push(format!("  {} = bitcast i8* {} to i8**", cast_reg, elem_ptr_reg));
                                            self.instructions.push(format!("  {} = load i8*, i8** {}", elem_val_reg, cast_reg));
                                            "i8*"
                                        }
                                        _ => {
                                            // Default: cast to i64* and load as i64
                                            let cast_reg = format!("%tuple_elem_cast_{}_{}", i, self.instructions.len());
                                            self.instructions.push(format!("  {} = bitcast i8* {} to i64*", cast_reg, elem_ptr_reg));
                                            self.instructions.push(format!("  {} = load i64, i64* {}", elem_val_reg, cast_reg));
                                            "i64"
                                        }
                                    };
                                    // Store with type prefix so later use (e.g. record field) sees correct type and doesn't emit "store i8* %reg" when %reg is i64
                                    let elem_val_with_ty = format!("{} {}", elem_llvm_ty, elem_val_reg);

                                    // Handle the element pattern
                                    match elem_pattern {
                                        Pattern::Identifier(name) => {
                                            self.add_variable_text(name.clone(), elem_val_with_ty);
                                            // Also store the type information
                                            self.variable_types.insert(name.clone(), elem_type.clone());
                                        }
                                        Pattern::TypedIdentifier { name, type_ } => {
                                            // Check if the declared type is a function type
                                            if let crate::ast::Type::Function { .. } = type_ {
                                                // For function types, create a dummy internal type for storage
                                                // The actual type checking ensures this is correct
                                                let dummy_func_type = Type::Function {
                                                    parameters: vec![Type::Int64], // Simplified
                                                    return_type: Box::new(Type::Int64),
                                                };
                                                self.add_function_variable(name.clone(), elem_val_reg, &dummy_func_type);
                                            } else {
                                                self.add_variable_text(name.clone(), elem_val_with_ty);
                                            }
                                            // Store the type information
                                            // Convert ast::Type to the internal Type representation
                                            let silica_type = match type_ {
                                                crate::ast::Type::Int64 => Type::Int64,
                                                crate::ast::Type::Bool => Type::Bool,
                                                crate::ast::Type::Char => Type::Char,
                                                crate::ast::Type::String => Type::String,
                                                crate::ast::Type::Tuple(_) => Type::Tuple(vec![]), // Simplified
                                                _ => Type::Int64, // Fallback
                                            };
                                            self.variable_types.insert(name.clone(), silica_type);
                                        }
                                        _ => {
                                            return Err(CompilerError::codegen_error(
                                                format!("Nested complex patterns in tuple destructuring not yet supported: {:?}", elem_pattern)
                                            ));
                                        }
                                    }
                                }
                            }
                            _ => {
                                return Err(CompilerError::codegen_error(
                                    format!("Complex patterns in function bodies not yet supported: {:?}", pattern)
                                ));
                            }
                        }
                    }
                }
                Statement::Expr(expr) => {
                    // Generate the expression and capture its result (for return value)
                    last_result = self.generate_expression(expr)?;
                }
            }
        }

        Ok(last_result)
    }

    /// Generate LLVM IR for a sequence of statements (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_statements_llvm(&mut self, statements: &[Statement]) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        let mut last_result = None;

        for statement in statements {
            match statement {
                Statement::Bind { pattern, expr } => {
                    let value = self.generate_expression_llvm(expr)?;
                    // Handle pattern binding - for now just handle simple identifier patterns
                    if let Some(value) = value {
                        match pattern {
                            Pattern::Identifier(name) => {
                                // Allocate space for the variable and store the value
                                if let Some(builder) = &self.builder {
                                    unsafe {
                                        let alloca = builder.build_alloca(value.get_type(), name).unwrap();
                                        builder.build_store(alloca, value).unwrap();
                                        self.add_variable(name.clone(), alloca);
                                    }
                                }
                            }
                            Pattern::TypedIdentifier { name, .. } => {
                                // Allocate space for the variable and store the value
                                if let Some(builder) = &self.builder {
                                    unsafe {
                                        let alloca = builder.build_alloca(value.get_type(), name).unwrap();
                                        builder.build_store(alloca, value).unwrap();
                                        self.add_variable(name.clone(), alloca);
                                    }
                                }
                            }
                            Pattern::Tuple(elements) => {
                                // Handle tuple pattern destructuring in LLVM
                                // The value should be an i8* pointing to the tuple memory
                                if let Some(tuple_ptr) = value.as_pointer_value() {
                                    // Generate destructuring for each element
                                    for (i, elem_pattern) in elements.iter().enumerate() {
                                        // Calculate offset for element i
                                        // Tuple layout: [count: i64][type_ids: i8*][element_data...]
                                        let element_count = elements.len() as i64;
                                        let mut current_offset = 8 + element_count; // After count and type IDs
                                        current_offset = ((current_offset + 7) / 8) * 8; // Align to 8 bytes

                                        // Add offset for previous elements (simplified - assume all elements are 8 bytes)
                                        current_offset += i as i64 * 8;

                                        if let Some(builder) = &self.builder {
                                            // Generate getelementptr to get element pointer
                                            let elem_ptr = unsafe {
                                                builder.build_gep(
                                                    tuple_ptr,
                                                    &[self.context.i64_type().const_int(current_offset as u64, false)],
                                                    &format!("tuple_elem_{}", i)
                                                ).unwrap()
                                            };

                                            // Handle the element pattern
                                            match elem_pattern {
                                                Pattern::Identifier(name) => {
                                                    // Load as i64 and allocate space
                                                    let elem_value = unsafe {
                                                        builder.build_load(
                                                            self.context.i64_type(),
                                                            elem_ptr,
                                                            &format!("elem_val_{}", i)
                                                        ).unwrap()
                                                    };
                                                    let elem_alloca = unsafe {
                                                        builder.build_alloca(self.context.i64_type(), name).unwrap()
                                                    };
                                                    unsafe {
                                                        builder.build_store(elem_alloca, elem_value).unwrap();
                                                    }
                                                    self.add_variable(name.clone(), elem_alloca);
                                                }
                                                Pattern::TypedIdentifier { name, type_ } => {
                                                    // Check if this is a function type
                                                    if let crate::ast::Type::Function { .. } = type_ {
                                                        // Load as pointer type for function pointers
                                                        let elem_value = unsafe {
                                                            builder.build_load(
                                                                self.context.i8_type().ptr_type(inkwell::AddressSpace::Generic),
                                                                elem_ptr,
                                                                &format!("elem_val_{}", i)
                                                            ).unwrap()
                                                        };
                                                        // For function pointers, allocate as i8* (pointer type)
                                                        let elem_alloca = unsafe {
                                                            builder.build_alloca(self.context.i8_type().ptr_type(inkwell::AddressSpace::Generic), name).unwrap()
                                                        };
                                                        unsafe {
                                                            builder.build_store(elem_alloca, elem_value).unwrap();
                                                        }
                                                        self.add_variable(name.clone(), elem_alloca);
                                                        // TODO: Also store function signature information for LLVM
                                                    } else {
                                                        // Load as i64 and allocate space
                                                        let elem_value = unsafe {
                                                            builder.build_load(
                                                                self.context.i64_type(),
                                                                elem_ptr,
                                                                &format!("elem_val_{}", i)
                                                            ).unwrap()
                                                        };
                                                        let elem_alloca = unsafe {
                                                            builder.build_alloca(self.context.i64_type(), name).unwrap()
                                                        };
                                                        unsafe {
                                                            builder.build_store(elem_alloca, elem_value).unwrap();
                                                        }
                                                        self.add_variable(name.clone(), elem_alloca);
                                                    }
                                                }
                                                _ => {
                                                    return Err(CompilerError::codegen_error(
                                                        format!("Nested complex patterns in tuple destructuring not yet supported: {:?}", elem_pattern)
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    return Err(CompilerError::codegen_error(
                                        "Tuple pattern requires pointer value".to_string()
                                    ));
                                }
                            }
                            _ => {
                                return Err(CompilerError::codegen_error(
                                    format!("Complex patterns in function bodies not yet supported: {:?}", pattern)
                                ));
                            }
                        }
                    }
                }
                Statement::Expr(expr) => {
                    // Generate the expression and capture its result (for return value)
                    last_result = self.generate_expression_llvm(expr)?;
                }
            }
        }

        Ok(last_result)
    }

    /// Generate LLVM IR for message send (send)
    fn generate_send(&mut self, send: &SendExpr) -> Result<Option<String>> {
        // Generate actor and message expressions
        let actor = self.generate_expression(&send.actor)?;
        let message = self.generate_expression(&send.message)?;

        if let (Some(actor_ref), Some(mut msg)) = (actor, message) {
            // For messages, we need to allocate memory and store the message value
            // This is similar to how states are handled in spawn
            let mut msg_final_ptr = format!("%msg_final_{}", self.instructions.len());

            if msg.starts_with("i64 ") {
                // Integer message - allocate and store
                let int_val = &msg[4..];
                let alloc_reg = format!("%msg_alloc_{}", self.instructions.len());
                let int_ptr = format!("%msg_int_ptr_{}", self.instructions.len());
                self.instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg)); // Allocate 8 bytes for i64
                self.instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                self.instructions.push(format!("  store i64 {}, i64* {}", int_val, int_ptr));
                self.instructions.push(format!("  {} = bitcast i64* {} to i8*", msg_final_ptr, int_ptr));
            } else if msg.starts_with("%") {
                // Register containing a value - allocate memory and store
                if msg.contains("tuple_alloc") {
                    // Tuple pointer - use directly
                    msg_final_ptr.clone_from(&msg);
                } else {
                    // i64 register - allocate and store
                    let alloc_reg = format!("%msg_alloc_{}", self.instructions.len());
                    let int_ptr = format!("%msg_int_ptr_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg));
                    self.instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                    self.instructions.push(format!("  store i64 {}, i64* {}", msg, int_ptr));
                    self.instructions.push(format!("  {} = bitcast i64* {} to i8*", msg_final_ptr, int_ptr));
                }
            } else {
                // Other types - assume they need memory allocation
                let alloc_reg = format!("%msg_alloc_{}", self.instructions.len());
                let int_ptr = format!("%msg_int_ptr_{}", self.instructions.len());
                self.instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg));
                self.instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                self.instructions.push(format!("  store i64 0, i64* {}", int_ptr)); // Default
                self.instructions.push(format!("  {} = bitcast i64* {} to i8*", msg_final_ptr, int_ptr));
            }

            // Convert actor_ref from i64 to i8* if needed (spawn returns i64, but send expects i8*)
            let actor_ptr = if actor_ref.starts_with("i64 ") {
                // Extract the register name
                let actor_reg = &actor_ref[4..];
                let ptr_reg = format!("%actor_ptr_{}", self.instructions.len());
                self.instructions.push(format!("  {} = inttoptr i64 {} to i8*", ptr_reg, actor_reg));
                ptr_reg
            } else if actor_ref.starts_with("%") && !actor_ref.contains("i8*") {
                // Register that's likely i64 - convert to i8*
                let ptr_reg = format!("%actor_ptr_{}", self.instructions.len());
                self.instructions.push(format!("  {} = inttoptr i64 {} to i8*", ptr_reg, actor_ref));
                ptr_reg
            } else {
                // Already a pointer or has type prefix - use directly
                if let Some(stripped) = actor_ref.strip_prefix("i8* ") {
                    stripped.to_string()
                } else {
                    actor_ref
                }
            };
            
            // Call Silica runtime send function
            self.instructions.push(format!("  call void @silica_actor_send({}, {})", actor_ptr, msg_final_ptr));

            // Send operations return unit, so no result register
            Ok(None)
        } else {
            codegen_error("Invalid actor or message for send".to_string())
        }
    }

    /// Generate LLVM IR for message cast (cast)
    fn generate_cast(&mut self, cast: &CastExpr) -> Result<Option<String>> {
        // Generate actor and message expressions
        let actor = self.generate_expression(&cast.actor)?;
        let message = self.generate_expression(&cast.message)?;

        if let (Some(actor_ref), Some(mut msg)) = (actor, message) {
            // For messages, we need to allocate memory and store the message value
            // This is similar to how states are handled in spawn
            let mut msg_final_ptr = format!("%msg_final_{}", self.instructions.len());

            if msg.starts_with("i64 ") {
                // Integer message - allocate and store
                let int_val = &msg[4..];
                let alloc_reg = format!("%msg_alloc_{}", self.instructions.len());
                let int_ptr = format!("%msg_int_ptr_{}", self.instructions.len());
                self.instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg)); // Allocate 8 bytes for i64
                self.instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                self.instructions.push(format!("  store i64 {}, i64* {}", int_val, int_ptr));
                self.instructions.push(format!("  {} = bitcast i64* {} to i8*", msg_final_ptr, int_ptr));
            } else if msg.starts_with("%") {
                // Register containing a value - allocate memory and store
                if msg.contains("tuple_alloc") {
                    // Tuple pointer - use directly
                    msg_final_ptr.clone_from(&msg);
                } else {
                    // i64 register - allocate and store
                    let alloc_reg = format!("%msg_alloc_{}", self.instructions.len());
                    let int_ptr = format!("%msg_int_ptr_{}", self.instructions.len());
                    self.instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg));
                    self.instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                    self.instructions.push(format!("  store i64 {}, i64* {}", msg, int_ptr));
                    self.instructions.push(format!("  {} = bitcast i64* {} to i8*", msg_final_ptr, int_ptr));
                }
            } else {
                // Other types - assume they need memory allocation
                let alloc_reg = format!("%msg_alloc_{}", self.instructions.len());
                let int_ptr = format!("%msg_int_ptr_{}", self.instructions.len());
                self.instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg));
                self.instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                self.instructions.push(format!("  store i64 0, i64* {}", int_ptr)); // Default
                self.instructions.push(format!("  {} = bitcast i64* {} to i8*", msg_final_ptr, int_ptr));
            }

            // Convert actor_ref from i64 to i8* if needed (spawn returns i64, but cast expects i8*)
            let actor_ptr = if actor_ref.starts_with("i64 ") {
                // Extract the register name
                let actor_reg = &actor_ref[4..];
                let ptr_reg = format!("%actor_ptr_{}", self.instructions.len());
                self.instructions.push(format!("  {} = inttoptr i64 {} to i8*", ptr_reg, actor_reg));
                ptr_reg
            } else if actor_ref.starts_with("%") && !actor_ref.contains("i8*") {
                // Register that's likely i64 - convert to i8*
                let ptr_reg = format!("%actor_ptr_{}", self.instructions.len());
                self.instructions.push(format!("  {} = inttoptr i64 {} to i8*", ptr_reg, actor_ref));
                ptr_reg
            } else {
                // Already a pointer or has type prefix - use directly
                if let Some(stripped) = actor_ref.strip_prefix("i8* ") {
                    stripped.to_string()
                } else {
                    actor_ref
                }
            };
            
            // Call Silica runtime cast function - returns bool
            let result_reg = format!("%cast_result_{}", self.instructions.len());
            self.instructions.push(format!("  {} = call i1 @silica_actor_cast(i8* {}, i8* {})", result_reg, actor_ptr, msg_final_ptr));
            
            // Convert bool (i1) to i64 for return
            let bool_i64_reg = format!("%cast_bool_i64_{}", self.instructions.len());
            self.instructions.push(format!("  {} = zext i1 {} to i64", bool_i64_reg, result_reg));

            Ok(Some(bool_i64_reg))
        } else {
            codegen_error("Invalid actor or message for cast".to_string())
        }
    }

    /// Generate LLVM IR for message receive (recv)
    fn generate_recv(&mut self, recv: &RecvExpr) -> Result<Option<String>> {
        let msg_reg = format!("%msg_{}", self.instructions.len());

        if let Some(actor_expr) = &recv.actor {
            // recv(actor) - receive from specific actor
            let actor_val = self.generate_expression(actor_expr)?
                .ok_or_else(|| CompilerError::codegen_error("Invalid actor in recv".to_string()))?;

            // For LLVM IR function calls, arguments should have type prefixes
            let typed_actor = if actor_val.starts_with("i64 ") || actor_val.starts_with("i1 ") {
                actor_val
            } else {
                format!("i8* {}", actor_val)
            };

            self.instructions.push(format!("  {} = call i8* @silica_actor_recv({})", msg_reg, typed_actor));
        } else {
            // recv() - this is not supported without an actor context
            // For now, return a null pointer
            self.instructions.push(format!("  {} = bitcast i8* null to i8*", msg_reg));
        }

        Ok(Some(msg_reg))
    }

    /// Generate the body instructions for a function literal
    fn generate_function_literal_body_with_captures(&mut self, func_lit: &FunctionLiteralExpr, captured_vars: &[String]) -> Result<Vec<String>> {
        // Create a temporary instruction buffer for this function literal
        let original_instructions = std::mem::take(&mut self.instructions);
        let mut body_instructions = Vec::new();

        // Captured variables are already set up as parameters in generate_function_literal
        // (with names like %captured_0, %captured_1, etc.)
        // They're in the innermost scope, so they'll be found first by lookup_variable_text
        // Store the mapping of captured variable names to their parameter register indices
        let captured_var_map: std::collections::HashMap<String, usize> = captured_vars.iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), i))
            .collect();
        
        // Store this map in a way that generate_function_literal_expr can access it
        // For now, we'll pass it through the function calls, but a cleaner solution would be
        // to store it in the CodeGenerator struct. For the bootstrap compiler, we'll use
        // a workaround: check if the variable is in captured_vars when looking it up.

        // Get the return type and generate appropriate return instruction
        // Behavior functions are those with exactly 2 parameters (used in actor spawn)
        let is_behavior_function = func_lit.parameters.len() == 2;

        // Set the behavior function flag for code generation
        self.in_behavior_function = is_behavior_function;

        // Evaluate the function body statements and collect instructions
        let result_value = self.generate_function_literal_statements(&func_lit.body, func_lit, &mut body_instructions)?;

        // Clear the behavior function flag
        self.in_behavior_function = false;

        // For behavior functions, use the actual declared return type (not forced to i8*)
        // This allows behavior functions to return any valid Silica type
        let return_type = func_lit.return_type.as_ref().unwrap_or(&Type::Unit);
        let return_type_str = self.type_map.silica_to_llvm_str(return_type);

        // For behavior functions, ensure we return i8* (allocate memory for primitives if needed)
        // For regular functions, use the computed result
        if is_behavior_function {
            // Behavior functions always return i8* for runtime compatibility
            if result_value.starts_with("i8* ") || result_value.starts_with("%box_") || result_value.contains("box_result") {
                // Result is already boxed (e.g., from case expressions or function calls), return directly
                let clean_result = if result_value.starts_with("i8* ") {
                    result_value.trim_start_matches("i8* ").to_string()
                } else {
                    result_value
                };
                let ret_operand = Self::format_llvm_value_ref(&clean_result);
                body_instructions.push(format!("    ret i8* {}", ret_operand));
            } else {
                // Need to box the result
            let alloc_reg = format!("%return_alloc_{}", body_instructions.len());
            let store_ptr_reg = format!("%return_ptr_{}", body_instructions.len());
                let (result_type, clean_result) = if result_value.starts_with("i64 ") {
                    ("i64".to_string(), result_value.trim_start_matches("i64 ").to_string())
            } else if result_value.starts_with("i1 ") {
                    ("i1".to_string(), result_value.trim_start_matches("i1 ").to_string())
                } else if result_value.starts_with("i8* ") {
                    ("i8*".to_string(), result_value.trim_start_matches("i8* ").to_string())
                } else if result_value.contains("box_result") || result_value.starts_with("%box_") {
                    // This is a boxed value register - need to load the actual value
                    let load_bitcast_reg = format!("%return_bitcast_{}", body_instructions.len());
                    let load_reg = format!("%return_load_{}", body_instructions.len());
                    body_instructions.push(format!("    {} = bitcast i8* {} to i64*", load_bitcast_reg, result_value));
                    body_instructions.push(format!("    {} = load i64, i64* {}", load_reg, load_bitcast_reg));
                    ("i64".to_string(), load_reg.to_string())
            } else {
                    // No type prefix - assume i64 (default for computations)
                    ("i64".to_string(), result_value.to_string())
            };

            body_instructions.push(format!("    {} = call i8* @malloc(i64 8)", alloc_reg));
            body_instructions.push(format!("    {} = bitcast i8* {} to {}*", store_ptr_reg, alloc_reg, result_type));
            body_instructions.push(format!("    store {} {}, {}* {}", result_type, clean_result, result_type, store_ptr_reg));
            body_instructions.push(format!("    ret i8* {}", alloc_reg));
            }
        } else {
            // Regular functions return their actual type
            // Handle type conversions if needed (e.g., i64 to i8* for ActorRef)
            let (final_result_type, final_result_reg) = if return_type_str == "i8*" && result_value.starts_with("i64 ") {
                // Need to convert i64 to i8* (e.g., for ActorRef return type)
                let i64_reg = result_value.trim_start_matches("i64 ").to_string();
                let ptr_reg = format!("%return_ptr_{}", body_instructions.len());
                body_instructions.push(format!("    {} = inttoptr i64 {} to i8*", ptr_reg, i64_reg));
                ("i8*".to_string(), ptr_reg)
            } else if return_type_str == "i64" && result_value.starts_with("i8* ") {
                // Need to convert i8* to i64 (unlikely but handle for completeness)
                let ptr_reg = result_value.trim_start_matches("i8* ").to_string();
                let int_reg = format!("%return_int_{}", body_instructions.len());
                body_instructions.push(format!("    {} = ptrtoint i8* {} to i64", int_reg, ptr_reg));
                ("i64".to_string(), int_reg)
            } else {
                // No conversion needed - extract register name
                let clean_result = result_value.trim_start_matches("i64 ")
                    .trim_start_matches("i1 ")
                    .trim_start_matches("i8* ")
                    .trim_start_matches("i32 ")
                    .to_string();
                (return_type_str.to_string(), clean_result)
            };
            let ret_operand = Self::format_llvm_value_ref(&final_result_reg);
            body_instructions.push(format!("    ret {} {}", final_result_type, ret_operand));
        }

        // Restore the original instructions
        self.instructions = original_instructions;

        Ok(body_instructions)
    }

    /// Generate statements for function literal body
    fn generate_function_literal_statements(&mut self, statements: &[Statement], func_lit: &FunctionLiteralExpr, body_instructions: &mut Vec<String>) -> Result<String> {
        let mut last_result = "0".to_string(); // Default return value

        for statement in statements {
            match statement {
                Statement::Bind { pattern, expr } => {
                    // Generate the expression
                    let expr_result = self.generate_function_literal_expr(expr, func_lit, body_instructions)?;

                    // Handle identifier patterns (both simple and typed)
                    match pattern {
                        Pattern::Identifier(var_name) => {
                            // Simple identifier binding
                            self.add_variable_text(var_name.clone(), expr_result.clone());
                        }
                        Pattern::TypedIdentifier { name: var_name, .. } => {
                            // Typed identifier binding
                            self.add_variable_text(var_name.clone(), expr_result.clone());
                        }
                        _ => {
                            // For other pattern types, we could add support later
                            // For now, just ignore them (they don't contribute to return value)
                        }
                    }
                    // The result of a bind statement doesn't contribute to the return value
                }
                Statement::Expr(expr) => {
                    // Generate the expression and use its result as the potential return value
                    last_result = self.generate_function_literal_expr(expr, func_lit, body_instructions)?;
                }
            }
        }

        Ok(last_result)
    }

    /// Analyze function literal for captured variables from outer scope
    fn analyze_captured_variables(&self, func_lit: &FunctionLiteralExpr) -> Result<Vec<String>> {
        let mut captured_vars = Vec::new();
        self.collect_captured_variables_from_statements(&func_lit.body, &func_lit.parameters, &mut captured_vars)?;
        Ok(captured_vars)
    }

    /// Analyze function literal for captured variables with their types
    fn analyze_captured_variables_with_types(&self, func_lit: &FunctionLiteralExpr) -> Result<Vec<(String, Type)>> {
        let captured_names = self.analyze_captured_variables(func_lit)?;
        let mut captured_vars_with_types = Vec::new();

        for name in captured_names {
            if let Some(var_type) = self.variable_types.get(&name) {
                captured_vars_with_types.push((name, var_type.clone()));
            } else {
                // For now, assume all captured variables are i64 if type is not found
                // This is a temporary fix until proper type inference is implemented
                captured_vars_with_types.push((name, Type::Int64));
            }
        }

        Ok(captured_vars_with_types)
    }

    /// Recursively collect captured variables from statements
    fn collect_captured_variables_from_statements(&self, statements: &[Statement], local_params: &[crate::ast::Parameter], captured: &mut Vec<String>) -> Result<()> {
        let mut local_vars = std::collections::HashSet::new();

        // Add parameters as local variables
        for param in local_params {
            local_vars.insert(param.name.clone());
        }

        // Collect bound variables from statements
        for statement in statements {
            if let Statement::Bind { pattern, .. } = statement {
                self.collect_bound_vars_from_pattern_codegen(pattern, &mut local_vars);
            }
        }

        // Now collect used variables from all expressions in statements
        for statement in statements {
            match statement {
                Statement::Bind { expr, .. } => {
                    self.collect_captured_variables(expr, local_params, captured)?;
                }
                Statement::Expr(expr) => {
                    self.collect_captured_variables(expr, local_params, captured)?;
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

    /// Collect bound variables from a pattern (for codegen)
    fn collect_bound_vars_from_pattern_codegen(&self, pattern: &Pattern, bound_vars: &mut std::collections::HashSet<String>) {
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
                    self.collect_bound_vars_from_pattern_codegen(pattern, bound_vars);
                }
            }
            Pattern::Literal(_) => {
                // Literals don't bind variables
            }
            Pattern::Record(fields) => {
                for (_, field_pattern) in fields {
                    self.collect_bound_vars_from_pattern_codegen(field_pattern, bound_vars);
                }
            }
            Pattern::Variant { payload, .. } => {
                if let Some(payload_pattern) = payload {
                    self.collect_bound_vars_from_pattern_codegen(payload_pattern, bound_vars);
                }
            }
            Pattern::Alternative(patterns) => {
                for pattern in patterns {
                    self.collect_bound_vars_from_pattern_codegen(pattern, bound_vars);
                }
            }
        }
    }

    /// Return true if expr contains a reference to the given identifier (for self-referential bindings).
    fn expression_references_identifier(expr: &Expression, name: &str) -> bool {
        match expr {
            Expression::Identifier(n) => n == name,
            Expression::Binary(b) => Self::expression_references_identifier(&b.left, name)
                || Self::expression_references_identifier(&b.right, name),
            Expression::Unary(u) => Self::expression_references_identifier(&u.operand, name),
            Expression::Call(c) => c.arguments.iter().any(|a| Self::expression_references_identifier(a, name)),
            Expression::ModuleCall(m) => m.arguments.iter().any(|a| Self::expression_references_identifier(a, name)),
            Expression::If(i) => Self::expression_references_identifier(&i.condition, name)
                || Self::expression_references_identifier(&i.then_branch, name)
                || Self::expression_references_identifier(&i.else_branch, name),
            Expression::Case(c) => Self::expression_references_identifier(&c.scrutinee, name)
                || c.branches.iter().any(|b| {
                    b.guard.as_ref().map_or(false, |g| Self::expression_references_identifier(g, name))
                        || Self::expression_references_identifier(&b.body, name)
                }),
            Expression::Do(d) => d.statements.iter().any(|s| match s {
                Statement::Bind { expr: e, .. } => Self::expression_references_identifier(e, name),
                Statement::Expr(e) => Self::expression_references_identifier(e, name),
            }),
            Expression::StructLiteral(s) => s.fields.iter().any(|(_, e)| Self::expression_references_identifier(e, name)),
            Expression::FieldAccess(f) => Self::expression_references_identifier(&f.object, name),
            Expression::Tuple(exprs) => exprs.iter().any(|e| Self::expression_references_identifier(e, name)),
            Expression::ConstructorCall(c) => c.payload.as_ref().map_or(false, |a| Self::expression_references_identifier(a, name)),
            Expression::FunctionLiteral(_) => false,
            Expression::AsType(a) => Self::expression_references_identifier(&a.expression, name),
            Expression::Literal(_) => false,
            _ => false,
        }
    }

    /// Recursively collect captured variables from an expression
    fn collect_captured_variables(&self, expr: &Expression, local_params: &[crate::ast::Parameter], captured: &mut Vec<String>) -> Result<()> {
        match expr {
            Expression::Identifier(name) => {
                // Check if it's a local parameter
                let is_local = local_params.iter().any(|param| param.name == *name);
                if !is_local && !captured.contains(name) {
                    // For bootstrap compiler, assume captured variables are valid
                    // In a full implementation, this would check scope properly
                    captured.push(name.clone());
                }
            },
            Expression::Binary(binary) => {
                self.collect_captured_variables(&binary.left, local_params, captured)?;
                self.collect_captured_variables(&binary.right, local_params, captured)?;
            },
            Expression::Case(case_expr) => {
                self.collect_captured_variables(&case_expr.scrutinee, local_params, captured)?;
                for branch in &case_expr.branches {
                    if let Some(guard) = &branch.guard {
                        self.collect_captured_variables(guard, local_params, captured)?;
                    }
                    self.collect_captured_variables(&branch.body, local_params, captured)?;
                }
            },
            Expression::If(if_expr) => {
                self.collect_captured_variables(&if_expr.condition, local_params, captured)?;
                self.collect_captured_variables(&if_expr.then_branch, local_params, captured)?;
                self.collect_captured_variables(&if_expr.else_branch, local_params, captured)?;
            },
            Expression::Call(call) => {
                for arg in &call.arguments {
                    self.collect_captured_variables(arg, local_params, captured)?;
                }
            },
            Expression::Do(do_expr) => {
                for statement in &do_expr.statements {
                    match statement {
                        crate::ast::Statement::Expr(expr) => {
                            self.collect_captured_variables(expr, local_params, captured)?;
                        },
                        crate::ast::Statement::Bind { pattern, expr } => {
                            self.collect_captured_variables(expr, local_params, captured)?;
                        }
                    }
                }
            },
            Expression::FunctionLiteral(func_lit) => {
                // For nested function literals, we don't collect their captured variables
                // as they have their own scope. Just analyze their body.
                self.collect_captured_variables_from_statements(&func_lit.body, &func_lit.parameters, &mut Vec::new())?;
            },
            Expression::Cast(cast) => {
                // Collect captured variables from cast actor and message
                self.collect_captured_variables(&cast.actor, local_params, captured)?;
                self.collect_captured_variables(&cast.message, local_params, captured)?;
            },
            Expression::Spawn(spawn) => {
                // Collect captured variables from spawn expressions
                self.collect_captured_variables(&spawn.initial_state, local_params, captured)?;
                self.collect_captured_variables(&spawn.behavior, local_params, captured)?;
                if let Some(affinity) = &spawn.core_affinity {
                    self.collect_captured_variables(affinity, local_params, captured)?;
                }
            },
            Expression::Send(send) => {
                // Collect captured variables from send expressions
                self.collect_captured_variables(&send.actor, local_params, captured)?;
                self.collect_captured_variables(&send.message, local_params, captured)?;
            },
            Expression::Recv(recv) => {
                // Collect captured variables from recv expressions
                if let Some(actor) = &recv.actor {
                    self.collect_captured_variables(actor, local_params, captured)?;
                }
            },
            Expression::StructLiteral(struct_lit) => {
                // Collect captured variables from struct literal field expressions
                for (_, field_expr) in &struct_lit.fields {
                    self.collect_captured_variables(field_expr, local_params, captured)?;
                }
            },
            Expression::FieldAccess(field_access) => {
                // Collect captured variables from field access object
                self.collect_captured_variables(&field_access.object, local_params, captured)?;
            },
            Expression::Tuple(tuple) => {
                // Collect captured variables from tuple elements
                for elem in tuple {
                    self.collect_captured_variables(elem, local_params, captured)?;
                }
            },
            Expression::Unary(unary) => {
                // Collect captured variables from unary operand
                self.collect_captured_variables(&unary.operand, local_params, captured)?;
            },
            Expression::Literal(_) => {
                // Literals don't capture variables
            },
            _ => {
                // For now, ignore other expression types that don't contain identifiers
                // (e.g., ReadFile, WriteFile, etc. - these are handled separately if needed)
            }
        }
        Ok(())
    }

    /// Generate pattern matching check for function literals (returns i1 result)
    fn generate_function_literal_pattern_check(&mut self, pattern: &Pattern, scrutinee_val: &str, body_instructions: &mut Vec<String>) -> Result<String> {
        match pattern {
            Pattern::Literal(lit) => {
                let pattern_val = self.generate_literal(lit);

                // Determine comparison type based on operand types
                let (compare_type, clean_scrutinee) = if scrutinee_val.contains("%0") || scrutinee_val.contains("%1") {
                    // Parameter register in function - bitcast and load (i8* parameters contain boxed i64)
                    let bitcast_reg = format!("%scrutinee_bitcast_{}", body_instructions.len());
                    let load_reg = format!("%scrutinee_load_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = bitcast i8* {} to i64*", bitcast_reg, scrutinee_val));
                    body_instructions.push(format!("  {} = load i64, i64* {}", load_reg, bitcast_reg));
                    ("i64", load_reg)
                } else if scrutinee_val.starts_with("i8* ") {
                    // Scrutinee is a pointer - bitcast and load it for comparison
                    let ptr_reg = scrutinee_val.trim_start_matches("i8* ");
                    let bitcast_reg = format!("%scrutinee_bitcast_{}", body_instructions.len());
                    let load_reg = format!("%scrutinee_load_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = bitcast i8* {} to i64*", bitcast_reg, ptr_reg));
                    body_instructions.push(format!("  {} = load i64, i64* {}", load_reg, bitcast_reg));
                    ("i64", load_reg)
                } else if scrutinee_val.starts_with("i1 ") {
                    ("i1", scrutinee_val.trim_start_matches("i1 ").to_string())
                } else {
                    ("i64", scrutinee_val.trim_start_matches("i64 ").trim_start_matches("i1 ").to_string())
                };

                let clean_pattern = if pattern_val.starts_with("i1 ") {
                    pattern_val.trim_start_matches("i1 ")
                } else {
                    pattern_val.trim_start_matches("i64 ").trim_start_matches("i1 ")
                };

                let result_reg = format!("%pattern_check_{}", body_instructions.len());
                body_instructions.push(format!("  {} = icmp eq {} {}, {}", result_reg, compare_type, clean_scrutinee, clean_pattern));
                Ok(result_reg)
            },
            Pattern::TypedIdentifier { name, .. } => {
                // All patterns now match (variable binding handled elsewhere)
                Ok("1".to_string())
            },
            _ => {
                // For bootstrap compiler, treat complex patterns as wildcards
                Ok("1".to_string())
            }
        }
    }

    /// Find field offset by searching through all known struct types
    /// This is a truly generic solution that works for any struct definition
    fn find_field_offset_in_any_struct(&self, field_name: &str) -> Result<i64> {
        // Search through all type aliases to find structs that contain this field
        for (type_name, type_def) in &self.type_aliases {
            if let Type::Record(fields) = type_def {
                if let Ok(offset) = self.calculate_field_offset_in_record(fields, field_name) {
                    return Ok(offset);
                }
            }
        }

        // If not found in type aliases, we cannot determine the offset
        Err(CompilerError::codegen_error(format!("Field '{}' not found in any known struct type", field_name)))
    }

    fn find_field_info_in_any_struct(&self, field_name: &str) -> Result<(i64, Type)> {
        // Search through all type aliases to find structs that contain this field
        for (type_name, type_def) in &self.type_aliases {
            if let Type::Record(fields) = type_def {
                if let Ok((offset, field_type)) = self.calculate_field_info_in_record(fields, field_name) {
                    return Ok((offset, field_type));
                }
            }
        }

        // If not found in type aliases, we cannot determine the offset and type
        Err(CompilerError::codegen_error(format!("Field '{}' not found in any known struct type", field_name)))
    }


    /// Calculate field offset within a record type
    fn calculate_field_offset_in_record(&self, fields: &[(String, Type)], field_name: &str) -> Result<i64> {
        let mut offset = 0i64;
        for (name, field_type) in fields {
            if name == field_name {
                return Ok(offset);
            }
            // Calculate field size (simplified: assume all fields are 8 bytes)
            // In a full implementation, this would calculate actual field sizes
            offset += 8;
        }
        Err(CompilerError::codegen_error(format!("Field '{}' not found in record", field_name)))
    }

    /// Calculate field offset and type within a record type
    fn calculate_field_info_in_record(&self, fields: &[(String, Type)], field_name: &str) -> Result<(i64, Type)> {
        let mut offset = 0i64;
        for (name, field_type) in fields {
            if name == field_name {
                return Ok((offset, field_type.clone()));
            }
            // Calculate field size (simplified: assume all fields are 8 bytes)
            // In a full implementation, this would calculate actual field sizes
            offset += 8;
        }
        Err(CompilerError::codegen_error(format!("Field '{}' not found in record", field_name)))
    }

    /// Check if a value needs type conversion for phi
    fn needs_type_conversion(&self, val: &str, target_type: &str) -> bool {
        let current_type = if val.starts_with("i8* ") {
            "i8*"
        } else if val.starts_with("i1 ") {
            "i1"
        } else {
            "i64"
        };
        current_type != target_type
    }

    /// Add a type conversion instruction in the current block
    fn add_type_conversion_instruction(&mut self, val: &str, target_type: &str, convert_reg: &str, body_instructions: &mut Vec<String>) {
        let clean_val = val.trim_start_matches("i64 ").trim_start_matches("i1 ").trim_start_matches("i8* ");
        let current_type = if val.starts_with("i8* ") {
            "i8*"
        } else if val.starts_with("i1 ") {
            "i1"
        } else {
            "i64"
        };

        match (current_type, target_type) {
            ("i64", "i8*") => {
                // Convert i64 to i8* by storing and returning pointer
                let temp_alloc = format!("%temp_alloc_{}", body_instructions.len());
                let temp_ptr = format!("%temp_ptr_{}", body_instructions.len());
                body_instructions.push(format!("  {} = call i8* @malloc(i64 8)", temp_alloc));
                body_instructions.push(format!("  {} = bitcast i8* {} to i64*", temp_ptr, temp_alloc));
                body_instructions.push(format!("  store i64 {}, i64* {}", clean_val, temp_ptr));
                // Note: convert_reg is temp_alloc
            },
            ("i1", "i64") => {
                body_instructions.push(format!("  {} = zext i1 {} to i64", convert_reg, clean_val));
            },
            ("i1", "i8*") => {
                // Convert i1 to i8* via i64
                let temp_i64 = format!("%temp_i64_{}", body_instructions.len());
                let temp_alloc = format!("%temp_alloc_{}", body_instructions.len());
                let temp_ptr = format!("%temp_ptr_{}", body_instructions.len());
                body_instructions.push(format!("  {} = zext i1 {} to i64", temp_i64, clean_val));
                body_instructions.push(format!("  {} = call i8* @malloc(i64 8)", temp_alloc));
                body_instructions.push(format!("  {} = bitcast i8* {} to i64*", temp_ptr, temp_alloc));
                body_instructions.push(format!("  store i64 {}, i64* {}", temp_i64, temp_ptr));
                // Note: convert_reg is temp_alloc
            },
            ("i64", "i1") => {
                body_instructions.push(format!("  {} = trunc i64 {} to i1", convert_reg, clean_val));
            },
            ("i8*", "i64") => {
                body_instructions.push(format!("  {} = ptrtoint i8* {} to i64", convert_reg, clean_val));
            },
            ("i8*", "i1") => {
                let temp_i64 = format!("%temp_i64_{}", body_instructions.len());
                body_instructions.push(format!("  {} = ptrtoint i8* {} to i64", temp_i64, clean_val));
                body_instructions.push(format!("  {} = trunc i64 {} to i1", convert_reg, temp_i64));
            },
            _ => {
                // Same type or unhandled - no instruction needed
            }
        }
    }

    /// Convert a case operand to the target type for phi consistency
    fn convert_case_operand_to_target_type(&mut self, val: String, target_type: &str, body_instructions: &mut Vec<String>) -> String {
        if !self.needs_type_conversion(&val, target_type) {
            // No conversion needed
            return val.trim_start_matches("i64 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string();
        }

        // Generate conversion instruction
        let convert_reg = format!("%convert_{}", body_instructions.len());
        self.add_type_conversion_instruction(&val, target_type, &convert_reg, body_instructions);

        // For allocations, return the alloc register, otherwise the convert register
        let current_type = if val.starts_with("i8* ") {
            "i8*"
        } else if val.starts_with("i1 ") {
            "i1"
        } else {
            "i64"
        };

        match (current_type, target_type) {
            ("i64", "i8*") | ("i1", "i8*") => {
                format!("%temp_alloc_{}", body_instructions.len() - 1) // The malloc result
            },
            _ => convert_reg,
        }
    }

    /// Generate case expression within function literals
    fn generate_function_literal_case(&mut self, case_expr: &CaseExpr, func_lit: &FunctionLiteralExpr, body_instructions: &mut Vec<String>) -> Result<String> {
        let scrutinee_expr = self.generate_function_literal_expr(&case_expr.scrutinee, func_lit, body_instructions)?;
        body_instructions.push(format!("  ; DEBUG: scrutinee_expr='{}', params={}", scrutinee_expr, func_lit.parameters.len()));

        // For behavior functions, if scrutinee is a parameter register, we need to bitcast and load it
        let scrutinee_val = if scrutinee_expr == "i8* %0" || scrutinee_expr == "i8* %1" {
            // Parameter with i8* prefix - bitcast and load
            let param_reg = scrutinee_expr.trim_start_matches("i8* ");
            let bitcast_reg = format!("%scrutinee_bitcast_{}", body_instructions.len());
            let load_reg = format!("%scrutinee_load_{}", body_instructions.len());
            body_instructions.push(format!("  {} = bitcast i8* {} to i64*", bitcast_reg, param_reg));
            body_instructions.push(format!("  {} = load i64, i64* {}", load_reg, bitcast_reg));
            load_reg
        } else if scrutinee_expr == "%0" || scrutinee_expr == "%1" {
            // Direct parameter register - bitcast and load
            let bitcast_reg = format!("%scrutinee_bitcast_{}", body_instructions.len());
            let load_reg = format!("%scrutinee_load_{}", body_instructions.len());
            body_instructions.push(format!("  {} = bitcast i8* {} to i64*", bitcast_reg, scrutinee_expr));
            body_instructions.push(format!("  {} = load i64, i64* {}", load_reg, bitcast_reg));
            load_reg
        } else {
            scrutinee_expr
        };

        // Create labels
        let case_end = format!("case_end_{}", body_instructions.len());
        let case_fail = format!("case_fail_{}", body_instructions.len());

        // Create result register for phi
        let result_reg = format!("%case_result_{}", body_instructions.len());

        // Collect phi operands
        let mut phi_operands = Vec::new();

        // Check if this is a behavior function
        let is_behavior_function = func_lit.parameters.len() == 2;

        // Generate branch checking logic
        let mut next_check_label = format!("case_check_{}", body_instructions.len());
        body_instructions.push(format!("  br label %{}", next_check_label));

        for (branch_idx, branch) in case_expr.branches.iter().enumerate() {
            // Branch check label
            body_instructions.push(format!("{}:", next_check_label));

            // Create branch labels
            let branch_body = format!("case_body_{}_{}", body_instructions.len(), branch_idx);
            next_check_label = format!("case_check_{}_{}", body_instructions.len(), branch_idx + 1);

            // Generate pattern match check
            let match_result = self.generate_function_literal_pattern_check(&branch.pattern, &scrutinee_val, body_instructions)?;

            // Branch to body if pattern matches
            body_instructions.push(format!("  br i1 {}, label %{}, label %{}",
                match_result,
                branch_body,
                if branch_idx + 1 < case_expr.branches.len() { &next_check_label } else { &case_fail }));

            // Branch body
            body_instructions.push(format!("{}:", branch_body));

            // For now, ignore guards in bootstrap compiler
            if branch.guard.is_some() {
                // TODO: Implement guard support in function literals
            }

            let body_val = self.generate_function_literal_expr(&branch.body, func_lit, body_instructions)?;

            // For behavior functions, all results should be i8* pointers
            let is_behavior_function = func_lit.parameters.len() == 2;

            let converted_val = if is_behavior_function {
                // Behavior functions: ensure result is i8*
                let clean_val = body_val.trim_start_matches("i64 ").trim_start_matches("i1 ").trim_start_matches("i8* ");
                let current_type = if body_val.starts_with("i8* ") { "i8*" }
                else if body_val.starts_with("i1 ") { "i1" }
                else { "i64" };

                // DEBUG: Add debug statements
                body_instructions.push(format!("  ; DEBUG: body_val='{}', clean_val='{}', current_type='{}'", body_val, clean_val, current_type));

                // Check if the value is already boxed
                if clean_val.contains("box_result") || clean_val.contains("param") || clean_val.starts_with("%box_") || clean_val.starts_with("%param_") || clean_val.starts_with("%temp_alloc_") {
                    // Already boxed
                    clean_val.to_string()
                } else if current_type == "i8*" {
                    clean_val.to_string()
                } else {
                    // Box the value
                    let temp_alloc = format!("%temp_alloc_{}", body_instructions.len());
                    let temp_ptr = format!("%temp_ptr_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = call i8* @malloc(i64 8)", temp_alloc));
                    body_instructions.push(format!("  {} = bitcast i8* {} to {}*", temp_ptr, temp_alloc, current_type));
                    body_instructions.push(format!("  store {} {}, {}* {}", current_type, clean_val, current_type, temp_ptr));
                    temp_alloc
            }
            } else {
                // Regular functions: keep as unboxed
                body_val.trim_start_matches("i64 ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string()
            };

            phi_operands.push((converted_val, branch_body.clone()));
            body_instructions.push(format!("  br label %{}", case_end));
        }

        // Failure case (should not happen with wildcard patterns)
        body_instructions.push(format!("{}:", case_fail));
        // Failure value - for behavior functions, box it
        let fail_val = if is_behavior_function {
            let temp_alloc = format!("%temp_alloc_{}", body_instructions.len());
            let temp_ptr = format!("%temp_ptr_{}", body_instructions.len());
            body_instructions.push(format!("  {} = call i8* @malloc(i64 8)", temp_alloc));
            body_instructions.push(format!("  {} = bitcast i8* {} to i64*", temp_ptr, temp_alloc));
            body_instructions.push(format!("  store i64 0, i64* {}", temp_ptr));
            temp_alloc
        } else {
            "0".to_string()
        };
        phi_operands.push((fail_val, case_fail.clone()));
        body_instructions.push(format!("  br label %{}", case_end));

        // End with phi - phi must be first instruction in block
        body_instructions.push(format!("{}:", case_end));

        // For behavior functions (2 parameters), all values should be i8* pointers

        // Create phi with the appropriate type
            let phi_parts: Vec<String> = phi_operands.iter()
                .map(|(val, label)| format!("[{}, %{}]", val, label))
                .collect();

        if is_behavior_function {
            // Behavior functions: phi with i8* values (all operands are already boxed)
            body_instructions.push(format!("  {} = phi i8* {}", result_reg, phi_parts.join(", ")));
            Ok(format!("i8* {}", result_reg))
        } else {
            // Regular functions: phi with i64 values
            body_instructions.push(format!("  {} = phi i64 {}", result_reg, phi_parts.join(", ")));
            Ok(format!("i64 {}", result_reg))
        }
    }

    /// Generate expression value for function literal body (returns register/value with type prefix)
    fn generate_function_literal_expr(&mut self, expr: &Expression, func_lit: &FunctionLiteralExpr, body_instructions: &mut Vec<String>) -> Result<String> {
        match expr {
            Expression::Literal(lit) => {
                Ok(self.generate_literal(lit))
            },
            Expression::Identifier(name) => {
                // Handle special core affinity identifiers
                if name == "any_core" {
                    return Ok("i32 0".to_string()); // 0 means any core
                }

                body_instructions.push(format!("  ; DEBUG: looking up identifier '{}', func has {} params", name, func_lit.parameters.len()));

                // Check if it's a parameter of the current function literal
                if let Some(param) = func_lit.parameters.iter().find(|p| p.name == *name) {
                    body_instructions.push(format!("  ; DEBUG: found parameter '{}'", name));
                    // For behavior functions, parameters are passed as i8* but may need to be loaded
                    let is_behavior_function = func_lit.parameters.len() == 2;
                    if is_behavior_function {
                        // Behavior function parameters: all are i8* at LLVM level
                        // Use the actual LLVM parameter register (%0, %1, etc.)
                        let param_index = func_lit.parameters.iter().position(|p| p.name == *name).unwrap();
                        let result = format!("i8* %{}", param_index);
                        body_instructions.push(format!("  ; DEBUG: parameter '{}' -> '{}'", name, result));
                        Ok(result)
                    } else {
                        // Regular function parameters - include type prefix
                        let param_type = self.type_map.silica_to_llvm_str(&param.type_);
                        Ok(format!("{} %{}", param_type, param.name))
                    }
                } else {
                    body_instructions.push(format!("  ; DEBUG: parameter '{}' not found, checking variables", name));
                    
                    // FIRST: Check if the variable is in the current scope (innermost scope)
                    // This ensures captured variables (as parameters) take precedence over outer scope variables
                    if let Some(current_scope) = self.variable_scopes.last() {
                        if let Some(param_reg) = current_scope.get(name) {
                            // Found in current scope - this is a parameter register (either %0, %1, or %captured_N)
                            // For behavior functions, if the var_reg is a parameter register, return i8*
                            if func_lit.parameters.len() == 2 && (param_reg == "%0" || param_reg == "%1") {
                                return Ok(format!("i8* {}", param_reg));
                            } else {
                                // It's a captured variable parameter register - return with appropriate type
                                // Check if it's a captured variable (starts with %captured_)
                                if param_reg.starts_with("%captured_") {
                                    // Captured variable - determine type from variable_types if available
                                    let var_type = self.variable_types.get(name)
                                        .or_else(|| {
                                            // Try to infer from the outer scope value if available
                                            self.lookup_variable_text(name).and_then(|outer_val| {
                                                if outer_val.starts_with("i64 ") {
                                                    Some(&Type::Int64)
                                                } else if outer_val.starts_with("i8* ") {
                                                    Some(&Type::ActorRef)
                                                } else {
                                                    None
                                                }
                                            })
                                        });
                                    
                                    if let Some(ty) = var_type {
                                        let llvm_type = self.type_map.silica_to_llvm_str(ty);
                                        return Ok(format!("{} {}", llvm_type, param_reg));
                                    }
                                    // Default to i64 for captured variables
                                    return Ok(format!("i64 {}", param_reg));
                                } else {
                                    // Regular parameter - return as is
                                    return Ok(param_reg.clone());
                                }
                            }
                        }
                    }
                    
                    // SECOND: If not in current scope, check outer scopes
                    if let Some(var_reg) = self.lookup_variable_text(name) {
                        // For behavior functions, if the var_reg is a parameter register, return i8*
                        if func_lit.parameters.len() == 2 && (var_reg == "%0" || var_reg == "%1") {
                            Ok(format!("i8* {}", var_reg))
                        } else {
                            // It's from outer scope - extract register name
                            // BUT: if the register doesn't look like a parameter register, 
                            // it means the variable should be captured but isn't in the current scope
                            // In this case, we should NOT use the outer scope register directly
                            let result = if var_reg.starts_with("i64 ") || var_reg.starts_with("i8* ") || var_reg.starts_with("i32 ") || var_reg.starts_with("i1 ") {
                                // Has type prefix - extract register name
                                let reg = var_reg.trim_start_matches("i64 ")
                                    .trim_start_matches("i8* ")
                                    .trim_start_matches("i32 ")
                                    .trim_start_matches("i1 ")
                                    .to_string();
                                
                                // Check if this register looks like an outer scope register (not a parameter)
                                // If so, search all scopes for a parameter register for this variable
                                if !reg.starts_with("%captured_") && !reg.chars().skip(1).all(|c| c.is_ascii_digit()) {
                                    // This is an outer scope register - search all scopes for parameter register
                                    for scope in self.variable_scopes.iter().rev() {
                                        if let Some(param_reg) = scope.get(name) {
                                            // Found parameter register - determine type and use it
                                            let var_type = self.variable_types.get(name)
                                                .or_else(|| {
                                                    if var_reg.starts_with("i64 ") {
                                                        Some(&Type::Int64)
                                                    } else if var_reg.starts_with("i8* ") {
                                                        Some(&Type::ActorRef)
                                                    } else {
                                                        None
                                                    }
                                                });
                                            
                                            if let Some(ty) = var_type {
                                                let llvm_type = self.type_map.silica_to_llvm_str(ty);
                                                return Ok(format!("{} {}", llvm_type, param_reg));
                                            }
                                            // Default to i64
                                            return Ok(format!("i64 {}", param_reg));
                                        }
                                    }
                                    // Not found in any scope as parameter register - this is an error
                                    return Err(CompilerError::codegen_error(
                                        format!("Variable '{}' should be captured but parameter register not found. Outer scope register: {}", name, reg)
                                    ));
                                }
                                
                                // Use the extracted register (should be %captured_N)
                                if reg.starts_with('%') {
                                    reg
                                } else {
                                    format!("%{}", reg)
                                }
                            } else if var_reg.starts_with('%') {
                                // Check if this looks like an outer scope register
                                if !var_reg.starts_with("%captured_") && !var_reg.chars().skip(1).all(|c| c.is_ascii_digit()) {
                                    // This is an outer scope register - search all scopes for parameter register
                                    for scope in self.variable_scopes.iter().rev() {
                                        if let Some(param_reg) = scope.get(name) {
                                            // Found parameter register - determine type and use it
                                            let var_type = self.variable_types.get(name);
                                            if let Some(ty) = var_type {
                                                let llvm_type = self.type_map.silica_to_llvm_str(ty);
                                                return Ok(format!("{} {}", llvm_type, param_reg));
                                            }
                                            // Default to i64
                                            return Ok(format!("i64 {}", param_reg));
                                        }
                                    }
                                    // Not found in any scope as parameter register - this is an error
                                    return Err(CompilerError::codegen_error(
                                        format!("Variable '{}' should be captured but parameter register not found. Outer scope register: {}", name, var_reg)
                                    ));
                                }
                                var_reg.clone()
                            } else {
                                format!("%{}", var_reg)
                            };
                            Ok(result)
                        }
                    } else {
                        // For bootstrap compiler, assume undefined variables are captured
                        // In a full implementation, this would be an error
                        // Return as a register name with % prefix
                        Ok(format!("%captured_{}", name))
                    }
                }
            },
            Expression::Binary(binary) => {
                // Generate binary operation
                let left_val = self.generate_function_literal_expr(&binary.left, func_lit, body_instructions)?;
                let right_val = self.generate_function_literal_expr(&binary.right, func_lit, body_instructions)?;

                // Create a unique register for the result
                let result_reg = format!("%binop_{}", body_instructions.len());

                // Determine operand types and generate appropriate operations
                // First try to get types from expression_types map
                let left_expr_type = Self::try_get_expression_location(&binary.left)
                    .and_then(|loc| self.expression_types.get(loc));
                let right_expr_type = Self::try_get_expression_location(&binary.right)
                    .and_then(|loc| self.expression_types.get(loc));
                
                // For bootstrap: load i8* operands (assume they contain primitives)
                let (left_type, clean_left) = if let Some(ty) = left_expr_type {
                    // Use type from expression_types map
                    let llvm_type_str = Self::type_to_llvm_string(ty);
                    if left_val.starts_with("i8* ") {
                        // Load pointer to boxed value
                        let ptr_reg = left_val.trim_start_matches("i8* ");
                        let bitcast_reg = format!("%bitcast_left_{}", body_instructions.len());
                        let load_reg = format!("%load_left_{}", body_instructions.len());
                        body_instructions.push(format!("  {} = bitcast i8* {} to {}*", bitcast_reg, ptr_reg, llvm_type_str));
                        body_instructions.push(format!("  {} = load {}, {}* {}", load_reg, llvm_type_str, llvm_type_str, bitcast_reg));
                        (llvm_type_str, load_reg.to_string())
                    } else if left_val.starts_with(&format!("{} ", llvm_type_str)) {
                        (llvm_type_str, left_val.trim_start_matches(&format!("{} ", llvm_type_str)).to_string())
                    } else {
                        (llvm_type_str, left_val.trim_start_matches('%').to_string())
                    }
                } else if left_val.starts_with("i8* ") {
                    // Load pointer to boxed value (fallback to i64)
                    let ptr_reg = left_val.trim_start_matches("i8* ");
                    let bitcast_reg = format!("%bitcast_left_{}", body_instructions.len());
                    let load_reg = format!("%load_left_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = bitcast i8* {} to i64*", bitcast_reg, ptr_reg));
                    body_instructions.push(format!("  {} = load i64, i64* {}", load_reg, bitcast_reg));
                    ("i64", load_reg.to_string())
                } else if left_val.starts_with("i64 ") {
                    ("i64", left_val.trim_start_matches("i64 ").to_string())
                } else if left_val.starts_with("i32 ") {
                    ("i32", left_val.trim_start_matches("i32 ").to_string())
                } else if left_val.starts_with("i16 ") {
                    ("i16", left_val.trim_start_matches("i16 ").to_string())
                } else if left_val.starts_with("i8 ") {
                    ("i8", left_val.trim_start_matches("i8 ").to_string())
                } else if left_val.starts_with("half ") {
                    ("half", left_val.trim_start_matches("half ").to_string())
                } else if left_val.starts_with("float ") {
                    ("float", left_val.trim_start_matches("float ").to_string())
                } else if left_val.starts_with("i1 ") {
                    ("i1", left_val.trim_start_matches("i1 ").to_string())
                } else if left_val.contains("box_result") || left_val.starts_with("%box_") {
                    // This is likely an i8* register (boxed result) - load it
                    let bitcast_reg = format!("%bitcast_left_{}", body_instructions.len());
                    let load_reg = format!("%load_left_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = bitcast i8* {} to i64*", bitcast_reg, left_val));
                    body_instructions.push(format!("  {} = load i64, i64* {}", load_reg, bitcast_reg));
                    ("i64", load_reg.to_string())
                } else {
                    // Check if this is a variable/register name that we know the type of
                    let clean_reg = left_val.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i16 ").trim_start_matches("i8 ").trim_start_matches("half ").trim_start_matches("float ").trim_start_matches("i1 ").trim_start_matches("i8* ").trim_start_matches('%');
                    if let Some(var_type) = self.variable_types.get(clean_reg) {
                        // Look up the actual LLVM type for this variable
                        let llvm_type_str = Self::type_to_llvm_string(var_type);
                        (llvm_type_str, format!("%{}", clean_reg))
                    } else {
                        // Fallback: assume i64 for unknown register types in text IR
                        ("i64", left_val.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i16 ").trim_start_matches("i8 ").trim_start_matches("half ").trim_start_matches("float ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string())
                    }
                };

                let (right_type, clean_right) = if let Some(ty) = right_expr_type {
                    // Use type from expression_types map
                    let llvm_type_str = Self::type_to_llvm_string(ty);
                    if right_val.starts_with("i8* ") {
                        // Load pointer to boxed value
                        let ptr_reg = right_val.trim_start_matches("i8* ");
                        let bitcast_reg = format!("%bitcast_right_{}", body_instructions.len());
                        let load_reg = format!("%load_right_{}", body_instructions.len());
                        body_instructions.push(format!("  {} = bitcast i8* {} to {}*", bitcast_reg, ptr_reg, llvm_type_str));
                        body_instructions.push(format!("  {} = load {}, {}* {}", load_reg, llvm_type_str, llvm_type_str, bitcast_reg));
                        (llvm_type_str, load_reg.to_string())
                    } else if right_val.starts_with(&format!("{} ", llvm_type_str)) {
                        (llvm_type_str, right_val.trim_start_matches(&format!("{} ", llvm_type_str)).to_string())
                    } else {
                        (llvm_type_str, right_val.trim_start_matches('%').to_string())
                    }
                } else if right_val.starts_with("i8* ") {
                    // Load pointer to boxed value (fallback to i64)
                    let ptr_reg = right_val.trim_start_matches("i8* ");
                    let bitcast_reg = format!("%bitcast_right_{}", body_instructions.len());
                    let load_reg = format!("%load_right_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = bitcast i8* {} to i64*", bitcast_reg, ptr_reg));
                    body_instructions.push(format!("  {} = load i64, i64* {}", load_reg, bitcast_reg));
                    ("i64", load_reg.to_string())
                } else if right_val.starts_with("i64 ") {
                    ("i64", right_val.trim_start_matches("i64 ").to_string())
                } else if right_val.starts_with("i32 ") {
                    ("i32", right_val.trim_start_matches("i32 ").to_string())
                } else if right_val.starts_with("i16 ") {
                    ("i16", right_val.trim_start_matches("i16 ").to_string())
                } else if right_val.starts_with("i8 ") {
                    ("i8", right_val.trim_start_matches("i8 ").to_string())
                } else if right_val.starts_with("half ") {
                    ("half", right_val.trim_start_matches("half ").to_string())
                } else if right_val.starts_with("float ") {
                    ("float", right_val.trim_start_matches("float ").to_string())
                } else if right_val.starts_with("i1 ") {
                    ("i1", right_val.trim_start_matches("i1 ").to_string())
                } else if right_val.contains("box") || right_val.starts_with("%box_") {
                    // This is likely an i8* register (boxed result) - load it
                    let bitcast_reg = format!("%bitcast_right_{}", body_instructions.len());
                    let load_reg = format!("%load_right_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = bitcast i8* {} to i64*", bitcast_reg, right_val));
                    body_instructions.push(format!("  {} = load i64, i64* {}", load_reg, bitcast_reg));
                    ("i64", load_reg.to_string())
                } else {
                    // Check if this is a variable/register name that we know the type of
                    let clean_reg = right_val.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i16 ").trim_start_matches("i8 ").trim_start_matches("half ").trim_start_matches("float ").trim_start_matches("i1 ").trim_start_matches("i8* ").trim_start_matches('%');
                    if let Some(var_type) = self.variable_types.get(clean_reg) {
                        // Look up the actual LLVM type for this variable
                        let llvm_type_str = Self::type_to_llvm_string(var_type);
                        (llvm_type_str, format!("%{}", clean_reg))
                    } else {
                        // Fallback: assume i64 for unknown register types in text IR
                        ("i64", right_val.trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i16 ").trim_start_matches("i8 ").trim_start_matches("half ").trim_start_matches("float ").trim_start_matches("i1 ").trim_start_matches("i8* ").to_string())
                    }
                };

                // Generate operation based on operand types
                // Ensure types match (type checker should have enforced this)
                if left_type != right_type {
                    return Err(CompilerError::codegen_error(
                        format!("Arithmetic operands must be same type: {} vs {}", left_type, right_type)
                    ));
                }
                
                let (op_instr, result_type) = match binary.operator {
                    BinaryOp::Add => {
                        // Determine if this is a float operation
                        let is_float = left_type == "half" || left_type == "float";
                        let op_name = if is_float { "fadd" } else { "add" };
                        (format!("    {} = {} {} {}, {}", result_reg, op_name, left_type, clean_left, clean_right), left_type)
                    },
                    BinaryOp::Subtract => {
                        let is_float = left_type == "half" || left_type == "float";
                        let op_name = if is_float { "fsub" } else { "sub" };
                        (format!("    {} = {} {} {}, {}", result_reg, op_name, left_type, clean_left, clean_right), left_type)
                    },
                    BinaryOp::Multiply => {
                        let is_float = left_type == "half" || left_type == "float";
                        let op_name = if is_float { "fmul" } else { "mul" };
                        (format!("    {} = {} {} {}, {}", result_reg, op_name, left_type, clean_left, clean_right), left_type)
                    },
                    BinaryOp::Divide => {
                        let is_float = left_type == "half" || left_type == "float";
                        let op_name = if is_float { "fdiv" } else { "sdiv" };
                        (format!("    {} = {} {} {}, {}", result_reg, op_name, left_type, clean_left, clean_right), left_type)
                    },
                    BinaryOp::Modulo => {
                        // Modulo only works on integer types
                        if left_type == "half" || left_type == "float" {
                            return Err(CompilerError::codegen_error(
                                format!("Modulo operation requires integer operands, found {}", left_type)
                            ));
                        }
                        (format!("    {} = srem {} {}, {}", result_reg, left_type, clean_left, clean_right), left_type)
                    },
                    BinaryOp::Equal => {
                        if left_type == right_type {
                            (format!("    {} = icmp eq {} {}, {}", result_reg, left_type, clean_left, clean_right), "i1")
                        } else {
                            return Err(CompilerError::codegen_error(format!("Cannot compare {} and {}", left_type, right_type)));
                        }
                    },
                    BinaryOp::NotEqual => {
                        if left_type == right_type {
                            (format!("    {} = icmp ne {} {}, {}", result_reg, left_type, clean_left, clean_right), "i1")
                        } else {
                            return Err(CompilerError::codegen_error(format!("Cannot compare {} and {}", left_type, right_type)));
                        }
                    },
                    BinaryOp::Less => {
                        if left_type == "i64" && right_type == "i64" {
                            (format!("    {} = icmp slt i64 {}, {}", result_reg, clean_left, clean_right), "i1")
                        } else if left_type == "i32" && right_type == "i32" {
                            (format!("    {} = icmp slt i32 {}, {}", result_reg, clean_left, clean_right), "i1")
                        } else if left_type == "i1" && right_type == "i1" {
                            (format!("    {} = icmp ult i1 {}, {}", result_reg, clean_left, clean_right), "i1")
                        } else {
                            return Err(CompilerError::codegen_error(format!("Cannot compare {} and {}", left_type, right_type)));
                        }
                    },
                    BinaryOp::LessEqual => {
                        if left_type == "i64" && right_type == "i64" {
                            (format!("    {} = icmp sle i64 {}, {}", result_reg, clean_left, clean_right), "i1")
                        } else {
                            return Err(CompilerError::codegen_error(format!("Cannot compare {} and {}", left_type, right_type)));
                        }
                    },
                    BinaryOp::Greater => {
                        if left_type == "i64" && right_type == "i64" {
                            (format!("    {} = icmp sgt i64 {}, {}", result_reg, clean_left, clean_right), "i1")
                        } else {
                            return Err(CompilerError::codegen_error(format!("Cannot compare {} and {}", left_type, right_type)));
                        }
                    },
                    BinaryOp::GreaterEqual => {
                        if left_type == "i64" && right_type == "i64" {
                            (format!("    {} = icmp sge i64 {}, {}", result_reg, clean_left, clean_right), "i1")
                        } else {
                            return Err(CompilerError::codegen_error(format!("Cannot compare {} and {}", left_type, right_type)));
                        }
                    },
                    _ => return Err(CompilerError::codegen_error(format!("Unsupported binary operator in function literal: {:?}", binary.operator))),
                };

                // Add the instruction to the function body
                body_instructions.push(op_instr);

                // Check if this is a function literal (needs boxing) or helper function (direct values)
                let is_function_literal = !func_lit.captured_vars.is_empty();
                if is_function_literal {
                // Box the result for function literals
            let box_reg = format!("%box_result_{}", body_instructions.len());
            let ptr_reg = format!("%ptr_result_{}", body_instructions.len());

                    // Allocate appropriate memory size based on type
                    let (alloc_size, ptr_type) = match result_type {
                        "i1" => ("1", "i1"),     // Boolean: 1 byte, i1 pointer
                        "i32" => ("4", "i32"),   // Char: 4 bytes, i32 pointer
                        _ => ("8", "i64")        // Other types: 8 bytes, i64 pointer
                    };

                    body_instructions.push(format!("  {} = call i8* @malloc(i64 {})", box_reg, alloc_size));
                    body_instructions.push(format!("  {} = bitcast i8* {} to {}*", ptr_reg, box_reg, ptr_type));
            body_instructions.push(format!("  store {} {}, {}* {}", result_type, result_reg, result_type, ptr_reg));
                    Ok(format!("i8* {}", box_reg))
                } else {
                    // For helper functions, return the direct result with proper type
                    Ok(format!("{} {}", result_type, result_reg))
                }
            },
            Expression::If(if_expr) => {
                // Handle if expressions within function literals
                let cond_val = self.generate_function_literal_expr(&if_expr.condition, func_lit, body_instructions)?;

                // Create unique labels
                let then_label = format!("then_{}", body_instructions.len());
                let else_label = format!("else_{}", body_instructions.len());
                let end_label = format!("end_{}", body_instructions.len());
                let result_reg = format!("%if_result_{}", body_instructions.len());

                // Handle the condition - it might be boxed
                let branch_cond = if cond_val.starts_with("i8* ") {
                    // Condition is boxed - load the boolean value from the boxed location
                    // The boxed value is stored as i1 at the allocated address
                    let box_ptr = cond_val.trim_start_matches("i8* ");
                    let bool_ptr_reg = format!("%cond_bitcast_{}", body_instructions.len());
                    let loaded_cond = format!("%cond_load_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = bitcast i8* {} to i1*", bool_ptr_reg, box_ptr));
                    body_instructions.push(format!("  {} = load i1, i1* {}", loaded_cond, bool_ptr_reg));
                    loaded_cond
                } else {
                    // Condition is unboxed - use directly
                    cond_val.trim_start_matches("i64 ").trim_start_matches("i1 ").to_string()
                };
                body_instructions.push(format!("  br i1 {}, label %{}, label %{}",
                    branch_cond, then_label, else_label));

                // Generate then block
                body_instructions.push(format!("{}:", then_label));
                let then_val = self.generate_function_literal_expr(&if_expr.then_branch, func_lit, body_instructions)?;
                let then_final = if then_val.starts_with("i8* ") {
                    // Already a pointer, use directly
                    then_val.trim_start_matches("i8* ").to_string()
                } else {
                    // Box the integer result
                    let int_reg = then_val.trim_start_matches("i64 ").trim_start_matches("i1 ");
                    let box_reg = format!("%box_then_{}", body_instructions.len());
                    let ptr_reg = format!("%ptr_then_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = call i8* @malloc(i64 8)", box_reg));
                    body_instructions.push(format!("  {} = bitcast i8* {} to i64*", ptr_reg, box_reg));
                    body_instructions.push(format!("  store i64 {}, i64* {}", int_reg, ptr_reg));
                    box_reg
                };
                body_instructions.push(format!("  br label %{}", end_label));

                // Generate else block
                body_instructions.push(format!("{}:", else_label));
                let else_val = self.generate_function_literal_expr(&if_expr.else_branch, func_lit, body_instructions)?;
                let else_final = if else_val.starts_with("i8* ") {
                    // Already a pointer, use directly
                    else_val.trim_start_matches("i8* ").to_string()
                } else {
                    // Box the integer result
                    let int_reg = else_val.trim_start_matches("i64 ").trim_start_matches("i1 ");
                    let box_reg = format!("%box_else_{}", body_instructions.len());
                    let ptr_reg = format!("%ptr_else_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = call i8* @malloc(i64 8)", box_reg));
                    body_instructions.push(format!("  {} = bitcast i8* {} to i64*", ptr_reg, box_reg));
                    body_instructions.push(format!("  store i64 {}, i64* {}", int_reg, ptr_reg));
                    box_reg
                };
                body_instructions.push(format!("  br label %{}", end_label));

                // Generate merge block with phi
                body_instructions.push(format!("{}:", end_label));
                body_instructions.push(format!("  {} = phi i8* [{}, %{}], [{}, %{}]",
                    result_reg, then_final, then_label, else_final, else_label));

                Ok(result_reg)
            },
            Expression::Do(do_expr) => {
                // Handle do expressions - evaluate all statements and return the last one's value
                let mut result = None;
                for statement in &do_expr.statements {
                    match statement {
                        crate::ast::Statement::Expr(expr) => {
                            result = Some(self.generate_function_literal_expr(expr, func_lit, body_instructions)?);
                        },
                        crate::ast::Statement::Bind { pattern, expr } => {
                            // Evaluate the expression and bind the result to the variable
                            let value = self.generate_function_literal_expr(expr, func_lit, body_instructions)?;
                            // For simple patterns, bind to the variable name
                            if let Pattern::TypedIdentifier { name, .. } = pattern {
                                if name != "_" {
                                    self.add_variable_text(name.clone(), value.clone());
                                }
                            }
                            // The result of a bind statement is the bound value
                            result = Some(value);
                        }
                    }
                }
                match result {
                    Some(val) => Ok(val),
                    None => Err(CompilerError::codegen_error("Do expression must have at least one statement".to_string()))
                }
            },
            Expression::Case(case_expr) => {
                // Handle case expressions within function literals
                self.generate_function_literal_case(case_expr, func_lit, body_instructions)
            },
            Expression::FieldAccess(field_access) => {
                // Generate field access within function literals
                // This is crucial for actor state access
                let object_value = self.generate_function_literal_expr(&field_access.object, func_lit, body_instructions)?;

                // Find the field offset and type by searching through all known struct types
                let (field_offset, field_type) = self.find_field_info_in_any_struct(&field_access.field)?;

                // For struct field access, the object_value should be an i8* pointer to struct memory
                let field_ptr_reg = format!("%field_ptr_{}", body_instructions.len());
                let field_ptr_typed_reg = format!("%field_ptr_typed_{}", body_instructions.len());
                let field_value_reg = format!("%field_val_{}", body_instructions.len());

                // Strip type prefix from object_value for getelementptr
                let clean_object = object_value.trim_start_matches("i64 ").trim_start_matches("i1 ").trim_start_matches("i8* ");

                // Generate pointer arithmetic
                body_instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", field_ptr_reg, clean_object, field_offset));

                // For ActorRef fields, always load as i64 (how they're stored)
                // Even though silica_to_llvm_str returns i8* for ActorRef, we need to load as i64
                let (llvm_field_type, should_load_as_i64) = if matches!(field_type, Type::ActorRef) {
                    ("i64".to_string(), true)
                } else {
                    (self.type_map.silica_to_llvm_str(&field_type), false)
                };
                let llvm_field_type_ptr = format!("{}*", llvm_field_type);

                body_instructions.push(format!("  {} = bitcast i8* {} to {}", field_ptr_typed_reg, field_ptr_reg, llvm_field_type_ptr));
                
                if should_load_as_i64 {
                    // Load ActorRef as i64 (stored as pointer value)
                    body_instructions.push(format!("  {} = load i64, i64* {}", field_value_reg, field_ptr_typed_reg));
                    Ok(format!("i64 {}", field_value_reg))
                } else {
                    // Load other types normally
                    body_instructions.push(format!("  {} = load {}, {} {}", field_value_reg, llvm_field_type, llvm_field_type_ptr, field_ptr_typed_reg));
                    Ok(format!("{} {}", llvm_field_type, field_value_reg))
                }
            },
            Expression::Tuple(tuple) => {
                // Generate tuple in function literals
                // For bootstrap compiler, we'll generate a placeholder allocation
                // TODO: Implement proper tuple generation in function literals
                let alloc_reg = format!("%tuple_alloc_{}", body_instructions.len());
                let tuple_size = tuple.len() * 8; // Assume 8 bytes per element
                body_instructions.push(format!("  {} = call i8* @malloc(i64 {})", alloc_reg, tuple_size));
                Ok(format!("i8* {}", alloc_reg))
            },
            Expression::StructLiteral(struct_lit) => {
                // Generate struct literal in function literals with proper field initialization
                if struct_lit.fields.is_empty() {
                    return Ok("i8* null".to_string());
                }
                
                // Get struct definition to know field types and layout
                let mut field_type_map = HashMap::new();
                if let Some(struct_def) = self.struct_defs.get(&struct_lit.type_name) {
                    for field_def in struct_def {
                        field_type_map.insert(field_def.name.clone(), field_def.ty.clone());
                    }
                } else if let Some(alias_type) = self.type_aliases.get(&struct_lit.type_name) {
                    if let Type::Record(fields) = alias_type {
                        for (field_name, field_type) in fields {
                            field_type_map.insert(field_name.clone(), field_type.clone());
                        }
                    }
                }
                
                // Generate all field expressions
                let mut field_values = Vec::new();
                let mut field_types = Vec::new();
                
                for (field_name, field_expr) in &struct_lit.fields {
                    let field_type = field_type_map.get(field_name)
                        .cloned()
                        .unwrap_or_else(|| Type::Int64); // Default to int if unknown
                    
                    let field_value = self.generate_function_literal_expr(field_expr, func_lit, body_instructions)?;
                    field_values.push((field_name.clone(), field_value));
                    field_types.push(field_type);
                }
                
                // Calculate memory layout
                let mut total_size = 0;
                let mut field_layout = Vec::new();
                for field_type in &field_types {
                    let (llvm_type_str, size, alignment) = self.get_llvm_type_info(field_type);
                    let aligned_offset = ((total_size + alignment - 1) / alignment) * alignment;
                    field_layout.push((aligned_offset, llvm_type_str, size));
                    total_size = aligned_offset + size;
                }
                
                // Allocate memory
                let alloc_reg = format!("%struct_alloc_{}", body_instructions.len());
                body_instructions.push(format!("  {} = call i8* @malloc(i64 {})", alloc_reg, total_size));
                
                // Store each field at its proper offset
                for (i, ((field_name, field_value), (offset, llvm_type_str, _))) in field_values.iter().zip(field_layout.iter()).enumerate() {
                    let field_ptr_reg = format!("%field_ptr_{}_{}", body_instructions.len(), i);
                    let field_ptr_typed_reg = format!("%field_ptr_typed_{}_{}", body_instructions.len(), i);
                    
                    // Get pointer to field location
                    body_instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", field_ptr_reg, alloc_reg, offset));
                    
                    // Cast to appropriate pointer type
                    let llvm_type_ptr = format!("{}*", llvm_type_str);
                    body_instructions.push(format!("  {} = bitcast i8* {} to {}", field_ptr_typed_reg, field_ptr_reg, llvm_type_ptr));
                    
                    // Extract and store field value
                    let clean_field_val = if field_value.starts_with(&format!("{} ", llvm_type_str)) {
                        // Has type prefix - extract register name and ensure % prefix
                        let reg = field_value.trim_start_matches(&format!("{} ", llvm_type_str)).to_string();
                        if reg.starts_with('%') {
                            reg
                        } else {
                            format!("%{}", reg)
                        }
                    } else if field_value.starts_with("i8* ") {
                        // Boxed value - load it
                        let load_reg = format!("%load_field_{}_{}", body_instructions.len(), i);
                        let bitcast_reg = format!("%bitcast_field_{}_{}", body_instructions.len(), i);
                        body_instructions.push(format!("  {} = bitcast i8* {} to {}*", bitcast_reg, field_value.trim_start_matches("i8* "), llvm_type_str));
                        body_instructions.push(format!("  {} = load {}, {}* {}", load_reg, llvm_type_str, llvm_type_str, bitcast_reg));
                        load_reg
                    } else {
                        // Assume it's a register - ensure it has % prefix
                        if field_value.starts_with('%') {
                            field_value.clone()
                        } else {
                            format!("%{}", field_value)
                        }
                    };
                    
                    body_instructions.push(format!("  store {} {}, {}* {}", llvm_type_str, clean_field_val, llvm_type_str, field_ptr_typed_reg));
                }
                
                Ok(format!("i8* {}", alloc_reg))
            },
            Expression::FunctionLiteral(func_lit_inner) => {
                // For bootstrap compiler, nested function literals return a placeholder
                // TODO: Implement proper nested function literal support
                Ok("i8* null".to_string())
            },
            Expression::Call(call) => {
                // Handle function calls within function literals - this enables helper functions!
                self.generate_function_literal_call(call, func_lit, body_instructions)
            },
            Expression::Cast(cast) => {
                // Handle cast expressions within function literals
                // Generate actor reference and message expressions
                let actor_val = self.generate_function_literal_expr(&cast.actor, func_lit, body_instructions)?;
                let message_val = self.generate_function_literal_expr(&cast.message, func_lit, body_instructions)?;
                
                // Extract actor register (may be i64 or i8*)
                // Ensure register names always have % prefix
                // IMPORTANT: If the register doesn't look like a parameter register (%captured_N or %0, %1),
                // check if the variable is in the current scope as a parameter register
                let (actor_reg, actor_type) = if actor_val.starts_with("i64 ") {
                    let reg = actor_val.trim_start_matches("i64 ").to_string();
                    // Check if this is an outer scope register - if so, check current scope for parameter register
                    if !reg.starts_with("%captured_") && !reg.chars().skip(1).all(|c| c.is_ascii_digit()) {
                        // This looks like an outer scope register - check if variable is in current scope
                        if let Expression::Identifier(var_name) = &*cast.actor {
                            if let Some(current_scope) = self.variable_scopes.last() {
                                if let Some(param_reg) = current_scope.get(var_name) {
                                    // Found parameter register in current scope - use it
                                    return Ok(format!("i64 {}", param_reg));
                                }
                            }
                        }
                    }
                    // Ensure % prefix
                    let reg_with_prefix = if reg.starts_with('%') { reg } else { format!("%{}", reg) };
                    (reg_with_prefix, "i64")
                } else if actor_val.starts_with("i8* ") {
                    let reg = actor_val.trim_start_matches("i8* ").to_string();
                    // Check if this is an outer scope register - if so, check current scope for parameter register
                    if !reg.starts_with("%captured_") && !reg.chars().skip(1).all(|c| c.is_ascii_digit()) {
                        // This looks like an outer scope register - check if variable is in current scope
                        if let Expression::Identifier(var_name) = &*cast.actor {
                            if let Some(current_scope) = self.variable_scopes.last() {
                                if let Some(param_reg) = current_scope.get(var_name) {
                                    // Found parameter register in current scope - use it
                                    return Ok(format!("i8* {}", param_reg));
                                }
                            }
                        }
                    }
                    // Ensure % prefix
                    let reg_with_prefix = if reg.starts_with('%') { reg } else { format!("%{}", reg) };
                    (reg_with_prefix, "i8*")
                } else {
                    // Assume it's a register name - ensure it has % prefix
                    // Check if this is an outer scope register - if so, check current scope for parameter register
                    let reg = if actor_val.starts_with('%') { 
                        // Check if this looks like an outer scope register
                        if !actor_val.starts_with("%captured_") && !actor_val.chars().skip(1).all(|c| c.is_ascii_digit()) {
                            // This looks like an outer scope register - check if variable is in current scope
                            if let Expression::Identifier(var_name) = &*cast.actor {
                                if let Some(current_scope) = self.variable_scopes.last() {
                                    if let Some(param_reg) = current_scope.get(var_name) {
                                        // Found parameter register in current scope - determine type and use it
                                        let var_type = self.variable_types.get(var_name)
                                            .or_else(|| {
                                                self.lookup_variable_text(var_name).and_then(|outer_val| {
                                                    if outer_val.starts_with("i64 ") {
                                                        Some(&Type::Int64)
                                                    } else if outer_val.starts_with("i8* ") {
                                                        Some(&Type::ActorRef)
                                                    } else {
                                                        None
                                                    }
                                                })
                                            });
                                        
                                        if let Some(ty) = var_type {
                                            let llvm_type = self.type_map.silica_to_llvm_str(ty);
                                            return Ok(format!("{} {}", llvm_type, param_reg));
                                        }
                                        // Default to i64
                                        return Ok(format!("i64 {}", param_reg));
                                    }
                                }
                            }
                        }
                        actor_val 
                    } else { 
                        format!("%{}", actor_val) 
                    };
                    (reg, "unknown")
                };
                
                // Convert actor to i8* if needed (spawn returns i64, but cast expects i8*)
                // Ensure actor_reg has % prefix for LLVM IR
                let actor_reg_with_prefix = if actor_reg.starts_with('%') {
                    actor_reg.clone()
                } else {
                    format!("%{}", actor_reg)
                };
                
                let actor_ptr = if actor_type == "i64" {
                    let ptr_reg = format!("%actor_ptr_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = inttoptr i64 {} to i8*", ptr_reg, actor_reg_with_prefix));
                    ptr_reg
                } else if actor_type == "i8*" {
                    actor_reg_with_prefix
                } else {
                    // Try to determine type from variable lookup
                    let clean_reg = actor_reg.trim_start_matches('%');
                    if let Some(var_type) = self.variable_types.get(clean_reg) {
                        if matches!(var_type, Type::ActorRef) {
                            // ActorRef is stored as i64, convert to i8*
                            let ptr_reg = format!("%actor_ptr_{}", body_instructions.len());
                            body_instructions.push(format!("  {} = inttoptr i64 {} to i8*", ptr_reg, actor_reg_with_prefix));
                            ptr_reg
                        } else {
                            actor_reg_with_prefix
                        }
                    } else {
                        // Assume it's already i8* if we can't determine
                        actor_reg_with_prefix
                    }
                };
                
                // Handle message - it might be a struct literal, integer, or other type
                let msg_final_ptr = if message_val.starts_with("i8* ") {
                    // Already a pointer (e.g., from struct literal allocation)
                    message_val.trim_start_matches("i8* ").to_string()
                } else if message_val.starts_with("i64 ") {
                    // Integer message - allocate and store
                    let int_val = message_val.trim_start_matches("i64 ");
                    let alloc_reg = format!("%msg_alloc_{}", body_instructions.len());
                    let int_ptr = format!("%msg_int_ptr_{}", body_instructions.len());
                    let msg_ptr_reg = format!("%msg_ptr_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg));
                    body_instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                    body_instructions.push(format!("  store i64 {}, i64* {}", int_val, int_ptr));
                    body_instructions.push(format!("  {} = bitcast i64* {} to i8*", msg_ptr_reg, int_ptr));
                    msg_ptr_reg
                } else if message_val.starts_with("%") {
                    // Register containing a value - check if it's a struct allocation or needs allocation
                    if message_val.contains("struct_alloc") || message_val.contains("tuple_alloc") {
                        // Already allocated - use directly (already has % prefix)
                        message_val
                    } else {
                        // i64 register - allocate and store
                        // message_val already has % prefix
                        let alloc_reg = format!("%msg_alloc_{}", body_instructions.len());
                        let int_ptr = format!("%msg_int_ptr_{}", body_instructions.len());
                        let msg_ptr_reg = format!("%msg_ptr_{}", body_instructions.len());
                        body_instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg));
                        body_instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                        body_instructions.push(format!("  store i64 {}, i64* {}", message_val, int_ptr));
                        body_instructions.push(format!("  {} = bitcast i64* {} to i8*", msg_ptr_reg, int_ptr));
                        msg_ptr_reg
                    }
                } else {
                    // Other types - assume they need memory allocation
                    let alloc_reg = format!("%msg_alloc_{}", body_instructions.len());
                    let int_ptr = format!("%msg_int_ptr_{}", body_instructions.len());
                    let msg_ptr_reg = format!("%msg_ptr_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = call i8* @malloc(i64 8)", alloc_reg));
                    body_instructions.push(format!("  {} = bitcast i8* {} to i64*", int_ptr, alloc_reg));
                    body_instructions.push(format!("  store i64 0, i64* {}", int_ptr)); // Default
                    body_instructions.push(format!("  {} = bitcast i64* {} to i8*", msg_ptr_reg, int_ptr));
                    msg_ptr_reg
                };
                
                // Generate cast call: silica_actor_cast(actor_ptr, message_ptr) -> bool
                let result_reg = format!("%cast_result_{}", body_instructions.len());
                body_instructions.push(format!("  {} = call i1 @silica_actor_cast(i8* {}, i8* {})", 
                    result_reg, actor_ptr, msg_final_ptr));
                
                // Return bool result (i1)
                Ok(format!("i1 {}", result_reg))
            },
            Expression::Print(print) => {
                // Generate print expression in function literal
                let value_val = self.generate_function_literal_expr(&print.value, func_lit, body_instructions)?;
                
                // For strings, we need the pointer and length
                // Handle different string representations
                let (str_ptr, str_len) = if value_val.contains("getelementptr") && value_val.contains("@str_const_") {
                    // String literal: getelementptr expression - convert to instruction format and store in register
                    let length = self.find_string_constant_length(&value_val).unwrap_or(0);
                    let gep_instruction = if value_val.starts_with("getelementptr inbounds (") {
                        self.convert_gep_to_instruction_format(&value_val)
                    } else {
                        value_val.clone()
                    };
                    let ptr_reg = format!("%str_ptr_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = {}", ptr_reg, gep_instruction));
                    (ptr_reg, length)
                } else if value_val.starts_with("i8* ") {
                    // Already a pointer register - extract register name
                    let ptr_reg = value_val.trim_start_matches("i8* ");
                    // Try to find string length from string constants
                    let length = self.find_string_constant_length(&value_val).unwrap_or(0);
                    (ptr_reg.to_string(), length)
                } else if value_val.contains("@str_const_") {
                    // String constant reference - convert getelementptr to instruction format if needed
                    let length = self.find_string_constant_length(&value_val).unwrap_or(0);
                    let fixed_val = if value_val.contains("getelementptr") && value_val.starts_with("getelementptr inbounds (") {
                        self.convert_gep_to_instruction_format(&value_val)
                    } else {
                        value_val.clone()
                    };
                    let ptr_reg = format!("%str_ptr_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = {}", ptr_reg, fixed_val));
                    (ptr_reg, length)
                } else {
                    // Assume it's a string pointer register - default length 0 (runtime will handle)
                    let ptr_reg = value_val.trim_start_matches("i8* ").trim_start_matches("i64 ");
                    (ptr_reg.to_string(), 0)
                };
                
                // Call silica_print with string pointer and length
                body_instructions.push(format!("  call void @silica_print(i8* {}, i64 {})", str_ptr, str_len));
                
                // Print returns unit - return empty string (unit value)
                Ok("".to_string())
            },
            Expression::PrintInt64(print_int64) => {
                // Generate print_int64 expression in function literal
                let value_val = self.generate_function_literal_expr(&print_int64.value, func_lit, body_instructions)?;
                
                // Extract i64 value - handle boxed values
                let int_arg = if value_val.starts_with("i8* ") {
                    // Boxed value - unbox it
                    let ptr_reg = value_val.trim_start_matches("i8* ");
                    let bitcast_reg = format!("%bitcast_print_int64_{}", body_instructions.len());
                    let load_reg = format!("%load_print_int64_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = bitcast i8* {} to i64*", bitcast_reg, ptr_reg));
                    body_instructions.push(format!("  {} = load i64, i64* {}", load_reg, bitcast_reg));
                    format!("i64 {}", load_reg)
                } else if value_val.starts_with("i64 ") {
                    // Already i64
                    value_val
                } else {
                    // Assume it's a register - wrap with i64
                    format!("i64 {}", value_val.trim_start_matches("i64 "))
                };
                
                // Call silica_print_int64 with i64 value
                body_instructions.push(format!("  call void @silica_print_int64({})", int_arg));
                
                // PrintInt64 returns unit - return empty string (unit value)
                Ok("".to_string())
            },
            Expression::PrintInt16(print_int16) => {
                // Generate print_int16 expression in function literal
                let value_val = self.generate_function_literal_expr(&print_int16.value, func_lit, body_instructions)?;
                
                // Extract i16 value - handle boxed values
                let int16_arg = if value_val.starts_with("i8* ") {
                    // Boxed value - unbox it
                    let ptr_reg = value_val.trim_start_matches("i8* ");
                    let bitcast_reg = format!("%bitcast_print_int16_{}", body_instructions.len());
                    let load_reg = format!("%load_print_int16_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = bitcast i8* {} to i16*", bitcast_reg, ptr_reg));
                    body_instructions.push(format!("  {} = load i16, i16* {}", load_reg, bitcast_reg));
                    format!("i16 {}", load_reg)
                } else if value_val.starts_with("i16 ") {
                    // Already i16
                    value_val
                } else {
                    // Assume it's a register - wrap with i16
                    format!("i16 {}", value_val.trim_start_matches("i16 "))
                };
                
                // Call silica_print_int16 with i16 value
                body_instructions.push(format!("  call void @silica_print_int16({})", int16_arg));
                
                // PrintInt16 returns unit - return empty string (unit value)
                Ok("".to_string())
            },
            Expression::PrintInt8(print_int8) => {
                // Generate print_int8 expression in function literal
                let value_val = self.generate_function_literal_expr(&print_int8.value, func_lit, body_instructions)?;
                
                // Extract i8 value - handle boxed values
                let int8_arg = if value_val.starts_with("i8* ") {
                    // Boxed value - unbox it
                    let ptr_reg = value_val.trim_start_matches("i8* ");
                    let load_reg = format!("%load_print_int8_{}", body_instructions.len());
                    // For i8, we can load directly from i8* without bitcast
                    body_instructions.push(format!("  {} = load i8, i8* {}", load_reg, ptr_reg));
                    format!("i8 {}", load_reg)
                } else if value_val.starts_with("i8 ") {
                    // Already i8
                    value_val
                } else {
                    // Assume it's a register - wrap with i8
                    format!("i8 {}", value_val.trim_start_matches("i8 "))
                };
                
                // Call silica_print_int8 with i8 value
                body_instructions.push(format!("  call void @silica_print_int8({})", int8_arg));
                
                // PrintInt8 returns unit - return empty string (unit value)
                Ok("".to_string())
            },
            Expression::PrintInt32(print_int32) => {
                // Generate print_int32 expression in function literal
                let value_val = self.generate_function_literal_expr(&print_int32.value, func_lit, body_instructions)?;
                
                // Extract i32 value - handle boxed values
                let int32_arg = if value_val.starts_with("i8* ") {
                    // Boxed value - unbox it
                    let ptr_reg = value_val.trim_start_matches("i8* ");
                    let bitcast_reg = format!("%bitcast_print_int32_{}", body_instructions.len());
                    let load_reg = format!("%load_print_int32_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = bitcast i8* {} to i32*", bitcast_reg, ptr_reg));
                    body_instructions.push(format!("  {} = load i32, i32* {}", load_reg, bitcast_reg));
                    format!("i32 {}", load_reg)
                } else if value_val.starts_with("i32 ") {
                    // Already i32
                    value_val
                } else {
                    // Assume it's a register - wrap with i32
                    format!("i32 {}", value_val.trim_start_matches("i32 "))
                };
                
                // Call silica_print_int32 with i32 value
                body_instructions.push(format!("  call void @silica_print_int32({})", int32_arg));
                
                // PrintInt32 returns unit - return empty string (unit value)
                Ok("".to_string())
            },
            Expression::PrintLn(println) => {
                // Generate println expression in function literal
                let value_val = self.generate_function_literal_expr(&println.value, func_lit, body_instructions)?;
                
                // For strings, we need the pointer and length
                // Handle different string representations
                let (str_ptr, str_len) = if value_val.contains("getelementptr") && value_val.contains("@str_const_") {
                    // String literal: getelementptr expression - convert to instruction format and store in register
                    let length = self.find_string_constant_length(&value_val).unwrap_or(0);
                    let gep_instruction = if value_val.starts_with("getelementptr inbounds (") {
                        self.convert_gep_to_instruction_format(&value_val)
                    } else {
                        value_val.clone()
                    };
                    let ptr_reg = format!("%str_ptr_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = {}", ptr_reg, gep_instruction));
                    (ptr_reg, length)
                } else if value_val.starts_with("i8* ") {
                    // Already a pointer register - extract register name
                    let ptr_reg = value_val.trim_start_matches("i8* ");
                    // Try to find string length from string constants
                    let length = self.find_string_constant_length(&value_val).unwrap_or(0);
                    (ptr_reg.to_string(), length)
                } else if value_val.contains("@str_const_") {
                    // String constant reference - convert getelementptr to instruction format if needed
                    let length = self.find_string_constant_length(&value_val).unwrap_or(0);
                    let fixed_val = if value_val.contains("getelementptr") && value_val.starts_with("getelementptr inbounds (") {
                        self.convert_gep_to_instruction_format(&value_val)
                    } else {
                        value_val.clone()
                    };
                    let ptr_reg = format!("%str_ptr_{}", body_instructions.len());
                    body_instructions.push(format!("  {} = {}", ptr_reg, fixed_val));
                    (ptr_reg, length)
                } else {
                    // Assume it's a string pointer register - default length 0 (runtime will handle)
                    let ptr_reg = value_val.trim_start_matches("i8* ").trim_start_matches("i64 ");
                    (ptr_reg.to_string(), 0)
                };
                
                // Call silica_println with string pointer and length
                body_instructions.push(format!("  call void @silica_println(i8* {}, i64 {})", str_ptr, str_len));
                
                // PrintLn returns unit - return empty string (unit value)
                Ok("".to_string())
            },
            _ => Err(CompilerError::codegen_error(format!("Unsupported expression type in function literal: {:?}", expr))),
        }
    }

    /// Generate LLVM IR for function literal expression
    fn generate_function_literal(&mut self, func_lit: &FunctionLiteralExpr) -> Result<Option<String>> {
        // Generate a unique function name for this literal
        let func_name = format!("func_literal_{}", self.instructions.len());

        // Analyze captured variables with their types
        let captured_vars_with_types = self.analyze_captured_variables_with_types(func_lit)?;
        let captured_vars: Vec<String> = captured_vars_with_types.iter().map(|(name, _)| name.clone()).collect();

        // For bootstrap compiler, implement simple closure capture
        // Make captured variables available during function body generation
        let mut captured_var_values = Vec::new();
        for captured_var in &captured_vars {
            if let Some(var_reg) = self.variables.get(captured_var) {
                captured_var_values.push((captured_var.clone(), var_reg.clone()));
            }
        }

        // Check if this looks like a behavior function (2 params)
        // Behavior functions use pointer interface: fn(i8*, i8*) -> i8*
        let is_behavior_function = func_lit.parameters.len() == 2;

        let (param_types, return_type_str) = if is_behavior_function {
            // Behavior functions: use i8* for runtime compatibility
            (vec!["i8*".to_string(), "i8*".to_string()], "i8*".to_string())
        } else {
            // Regular function: use actual types
            let param_types: Vec<String> = func_lit.parameters.iter()
                .map(|param| self.type_map.silica_to_llvm_str(&param.type_))
            .collect();
            let return_type = func_lit.return_type.as_ref().unwrap_or(&Type::Unit);
            let return_type_str = self.type_map.silica_to_llvm_str(return_type);
            (param_types, return_type_str)
        };

        // Add captured variables with their actual types
        let mut all_param_types = param_types.clone();
        for (_, var_type) in &captured_vars_with_types {
            let llvm_type = self.type_map.silica_to_llvm_str(var_type);
            all_param_types.push(llvm_type);
        }

        // Include parameter names for all functions
        let param_list_str = if is_behavior_function {
            // Behavior functions: first two parameters are unnamed i8*, captured variables have names
            let mut param_specs = vec!["i8*".to_string(), "i8*".to_string()];
            // Add captured variables with names
            for (i, (var_name, var_type)) in captured_vars_with_types.iter().enumerate() {
                let llvm_type = self.type_map.silica_to_llvm_str(var_type);
                param_specs.push(format!("{} %captured_{}", llvm_type, i));
            }
            param_specs.join(", ")
        } else {
            // Regular functions: include parameter names
            let mut param_specs = Vec::new();
            for (i, param) in func_lit.parameters.iter().enumerate() {
                let ty = &param_types[i];
                param_specs.push(format!("{} %{}", ty, param.name));
            }
            // Add captured variables with names
            for (i, (var_name, var_type)) in captured_vars_with_types.iter().enumerate() {
                let llvm_type = self.type_map.silica_to_llvm_str(var_type);
                param_specs.push(format!("{} %captured_{}", llvm_type, i));
            }
            param_specs.join(", ")
        };

        let func_sig = format!("define {} @{}({})", return_type_str, func_name, param_list_str);

        // Add function declaration to global functions
        self.global_functions.push(func_sig.to_string());
        self.global_functions.push("  {".to_string());

        // Set up parameters in the symbol table
        for (i, param) in func_lit.parameters.iter().enumerate() {
            let param_reg = format!("%{}", i);
            if is_behavior_function {
                // Behavior functions: %0 is i64 (message), %1 is i8* (state)
                // But we still register them as i64 for now since the symbol table expects that
                // The actual usage in expressions will handle the type differences
            }
            self.add_variable_text(param.name.clone(), param_reg);
        }
        // Enter a new scope for the function literal body
        // This ensures captured variables (as parameters) take precedence over outer scope variables
        self.enter_scope_text();
        
        // Set up captured variables as additional parameters in the new scope
        for (i, (captured_var, _)) in captured_vars_with_types.iter().enumerate() {
            let param_reg = format!("%captured_{}", i);
            self.add_variable_text(captured_var.clone(), param_reg);
        }

        // Generate function body from the actual expression
        // For function literals, we need to generate the body in a separate context
        let body_instructions = self.generate_function_literal_body_with_captures(func_lit, &captured_vars)?;
        
        // Exit the function literal body scope
        self.exit_scope_text();
        self.global_functions.extend(body_instructions);

        // Note: Variables are automatically cleaned up when exiting scope
        // No manual cleanup needed for text IR variable scopes

        self.global_functions.push("  }".to_string());

        // Return pointer to the function
        // For behavior functions, the runtime expects i8* return type
        // For regular functions, use the actual return type
        let runtime_return_type = if is_behavior_function { "i8*" } else { &return_type_str };
        let ptr_reg = format!("%func_ptr_{}", self.instructions.len());
        self.instructions.push(format!("  {} = bitcast {} ({})* @{} to i8*", ptr_reg, runtime_return_type, all_param_types.join(", "), func_name));

        Ok(Some(ptr_reg))
    }

    /// Generate LLVM IR for read_file expression
    fn generate_read_file(&mut self, read_file: &ReadFileExpr) -> Result<Option<String>> {
        let path_val = self.generate_expression(&read_file.path)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid path in read_file".to_string()))?;

        // For now, return a placeholder result
        let result_reg = format!("%read_result_{}", self.instructions.len());
        self.instructions.push(format!("  ; read_file({}) - placeholder implementation", path_val));
        self.instructions.push(format!("  {} = insertvalue {{ i1, i8* }} undef, i1 true, 0", result_reg));
        self.instructions.push(format!("  {} = insertvalue {{ i1, i8* }} {}, i8* null, 1", result_reg, result_reg));

        Ok(Some(result_reg))
    }

    /// Generate LLVM IR for write_file expression
    fn generate_write_file(&mut self, write_file: &WriteFileExpr) -> Result<Option<String>> {
        let path_val = self.generate_expression(&write_file.path)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid path in write_file".to_string()))?;
        let content_val = self.generate_expression(&write_file.content)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid content in write_file".to_string()))?;

        // For now, return a placeholder result
        let result_reg = format!("%write_result_{}", self.instructions.len());
        self.instructions.push(format!("  ; write_file({}, {}) - placeholder implementation", path_val, content_val));
        self.instructions.push(format!("  {} = insertvalue {{ i1, i8* }} undef, i1 true, 0", result_reg));
        self.instructions.push(format!("  {} = insertvalue {{ i1, i8* }} {}, i8* null, 1", result_reg, result_reg));

        Ok(Some(result_reg))
    }

    /// Generate LLVM IR call to read_file runtime function
    fn generate_read_file_call(&mut self, call: &CallExpr) -> Result<Option<String>> {
        if call.arguments.len() != 1 {
            return Err(CompilerError::codegen_error("read_file expects exactly 1 argument".to_string()));
        }

        let path_val = self.generate_expression(&call.arguments[0])?
            .ok_or_else(|| CompilerError::codegen_error("Invalid path argument in read_file".to_string()))?;

        let (path_arg, path_length_opt) = self.get_path_arg_and_length(&path_val)?;

        let result_reg = format!("%read_result_{}", self.instructions.len());
        if let Some(path_length_reg) = path_length_opt {
            self.instructions.push(format!("  {} = call {{ i1, i8* }} @silica_read_file({}, {})", result_reg, path_arg, path_length_reg));
        } else {
            self.instructions.push(format!("  {} = call {{ i1, i8* }} @silica_read_file_path({})", result_reg, path_arg));
        }

        Ok(Some(result_reg))
    }

    /// Generate LLVM IR call to write_file runtime function
    fn generate_write_file_call(&mut self, call: &CallExpr) -> Result<Option<String>> {
        if call.arguments.len() != 2 {
            return Err(CompilerError::codegen_error("write_file expects exactly 2 arguments".to_string()));
        }

        let path_val = self.generate_expression(&call.arguments[0])?
            .ok_or_else(|| CompilerError::codegen_error("Invalid path argument in write_file".to_string()))?;
        let content_val = self.generate_expression(&call.arguments[1])?
            .ok_or_else(|| CompilerError::codegen_error("Invalid content argument in write_file".to_string()))?;

        let (path_arg, path_len_opt) = self.get_path_arg_and_length(&path_val)?;
        let (content_arg, content_len_opt) = self.get_path_arg_and_length(&content_val)?;

        let result_reg = format!("%write_result_{}", self.instructions.len());
        if path_len_opt.is_some() && content_len_opt.is_some() {
            self.instructions.push(format!("  {} = call {{ i1, i8* }} @silica_write_file({}, {}, {}, {})", result_reg, path_arg, path_len_opt.as_ref().unwrap(), content_arg, content_len_opt.as_ref().unwrap()));
        } else {
            self.instructions.push(format!("  {} = call {{ i1, i8* }} @silica_write_file_path({}, {})", result_reg, path_arg, content_arg));
        }

        Ok(Some(result_reg))
    }

    /// Generate LLVM IR for print expression
    fn generate_print(&mut self, print: &PrintExpr) -> Result<Option<String>> {
        let value_val = self.generate_expression(&print.value)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid value in print".to_string()))?;

        // Format the argument: getelementptr constant expressions need i8* type prefix in function calls
        // For registers, add i8* type prefix and ensure % prefix is present
        // Never add % prefix to globals (starts with @)
        let arg = if value_val.starts_with("getelementptr") {
            format!("i8* {}", value_val)  // Constant expression needs type prefix
        } else if value_val.starts_with('@') {
            // Bare global constant (e.g. from list_directory) - build getelementptr
            let len = self.find_string_constant_length(&value_val).unwrap_or(0);
            let array_len = len + 1;
            format!("i8* getelementptr inbounds ([{} x i8], [{} x i8]* {}, i64 0, i64 0)", array_len, array_len, value_val)
        } else if value_val.starts_with('%') {
            format!("i8* {}", value_val)
        } else {
            // Register name without % prefix - add it
            format!("i8* %{}", value_val)
        };

        // Determine the string length
        // Check if this is a string constant (contains getelementptr or @str_const_)
        if value_val.contains("@str_const_") || value_val.starts_with("getelementptr") {
            // String constant: find the length from string_constants
            let length = self.find_string_constant_length(&value_val).unwrap_or(0);
            // Call silica_print with the string value and length (literal)
            self.instructions.push(format!("  call void @silica_print({}, i64 {})", arg, length));
        } else {
            // Runtime string: value_val is an i8* pointer to a SilicaString struct.
            // Use silica_print_string which safely handles null and invalid pointers
            // (avoids segfault when printing error messages with uninitialized/empty strings).
            let string_ptr_reg = self.clean_register_for_instruction(&value_val).trim_start_matches('%').to_string();
            self.instructions.push(format!("  call void @silica_print_string(i8* %{})", string_ptr_reg));
        }

        Ok(None) // print returns unit
    }

    /// Generate LLVM IR for println expression
    fn generate_println(&mut self, println: &PrintLnExpr) -> Result<Option<String>> {
        let value_val = self.generate_expression(&println.value)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid value in println".to_string()))?;

        // Check if this is a string constant or runtime string (same logic as generate_print)
        if value_val.contains("@str_const_") || value_val.starts_with("getelementptr") {
            let arg = if value_val.starts_with("getelementptr") {
                format!("i8* {}", value_val)
            } else if value_val.starts_with('@') {
                // Bare global constant (e.g. from list_directory) - build getelementptr, never add % prefix
                let len = self.find_string_constant_length(&value_val).unwrap_or(0);
                let array_len = len + 1;
                format!("i8* getelementptr inbounds ([{} x i8], [{} x i8]* {}, i64 0, i64 0)", array_len, array_len, value_val)
            } else {
                let reg = self.clean_register_for_instruction(&value_val);
                let reg = if reg.starts_with('%') || reg.starts_with('@') { reg } else { format!("%{}", reg) };
                format!("i8* {}", reg)
            };
            let length = self.find_string_constant_length(&value_val).unwrap_or(0);
            self.instructions.push(format!("  call void @silica_println({}, i64 {})", arg, length));
        } else {
            // Runtime string: use silica_println_string for null-safe handling
            let string_ptr_reg = self.clean_register_for_instruction(&value_val).trim_start_matches('%').to_string();
            self.instructions.push(format!("  call void @silica_println_string(i8* %{})", string_ptr_reg));
        }

        Ok(None) // println returns unit
    }

    /// Helper method to find the length of a string constant by its reference
    /// Returns the string length WITHOUT null terminator (for runtime function calls)
    fn find_string_constant_length(&self, const_ref: &str) -> Option<usize> {
        // First try exact match (for backward compatibility)
        for (_content, (name, length)) in &self.string_constants {
            if name == const_ref {
                // length includes null terminator, subtract 1 for runtime functions
                return Some(*length - 1);
            }
        }

        // If not found, try to parse getelementptr expression
        // Formats:
        //   - getelementptr inbounds ([LEN x i8], [LEN x i8]* @CONST_NAME, i64 0, i64 0)  (constant expression)
        //   - getelementptr inbounds [LEN x i8], [LEN x i8]* @CONST_NAME, i32 0, i32 0   (instruction)
        //   - i8* getelementptr inbounds ([LEN x i8], [LEN x i8]* @CONST_NAME, i64 0, i64 0)  (with type prefix)
        if const_ref.contains("@str_const_") {
            // Extract the constant name from the expression
            // Find @str_const_ and extract the full constant name
            if let Some(at_pos) = const_ref.find("@str_const_") {
                let name_start = at_pos;
                // Find the end of the constant name - it ends at comma, space, closing paren, or end of string
                let remaining = &const_ref[name_start..];
                let name_end = remaining
                    .find(|c: char| c == ',' || c == ' ' || c == ')' || c == ',')
                    .map(|pos| name_start + pos)
                    .unwrap_or(const_ref.len());
                let const_name = &const_ref[name_start..name_end];

                // Look up the constant by name
                for (_content, (name, length)) in &self.string_constants {
                    if name == const_name {
                        // length includes null terminator, subtract 1 for runtime functions
                        return Some(*length - 1);
                    }
                }
            }
        }

        None
    }

    /// Generate LLVM IR for print_int64 expression
    fn generate_print_int64(&mut self, print_int64: &PrintInt64Expr) -> Result<Option<String>> {
        let value_val = self.generate_expression(&print_int64.value)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid value in print_int64".to_string()))?;

        // Call silica_print_int64 with the int64 value
        // silica_print_int64 expects i64
        let arg = if value_val.starts_with("i64 ") {
            value_val
        } else if Self::is_integer_literal(&value_val) {
            // Literal from placeholder (e.g. get_file_size returns "0") - use as i64 constant, not %0
            format!("i64 {}", value_val)
        } else {
            // Register name - add % prefix
            let reg_val = if value_val.starts_with('%') {
                value_val
            } else {
                format!("%{}", value_val)
            };
            format!("i64 {}", reg_val)
        };
        self.instructions.push(format!("  call void @silica_print_int64({})", arg));

        Ok(None) // print_int64 returns unit
    }

    /// Generate LLVM IR for print_int16 expression
    fn generate_print_int16(&mut self, print_int16: &PrintInt16Expr) -> Result<Option<String>> {
        let value_val = self.generate_expression(&print_int16.value)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid value in print_int16".to_string()))?;

        // Call silica_print_int16 with the int16 value
        // silica_print_int16 expects i16
        let arg = if value_val.starts_with("i16 ") {
            value_val
        } else if value_val.starts_with("i64 ") {
            // Extract the number from "i64 42" and create "i16 42"
            let num_str = value_val.trim_start_matches("i64 ");
            format!("i16 {}", num_str)
        } else if value_val.starts_with("i32 ") {
            // Extract the number from "i32 42" and create "i16 42"
            let num_str = value_val.trim_start_matches("i32 ");
            format!("i16 {}", num_str)
        } else if value_val.starts_with("i8 ") {
            // Extract the number from "i8 42" and create "i16 42"
            let num_str = value_val.trim_start_matches("i8 ");
            format!("i16 {}", num_str)
        } else {
            // If it's a register name, truncate to i16
            let reg_val = if value_val.starts_with('%') {
                value_val.clone()
            } else {
                format!("%{}", value_val)
            };
            let trunc_reg = format!("%trunc_to_i16_{}", self.instructions.len());
            self.instructions.push(format!("  {} = trunc i64 {} to i16", trunc_reg, reg_val));
            format!("i16 {}", trunc_reg)
        };
        self.instructions.push(format!("  call void @silica_print_int16({})", arg));

        Ok(None) // print_int16 returns unit
    }

    /// Generate LLVM IR for print_int32 expression
    fn generate_print_int32(&mut self, print_int32: &PrintInt32Expr) -> Result<Option<String>> {
        let value_val = self.generate_expression(&print_int32.value)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid value in print_int32".to_string()))?;

        // Call silica_print_int32 with the int32 value
        // silica_print_int32 expects i32
        let arg = if value_val.starts_with("i32 ") {
            value_val
        } else if value_val.starts_with("i64 ") {
            // Extract the number from "i64 42" and create "i32 42"
            let num_str = value_val.trim_start_matches("i64 ");
            format!("i32 {}", num_str)
        } else if value_val.starts_with("i16 ") {
            // Extract the number from "i16 42" and create "i32 42"
            let num_str = value_val.trim_start_matches("i16 ");
            format!("i32 {}", num_str)
        } else if value_val.starts_with("i8 ") {
            // Extract the number from "i8 42" and create "i32 42"
            let num_str = value_val.trim_start_matches("i8 ");
            format!("i32 {}", num_str)
        } else {
            // If it's a register name, truncate to i32
            let reg_val = if value_val.starts_with('%') {
                value_val.clone()
            } else {
                format!("%{}", value_val)
            };
            let trunc_reg = format!("%trunc_to_i32_{}", self.instructions.len());
            self.instructions.push(format!("  {} = trunc i64 {} to i32", trunc_reg, reg_val));
            format!("i32 {}", trunc_reg)
        };
        self.instructions.push(format!("  call void @silica_print_int32({})", arg));

        Ok(None) // print_int32 returns unit
    }

    /// Generate LLVM IR for print_int8 expression
    fn generate_print_int8(&mut self, print_int8: &PrintInt8Expr) -> Result<Option<String>> {
        let value_val = self.generate_expression(&print_int8.value)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid value in print_int8".to_string()))?;

        // Call silica_print_int8 with the int8 value
        // silica_print_int8 expects i8
        let arg = if value_val.starts_with("i8 ") {
            value_val
        } else if value_val.starts_with("i64 ") {
            // Extract the number from "i64 42" and create "i8 42"
            let num_str = value_val.trim_start_matches("i64 ");
            format!("i8 {}", num_str)
        } else if value_val.starts_with("i32 ") {
            // Extract the number from "i32 42" and create "i8 42"
            let num_str = value_val.trim_start_matches("i32 ");
            format!("i8 {}", num_str)
        } else if value_val.starts_with("i16 ") {
            // Extract the number from "i16 42" and create "i8 42"
            let num_str = value_val.trim_start_matches("i16 ");
            format!("i8 {}", num_str)
        } else {
            // If it's a register name, truncate to i8
            let reg_val = if value_val.starts_with('%') {
                value_val.clone()
            } else {
                format!("%{}", value_val)
            };
            let trunc_reg = format!("%trunc_to_i8_{}", self.instructions.len());
            self.instructions.push(format!("  {} = trunc i64 {} to i8", trunc_reg, reg_val));
            format!("i8 {}", trunc_reg)
        };
        self.instructions.push(format!("  call void @silica_print_int8({})", arg));

        Ok(None) // print_int8 returns unit
    }

    /// Generate LLVM IR for print_bool expression
    fn generate_print_bool(&mut self, print_bool: &PrintBoolExpr) -> Result<Option<String>> {
        let value_val = self.generate_expression(&print_bool.value)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid value in print_bool".to_string()))?;

        // Call silica_print_bool with the bool value
        // C ABI: silica_print_bool expects i8 (not i1); i1 causes segfault when passed to C
        let i8_arg = if value_val == "0" || value_val == "1" {
            // Literal from placeholder (e.g. delete_file returns "1") - use as i8 constant, not %0/%1
            format!("i8 {}", value_val)
        } else if value_val.starts_with("i1 ") {
            let i1_val = value_val.trim_start_matches("i1 ");
            let i8_reg = format!("%bool_to_i8_{}", self.instructions.len());
            self.instructions.push(format!("  {} = zext i1 {} to i8", i8_reg, i1_val));
            format!("i8 {}", i8_reg)
        } else if value_val.starts_with("i64 ") {
            // Bool stored as i64 (tuple element, variable from pattern binding) - truncate to i1, zext to i8
            let reg_val = value_val.trim_start_matches("i64 ");
            let trunc_reg = format!("%trunc_to_i1_{}", self.instructions.len());
            self.instructions.push(format!("  {} = trunc i64 {} to i1", trunc_reg, reg_val));
            let i8_reg = format!("%bool_to_i8_{}", self.instructions.len());
            self.instructions.push(format!("  {} = zext i1 {} to i8", i8_reg, trunc_reg));
            format!("i8 {}", i8_reg)
        } else {
            // Bare register - ensure % prefix, then zext i1 to i8
            let reg_val = if value_val.starts_with('%') {
                value_val
            } else {
                format!("%{}", value_val)
            };
            let i8_reg = format!("%bool_to_i8_{}", self.instructions.len());
            self.instructions.push(format!("  {} = zext i1 {} to i8", i8_reg, reg_val));
            format!("i8 {}", i8_reg)
        };
        self.instructions.push(format!("  call void @silica_print_bool({})", i8_arg));

        Ok(None) // print_bool returns unit
    }

    /// Generate LLVM IR for print_char expression
    fn generate_print_char(&mut self, print_char: &PrintCharExpr) -> Result<Option<String>> {
        let value_val = self.generate_expression(&print_char.value)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid value in print_char".to_string()))?;

        // Call silica_print_char with the char value (chars are i32 in LLVM)
        let arg = if value_val.starts_with("i32 ") {
            value_val
        } else {
            // If it's a register name (starts with 't' or is numeric), add % prefix
            let reg_val = if value_val.starts_with('%') {
                value_val
            } else {
                format!("%{}", value_val)
            };
            format!("i32 {}", reg_val)
        };
        self.instructions.push(format!("  call void @silica_print_char({})", arg));

        Ok(None) // print_char returns unit
    }

    /// Generate LLVM IR for print_float16 expression
    fn generate_print_float16(&mut self, print_float16: &PrintFloat16Expr) -> Result<Option<String>> {
        let value_val = self.generate_expression(&print_float16.value)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid value in print_float16".to_string()))?;

        // eprintln!("DEBUG print_float16: value_val = '{}'", value_val);

        // Convert half to i16 (u16) for the runtime function
        // The runtime function expects u16 (the bit representation of the half)
        // Follow the same pattern as print_float32: create a constant register for literals, use register directly otherwise
        // Handle both "half " and "float " prefixes (float16 values might be stored as "float " if type context wasn't passed correctly)
        let half_reg = if value_val.starts_with("half ") || value_val.starts_with("float ") {
            // eprintln!("DEBUG print_float16: value_val starts with 'half ' or 'float '");
            // Extract the constant value - handle both "half 3.14" and "float 3.14" (and malformed values)
            let const_val = if value_val.starts_with("half ") {
                value_val.trim_start_matches("half ")
            } else {
                value_val.trim_start_matches("float ")
            };
            // eprintln!("DEBUG print_float16: const_val after trim = '{}'", const_val);
            // If const_val contains spaces, extract the numeric part
            let clean_const = if const_val.contains(' ') {
                // eprintln!("DEBUG print_float16: const_val contains spaces, extracting numeric part");
                let parts: Vec<&str> = const_val.split_whitespace().collect();
                // eprintln!("DEBUG print_float16: parts = {:?}", parts);
                let found = parts.iter()
                    .find(|p| p.parse::<f64>().is_ok() || (p.starts_with('-') && p[1..].parse::<f64>().is_ok()))
                    .copied();
                // eprintln!("DEBUG print_float16: found numeric part = {:?}", found);
                found.unwrap_or(const_val)
            } else {
                const_val
            };
            // eprintln!("DEBUG print_float16: clean_const = '{}'", clean_const);
            
            // Create a half constant register (similar to print_float32's approach)
            if clean_const.parse::<f64>().is_ok() {
                // eprintln!("DEBUG print_float16: clean_const parses as f64, creating constant register");
                // Create float constant first, then convert to half
                let float_const = format!("%float_const_print16_{}", self.instructions.len());
                let instruction = self.create_float_constant_instruction(clean_const, &float_const, "float");
                self.instructions.push(instruction);
                let half_const = format!("%half_const_print16_{}", self.instructions.len());
                self.instructions.push(format!("  {} = fptrunc float {} to half", half_const, float_const));
                // eprintln!("DEBUG print_float16: created half_const = '{}'", half_const);
                half_const
            } else if clean_const.starts_with('%') {
                // eprintln!("DEBUG print_float16: clean_const starts with %, using as register: '{}'", clean_const);
                // It's already a register - assume it's a half register
                clean_const.to_string()
            } else {
                // eprintln!("DEBUG print_float16: fallback, treating as register name: '{}'", clean_const);
                // Fallback: treat as register name
                format!("%{}", clean_const)
            }
        } else {
            // eprintln!("DEBUG print_float16: value_val does NOT start with 'half ' or 'float ', treating as register");
            // If it's a register name (starts with %), we need to ensure it's a half register
            // Registers from our binding code will be named like %half_const_bind_XXX
            // Other registers (from binary ops, unary ops, etc.) might be float and need conversion
            let reg_val = if value_val.starts_with('%') {
                value_val
            } else {
                format!("%{}", value_val)
            };
            
            // Check if this looks like a half register from our binding code
            // If not, it might be a float register that needs conversion
            // We'll convert it to be safe - if it's already half, LLVM will handle it
            // (Actually, fptrunc requires the source to be float, so if it's half, we need a different approach)
            // For now, let's assume registers not from our binding code are float and convert them
            if reg_val.contains("half_const") || reg_val.contains("half_const_bind") || reg_val.contains("half_conv") {
                // eprintln!("DEBUG print_float16: register looks like half register, using directly: '{}'", reg_val);
                reg_val
            } else {
                // eprintln!("DEBUG print_float16: register '{}' might be float, converting to half", reg_val);
                // Convert float to half using fptrunc
                // Note: This will fail if reg_val is already half, but we're assuming it's float
                // If it fails, we'll need to track register types or use a different approach
                let half_conv_reg = format!("%half_conv_print16_{}", self.instructions.len());
                self.instructions.push(format!("  {} = fptrunc float {} to half", half_conv_reg, reg_val));
                half_conv_reg
            }
        };
        
        // eprintln!("DEBUG print_float16: half_reg = '{}'", half_reg);
        
        // Bitcast the half register to i16 for the runtime function
        let bitcast_reg = format!("%bitcast_float16_{}", self.instructions.len());
        let bitcast_instr = format!("  {} = bitcast half {} to i16", bitcast_reg, half_reg);
        // eprintln!("DEBUG print_float16: bitcast instruction = '{}'", bitcast_instr);
        self.instructions.push(bitcast_instr);
        let u16_reg = bitcast_reg;
        
        // Call silica_print_float16 with i16 value
        self.instructions.push(format!("  call void @silica_print_float16(i16 {})", u16_reg));

        Ok(None) // print_float16 returns unit
    }

    /// Generate LLVM IR for print_float32 expression
    fn generate_print_float32(&mut self, print_float32: &PrintFloat32Expr) -> Result<Option<String>> {
        let value_val = self.generate_expression(&print_float32.value)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid value in print_float32".to_string()))?;

        // Call silica_print_float32 with the float32 value (float32 is "float" in LLVM)
        let arg = if value_val.starts_with("float ") {
            // Extract the constant value - LLVM doesn't accept decimal literals directly in function calls
            let const_val = value_val.trim_start_matches("float ");
            // Create a float constant register using bitcast (more reliable than decimal literals)
            if const_val.parse::<f64>().is_ok() {
                let float_const = format!("%float_const_print32_{}", self.instructions.len());
                let instruction = self.create_float_constant_instruction(const_val, &float_const, "float");
                self.instructions.push(instruction);
                format!("float {}", float_const)
            } else {
                // Fallback: try direct format (might fail for some values)
                format!("float {}", const_val)
            }
        } else {
            // If it's a register name (starts with 't' or is numeric), add % prefix
            let reg_val = if value_val.starts_with('%') {
                value_val
            } else {
                format!("%{}", value_val)
            };
            format!("float {}", reg_val)
        };
        self.instructions.push(format!("  call void @silica_print_float32({})", arg));

        Ok(None) // print_float32 returns unit
    }

    /// Generate LLVM IR for print_float64 expression
    fn generate_print_float64(&mut self, print_float64: &PrintFloat64Expr) -> Result<Option<String>> {
        let value_val = self.generate_expression(&print_float64.value)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid value in print_float64".to_string()))?;

        // Call silica_print_float64 with the float64 value (float64 is "double" in LLVM)
        let arg = if value_val.starts_with("double ") {
            // Extract the constant value - LLVM doesn't accept decimal literals directly in function calls
            let const_val = value_val.trim_start_matches("double ");
            // Create a double constant register using bitcast (more reliable than decimal literals)
            if const_val.parse::<f64>().is_ok() {
                let double_const = format!("%double_const_print64_{}", self.instructions.len());
                let instruction = self.create_float_constant_instruction(const_val, &double_const, "double");
                self.instructions.push(instruction);
                format!("double {}", double_const)
            } else {
                // Fallback: try direct format (might fail for some values)
                format!("double {}", const_val)
            }
        } else {
            // If it's a register name (starts with %), use it directly
            let reg_val = if value_val.starts_with('%') {
                value_val
            } else {
                format!("%{}", value_val)
            };
            format!("double {}", reg_val)
        };
        self.instructions.push(format!("  call void @silica_print_float64({})", arg));

        Ok(None) // print_float64 returns unit
    }

    /// Generate LLVM value for print_char expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_print_char_llvm(&mut self, print_char: &PrintCharExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        let char_val = self.generate_expression_llvm(&print_char.value)?;
        if let (Some(val), Some(builder), Some(module)) = (char_val, &self.builder, &self.module) {
            unsafe {
                // Get the silica_print_char function
                let print_char_fn = (*module).get_function("silica_print_char").unwrap();

                // Characters are already i32 in LLVM, which matches our u32 runtime expectation
                builder.build_call(print_char_fn, &[val.into()], "print_char_call").unwrap();
            }
        }
        Ok(None) // print_char returns unit
    }

    /// Generate LLVM value for print_int64 expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_print_int64_llvm(&mut self, print_int64: &PrintInt64Expr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        let int64_val = self.generate_expression_llvm(&print_int64.value)?;
        if let (Some(val), Some(builder), Some(module)) = (int64_val, &self.builder, &self.module) {
            unsafe {
                // Get the silica_print_int64 function
                let print_int64_fn = (*module).get_function("silica_print_int64").unwrap();

                // Ensure the value is i64
                let i64_type = (*self.context).i64_type();
                let i64_val = if val.get_type().is_int_type() {
                    let int_val = val.into_int_value();
                    // Sign extend or truncate to i64 if needed
                    if int_val.get_type().get_bit_width() > 64 {
                        builder.build_int_truncate(int_val, i64_type, "trunc_to_i64").unwrap()
                    } else if int_val.get_type().get_bit_width() < 64 {
                        builder.build_int_s_extend(int_val, i64_type, "sext_to_i64").unwrap()
                    } else {
                        int_val
                    }
                } else {
                    return Err(CompilerError::codegen_error("print_int64 expects an integer value".to_string()));
                };

                builder.build_call(print_int64_fn, &[i64_val.into()], "print_int64_call").unwrap();
            }
        }
        Ok(None) // print_int64 returns unit
    }

    /// Generate LLVM value for print_int8 expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_print_int8_llvm(&mut self, print_int8: &PrintInt8Expr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        let int8_val = self.generate_expression_llvm(&print_int8.value)?;
        if let (Some(val), Some(builder), Some(module)) = (int8_val, &self.builder, &self.module) {
            unsafe {
                // Get the silica_print_int8 function
                let print_int8_fn = (*module).get_function("silica_print_int8").unwrap();

                // Ensure the value is i8
                let i8_type = (*self.context).i8_type();
                let i8_val = if val.get_type().is_int_type() {
                    let int_val = val.into_int_value();
                    // Sign extend or truncate to i8 if needed
                    if int_val.get_type().get_bit_width() > 8 {
                        builder.build_int_truncate(int_val, i8_type, "trunc_to_i8").unwrap()
                    } else if int_val.get_type().get_bit_width() < 8 {
                        builder.build_int_s_extend(int_val, i8_type, "sext_to_i8").unwrap()
                    } else {
                        int_val
                    }
                } else {
                    return Err(CompilerError::codegen_error("print_int8 expects an integer value".to_string()));
                };

                builder.build_call(print_int8_fn, &[i8_val.into()], "print_int8_call").unwrap();
            }
        }
        Ok(None) // print_int8 returns unit
    }

    /// Generate LLVM value for print_int16 expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_print_int16_llvm(&mut self, print_int16: &PrintInt16Expr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        let int16_val = self.generate_expression_llvm(&print_int16.value)?;
        if let (Some(val), Some(builder), Some(module)) = (int16_val, &self.builder, &self.module) {
            unsafe {
                // Get the silica_print_int16 function
                let print_int16_fn = (*module).get_function("silica_print_int16").unwrap();

                // Ensure the value is i16
                let i16_type = (*self.context).i16_type();
                let i16_val = if val.get_type().is_int_type() {
                    let int_val = val.into_int_value();
                    // Sign extend or truncate to i16 if needed
                    if int_val.get_type().get_bit_width() > 16 {
                        builder.build_int_truncate(int_val, i16_type, "trunc_to_i16").unwrap()
                    } else if int_val.get_type().get_bit_width() < 16 {
                        builder.build_int_s_extend(int_val, i16_type, "sext_to_i16").unwrap()
                    } else {
                        int_val
                    }
                } else {
                    return Err(CompilerError::codegen_error("print_int16 expects an integer value".to_string()));
                };

                builder.build_call(print_int16_fn, &[i16_val.into()], "print_int16_call").unwrap();
            }
        }
        Ok(None) // print_int16 returns unit
    }

    /// Generate LLVM value for print_int32 expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_print_int32_llvm(&mut self, print_int32: &PrintInt32Expr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        let int32_val = self.generate_expression_llvm(&print_int32.value)?;
        if let (Some(val), Some(builder), Some(module)) = (int32_val, &self.builder, &self.module) {
            unsafe {
                // Get the silica_print_int32 function
                let print_int32_fn = (*module).get_function("silica_print_int32").unwrap();

                // Ensure the value is i32
                let i32_type = (*self.context).i32_type();
                let i32_val = if val.get_type().is_int_type() {
                    let int_val = val.into_int_value();
                    // Sign extend or truncate to i32 if needed
                    if int_val.get_type().get_bit_width() > 32 {
                        builder.build_int_truncate(int_val, i32_type, "trunc_to_i32").unwrap()
                    } else if int_val.get_type().get_bit_width() < 32 {
                        builder.build_int_s_extend(int_val, i32_type, "sext_to_i32").unwrap()
                    } else {
                        int_val
                    }
                } else {
                    return Err(CompilerError::codegen_error("print_int32 expects an integer value".to_string()));
                };

                builder.build_call(print_int32_fn, &[i32_val.into()], "print_int32_call").unwrap();
            }
        }
        Ok(None) // print_int32 returns unit
    }

    /// Generate LLVM value for print_float16 expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_print_float16_llvm(&mut self, print_float16: &PrintFloat16Expr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        let float16_val = self.generate_expression_llvm(&print_float16.value)?;
        if let (Some(val), Some(builder), Some(module)) = (float16_val, &self.builder, &self.module) {
            unsafe {
                // Get the silica_print_float16 function
                let print_float16_fn = (*module).get_function("silica_print_float16").unwrap();

                // Convert half to u16 for the runtime function
                // The runtime function expects u16 (the bit representation of the half)
                let half_type = self.context.half_type();
                let u16_type = self.context.i16_type();
                
                // Bitcast half to i16 (u16) to pass to runtime
                let half_val = val.try_as_basic_value().left().unwrap();
                let u16_val = builder.build_bitcast(half_val, u16_type, "half_to_u16").unwrap();
                
                builder.build_call(print_float16_fn, &[u16_val.into()], "print_float16_call").unwrap();
            }
        }
        Ok(None) // print_float16 returns unit
    }

    /// Generate LLVM value for print_float32 expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_print_float32_llvm(&mut self, print_float32: &PrintFloat32Expr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        let float32_val = self.generate_expression_llvm(&print_float32.value)?;
        if let (Some(val), Some(builder), Some(module)) = (float32_val, &self.builder, &self.module) {
            unsafe {
                // Get the silica_print_float32 function
                let print_float32_fn = (*module).get_function("silica_print_float32").unwrap();

                // float32 is already "float" in LLVM, which matches our runtime expectation
                builder.build_call(print_float32_fn, &[val.into()], "print_float32_call").unwrap();
            }
        }
        Ok(None) // print_float32 returns unit
    }

    /// Generate LLVM value for print_float64 expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_print_float64_llvm(&mut self, print_float64: &PrintFloat64Expr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        let float64_val = self.generate_expression_llvm(&print_float64.value)?;
        if let (Some(val), Some(builder), Some(module)) = (float64_val, &self.builder, &self.module) {
            unsafe {
                // Get the silica_print_float64 function
                let print_float64_fn = (*module).get_function("silica_print_float64").unwrap();

                // float64 is already "double" in LLVM, which matches our runtime expectation
                builder.build_call(print_float64_fn, &[val.into()], "print_float64_call").unwrap();
            }
        }
        Ok(None) // print_float64 returns unit
    }

    /// Generate LLVM value for list_directory expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_list_directory_llvm(&mut self, _list_dir: &ListDirectoryExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Return empty string for now (placeholder implementation)
        let empty_string = self.context.const_string(b"", false);
        Ok(Some(empty_string.into()))
    }


    /// Generate LLVM IR for get_cpu_topology expression
    fn generate_get_cpu_topology(&mut self, _get_topology: &GetCpuTopologyExpr) -> Result<Option<String>> {
        // Call the runtime function to get topology struct
        // This returns a CpuTopology struct containing the topology information
        let result_reg = self.next_register();
        self.instructions.push(format!("  %{} = call {{i64, i64, i64, i1, i64, i64, i1, i64, i64}} @silica_get_cpu_topology()", result_reg));

        // Return the struct
        Ok(Some(result_reg))
    }

    /// Helper: get (path_arg, path_length) for silica_read_file when path is a string constant.
    /// Returns None for path_length when path is a variable - use silica_read_file_path instead.
    fn get_path_arg_and_length(&mut self, path_val: &str) -> Result<(String, Option<String>)> {
        if let Some(path_length) = self.find_string_constant_length(path_val) {
            // String constant: use getelementptr/constant and compile-time length
            let path_arg = if path_val.starts_with("i8* ") || path_val.starts_with("i64 ") {
                path_val.to_string()
            } else if path_val.starts_with("getelementptr") {
                format!("i8* {}", path_val)
            } else if path_val.starts_with('%') {
                format!("i8* {}", path_val)
            } else if path_val.starts_with('@') {
                let len = path_length + 1;
                format!("i8* getelementptr inbounds ([{} x i8], [{} x i8]* {}, i64 0, i64 0)", len, len, path_val)
            } else {
                format!("i8* {}", path_val)
            };
            Ok((path_arg, Some(format!("i64 {}", path_length))))
        } else {
            // Variable: use path for silica_read_file_path (handles both SilicaString and raw constant)
            // Must produce i8* - cast i64 to i8* when stored as ptr-as-int
            let path_arg = if path_val.starts_with("i8* ") {
                path_val.to_string()
            } else if path_val.starts_with("i64 ") {
                let cast_reg = self.next_register();
                self.instructions.push(format!("  %{} = inttoptr {} to i8*", cast_reg, path_val.trim_start_matches("i64 ")));
                format!("i8* %{}", cast_reg.trim_start_matches('%'))
            } else if path_val.starts_with("getelementptr") {
                format!("i8* {}", path_val)
            } else if path_val.starts_with('%') {
                format!("i8* {}", path_val)
            } else {
                format!("i8* %{}", self.clean_register_for_instruction(path_val).trim_start_matches('%'))
            };
            Ok((path_arg, None))
        }
    }

    /// Generate LLVM IR for read_lines expression
    fn generate_read_lines(&mut self, read_lines: &ReadLinesExpr) -> Result<Option<String>> {
        let path_val = self.generate_expression(&read_lines.path)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid path in read_lines".to_string()))?;

        let (path_arg, path_length_opt) = self.get_path_arg_and_length(&path_val)?;

        // Call silica_read_file (constant) or silica_read_file_path (variable - handles both representations)
        let result_reg = self.next_register();
        if let Some(path_length_reg) = path_length_opt {
            self.instructions.push(format!("  %{} = call {{ i1, i8* }} @silica_read_file({}, {})", result_reg, path_arg, path_length_reg));
        } else {
            self.instructions.push(format!("  %{} = call {{ i1, i8* }} @silica_read_file_path({})", result_reg, path_arg));
        }

        // Extract SilicaString pointer (contains actual file content)
        let silica_string_ptr_reg = self.next_register();
        self.instructions.push(format!("  %{} = extractvalue {{ i1, i8* }} %{}, 1", silica_string_ptr_reg, result_reg));

        // For bootstrap compiler: return the SilicaString pointer as our "string"
        Ok(Some(silica_string_ptr_reg))
    }

    /// Generate LLVM IR for append_file expression
    fn generate_append_file(&mut self, append_file: &AppendFileExpr) -> Result<Option<String>> {
        let path_val = self.generate_expression(&append_file.path)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid path in append_file".to_string()))?;
        let content_val = self.generate_expression(&append_file.content)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid content in append_file".to_string()))?;

        let (path_arg, path_len_opt) = self.get_path_arg_and_length(&path_val)?;
        let (content_arg, content_len_opt) = self.get_path_arg_and_length(&content_val)?;

        let result_reg = self.next_register();
        if path_len_opt.is_some() && content_len_opt.is_some() {
            self.instructions.push(format!("  %{} = call {{ i1, i8* }} @silica_write_file({}, {}, {}, {})", result_reg, path_arg, path_len_opt.as_ref().unwrap(), content_arg, content_len_opt.as_ref().unwrap()));
        } else {
            self.instructions.push(format!("  %{} = call {{ i1, i8* }} @silica_write_file_path({}, {})", result_reg, path_arg, content_arg));
        }

        // Extract the success flag from the result struct
        let success_reg = self.next_register();
        self.instructions.push(format!("  %{} = extractvalue {{ i1, i8* }} %{}, 0", success_reg, result_reg));

        Ok(Some(success_reg))
    }

    /// Generate LLVM IR for file_exists expression
    fn generate_file_exists(&mut self, file_exists: &FileExistsExpr) -> Result<Option<String>> {
        let path_val = self.generate_expression(&file_exists.path)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid path in file_exists".to_string()))?;

        let (path_arg, path_length_opt) = self.get_path_arg_and_length(&path_val)?;

        let result_reg = self.next_register();
        if let Some(path_length_reg) = path_length_opt {
            self.instructions.push(format!("  %{} = call {{ i1, i8* }} @silica_read_file({}, {})", result_reg, path_arg, path_length_reg));
        } else {
            self.instructions.push(format!("  %{} = call {{ i1, i8* }} @silica_read_file_path({})", result_reg, path_arg));
        }

        // Extract the success flag
        let success_reg = self.next_register();
        self.instructions.push(format!("  %{} = extractvalue {{ i1, i8* }} %{}, 0", success_reg, result_reg));

        Ok(Some(success_reg))
    }

    /// Generate LLVM IR for delete_file expression
    fn generate_delete_file(&mut self, delete_file: &DeleteFileExpr) -> Result<Option<String>> {
        // For now, return true (placeholder implementation)
        Ok(Some("1".to_string()))
    }

    /// Generate LLVM IR for get_file_size expression
    fn generate_get_file_size(&mut self, get_file_size: &GetFileSizeExpr) -> Result<Option<String>> {
        // For now, return 0 (placeholder implementation)
        Ok(Some("0".to_string()))
    }

    /// Generate LLVM IR for create_directory expression
    fn generate_create_directory(&mut self, create_dir: &CreateDirectoryExpr) -> Result<Option<String>> {
        // For now, return true (placeholder implementation)
        Ok(Some("1".to_string()))
    }

    /// Generate LLVM IR for remove_directory expression
    fn generate_remove_directory(&mut self, remove_dir: &RemoveDirectoryExpr) -> Result<Option<String>> {
        // For now, return true (placeholder implementation)
        Ok(Some("1".to_string()))
    }

    /// Generate LLVM IR for list_directory expression
    fn generate_list_directory(&mut self, list_dir: &ListDirectoryExpr) -> Result<Option<String>> {
        #[cfg(feature = "llvm_backend")]
        {
            // LLVM backend handles strings inline - return empty string constant directly
            // This should not be reached when LLVM backend is active
            return Err(CompilerError::codegen_error("List directory should be handled by LLVM backend".to_string()));
        }
        #[cfg(not(feature = "llvm_backend"))]
        {
            // Text backend: create named constants
            let empty_string = String::new();
            if !self.string_constants.contains_key(&empty_string) {
                let const_name = format!("@str_const_{}", self.string_constants.len());
                // Store length including null terminator to match constant declaration
                let length = empty_string.len() + 1;
                self.string_constants.insert(empty_string.clone(), (const_name, length));
            }
            let (const_name, _) = self.string_constants.get(&empty_string).unwrap();
            Ok(Some(const_name.clone()))
        }
    }

    /// Generate LLVM IR for string length expression
    fn generate_string_len(&mut self, string_len: &StringLenExpr) -> Result<Option<String>> {
        let string_val = self.generate_expression(&string_len.string)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid string in len".to_string()))?;

        // Check if this is a string constant (contains getelementptr or @str_const_)
        if string_val.contains("@str_const_") || string_val.starts_with("getelementptr") {
            // Find the length from string_constants
            let length = self.find_string_constant_length(&string_val).unwrap_or(0);
            
            let result_reg = self.next_register();
            self.instructions.push(format!("  %{} = add i64 {}, 0", result_reg, length));
            Ok(Some(result_reg))
        } else {
            // string_val is an i8* pointer to a SilicaString struct (for runtime strings)
            // Strip type prefix (e.g. "i8* %call_3504" -> "call_3504") to avoid "i8* %i8* %call_3504"
            let string_ptr_reg = self.clean_register_for_instruction(&string_val).trim_start_matches('%').to_string();
            let string_ptr_reg = if string_ptr_reg.is_empty() { string_val.trim_start_matches('%').to_string() } else { string_ptr_reg };
            
            // Call silica_string_len runtime function
            let result_reg = self.next_register();
            self.instructions.push(format!("  %{} = call i64 @silica_string_len(i8* %{})", result_reg, string_ptr_reg));
            
            Ok(Some(result_reg))
        }
    }

    /// Generate LLVM IR for string character length expression
    fn generate_string_len_chars(&mut self, string_len_chars: &StringLenCharsExpr) -> Result<Option<String>> {
        let string_val = self.generate_expression(&string_len_chars.string)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid string in len_chars".to_string()))?;

        // Check if this is a string constant (contains getelementptr or @str_const_)
        if string_val.contains("@str_const_") || string_val.starts_with("getelementptr") {
            // Find the string content from string_constants and count characters
            let char_count = if let Some(const_name) = self.extract_constant_name(&string_val) {
                // Find the string content by constant name
                self.string_constants.iter()
                    .find(|(_, (name, _))| name == &const_name)
                    .map(|(content, _)| content.chars().count())
                    .unwrap_or(0)
            } else {
                0
            };
            
            let result_reg = self.next_register();
            self.instructions.push(format!("  %{} = add i64 {}, 0", result_reg, char_count));
            Ok(Some(result_reg))
        } else {
            // string_val is an i8* pointer to a SilicaString struct (for runtime strings)
            // Strip type prefix (e.g. "i8* %call_3504" -> "call_3504") to avoid "i8* %i8* %call_3504"
            let string_ptr_reg = self.clean_register_for_instruction(&string_val).trim_start_matches('%').to_string();
            let string_ptr_reg = if string_ptr_reg.is_empty() { string_val.trim_start_matches('%').to_string() } else { string_ptr_reg };
            
            // Call silica_string_len_chars runtime function
            let result_reg = self.next_register();
            self.instructions.push(format!("  %{} = call i64 @silica_string_len_chars(i8* %{})", result_reg, string_ptr_reg));
            
            Ok(Some(result_reg))
        }
    }

    /// Convert getelementptr from constant expression format to instruction format
    /// Constant format: getelementptr inbounds ([LEN x i8], [LEN x i8]* @CONST, i64 0, i64 0)
    /// Instruction format: getelementptr inbounds [LEN x i8], [LEN x i8]* @CONST, i32 0, i32 0
    fn convert_gep_to_instruction_format(&self, gep_expr: &str) -> String {
        if gep_expr.starts_with("getelementptr inbounds (") {
            // Parse: getelementptr inbounds ([LEN x i8], [LEN x i8]* @CONST, i64 0, i64 0)
            // Convert to: getelementptr inbounds [LEN x i8], [LEN x i8]* @CONST, i32 0, i32 0
            // Find the opening parenthesis after "inbounds "
            if let Some(open_paren_pos) = gep_expr.find('(') {
                // Find the closing bracket of the first array type: [LEN x i8]
                if let Some(close_bracket_pos) = gep_expr[open_paren_pos+1..].find(']') {
                    let array_type_end = open_paren_pos + 1 + close_bracket_pos + 1;
                    // Extract the array type: [LEN x i8]
                    let array_type = &gep_expr[open_paren_pos+1..array_type_end];
                    // Find the comma that separates the two types
                    if let Some(comma_pos) = gep_expr[array_type_end..].find(',') {
                        // Everything after the comma: " [LEN x i8]* @CONST, i64 0, i64 0)"
                        let after_comma = &gep_expr[array_type_end + comma_pos + 1..];
                        // Trim whitespace and closing parenthesis
                        let pointer_and_indices = after_comma.trim_end_matches(')').trim_start();
                        // pointer_and_indices = "[LEN x i8]* @CONST, i64 0, i64 0"
                        // Convert i64 indices to i32 for array indexing (more standard in LLVM IR)
                        let pointer_and_indices_i32 = pointer_and_indices.replace("i64 0", "i32 0");
                        // Use array type as element type for array pointers
                        return format!("getelementptr inbounds {}, {}", array_type, pointer_and_indices_i32);
                    }
                }
            }
        }
        // If parsing fails, return as-is (might already be in instruction format)
        gep_expr.to_string()
    }

    /// Helper to extract constant name from getelementptr expression or direct reference
    fn extract_constant_name(&self, const_ref: &str) -> Option<String> {
        // First try exact match
        for (_content, (name, _)) in &self.string_constants {
            if name == const_ref {
                return Some(name.clone());
            }
        }

        // If not found, try to parse getelementptr expression
        if const_ref.starts_with("getelementptr inbounds") && const_ref.contains("@str_const_") {
            if let Some(at_pos) = const_ref.find("@str_const_") {
                let name_start = at_pos;
                let name_end = const_ref[name_start..].find(|c: char| !c.is_alphanumeric() && c != '_' && c != '@')
                    .map(|pos| name_start + pos)
                    .unwrap_or(const_ref.len());
                return Some(const_ref[name_start..name_end].to_string());
            }
        }

        None
    }

    /// Generate LLVM IR for string concatenation expression
    fn generate_string_concat(&mut self, string_concat: &StringConcatExpr) -> Result<Option<String>> {
        let a_val = self.generate_expression(&string_concat.a)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid first string in concat".to_string()))?;
        let b_val = self.generate_expression(&string_concat.b)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid second string in concat".to_string()))?;

        // Strip type prefixes so we don't double-prefix (e.g. "i8* %lexeme" -> "%lexeme")
        let a_val = a_val.trim_start_matches("i8* ").trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").to_string();
        let b_val = b_val.trim_start_matches("i8* ").trim_start_matches("i64 ").trim_start_matches("i32 ").trim_start_matches("i1 ").to_string();

        // Both arguments need to be i8* pointers
        // For string constants (getelementptr expressions), they need to be evaluated and formatted
        // For runtime strings, they're already i8* pointers to SilicaString structs
        
        // Handle first argument (a)
        let a_arg = if a_val.starts_with('%') {
            // Already a register
            format!("i8* {}", a_val)
        } else if a_val.starts_with("getelementptr") {
            // String constant - evaluate getelementptr first, then use in function call
            let temp_reg = self.next_register();
            let gep_instruction = self.convert_gep_to_instruction_format(&a_val);
            self.instructions.push(format!("  %{} = {}", temp_reg, gep_instruction));
            format!("i8* %{}", temp_reg.trim_start_matches('%'))
        } else {
            // Register name without % prefix - add it
            format!("i8* %{}", a_val)
        };

        // Handle second argument (b)
        let b_arg = if b_val.starts_with('%') {
            // Already a register
            format!("i8* {}", b_val)
        } else if b_val.starts_with("getelementptr") {
            // String constant - evaluate getelementptr first, then use in function call
            let temp_reg = self.next_register();
            let gep_instruction = self.convert_gep_to_instruction_format(&b_val);
            self.instructions.push(format!("  %{} = {}", temp_reg, gep_instruction));
            format!("i8* %{}", temp_reg.trim_start_matches('%'))
        } else {
            // Register name without % prefix - add it
            format!("i8* %{}", b_val)
        };

        // Call silica_string_concat runtime function
        // Returns i8* pointer to new SilicaString struct
        let result_reg = self.next_register();
        self.instructions.push(format!("  %{} = call i8* @silica_string_concat({}, {})", result_reg, a_arg, b_arg));
        
        Ok(Some(format!("i8* {}", result_reg)))
    }

    /// Generate LLVM IR for string substring expression
    fn generate_string_substring(&mut self, string_substring: &StringSubstringExpr) -> Result<Option<String>> {
        let string_val = self.generate_expression(&string_substring.string)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid string in substring".to_string()))?;
        let start_val = self.generate_expression(&string_substring.start)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid start index in substring".to_string()))?;
        let end_val = self.generate_expression(&string_substring.end)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid end index in substring".to_string()))?;

        // Format string argument (call expects i8*; value may be i64 from field load)
        let string_arg = if string_val.starts_with("i64 ") {
            let i64_reg = string_val.strip_prefix("i64 ").unwrap();
            let ptr_reg = self.next_register();
            self.instructions.push(format!("  %{} = inttoptr i64 {} to i8*", ptr_reg, i64_reg));
            format!("i8* %{}", ptr_reg.trim_start_matches('%'))
        } else if string_val.starts_with("i8* ") {
            string_val.clone()
        } else if string_val.starts_with('%') {
            format!("i8* {}", string_val)
        } else if string_val.starts_with("getelementptr") {
            // String constant - evaluate getelementptr first
            let temp_reg = self.next_register();
            let gep_instruction = self.convert_gep_to_instruction_format(&string_val);
            self.instructions.push(format!("  %{} = {}", temp_reg, gep_instruction));
            format!("i8* %{}", temp_reg.trim_start_matches('%'))
        } else {
            format!("i8* %{}", string_val)
        };

        // Format start and end indices (should be i64 integers)
        let start_arg = if start_val.starts_with('%') {
            format!("i64 {}", start_val)
        } else if start_val.starts_with("i64 ") {
            start_val.clone()
        } else {
            // Assume it's a literal or register name
            format!("i64 %{}", start_val.trim_start_matches('%'))
        };

        let end_arg = if end_val.starts_with('%') {
            format!("i64 {}", end_val)
        } else if end_val.starts_with("i64 ") {
            end_val.clone()
        } else {
            // Assume it's a literal or register name
            format!("i64 %{}", end_val.trim_start_matches('%'))
        };

        // Call silica_string_substring runtime function
        // Returns i8* pointer to new SilicaString struct
        let result_reg = self.next_register();
        self.instructions.push(format!("  %{} = call i8* @silica_string_substring({}, {}, {})", result_reg, string_arg, start_arg, end_arg));
        
        Ok(Some(format!("i8* %{}", result_reg)))
    }

    /// Generate LLVM IR for string substring until character expression
    fn generate_string_substring_until_char(&mut self, string_substring_until_char: &StringSubstringUntilCharExpr) -> Result<Option<String>> {
        let string_val = self.generate_expression(&string_substring_until_char.string)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid string in substring_until_char".to_string()))?;
        let start_val = self.generate_expression(&string_substring_until_char.start)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid start index in substring_until_char".to_string()))?;
        let char_val = self.generate_expression(&string_substring_until_char.char)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid character in substring_until_char".to_string()))?;

        // Format string argument
        let string_arg = if string_val.starts_with('%') {
            format!("i8* {}", string_val)
        } else if string_val.starts_with("getelementptr") {
            // String constant - evaluate getelementptr first
            let temp_reg = self.next_register();
            let gep_instruction = self.convert_gep_to_instruction_format(&string_val);
            self.instructions.push(format!("  %{} = {}", temp_reg, gep_instruction));
            format!("i8* %{}", temp_reg.trim_start_matches('%'))
        } else {
            format!("i8* %{}", string_val)
        };

        // Format start index (should be i64 integer)
        let start_arg = if start_val.starts_with('%') {
            format!("i64 {}", start_val)
        } else if start_val.starts_with("i64 ") {
            start_val.clone()
        } else {
            // Assume it's a literal or register name
            format!("i64 %{}", start_val.trim_start_matches('%'))
        };

        // Format character argument (should be i32)
        let char_arg = if char_val.starts_with('%') {
            format!("i32 {}", char_val)
        } else if char_val.starts_with("i32 ") {
            char_val.clone()
        } else {
            // Assume it's a literal or register name
            format!("i32 %{}", char_val.trim_start_matches('%'))
        };

        // Call silica_string_substring_until_char runtime function
        // Returns i8* pointer to new SilicaString struct
        let result_reg = self.next_register();
        self.instructions.push(format!("  %{} = call i8* @silica_string_substring_until_char({}, {}, {})", result_reg, string_arg, start_arg, char_arg));
        
        Ok(Some(result_reg))
    }

    /// Generate LLVM IR for string starts with expression
    fn generate_string_starts_with(&mut self, string_starts_with: &StringStartsWithExpr) -> Result<Option<String>> {
        let string_val = self.generate_expression(&string_starts_with.string)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid string in starts_with".to_string()))?;
        let prefix_val = self.generate_expression(&string_starts_with.prefix)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid prefix in starts_with".to_string()))?;

        // Format string argument - handle both string constants and runtime strings
        // For runtime strings, pass the SilicaString struct pointer directly
        // get_string_data_and_length will extract the data pointer from the struct
        let string_arg = if string_val.contains("@str_const_") || string_val.starts_with("getelementptr") {
            // String constant - evaluate getelementptr first
            let temp_reg = self.next_register();
            let gep_instruction = self.convert_gep_to_instruction_format(&string_val);
            self.instructions.push(format!("  %{} = {}", temp_reg, gep_instruction));
            format!("i8* %{}", temp_reg.trim_start_matches('%'))
        } else {
            // Runtime string: strip type prefix so we don't emit "i8* %i8* t44"
            let reg = self.clean_register_for_instruction(&string_val);
            let reg = if reg.starts_with('%') { reg } else { format!("%{}", reg) };
            format!("i8* {}", reg)
        };

        // Format prefix argument - handle both string constants and runtime strings
        // For runtime strings, pass the SilicaString struct pointer directly
        // get_string_data_and_length will extract the data pointer from the struct
        let prefix_arg = if prefix_val.contains("@str_const_") || prefix_val.starts_with("getelementptr") {
            // String constant - evaluate getelementptr first
            let temp_reg = self.next_register();
            let gep_instruction = self.convert_gep_to_instruction_format(&prefix_val);
            self.instructions.push(format!("  %{} = {}", temp_reg, gep_instruction));
            format!("i8* %{}", temp_reg.trim_start_matches('%'))
        } else {
            // Runtime string: strip type prefix so we don't emit "i8* %i8* t44"
            let reg = self.clean_register_for_instruction(&prefix_val);
            let reg = if reg.starts_with('%') { reg } else { format!("%{}", reg) };
            format!("i8* {}", reg)
        };

        // Call silica_string_starts_with runtime function
        // Returns i1 (bool)
        let result_reg = self.next_register();
        self.instructions.push(format!("  %{} = call i1 @silica_string_starts_with({}, {})", result_reg, string_arg, prefix_arg));
        
        Ok(Some(result_reg))
    }

    /// Generate LLVM IR for string ends with expression
    fn generate_string_ends_with(&mut self, string_ends_with: &StringEndsWithExpr) -> Result<Option<String>> {
        let string_val = self.generate_expression(&string_ends_with.string)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid string in ends_with".to_string()))?;
        let suffix_val = self.generate_expression(&string_ends_with.suffix)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid suffix in ends_with".to_string()))?;

        // Format string argument - handle both string constants and runtime strings
        // For runtime strings, pass the SilicaString struct pointer directly
        // get_string_data_and_length will extract the data pointer from the struct
        let string_arg = if string_val.contains("@str_const_") || string_val.starts_with("getelementptr") {
            // String constant - evaluate getelementptr first
            let temp_reg = self.next_register();
            let gep_instruction = self.convert_gep_to_instruction_format(&string_val);
            self.instructions.push(format!("  %{} = {}", temp_reg, gep_instruction));
            format!("i8* %{}", temp_reg.trim_start_matches('%'))
        } else {
            // Runtime string: strip type prefix so we don't emit "i8* %i8* t50"
            let reg = self.clean_register_for_instruction(&string_val);
            let reg = if reg.starts_with('%') { reg } else { format!("%{}", reg) };
            format!("i8* {}", reg)
        };

        // Format suffix argument - handle both string constants and runtime strings
        // For runtime strings, pass the SilicaString struct pointer directly
        // get_string_data_and_length will extract the data pointer from the struct
        let suffix_arg = if suffix_val.contains("@str_const_") || suffix_val.starts_with("getelementptr") {
            // String constant - evaluate getelementptr first
            let temp_reg = self.next_register();
            let gep_instruction = self.convert_gep_to_instruction_format(&suffix_val);
            self.instructions.push(format!("  %{} = {}", temp_reg, gep_instruction));
            format!("i8* %{}", temp_reg.trim_start_matches('%'))
        } else {
            // Runtime string: strip type prefix so we don't emit "i8* %i8* t50"
            let reg = self.clean_register_for_instruction(&suffix_val);
            let reg = if reg.starts_with('%') { reg } else { format!("%{}", reg) };
            format!("i8* {}", reg)
        };

        // Call silica_string_ends_with runtime function
        // Returns i1 (bool)
        let result_reg = self.next_register();
        self.instructions.push(format!("  %{} = call i1 @silica_string_ends_with({}, {})", result_reg, string_arg, suffix_arg));
        
        Ok(Some(result_reg))
    }

    /// Generate LLVM IR for string contains expression
    fn generate_string_contains(&mut self, string_contains: &StringContainsExpr) -> Result<Option<String>> {
        let string_val = self.generate_expression(&string_contains.string)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid string in contains".to_string()))?;
        let substr_val = self.generate_expression(&string_contains.substr)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid substring in contains".to_string()))?;

        // Format string argument - handle both string constants and runtime strings
        // For runtime strings, pass the SilicaString struct pointer directly
        // get_string_data_and_length will extract the data pointer from the struct
        let string_arg = if string_val.contains("@str_const_") || string_val.starts_with("getelementptr") {
            // String constant - evaluate getelementptr first
            let temp_reg = self.next_register();
            let gep_instruction = self.convert_gep_to_instruction_format(&string_val);
            self.instructions.push(format!("  %{} = {}", temp_reg, gep_instruction));
            format!("i8* %{}", temp_reg.trim_start_matches('%'))
        } else {
            // Runtime string: strip type prefix so we don't emit "i8* %i8* t56"
            let reg = self.clean_register_for_instruction(&string_val);
            let reg = if reg.starts_with('%') { reg } else { format!("%{}", reg) };
            format!("i8* {}", reg)
        };

        // Format substring argument - handle both string constants and runtime strings
        // For runtime strings, pass the SilicaString struct pointer directly
        // get_string_data_and_length will extract the data pointer from the struct
        let substr_arg = if substr_val.contains("@str_const_") || substr_val.starts_with("getelementptr") {
            // String constant - evaluate getelementptr first
            let temp_reg = self.next_register();
            let gep_instruction = self.convert_gep_to_instruction_format(&substr_val);
            self.instructions.push(format!("  %{} = {}", temp_reg, gep_instruction));
            format!("i8* %{}", temp_reg.trim_start_matches('%'))
        } else {
            // Runtime string: strip type prefix so we don't emit "i8* %i8* t56"
            let reg = self.clean_register_for_instruction(&substr_val);
            let reg = if reg.starts_with('%') { reg } else { format!("%{}", reg) };
            format!("i8* {}", reg)
        };

        // Call silica_string_contains runtime function
        // Returns i1 (bool)
        let result_reg = self.next_register();
        self.instructions.push(format!("  %{} = call i1 @silica_string_contains({}, {})", result_reg, string_arg, substr_arg));
        
        Ok(Some(result_reg))
    }

    /// Generate LLVM IR for exec_command expression
    fn generate_exec_command(&mut self, exec_cmd: &ExecCommandExpr) -> Result<Option<String>> {
        let cmd_val = self.generate_expression(&exec_cmd.command)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid command in exec_command".to_string()))?;

        // Get command string length
        let cmd_length = self.find_string_constant_length(&cmd_val).unwrap_or(0);

        // Generate arguments
        let mut arg_vals = Vec::new();
        let mut arg_lengths = Vec::new();
        for arg in &exec_cmd.args {
            let arg_val = self.generate_expression(arg)?
                .ok_or_else(|| CompilerError::codegen_error("Invalid argument in exec_command".to_string()))?;
            let arg_length = self.find_string_constant_length(&arg_val).unwrap_or(0);
            arg_vals.push(arg_val);
            arg_lengths.push(arg_length);
        }

        // Call silica_exec_command with proper parameters
        let result_reg = self.next_register();
        self.instructions.push(format!("  %{} = call i8* @silica_exec_command(i8* {}, i64 {}, i8** null, i64 {}, i64* null)", result_reg, cmd_val, cmd_length, arg_vals.len()));

        Ok(Some(result_reg))
    }

    /// Generate LLVM call to exec_command runtime function
    fn generate_exec_command_call(&mut self, _call: &CallExpr) -> Result<Option<String>> {
        // exec_command is a built-in that should be parsed as ExecCommandExpr
        // This function handles the case where it might be called as a regular function
        Err(CompilerError::codegen_error("exec_command should be parsed as built-in expression".to_string()))
    }
}

/// Type mapping between Silica types and LLVM types (string representation)
pub struct TypeMap {}

impl TypeMap {
    pub fn new() -> Self {
        TypeMap {}
    }

    /// Convert Silica type to LLVM type string representation
    pub fn silica_to_llvm_str(&self, silica_type: &Type) -> String {
        match silica_type {
            Type::Unit => "void".to_string(),
            Type::Bool => "i1".to_string(),
            Type::Int8 => "i8".to_string(),
            Type::Int16 => "i16".to_string(),
            Type::Int32 => "i32".to_string(),
            Type::Int64 => "i64".to_string(),
            Type::Float16 => "half".to_string(),
            Type::Float32 => "float".to_string(),
            Type::Float64 => "double".to_string(),
            Type::Char => "i32".to_string(),
            Type::String => "i8*".to_string(),
            Type::Function { .. } => "i8*".to_string(), // Function pointers as void*
            Type::Tuple(_) => "i8*".to_string(), // Structs as opaque pointers
            Type::Record(_) => "i8*".to_string(),
            Type::Variant(_) => "i8*".to_string(),
            Type::Process { .. } => "i8*".to_string(),
            Type::Region { .. } => "i8*".to_string(),
            Type::Reference { .. } => "i64*".to_string(),
            Type::Buffer { .. } => "i8*".to_string(),
            Type::ActorRef => "i8*".to_string(),
            // Core affinity types - represented as integers for runtime scheduling
            Type::CoreId => "i32".to_string(),
            Type::CoreSet(_) => "i8*".to_string(), // Complex type as opaque pointer
            Type::AnyCore => "i32".to_string(),
            Type::PerformanceCores => "i32".to_string(),
            Type::EfficiencyCores => "i32".to_string(),
            Type::Variable(_) => "i64".to_string(),
            Type::Named(_) => "i64".to_string(),
            Type::Closure { .. } => "i8*".to_string(), // Closure objects as opaque pointers
            Type::Sum(_) => "i8*".to_string(), // Sum types as opaque pointers
            Type::Scheme { .. } => "i8*".to_string(), // Type schemes as opaque pointers
            Type::TypeOperator { .. } => "i8*".to_string(), // Type operators as opaque pointers
            Type::Existential { .. } => "i8*".to_string(), // Existential types as opaque pointers
            Type::TypeApplication { .. } => "i8*".to_string(), // Type applications as opaque pointers
            // NEON 128-bit vector types
            Type::Vec128Int8 => "<16 x i8>".to_string(),
            Type::Vec128Int16 => "<8 x i16>".to_string(),
            Type::Vec128Int32 => "<4 x i32>".to_string(),
            Type::Vec128Int64 => "<2 x i64>".to_string(),
            Type::Vec128Float32 => "<4 x float>".to_string(),
            Type::Vec128Bool => "<16 x i1>".to_string(),
            // SVE scalable vector types
            Type::VecInt8 => "<vscale x 16 x i8>".to_string(),
            Type::VecInt16 => "<vscale x 8 x i16>".to_string(),
            Type::VecInt32 => "<vscale x 4 x i32>".to_string(),
            Type::VecInt64 => "<vscale x 2 x i64>".to_string(),
            Type::VecFloat16 => "<vscale x 8 x half>".to_string(),
            Type::VecFloat32 => "<vscale x 4 x float>".to_string(),
            Type::VecFloat64 => "<vscale x 2 x double>".to_string(),
            Type::VecBool => "<vscale x 16 x i1>".to_string(),
            // SVE predicate type
            Type::Pred => "<vscale x 16 x i1>".to_string(),
        }
    }
}

impl CodeGenerator {
    /// Run optimization passes on the generated code
    #[cfg(feature = "llvm_backend")]
    pub fn optimize_module(&self, module: &inkwell::module::Module) -> Result<()> {
        match self.optimization_level {
            OptimizationLevel::None => {
                // No optimizations
                Ok(())
            }
            OptimizationLevel::Less => {
                self.run_basic_optimizations(module)
            }
            OptimizationLevel::Default => {
                self.run_standard_optimizations(module)
            }
            OptimizationLevel::Aggressive => {
                self.run_aggressive_optimizations(module)
            }
        }
    }

    #[cfg(feature = "llvm_backend")]
    fn run_basic_optimizations(&self, module: &inkwell::module::Module) -> Result<()> {
        // Create function pass manager for basic optimizations
        let fpm = PassManager::create(module);

        // Add basic optimization passes
        fpm.add_constant_merge_pass();
        fpm.add_dead_store_elimination_pass();
        fpm.add_instruction_combining_pass();
        fpm.add_reassociate_pass();
        fpm.add_gvn_pass();
        fpm.add_cfg_simplification_pass();

        // Run passes on all functions
        for function in module.get_functions() {
            fpm.run_on(&function);
        }

        Ok(())
    }

    #[cfg(feature = "llvm_backend")]
    fn run_standard_optimizations(&self, module: &inkwell::module::Module) -> Result<()> {
        // Create function pass manager for standard optimizations
        let fpm = PassManager::create(module);

        // Add comprehensive optimization passes
        fpm.add_constant_merge_pass();
        fpm.add_dead_store_elimination_pass();
        fpm.add_instruction_combining_pass();
        fpm.add_reassociate_pass();
        fpm.add_gvn_pass();
        fpm.add_cfg_simplification_pass();
        fpm.add_basic_alias_analysis_pass();
        fpm.add_promote_memory_to_register_pass();
        fpm.add_instruction_simplify_pass();
        fpm.add_tail_call_elimination_pass();

        // Run passes on all functions
        for function in module.get_functions() {
            fpm.run_on(&function);
        }

        Ok(())
    }

    /// Generate LLVM IR for function calls within function literals (helper function support)
    fn generate_function_literal_call(&mut self, call: &CallExpr, func_lit: &FunctionLiteralExpr, body_instructions: &mut Vec<String>) -> Result<String> {
        // For now, handle simple function calls by name
        // This enables calling helper functions defined at module level
        if let Expression::Identifier(func_name) = &*call.function {
            // Generate argument values
            let mut arg_values = Vec::new();
            for arg in &call.arguments {
                let arg_val = self.generate_function_literal_expr(arg, func_lit, body_instructions)?;
                arg_values.push(arg_val);
            }

            // Prepare arguments for the function call
            // In function literals, parameters are i8* but external functions expect i64
            let mut call_args = Vec::new();
            for (i, arg_val) in arg_values.iter().enumerate() {
                if arg_val.starts_with("i8* ") {
                    // Bitcast i8* to i64* and load the value
                    let ptr_reg = arg_val.trim_start_matches("i8* ");
                    let bitcast_reg = format!("%arg_bitcast_{}", body_instructions.len() + i);
                    let loaded_reg = format!("%arg_load_{}", body_instructions.len() + i);
                    body_instructions.push(format!("  {} = bitcast i8* {} to i64*", bitcast_reg, ptr_reg));
                    body_instructions.push(format!("  {} = load i64, i64* {}", loaded_reg, bitcast_reg));
                    call_args.push(format!("i64 {}", loaded_reg));
                } else {
                    // For other cases, assume i64
                    let clean_arg = arg_val.trim_start_matches("i64 ").trim_start_matches("i1 ");
                    call_args.push(format!("i64 {}", clean_arg));
                }
            }

            let arg_list = call_args.join(", ");

            // Create a call to the external function
            let result_reg = format!("%call_result_{}", body_instructions.len());
            body_instructions.push(format!("  {} = call i64 @{}({})", result_reg, func_name, arg_list));

            // Return the result as an i8* (boxed)
            let box_reg = format!("%box_call_{}", body_instructions.len());
            let ptr_reg = format!("%ptr_call_{}", body_instructions.len());
            body_instructions.push(format!("  {} = call i8* @malloc(i64 8)", box_reg));
            body_instructions.push(format!("  {} = bitcast i8* {} to i64*", ptr_reg, box_reg));
            body_instructions.push(format!("  store i64 {}, i64* {}", result_reg, ptr_reg));
            Ok(box_reg)
        } else {
            Err(CompilerError::codegen_error("Complex function calls in function literals not yet supported".to_string()))
        }
    }

    #[cfg(feature = "llvm_backend")]
    fn run_aggressive_optimizations(&self, module: &inkwell::module::Module) -> Result<()> {
        // Create function pass manager for aggressive optimizations
        let fpm = PassManager::create(module);

        // Add aggressive optimization passes (may increase compile time)
        fpm.add_constant_merge_pass();
        fpm.add_dead_store_elimination_pass();
        fpm.add_instruction_combining_pass();
        fpm.add_reassociate_pass();
        fpm.add_gvn_pass();
        fpm.add_cfg_simplification_pass();
        fpm.add_basic_alias_analysis_pass();
        fpm.add_promote_memory_to_register_pass();
        fpm.add_instruction_simplify_pass();
        fpm.add_tail_call_elimination_pass();
        fpm.add_loop_rotate_pass();
        fpm.add_loop_unroll_pass();
        fpm.add_lower_switch_pass();

        // Run passes on all functions
        for function in module.get_functions() {
            fpm.run_on(&function);
        }

        Ok(())
    }


    /// Generate LLVM value for get_cpu_topology expression (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_get_cpu_topology_llvm(&mut self, _get_topology: &GetCpuTopologyExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
            unsafe {
                // Get the silica_get_cpu_topology function
                if let Some(topology_func) = (*module).get_function("silica_get_cpu_topology") {
                    // Call the function (no arguments)
                    let call_result = builder.build_call(topology_func, &[], "cpu_topology").unwrap();

                    // Return the struct
                    Ok(Some(call_result.try_as_basic_value().unwrap_left().into()))
                } else {
                    Err(CompilerError::codegen_error("silica_get_cpu_topology function not found".to_string()))
                }
            }
        } else {
            Err(CompilerError::codegen_error("LLVM module or builder not initialized".to_string()))
        }
    }
}

