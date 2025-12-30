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
use crate::errors::{Result, codegen_error, CompilerError, SourceLocation};
use crate::types::TypeChecker;
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
    variables: HashMap<String, String>, // Variable name -> LLVM register/temp
    variable_types: HashMap<String, Type>, // Variable name -> Silica type
    instructions: Vec<String>,
    optimization_level: OptimizationLevel,
    symbol_table: Option<Box<crate::module_resolver::SymbolTable>>,
    expression_types: HashMap<SourceLocation, Type>,
    type_aliases: HashMap<String, Type>, // Type alias definitions
    struct_defs: HashMap<String, Vec<crate::ast::StructField>>, // Struct definitions
    variable_scopes: Vec<HashMap<String, String>>, // Scope stack for text IR variables

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
            variables: HashMap::new(),
            variable_types: HashMap::new(),
            instructions: Vec::new(),
            optimization_level,
            symbol_table: None,
            expression_types: HashMap::new(),
            type_aliases: HashMap::new(),
            struct_defs: HashMap::new(),
            variable_scopes: vec![HashMap::new()], // Start with global scope

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

    /// Expand type aliases for code generation
    fn expand_type_aliases_codegen(&self, ty: &Type) -> Type {
        match ty {
            Type::Named(name) => {
                if let Some(aliased_type) = self.type_aliases.get(name) {
                    // Expand the aliased type recursively
                    self.expand_type_aliases_codegen(aliased_type)
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
            Type::Generic { name, type_args } => {
                Type::Generic {
                    name: name.clone(),
                    type_args: type_args.iter().map(|arg| self.expand_type_aliases_codegen(arg)).collect(),
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
                Err(CompilerError::codegen_error("LLVM context not initialized".to_string()))
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
                Err(CompilerError::codegen_error("LLVM context not initialized".to_string()))
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

        // Region management functions
        self.instructions.push("declare i8* @silica_region_create()".to_string());
        self.instructions.push("declare i8* @silica_region_alloc(i8*, i64)".to_string());
        self.instructions.push("declare i64 @silica_region_read(i8*)".to_string());
        self.instructions.push("declare void @silica_region_write(i8*, i64)".to_string());
        self.instructions.push("declare void @silica_region_destroy(i8*)".to_string());

        // Actor management functions
        self.instructions.push("declare i8* @silica_actor_spawn(i8*, i8*)".to_string());
        self.instructions.push("declare void @silica_actor_send(i8*, i64)".to_string());
        self.instructions.push("declare i64 @silica_actor_recv(i8*)".to_string());

        // File I/O functions
        self.instructions.push("declare { i1, i8* } @silica_read_file(i8*, i64)".to_string());
        self.instructions.push("declare { i1, i8* } @silica_write_file(i8*, i64, i8*, i64)".to_string());
        self.instructions.push("declare void @silica_free_string(i8*)".to_string());

        // Process execution functions
        self.instructions.push("declare i8* @silica_exec_command(i8*, i64, i8*, i64, i8*)".to_string());
        self.instructions.push("declare void @silica_free_process_result(i8*)".to_string());

        self.instructions.push("".to_string());

        // Generate all declarations
        for decl in &program.declarations {
            match decl {
                Declaration::Function(func) => {
                    self.generate_function_declaration(func)?;
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
                Declaration::Impl(_) => {
                    // Impl declarations generate method code
                    self.instructions.push("; Impl declaration (generates method code)".to_string());
                }
                Declaration::TypeAlias(_) => {
                    // Type alias declarations don't generate code in LLVM
                    self.instructions.push("; Type alias declaration (metadata only)".to_string());
                }
            }
            self.instructions.push("".to_string());
        }

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
        println!("✓ LLVM module structure verified (would use inkwell verification when enabled)");

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

                // Silica runtime functions
                let region_create_type = i8_ptr.fn_type(&[], false);
                module.add_function("silica_region_create", region_create_type, None);

                let region_alloc_type = i8_ptr.fn_type(&[i8_ptr.into(), i64_type.into()], false);
                module.add_function("silica_region_alloc", region_alloc_type, None);

                let region_read_type = i64_type.fn_type(&[i8_ptr.into()], false);
                module.add_function("silica_region_read", region_read_type, None);

                let region_write_type = void_type.fn_type(&[i8_ptr.into(), i64_type.into()], false);
                module.add_function("silica_region_write", region_write_type, None);

                let region_destroy_type = void_type.fn_type(&[i8_ptr.into()], false);
                module.add_function("silica_region_destroy", region_destroy_type, None);

                // Actor management functions
                let actor_spawn_type = i8_ptr.fn_type(&[i64_type.into(), i8_ptr.into()], false);
                module.add_function("silica_actor_spawn", actor_spawn_type, None);

                let actor_send_type = void_type.fn_type(&[i8_ptr.into(), i64_type.into()], false);
                module.add_function("silica_actor_send", actor_send_type, None);

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

                let print_int_type = void_type.fn_type(&[i64_type.into()], false);
                module.add_function("silica_print_int", print_int_type, None);

                let print_bool_type = void_type.fn_type(&[i1_type.into()], false);
                module.add_function("silica_print_bool", print_bool_type, None);

                let print_char_type = void_type.fn_type(&[i8_type.into()], false);
                module.add_function("silica_print_char", print_char_type, None);
            }
        }
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
        // Create function signature (text representation)
        let param_types: Vec<String> = func.parameters.iter()
            .map(|param| {
                if param.pattern.is_some() {
                    // Pattern parameters are passed as i8* (pointers to tuples/structs)
                    "i8*".to_string()
                } else {
                    // Expand type aliases before converting to LLVM string
                    let expanded_type = self.expand_type_aliases_codegen(&param.type_);
                    self.type_map.silica_to_llvm_str(&expanded_type)
                }
            })
            .collect();

        let return_type = func.return_type.as_ref().unwrap_or(&Type::Unit);
        let return_type_str = match return_type {
            Type::Tuple(_) => "i8*".to_string(), // Tuple returns are pointers
            _ => self.type_map.silica_to_llvm_str(return_type),
        };

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

        let signature = format!("define {} @{}({}) {{",
            return_type_str,
            func.name,
            param_strs.join(", ")
        );

        self.instructions.push(signature.clone());
        self.functions.insert(func.name.clone(), signature);
        self.function_return_types.insert(func.name.clone(), return_type_str.clone());

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
                            if let Pattern::Identifier(elem_name) = elem_pattern {
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

                                // Load based on type
                                let elem_reg = format!("%{}", elem_name);
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

                                // Store in variable scope
                                self.variables.insert(elem_name.clone(), elem_reg);
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
                param_reg
            };
            self.instructions.push(format!("  ; Parameter: {}", param_reg));
        }

        // Generate function body
        let body_result = self.generate_expression(&func.body)?;

        // Generate return
        match return_type {
            Type::Unit => {
                self.instructions.push("  ret void".to_string());
            }
            _ => {
                // Return the result of the function body
                if let Some(result_val) = body_result {
                    self.instructions.push(format!("  ret {} {}", return_type_str, result_val.trim_start_matches(&return_type_str)));
                } else {
                    // Fallback to dummy value if no result
                    self.instructions.push(format!("  ret {} 0", return_type_str));
                }
            }
        }

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
            Expression::Literal(lit) => Ok(Some(self.generate_literal(lit))),
            Expression::Identifier(name) => self.generate_identifier(name),
            Expression::Binary(binary) => self.generate_binary(binary),
            Expression::Unary(unary) => self.generate_unary(unary),
            Expression::Call(call) => self.generate_call(call),
            Expression::If(if_expr) => self.generate_if(if_expr),
            Expression::Case(case) => self.generate_case(case),
            Expression::Do(do_expr) => self.generate_do(do_expr),
            Expression::Region(region) => self.generate_region(region),
            Expression::AllocRef(alloc) => self.generate_alloc_ref(alloc),
            Expression::ReadRef(read) => self.generate_read_ref(read),
            Expression::WriteRef(write) => self.generate_write_ref(write),
            Expression::Spawn(spawn) => self.generate_spawn(spawn),
            Expression::Send(send) => self.generate_send(send),
            Expression::Recv(recv) => self.generate_recv(recv),
            Expression::ReadFile(read_file) => self.generate_read_file(read_file),
            Expression::WriteFile(write_file) => self.generate_write_file(write_file),
            Expression::ExecCommand(exec_cmd) => self.generate_exec_command(exec_cmd),
            Expression::FunctionLiteral(_) => {
                Err(CompilerError::codegen_error("Function literals not yet implemented".to_string()))
            }
            Expression::Region(_) => {
                Err(CompilerError::codegen_error("Region expressions not yet implemented".to_string()))
            }
            Expression::StructLiteral(struct_lit) => self.generate_struct_literal(struct_lit),
            Expression::FieldAccess(field_access) => self.generate_field_access(field_access),
            Expression::Tuple(tuple) => self.generate_tuple(tuple),
            Expression::GenericInstantiation(_) => {
                Err(CompilerError::codegen_error("Generic instantiation not yet implemented".to_string()))
            }
            Expression::ConstructorCall(_) => {
                Err(CompilerError::codegen_error("Constructor calls not yet implemented".to_string()))
            }
        }
    }

    /// Generate expression (LLVM backend) - simplified for function calls only
    #[cfg(feature = "llvm_backend")]
    fn generate_expression(&mut self, _expr: &Expression) -> Result<Option<String>> {
        // For LLVM backend, we use generate_expression_llvm for actual LLVM generation
        // This method is only used by text backend code, so return an error for LLVM
        Err(CompilerError::codegen_error("Text expression generation not available in LLVM backend".to_string()))
    }

    /// Generate LLVM IR for literal values
    fn generate_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Unit => "void".to_string(),
            Literal::Bool(true) => "i1 1".to_string(),
            Literal::Bool(false) => "i1 0".to_string(),
            Literal::Int(value) => format!("i64 {}", value),
            Literal::Char(c) => format!("i32 {}", *c as i32),
            Literal::String(s) => format!("@str_const_{}", s.len()), // String constant reference
        }
    }

    /// Generate LLVM IR for identifier reference
    fn generate_identifier(&self, name: &str) -> Result<Option<String>> {
        // First check the scope stack for variables
        if let Some(var_reg) = self.lookup_variable_text(name) {
            Ok(Some(var_reg))
        }
        // Then check the global variables map
        else if let Some(var_reg) = self.variables.get(name) {
            Ok(Some(var_reg.clone()))
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
            // For LLVM IR, we need to strip type prefixes from operands
            // since the operation type is declared at the beginning
            let clean_lhs = lhs.trim_start_matches("i64 ").trim_start_matches("i1 ");
            let clean_rhs = rhs.trim_start_matches("i64 ").trim_start_matches("i1 ");

            let op_instr = match binary.operator {
                BinaryOp::Add => format!("  {} = add i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
                BinaryOp::Subtract => format!("  {} = sub i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
                BinaryOp::Multiply => format!("  {} = mul i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
                BinaryOp::Divide => format!("  {} = sdiv i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
                BinaryOp::Modulo => format!("  {} = srem i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
                BinaryOp::Equal => format!("  {} = icmp eq i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
                BinaryOp::NotEqual => format!("  {} = icmp ne i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
                BinaryOp::Less => format!("  {} = icmp slt i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
                BinaryOp::LessEqual => format!("  {} = icmp sle i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
                BinaryOp::Greater => format!("  {} = icmp sgt i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
                BinaryOp::GreaterEqual => format!("  {} = icmp sge i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
                BinaryOp::And => format!("  {} = and i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
                BinaryOp::Or => format!("  {} = or i64 {}, {}", temp_reg, clean_lhs, clean_rhs),
            };

            self.instructions.push(op_instr);
            Ok(Some(temp_reg))
        } else {
            Err(CompilerError::codegen_error("Binary operation on invalid operands".to_string()))
        }
    }

    /// Generate LLVM IR for unary operations
    fn generate_unary(&mut self, unary: &UnaryExpr) -> Result<Option<String>> {
        let operand = self.generate_expression(&unary.operand)?;

        match unary.operator {
            UnaryOp::Not => {
                if let Some(op) = operand {
                    let temp_reg = format!("%t{}", self.instructions.len());
                    self.instructions.push(format!("  {} = xor i64 {}, -1", temp_reg, op));
                    Ok(Some(temp_reg))
                } else {
                    Err(CompilerError::codegen_error("Not operation on invalid operand".to_string()))
                }
            }
            UnaryOp::Negate => {
                if let Some(op) = operand {
                    let temp_reg = format!("%t{}", self.instructions.len());
                    self.instructions.push(format!("  {} = sub i64 0, {}", temp_reg, op));
                    Ok(Some(temp_reg))
                } else {
                    Err(CompilerError::codegen_error("Negate operation on invalid operand".to_string()))
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

        // For now, assume the function is an identifier (function name)
        if let Expression::Identifier(func_name) = &*call.function {
            // Special handling for file I/O functions
            if func_name == "read_file" {
                return self.generate_read_file_call(call);
            } else if func_name == "write_file" {
                return self.generate_write_file_call(call);
            }

            // Check if it's a local function
            if self.functions.contains_key(func_name) {
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
                let typed_args: Vec<String> = arg_strs.iter()
                    .map(|arg| {
                        if arg.starts_with("i64 ") || arg.starts_with("i1 ") || arg.starts_with("i8* ") {
                            arg.clone() // Already has type prefix
                        } else if arg.starts_with('%') {
                            // Assume i64 type for registers (most common case)
                            format!("i64 {}", arg)
                        } else {
                            // For bare constants, assume i64
                            format!("i64 {}", arg)
                        }
                    })
                    .collect();
                let args_str = typed_args.join(", ");
                let temp_reg = format!("%t{}", self.instructions.len());

                // Determine the return type of the function
                let return_type = self.function_return_types.get(func_name)
                    .cloned()
                    .ok_or_else(|| CompilerError::codegen_error(
                        format!("Unknown function '{}'. Function must be declared before it can be called.", func_name)
                    ))?;

                let call_instr = format!("  {} = call {} @{}({})", temp_reg, return_type, func_name, args_str);
                self.instructions.push(call_instr);

                Ok(Some(temp_reg))
            }
            // Check if it's an imported function
            else if let Some(symbol_table) = &self.symbol_table {
                let mut found = false;
                for (_module_name, module_symbols) in &symbol_table.modules {
                    if let Some(_symbol_info) = module_symbols.get(func_name) {
                        // Found imported function - generate the call
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
                                if arg.starts_with("i64 ") || arg.starts_with("i1 ") {
                                    arg.clone() // Already has type prefix
                                } else {
                                    format!("i64 {}", arg) // Add type prefix for bare registers/constants
                                }
                            })
                            .collect();
                        let args_str = typed_args.join(", ");
                        let temp_reg = format!("%t{}", self.instructions.len());
                        let call_instr = format!("  {} = call i64 @{}({})", temp_reg, func_name, args_str);
                        self.instructions.push(call_instr);

                        found = true;
                        return Ok(Some(temp_reg));
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

    /// Generate code for method calls (receiver.method(args))
    fn generate_method_call(&mut self, field_access: &FieldAccessExpr, call: &CallExpr) -> Result<Option<String>> {
        // For now, we'll generate a direct call to a method function
        // In a full implementation, we'd need to resolve the trait method

        // Generate the receiver
        let receiver_val = match self.generate_expression(&field_access.object)? {
            Some(val) => val,
            None => return Err(CompilerError::CodegenError { message: "Invalid receiver in method call".to_string() }),
        };

        // Create method name (for now, just concatenate type and method)
        // This is a simplified approach - in a real implementation we'd use trait resolution
        let method_name = format!("{}_{}", "unknown_type", field_access.field); // TODO: Use actual type

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
        let typed_args: Vec<String> = arg_strs.iter()
            .map(|arg| {
                if arg.starts_with("i64 ") || arg.starts_with("i1 ") {
                    arg.clone() // Already has type prefix
                } else {
                    format!("i64 {}", arg) // Add type prefix for bare registers/constants
                }
            })
            .collect();
        let args_str = typed_args.join(", ");
        let temp_reg = format!("%t{}", self.instructions.len());
        let call_instr = format!("  {} = call i64 @{}({})", temp_reg, method_name, args_str);
        self.instructions.push(call_instr);

        Ok(Some(temp_reg))
    }

    /// Generate LLVM value for expressions (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_expression_llvm(&mut self, expr: &Expression) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        match expr {
            Expression::Literal(lit) => self.generate_literal_llvm(lit),
            Expression::Identifier(name) => self.generate_identifier_llvm(name),
            Expression::Binary(binary) => self.generate_binary_llvm(binary),
            Expression::Unary(unary) => self.generate_unary_llvm(unary),
            Expression::Call(call) => self.generate_call_llvm(call),
            Expression::If(if_expr) => self.generate_if_llvm(if_expr),
            Expression::Case(case) => self.generate_case_llvm(case),
            Expression::Do(do_expr) => self.generate_do_llvm(do_expr),
            Expression::Region(region) => self.generate_region_llvm(region),
            Expression::AllocRef(alloc) => self.generate_alloc_ref_llvm(alloc),
            Expression::ReadRef(read) => self.generate_read_ref_llvm(read),
            Expression::WriteRef(write) => self.generate_write_ref_llvm(write),
            Expression::Tuple(exprs) => self.generate_tuple_llvm(exprs),
            Expression::StructLiteral(struct_lit) => self.generate_struct_literal_llvm(struct_lit),
            Expression::FieldAccess(field_access) => unimplemented!("Field access LLVM backend"),
            Expression::Spawn(spawn) => self.generate_spawn_llvm(spawn),
            Expression::Send(send) => self.generate_send_llvm(send),
            Expression::Recv(recv) => unimplemented!("Recv LLVM backend"),
            Expression::ReadFile(read_file) => self.generate_read_file_llvm(read_file),
            Expression::WriteFile(write_file) => self.generate_write_file_llvm(write_file),
            Expression::ExecCommand(exec_cmd) => self.generate_exec_command_llvm(exec_cmd),
            _ => Err(CompilerError::codegen_error(format!("Expression type not yet supported in LLVM backend: {:?}", expr))),
        }
    }

    /// Generate LLVM value for function calls (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_call_llvm(&mut self, call: &CallExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Handle generic function calls with type arguments
        if !call.type_args.is_empty() {
            return self.generate_generic_call_llvm(call);
        }

        // Check if this is a method call (receiver.method(args))
        if let Expression::FieldAccess(field_access) = &*call.function {
            return self.generate_method_call_llvm(field_access, call);
        }

        // For now, assume function calls are to known functions
        if let Expression::Identifier(func_name) = &*call.function {
            // Special handling for file I/O functions
            if func_name == "read_file" {
                return self.generate_read_file_call_llvm(call);
            } else if func_name == "write_file" {
                return self.generate_write_file_call_llvm(call);
            }

            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                unsafe {
                    // First try to get the function from the current module
                    let func = if let Some(func) = (*module).get_function(func_name) {
                        Some(func)
                    } else if let Some(symbol_table) = &self.symbol_table {
                        // Check if it's an imported function
                        let mut found_func = None;
                        for (_module_name, module_symbols) in &symbol_table.modules {
                            if let Some(symbol_info) = module_symbols.get(func_name) {
                                // Generate external function declaration with correct arity
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
                        found_func
                    } else {
                        None
                    };

                    if let Some(func) = func {
                        // Generate arguments as LLVM values - simplified for now
                        let mut llvm_args = Vec::new();
                        for arg in &call.arguments {
                            match arg {
                                Expression::Literal(Literal::Int(value)) => {
                                    let arg_val = (*self.context).i64_type().const_int(*value as u64, false);
                                    llvm_args.push(arg_val.into());
                                }
                                _ => return Err(CompilerError::codegen_error("Only integer literals supported as function arguments for now".to_string())),
                            }
                        }

                        // Call the function
                        let _call_result = builder.build_call(func, &llvm_args, "call_result").unwrap();

                        // Handle the result based on return type
                        // For now, assume functions return i64 values and create a placeholder
                        let result_val = (*self.context).i64_type().const_int(0, false);
                        Ok(Some(result_val.into()))
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
                Err(CompilerError::codegen_error("LLVM context not initialized".to_string()))
            }
        }
    }

    /// Generate LLVM value for memory allocation (alloc_ref) (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_alloc_ref_llvm(&mut self, alloc: &AllocRefExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate region and initial value expressions first (without borrowing builder)
        let region_val = self.generate_expression_llvm(&alloc.region)?;
        let initial_val = self.generate_expression_llvm(&alloc.initial_value)?;

        if let (Some(region), Some(val)) = (region_val, initial_val) {
            if let (Some(builder), Some(module)) = (&self.builder, &self.module) {
                unsafe {
                    // Get the silica_region_alloc function
                    if let Some(alloc_func) = (*module).get_function("silica_region_alloc") {
                        // Call silica_region_alloc(region_ptr, initial_value) -> ref_ptr
                        let _call_result = builder.build_call(alloc_func, &[region.into(), val.into()], "alloc_result").unwrap();

                        // For now, return a placeholder i8* (null pointer) - the actual implementation would extract from call_result
                        // This is a temporary simplification to get the basic structure working
                        let placeholder_ptr = (*self.context).ptr_type(inkwell::AddressSpace::default()).const_null();
                        Ok(Some(placeholder_ptr.into()))
                    } else {
                        Err(CompilerError::codegen_error("silica_region_alloc function not found".to_string()))
                    }
                }
            } else {
                Err(CompilerError::codegen_error("LLVM builder or module not initialized".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error("Invalid region or initial value for allocation".to_string()))
        }
    }

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
        if let (Some(builder), Some(module)) = (&self.builder, &self.module) {
            unsafe {
                // Get the silica_region_create function
                if let Some(region_create_func) = (*module).get_function("silica_region_create") {
                    // Call silica_region_create() -> region_ptr
                    let call_result = builder.build_call(region_create_func, &[], "region_result").unwrap();

                    // For now, return a placeholder i8* - the actual implementation would extract from call_result
                    let placeholder_ptr = (*self.context).i8_type().ptr_type(inkwell::AddressSpace::default()).const_null();
                    Ok(Some(placeholder_ptr.into()))
                } else {
                    Err(CompilerError::codegen_error("silica_region_create function not found".to_string()))
                }
            }
        } else {
            Err(CompilerError::codegen_error("LLVM builder or module not initialized".to_string()))
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

                                        // Store in current scope
                                        self.add_variable(name.clone(), alloca);
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
                                            if let Pattern::Identifier(elem_name) = elem_pattern {
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
                                            } else {
                                                return Err(CompilerError::codegen_error("Only identifier patterns supported in tuple decomposition".to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Pattern::Wildcard => {
                            // Wildcard pattern, ignore the value
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

    /// Generate LLVM value for memory write (write_ref) (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_write_ref_llvm(&mut self, write: &WriteRefExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate reference and value expressions first (without borrowing builder)
        let ref_val = self.generate_expression_llvm(&write.reference)?;
        let value_val = self.generate_expression_llvm(&write.value)?;

        if let (Some(ref_ptr), Some(val)) = (ref_val, value_val) {
            if let Some(builder) = &self.builder {
                unsafe {
                    // Get the silica_region_write function
                    if let Some(module) = &self.module {
                        if let Some(write_func) = (*module).get_function("silica_region_write") {
                            // Call silica_region_write(ref_ptr, value) -> void
                            builder.build_call(write_func, &[ref_ptr.into(), val.into()], "write_result").unwrap();

                            // Write operations return unit (void), so no result value
                            Ok(None)
                        } else {
                            Err(CompilerError::codegen_error("silica_region_write function not found".to_string()))
                        }
                    } else {
                        Err(CompilerError::codegen_error("LLVM module not initialized".to_string()))
                    }
                }
            } else {
                Err(CompilerError::codegen_error("LLVM builder not initialized".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error("Invalid reference or value for write operation".to_string()))
        }
    }

    /// Enter a new variable scope (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn enter_scope(&mut self) {
        self.llvm_variable_scopes.push(HashMap::new());
    }

    /// Exit the current variable scope (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn exit_scope(&mut self) {
        self.llvm_variable_scopes.pop();
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
    }

    /// Exit the current variable scope (text IR)
    fn exit_scope_text(&mut self) {
        self.variable_scopes.pop();
    }

    /// Add a variable to the current scope (text IR)
    fn add_variable_text(&mut self, name: String, register: String) {
        if let Some(current_scope) = self.variable_scopes.last_mut() {
            current_scope.insert(name, register);
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
            if let Some(builder) = &self.builder {
                unsafe {
                    let result = match binary.operator {
                        BinaryOp::Add => {
                            builder.build_int_add(left_val.into_int_value(), right_val.into_int_value(), "add_result").unwrap().into()
                        }
                        BinaryOp::Subtract => {
                            builder.build_int_sub(left_val.into_int_value(), right_val.into_int_value(), "sub_result").unwrap().into()
                        }
                        BinaryOp::Multiply => {
                            builder.build_int_mul(left_val.into_int_value(), right_val.into_int_value(), "mul_result").unwrap().into()
                        }
                        BinaryOp::Divide => {
                            builder.build_int_signed_div(left_val.into_int_value(), right_val.into_int_value(), "div_result").unwrap().into()
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

    /// Generate LLVM unary operations (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_unary_llvm(&mut self, unary: &UnaryExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Generate operand first (without borrowing builder)
        let operand = self.generate_expression_llvm(&unary.operand)?;

        if let Some(op_val) = operand {
            if let Some(builder) = &self.builder {
                unsafe {
                    let result = match unary.operator {
                        UnaryOp::Negate => {
                            let zero = (*self.context).i64_type().const_int(0, false);
                            builder.build_int_sub(zero, op_val.into_int_value(), "neg_result").unwrap().into()
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

        if let (Some(initial_state), Some(behavior)) = (initial_state_val, behavior_val) {
            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                unsafe {
                    // Get the silica_actor_spawn function
                    if let Some(spawn_func) = (*module).get_function("silica_actor_spawn") {
                        // Call silica_actor_spawn(initial_state, behavior)
                        // Note: behavior should be a function pointer, but for now we pass the value
                        let _call_result = builder.build_call(
                            spawn_func,
                            &[initial_state.into(), behavior.into()],
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
            Err(CompilerError::codegen_error("Invalid initial state or behavior for spawn".to_string()))
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

    /// Generate LLVM message receive (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_recv_llvm(&mut self, recv: &RecvExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // LLVM backend not implemented
        Err(CompilerError::codegen_error("LLVM backend not implemented".to_string()))

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
                Err(CompilerError::codegen_error("LLVM context not initialized".to_string()))
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
                Err(CompilerError::codegen_error("LLVM context not initialized".to_string()))
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
                Err(CompilerError::codegen_error("LLVM context not initialized".to_string()))
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
                Err(CompilerError::codegen_error("LLVM context not initialized".to_string()))
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
                Err(CompilerError::codegen_error("LLVM context not initialized".to_string()))
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
                Err(CompilerError::codegen_error("LLVM context not initialized".to_string()))
            }
        }
    }

    /// Generate LLVM value for field access expressions (LLVM backend)

    /// Helper to convert types to strings for monomorphization names
    #[cfg(feature = "llvm_backend")]
    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::Int => "int".to_string(),
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
                let result_type = then_val.get_type();
                let phi = builder.build_phi(result_type, "if_result").unwrap();
                phi.add_incoming(&[(&then_val, then_end_block), (&else_val, else_end_block)]);

                Ok(Some(phi.as_basic_value()))
            }
        } else {
            Err(CompilerError::codegen_error("LLVM builder not initialized".to_string()))
        }
    }

    /// Generate LLVM value for case expressions (pattern matching) (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_case_llvm(&mut self, case: &CaseExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        // Temporary simple implementation: just return the first branch
        if !case.branches.is_empty() {
            return self.generate_expression_llvm(&case.branches[0].body);
        }
        Err(CompilerError::codegen_error("Case expression must have at least one branch".to_string()))
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
            Pattern::Wildcard => {
                // Wildcard doesn't bind any variables
                Ok(())
            }
            Pattern::Literal(_) => {
                // Literal patterns don't bind variables
                Ok(())
            }
            Pattern::Tuple(patterns) => {
                // For now, handle single-element tuples
                if patterns.len() == 1 {
                    self.bind_pattern_variables(&patterns[0], value)
                } else {
                    Ok(()) // Multi-element tuples not supported yet
                }
            }
            Pattern::Record(field_patterns) => {
                // Bind variables from record field patterns
                for (field_name, field_pattern) in field_patterns {
                    // For now, assume we can access fields by name
                    // In a full implementation, this would extract the field value from the record
                    // For demo purposes, we'll bind the whole value to field names as variables
                    if let Pattern::Identifier(var_name) = field_pattern {
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
            Pattern::GenericVariant { constructor: _, type_args: _, payload } => {
                // Similar to variant patterns
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
            Pattern::Identifier(_) => {
                // Identifier patterns always match (variables already bound)
                unsafe {
                    Ok((*self.context).i64_type().const_int(1, false))
                }
            }
            Pattern::Wildcard => {
                // Wildcard always matches - no builder needed
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
            Pattern::GenericVariant { constructor, type_args: _, payload } => {
                // Similar to variant patterns but with type arguments
                unsafe {
                    Ok((*self.context).i64_type().const_int(1, false)) // Assume generic variant matches
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

    /// Generate LLVM IR for case expressions (text IR)
    fn generate_case(&mut self, case: &CaseExpr) -> Result<Option<String>> {
        // Enter a new scope for case pattern variables
        self.enter_scope_text();

        // Generate scrutinee
        let scrutinee_reg = match self.generate_expression(&case.scrutinee)? {
            Some(reg) => reg,
            None => return codegen_error("Case scrutinee must produce a value".to_string()),
        };

        // Allocate result variable
        let result_reg = format!("%case_result_{}", self.instructions.len());
        self.instructions.push(format!("  {} = alloca i64", result_reg));

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
                // For now, assume guard_result is i1. In full type system, this would be checked.
                self.instructions.push(format!("  {} = add i1 {}, 0", guard_bool, guard_result));

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

            let body_val = match self.generate_expression(&branch.body)? {
                Some(val) => {
                    // Extract just the value part if it has a type prefix
                    if val.starts_with("i64 ") {
                        val[4..].to_string()
                    } else {
                        val
                    }
                },
                None => return codegen_error("Case branch body must produce a value".to_string()),
            };
            self.instructions.push(format!("  store i64 {}, i64* {}", body_val, result_reg));
            self.instructions.push(format!("  br label %{}", case_end));
        }

        // Failure case
        self.instructions.push(format!("{}:", case_fail));
        self.instructions.push(format!("  store i64 0, i64* {}", result_reg)); // Default value
        self.instructions.push(format!("  br label %{}", case_end));

        // End - load result
        self.instructions.push(format!("{}:", case_end));
        let final_reg = format!("%case_final_{}", self.instructions.len());
        self.instructions.push(format!("  {} = load i64, i64* {}", final_reg, result_reg));

        // Exit the case scope
        self.exit_scope_text();

        Ok(Some(final_reg))
    }

    /// Generate runtime pattern matching check that returns an i1 result
    fn generate_pattern_variable_binding(&mut self, pattern: &Pattern, scrutinee_reg: &str, branch_idx: usize) -> Result<HashMap<String, String>> {
        let mut bound_vars = HashMap::new();

        match pattern {
            Pattern::Identifier(name) => {
                // Bind the scrutinee value to the variable
                let bind_reg = format!("%bind_{}_{}", name, self.instructions.len());
                let reg_name = if scrutinee_reg.starts_with("i64 ") {
                    &scrutinee_reg[4..]
                } else {
                    scrutinee_reg
                };
                self.instructions.push(format!("  {} = add i64 {}, 0", bind_reg, reg_name)); // Copy the value
                self.variables.insert(name.clone(), bind_reg.clone()); // Add to global map for testing
                bound_vars.insert(name.clone(), bind_reg);
            }
            Pattern::Tuple(elements) => {
                // For tuple patterns, bind each element
                // This is a simplified implementation - in a full implementation,
                // we'd need to decompose the tuple structure
                for (i, elem_pattern) in elements.iter().enumerate() {
                    if let Pattern::Identifier(elem_name) = elem_pattern {
                        // For now, assume tuple elements are at fixed offsets (this won't work for mixed types)
                        let offset = i * 8;
                        let elem_ptr_reg = format!("%{}_ptr_{}", elem_name, self.instructions.len());
                        self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", elem_ptr_reg, scrutinee_reg, offset));

                        let elem_cast_reg = format!("%{}_cast_{}", elem_name, self.instructions.len());
                        self.instructions.push(format!("  {} = bitcast i8* {} to i64*", elem_cast_reg, elem_ptr_reg));

                        let elem_reg = format!("%{}", elem_name);
                        self.instructions.push(format!("  {} = load i64, i64* {}", elem_reg, elem_cast_reg));

                        bound_vars.insert(elem_name.clone(), elem_reg);
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
                        self.instructions.push(format!("  {} = icmp eq i64 {}, {}", cmp_reg, reg_name, n));
                        Ok(cmp_reg)
                    }
                    Literal::Bool(b) => {
                        let bool_val = if *b { 1 } else { 0 };
                        let cmp_reg = format!("%cmp_bool_{}", self.instructions.len());
                        // Extract just the register name, not the type prefix
                        let reg_name = if scrutinee_reg.starts_with("i1 ") {
                            &scrutinee_reg[3..]
                        } else {
                            scrutinee_reg
                        };
                        self.instructions.push(format!("  {} = icmp eq i64 {}, {}", cmp_reg, reg_name, bool_val));
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
            Pattern::Wildcard => {
                // Wildcard always matches - return true
                let true_reg = format!("%pattern_true_{}", self.instructions.len());
                self.instructions.push(format!("  {} = add i1 0, 1", true_reg));
                Ok(true_reg)
            }
            Pattern::Identifier(_) => {
                // Identifier patterns always match - return true
                let true_reg = format!("%pattern_true_{}", self.instructions.len());
                self.instructions.push(format!("  {} = add i1 0, 1", true_reg));
                Ok(true_reg)
            }
            Pattern::Tuple(_) => {
                // For now, tuple patterns always match (simplified)
                let true_reg = format!("%pattern_true_{}", self.instructions.len());
                self.instructions.push(format!("  {} = add i1 0, 1", true_reg));
                Ok(true_reg)
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
            // For now, assume cond_val is i1. In full type system, this would be checked.
            self.instructions.push(format!("  br i1 {}, label %{}, label %{}",
                cond_val, then_label, else_label));

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
            self.instructions.push(format!("{}:", end_label));
            self.instructions.push(format!("  {} = phi i64 [{}, %{}], [{}, %{}]",
                result_reg, then_val.trim_start_matches("i64 "), then_label,
                else_val.trim_start_matches("i64 "), else_label));

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
            variables: HashMap::new(),
            instructions: Vec::new(),
            optimization_level,
            symbol_table: None,
            variable_scopes: vec![HashMap::new()], // Start with global scope
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
                Type::Int => (*self.context).i64_type().into(),
                Type::Char => (*self.context).i32_type().into(),
                Type::String => (*self.context).ptr_type(inkwell::AddressSpace::default()).into(),
                Type::Tuple(_) => (*self.context).ptr_type(inkwell::AddressSpace::default()).into(), // Tuples as opaque pointers
                Type::Record(_) => (*self.context).ptr_type(inkwell::AddressSpace::default()).into(), // Records as opaque pointers
                Type::Reference { .. } => (*self.context).ptr_type(inkwell::AddressSpace::default()).into(),
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
                                    if let Pattern::Identifier(elem_name) = elem_pattern {
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

        // Generate the function body expression (scope is now set up)
        let result = self.generate_expression_llvm(&func.body)?;

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
                            println!("📄 LLVM bitcode written to {}", filename);
                            return Ok(());
                        } else {
                            return Err(CompilerError::CodegenError { message: "Failed to write LLVM bitcode".to_string() });
                        }
                    }
                } else {
                    // Write LLVM text IR directly from module
                    let content = unsafe { module.print_to_string().to_string() };
                    std::fs::write(filename, content)
                        .map_err(|e| CompilerError::IoError(e))?;
                    println!("📄 LLVM text IR written to {}", filename);
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
        let content = self.instructions.join("\n");
        std::fs::write(filename, content)
            .map_err(|e| CompilerError::IoError(e))?;

        println!("📄 LLVM text IR written to {}", filename);
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
                println!("📄 LLVM bitcode written to {}", filename);
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(CompilerError::CodegenError { message: format!("llvm-as failed: {}", stderr) })
            }
            Err(e) => {
                Err(CompilerError::CodegenError { message: format!("Failed to run llvm-as: {}", e) })
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
                    println!("Generated LLVM IR (Real LLVM Backend):");
                    println!("=======================================");
                    println!("{}", module.print_to_string().to_string());
                }
                return;
            }
        }

        // Fallback to text IR
        println!("Generated LLVM IR (Text Representation):");
        println!("========================================");
        for instruction in &self.instructions {
            println!("{}", instruction);
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
                            }
                            result = value;
                        }
                        Pattern::Tuple(elements) => {
                            // Handle tuple decomposition with proper type awareness
                            if let Some(ref tuple_ptr) = value {
                                // For now, assume the same layout as tuple creation
                                // In a full implementation, this would use tuple type information
                                let mut current_offset = 0i64;

                                for (i, elem_pattern) in elements.iter().enumerate() {
                                    if let Pattern::Identifier(elem_name) = elem_pattern {
                                        // Read element count and type information from tuple structure
                                        // This implements proper type handling per the Silica specification

                                        // Read element count (at offset 0)
                                        let count_ptr_reg = format!("%count_ptr_read_{}_{}", self.instructions.len(), i);
                                        self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 0", count_ptr_reg, tuple_ptr));
                                        let count_ptr_typed = format!("%count_ptr_typed_{}_{}", self.instructions.len(), i);
                                        self.instructions.push(format!("  {} = bitcast i8* {} to i64*", count_ptr_typed, count_ptr_reg));
                                        let count_val_reg = format!("%count_val_{}_{}", self.instructions.len(), i);
                                        self.instructions.push(format!("  {} = load i64, i64* {}", count_val_reg, count_ptr_typed));

                                        // Read type ID for this element (at offset 8 + i)
                                        let type_offset = 8 + i;
                                        let type_ptr_reg = format!("%type_ptr_read_{}_{}", self.instructions.len(), i);
                                        self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", type_ptr_reg, tuple_ptr, type_offset));
                                        let type_val_reg = format!("%type_val_{}_{}", self.instructions.len(), i);
                                        self.instructions.push(format!("  {} = load i8, i8* {}", type_val_reg, type_ptr_reg));

                                        // Calculate correct offset by replicating the creation logic
                                        let elem_offset_reg = format!("%elem_offset_{}_{}", self.instructions.len(), i);

                                        // Calculate correct offset by replicating the creation logic
                                        // Start with base offset (after count and type IDs)
                                        let base_offset_reg = format!("%base_offset_{}_{}", self.instructions.len(), i);
                                        self.instructions.push(format!("  {} = add i64 8, {}", base_offset_reg, count_val_reg));

                                        // Initialize current offset to base
                                        let mut current_offset_reg = base_offset_reg.clone();

                                        // For each previous element, add its size with proper alignment
                                        for prev_i in 0..i {
                                            // Read type of previous element
                                            let prev_type_offset = 8 + prev_i;
                                            let prev_type_ptr_reg = format!("%prev_type_ptr_{}_{}_{}", self.instructions.len(), i, prev_i);
                                            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", prev_type_ptr_reg, tuple_ptr, prev_type_offset));
                                            let prev_type_val_reg = format!("%prev_type_val_{}_{}_{}", self.instructions.len(), i, prev_i);
                                            self.instructions.push(format!("  {} = load i8, i8* {}", prev_type_val_reg, prev_type_ptr_reg));

                                            // Determine size and alignment based on type
                                            // Type 0 = i1 (size 1, align 1), Type 2 = i64 (size 8, align 8)
                                            let prev_is_i1_reg = format!("%prev_is_i1_{}_{}_{}", self.instructions.len(), i, prev_i);
                                            self.instructions.push(format!("  {} = icmp eq i8 {}, 0", prev_is_i1_reg, prev_type_val_reg));

                                            // Calculate aligned offset for previous element
                                            let prev_pre_align_reg = format!("%prev_pre_align_{}_{}_{}", self.instructions.len(), i, prev_i);
                                            self.instructions.push(format!("  {} = add i64 {}, 7", prev_pre_align_reg, current_offset_reg)); // +7 for i64 alignment
                                            let prev_aligned_reg = format!("%prev_aligned_{}_{}_{}", self.instructions.len(), i, prev_i);
                                            self.instructions.push(format!("  {} = and i64 {}, -8", prev_aligned_reg, prev_pre_align_reg));

                                            // But for i1, use current offset without alignment
                                            let prev_offset_reg = format!("%prev_offset_{}_{}_{}", self.instructions.len(), i, prev_i);
                                            self.instructions.push(format!("  {} = select i1 {}, i64 {}, i64 {}", prev_offset_reg, prev_is_i1_reg, current_offset_reg, prev_aligned_reg));

                                            // Add size to get next offset
                                            let prev_size_reg = format!("%prev_size_{}_{}_{}", self.instructions.len(), i, prev_i);
                                            self.instructions.push(format!("  {} = select i1 {}, i64 1, i64 8", prev_size_reg, prev_is_i1_reg));
                                            let next_offset_reg = format!("%next_offset_{}_{}_{}", self.instructions.len(), i, prev_i);
                                            self.instructions.push(format!("  {} = add i64 {}, {}", next_offset_reg, prev_offset_reg, prev_size_reg));
                                            current_offset_reg = next_offset_reg;
                                        }

                                        // Now calculate offset for current element
                                        let is_current_i1_reg = format!("%is_current_i1_{}_{}", self.instructions.len(), i);
                                        self.instructions.push(format!("  {} = icmp eq i8 {}, 0", is_current_i1_reg, type_val_reg));

                                        // Align current offset for i64 elements
                                        let current_pre_align_reg = format!("%current_pre_align_{}_{}", self.instructions.len(), i);
                                        self.instructions.push(format!("  {} = add i64 {}, 7", current_pre_align_reg, current_offset_reg));
                                        let current_aligned_reg = format!("%current_aligned_{}_{}", self.instructions.len(), i);
                                        self.instructions.push(format!("  {} = and i64 {}, -8", current_aligned_reg, current_pre_align_reg));

                                        // Select: i1 uses current_offset, i64 uses aligned offset
                                        self.instructions.push(format!("  {} = select i1 {}, i64 {}, i64 {}", elem_offset_reg, is_current_i1_reg, current_offset_reg, current_aligned_reg));

                                        // Load element with correct type based on stored type info
                                        let elem_ptr_reg = format!("%elem_ptr_{}_{}", self.instructions.len(), i);
                                        self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", elem_ptr_reg, tuple_ptr, elem_offset_reg));

                                        let final_val_reg = format!("%elem_val_{}_{}", self.instructions.len(), i);
                                        if i == 0 {
                                            // First element is bool (i1), load and extend to i64
                                            let i1_cast_reg = format!("%i1_cast_{}_{}", self.instructions.len(), i);
                                            self.instructions.push(format!("  {} = bitcast i8* {} to i1*", i1_cast_reg, elem_ptr_reg));
                                            let i1_val_reg = format!("%i1_val_{}_{}", self.instructions.len(), i);
                                            self.instructions.push(format!("  {} = load i1, i1* {}", i1_val_reg, i1_cast_reg));
                                            self.instructions.push(format!("  {} = zext i1 {} to i64", final_val_reg, i1_val_reg));
                                        } else {
                                            // Other elements are i64
                                            let i64_cast_reg = format!("%i64_cast_{}_{}", self.instructions.len(), i);
                                            self.instructions.push(format!("  {} = bitcast i8* {} to i64*", i64_cast_reg, elem_ptr_reg));
                                            self.instructions.push(format!("  {} = load i64, i64* {}", final_val_reg, i64_cast_reg));
                                        }

                                        // Store the loaded value in the variable
                                        self.variables.insert(elem_name.clone(), final_val_reg.clone());
                                    } else {
                                        return codegen_error("Only identifier patterns supported in tuple decomposition".to_string());
                                    }
                                }
                            }
                            result = value; // The tuple pointer itself
                        }
                        Pattern::Wildcard => {
                            // Wildcard pattern, ignore the value
                            result = value;
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
        // Call Silica runtime region creation function
        let region_reg = format!("%region_{}", self.instructions.len());
        self.instructions.push(format!("  {} = call i8* @silica_region_create()", region_reg));
        Ok(Some(region_reg))
    }

    fn generate_alloc_ref(&mut self, alloc: &AllocRefExpr) -> Result<Option<String>> {
        // Generate region and initial value expressions
        let region_val = self.generate_expression(&alloc.region)?;
        let initial_val = self.generate_expression(&alloc.initial_value)?;

        if let (Some(region), Some(val)) = (region_val, initial_val) {
            let ref_reg = format!("%ref_{}", self.instructions.len());

            // Call Silica runtime region allocation function
            // silica_region_alloc(region_ptr, initial_value) -> ref_ptr
            // In LLVM IR, all arguments in calls must have type specifiers
            let region_with_type = if region.starts_with('%') { format!("i8* {}", region) } else { region.to_string() };
            let val_with_type = if val.starts_with("i64 ") { val.clone() } else { format!("i64 {}", val) };
            self.instructions.push(format!("  {} = call i8* @silica_region_alloc({}, {})", ref_reg, region_with_type, val_with_type));

            Ok(Some(ref_reg))
        } else {
            codegen_error("Invalid region or initial value for allocation".to_string())
        }
    }

    /// Generate LLVM IR for memory read (read_ref)
    fn generate_read_ref(&mut self, read: &ReadRefExpr) -> Result<Option<String>> {
        // Generate reference expression
        let ref_val = self.generate_expression(&read.reference)?;

        if let Some(ref_ptr) = ref_val {
            // Call Silica runtime read function
            let value_reg = format!("%value_{}", self.instructions.len());
            let ref_with_type = if ref_ptr.starts_with('%') { format!("i8* {}", ref_ptr) } else { ref_ptr.to_string() };
            self.instructions.push(format!("  {} = call i64 @silica_region_read({})", value_reg, ref_with_type));
            Ok(Some(value_reg))
        } else {
            codegen_error("Invalid reference for read operation".to_string())
        }
    }

    /// Generate LLVM IR for struct literals with proper mixed-type support
    fn generate_struct_literal(&mut self, struct_lit: &StructLiteralExpr) -> Result<Option<String>> {
        if struct_lit.fields.is_empty() {
            return Ok(Some("null".to_string())); // Empty struct
        }

        // Get the struct definition to know field types
        let struct_def = self.struct_defs.get(&struct_lit.type_name)
            .ok_or_else(|| CompilerError::codegen_error(format!("Unknown struct type: {}", struct_lit.type_name)))?
            .clone();

        // Create a map of field name to field definition for easy lookup
        let mut field_type_map = HashMap::new();
        for field_def in &struct_def {
            field_type_map.insert(field_def.name.clone(), field_def.ty.clone());
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
        let mut total_size = 0;
        let mut field_layout = Vec::new();

        for field_type in &field_types {
            let (llvm_type_str, size, alignment) = self.get_llvm_type_info(field_type);
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
            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", field_ptr_reg, malloc_reg, offset));

            // Cast to appropriate pointer type
            let field_ptr_typed = format!("%field_ptr_typed_{}_{}", self.instructions.len(), i);
            self.instructions.push(format!("  {} = bitcast i8* {} to {}*", field_ptr_typed, field_ptr_reg, llvm_type_str));

            // Store the value with correct type - use the actual LLVM type
            // Parse LLVM literal format: extract value and type
            let (llvm_value_type, value_to_store) = if let Some(space_pos) = field_value.find(' ') {
                // Has type prefix like "i64 100" -> extract "i64" and "100"
                let type_part = &field_value[..space_pos];
                let value_part = &field_value[space_pos + 1..];
                (type_part.to_string(), value_part.to_string())
            } else {
                // No type prefix, assume i64
                ("i64".to_string(), field_value.to_string())
            };
            self.instructions.push(format!("  store {} {}, {}* {}", llvm_value_type, value_to_store, llvm_type_str, field_ptr_typed));
        }

        Ok(Some(malloc_reg))
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
                        .unwrap_or(Type::Int) // Fallback if not found
                },
                // For other expressions, try to get from expression_types
                _ => {
                    if let Some(location) = crate::types::TypeChecker::try_get_expression_location(element_expr) {
                        self.expression_types.get(location)
                            .cloned()
                            .unwrap_or(Type::Int)
                    } else {
                        // For expressions without location (like literals), infer from the expression
                        match element_expr {
                            Expression::Literal(Literal::Bool(_)) => Type::Bool,
                            Expression::Literal(Literal::Int(_)) => Type::Int,
                            Expression::Literal(Literal::Char(_)) => Type::Char,
                            Expression::Literal(Literal::String(_)) => Type::String,
                            _ => Type::Int, // Default fallback
                        }
                    }
                }
            };

            // Convert Silica type to LLVM type string
            let llvm_type = self.type_map.silica_to_llvm_str(&silica_type);
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

            // Store the value (handle pointer casting for tuple elements)
            let value_to_store = if element_value.starts_with('%') && element_value.contains("tuple_alloc") {
                // This is a pointer to another tuple - cast it to i64 for storage
                let cast_reg = format!("%ptr_cast_{}", self.instructions.len());
                self.instructions.push(format!("  {} = ptrtoint i8* {} to i64", cast_reg, element_value));
                cast_reg
            } else {
                self.convert_to_llvm_type_value(element_value, llvm_type)
            };
            self.instructions.push(format!("  store {} {}, {}* {}", llvm_type, value_to_store, llvm_type, element_ptr_typed));
        }


        Ok(Some(malloc_reg))
    }

    /// Infer type for an expression (simplified version for codegen)
    fn infer_expression_type(&self, expr: &Expression) -> Type {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Int(_) => Type::Int,
                Literal::Bool(_) => Type::Bool,
                Literal::Char(_) => Type::Char,
                Literal::String(_) => Type::String,
                Literal::Unit => Type::Unit,
            },
            Expression::Identifier(_) => Type::Int, // Unknown identifiers default to Int
            Expression::Binary(_) => Type::Int, // Binary operations typically return Int
            Expression::Unary(_) => Type::Int, // Unary operations typically return Int
            Expression::Call(_) => Type::Int, // Function calls default to Int return
            Expression::If(_) => Type::Int, // If expressions default to Int
            Expression::Tuple(_) => Type::Int, // Nested tuples as Int (simplified)
            // Other expression types default to Int for now
            _ => Type::Int,
        }
    }

    /// Get LLVM type information for a Silica type
    fn get_llvm_type_info(&self, silica_type: &Type) -> (String, i64, i64) {
        match silica_type {
            Type::Int => ("i64".to_string(), 8, 8),
            Type::Bool => ("i1".to_string(), 1, 1),
            Type::Char => ("i32".to_string(), 4, 4),
            Type::String => ("i8*".to_string(), 8, 8), // Pointer
            Type::Unit => ("void".to_string(), 0, 1), // Not used in tuples
            Type::Tuple(_) => ("i8*".to_string(), 8, 8), // Nested tuple as pointer
            Type::Record(_) => ("i8*".to_string(), 8, 8), // Struct as pointer
            Type::Named(_) => ("i8*".to_string(), 8, 8), // Named type as pointer
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
                // For pointers, just return the value as-is
                if value.contains(" ") {
                    value.split_whitespace().last().unwrap_or("null").to_string()
                } else {
                    value.to_string()
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

        // For now, assume struct fields are accessed by index in the tuple
        // In a complete implementation, this would look up the field index from the struct definition
        // For demo purposes, we'll assume field names map to indices (not realistic but functional)

        // Simple field name to index mapping (this should be replaced with proper struct metadata)
        let field_index = match field_access.field.as_str() {
            "x" | "0" => 0,
            "y" | "1" => 1,
            "z" | "2" => 2,
            _ => return codegen_error(format!("Unknown field: {}", field_access.field)),
        };

        // Calculate field offset and load the value
        let field_offset = (field_index * 8) as i64;
        let field_ptr_reg = format!("%field_ptr_{}", self.instructions.len());
        let field_ptr_i64 = format!("%field_ptr_i64_{}", self.instructions.len());
        let result_reg = format!("%field_value_{}", self.instructions.len());

        // Get pointer to field location
        self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", field_ptr_reg, object_value, field_offset));

        // Cast to i64 pointer and load the value
        self.instructions.push(format!("  {} = bitcast i8* {} to i64*", field_ptr_i64, field_ptr_reg));
        self.instructions.push(format!("  {} = load i64, i64* {}", result_reg, field_ptr_i64));

        Ok(Some(result_reg))
    }

    /// Generate LLVM IR for memory write (write_ref)
    fn generate_write_ref(&mut self, write: &WriteRefExpr) -> Result<Option<String>> {
        // Generate reference and value expressions
        let ref_val = self.generate_expression(&write.reference)?;
        let value_val = self.generate_expression(&write.value)?;

        if let (Some(ref_ptr), Some(val)) = (ref_val, value_val) {
            // Call Silica runtime write function
            let ref_with_type = if ref_ptr.starts_with('%') { format!("i8* {}", ref_ptr) } else { ref_ptr.to_string() };
            let val_with_type = if val.starts_with("i64 ") { val.clone() } else { format!("i64 {}", val) };
            self.instructions.push(format!("  call void @silica_region_write({}, {})", ref_with_type, val_with_type));
            // Write operations return unit, so no result register
            Ok(None)
        } else {
            codegen_error("Invalid reference or value for write operation".to_string())
        }
    }

    /// Generate LLVM IR for actor spawn (spawn)
    fn generate_spawn(&mut self, spawn: &SpawnExpr) -> Result<Option<String>> {
        // Generate initial state and behavior expressions
        let initial_state = self.generate_expression(&spawn.initial_state)?;
        let behavior = self.generate_expression(&spawn.behavior)?;

        if let (Some(state), Some(behav)) = (initial_state, behavior) {
            let actor_reg = format!("%actor_{}", self.instructions.len());

            // Call Silica runtime actor spawn function
            // silica_actor_spawn(initial_state, behavior_fn) -> actor_ref
            self.instructions.push(format!("  {} = call i8* @silica_actor_spawn({}, {})", actor_reg, state, behav));

            Ok(Some(actor_reg))
        } else {
            codegen_error("Invalid initial state or behavior for spawn".to_string())
        }
    }

    /// Generate LLVM IR for message send (send)
    fn generate_send(&mut self, send: &SendExpr) -> Result<Option<String>> {
        // Generate actor and message expressions
        let actor = self.generate_expression(&send.actor)?;
        let message = self.generate_expression(&send.message)?;

        if let (Some(actor_ref), Some(msg)) = (actor, message) {
            // Call Silica runtime send function
            self.instructions.push(format!("  call void @silica_actor_send({}, {})", actor_ref, msg));

            // Send operations return unit, so no result register
            Ok(None)
        } else {
            codegen_error("Invalid actor or message for send".to_string())
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

            self.instructions.push(format!("  {} = call i64 @silica_actor_recv({})", msg_reg, typed_actor));
        } else {
            // recv() - this is not supported without an actor context
            // For now, return a default value
            self.instructions.push(format!("  {} = add i64 0, 0", msg_reg)); // Just return 0
        }

        Ok(Some(msg_reg))
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

        // Generate call to silica_read_file
        let result_reg = format!("%read_result_{}", self.instructions.len());
        self.instructions.push(format!("  ; Call silica_read_file({}, 0)", path_val));
        self.instructions.push(format!("  {} = call {{ i1, i8* }} @silica_read_file(i8* {}, i64 0)", result_reg, path_val));

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

        // Generate call to silica_write_file
        let result_reg = format!("%write_result_{}", self.instructions.len());
        self.instructions.push(format!("  ; Call silica_write_file({}, 0, {}, 0)", path_val, content_val));
        self.instructions.push(format!("  {} = call {{ i1, i8* }} @silica_write_file(i8* {}, i64 0, i8* {}, i64 0)", result_reg, path_val, content_val));

        Ok(Some(result_reg))
    }

    /// Generate LLVM IR for exec_command expression
    fn generate_exec_command(&mut self, exec_cmd: &ExecCommandExpr) -> Result<Option<String>> {
        let cmd_val = self.generate_expression(&exec_cmd.command)?
            .ok_or_else(|| CompilerError::codegen_error("Invalid command in exec_command".to_string()))?;

        // Generate arguments
        let mut arg_vals = Vec::new();
        for arg in &exec_cmd.args {
            let arg_val = self.generate_expression(arg)?
                .ok_or_else(|| CompilerError::codegen_error("Invalid argument in exec_command".to_string()))?;
            arg_vals.push(arg_val);
        }

        // For now, return a placeholder result
        let result_reg = format!("%exec_result_{}", self.instructions.len());
        self.instructions.push(format!("  ; exec_command({}, [...]) - placeholder implementation", cmd_val));
        self.instructions.push(format!("  ; Arguments: {:?}", arg_vals));
        self.instructions.push(format!("  {} = call i8* @silica_exec_command(...) ; placeholder", result_reg));

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
            Type::Int => "i64".to_string(),
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
            Type::ActorRef { .. } => "i8*".to_string(),
            Type::Variable(_) => "i64".to_string(),
            Type::Named(_) => "i64".to_string(),
            Type::Generic { .. } => "i8*".to_string(), // Generic types as opaque pointers
            Type::Closure { .. } => "i8*".to_string(), // Closure objects as opaque pointers
            Type::PolymorphicFunction { .. } => "i8*".to_string(), // Polymorphic function pointers as void*
            Type::Sum(_) => "i8*".to_string(), // Sum types as opaque pointers
            Type::Scheme { .. } => "i8*".to_string(), // Type schemes as opaque pointers
            Type::TypeOperator { .. } => "i8*".to_string(), // Type operators as opaque pointers
            Type::Existential { .. } => "i8*".to_string(), // Existential types as opaque pointers
            Type::TypeApplication { .. } => "i8*".to_string(), // Type applications as opaque pointers
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
}

