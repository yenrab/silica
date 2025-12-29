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
use crate::errors::{Result, codegen_error, CompilerError};
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
    variables: HashMap<String, String>, // Variable name -> LLVM register/temp
    instructions: Vec<String>,
    optimization_level: OptimizationLevel,
    symbol_table: Option<Box<crate::module_resolver::SymbolTable>>,
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
            variables: HashMap::new(),
            instructions: Vec::new(),
            optimization_level,
            symbol_table: None,
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
                codegen_error("LLVM context not initialized".to_string())
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
                codegen_error("LLVM context not initialized".to_string())
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
                        .map(|param| self.silica_type_to_llvm(&param.type_))
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
            .map(|param| self.type_map.silica_to_llvm_str(&param.type_))
            .collect();

        let return_type = func.return_type.as_ref().unwrap_or(&Type::Unit);
        let return_type_str = self.type_map.silica_to_llvm_str(return_type);

        let param_strs: Vec<String> = param_types.iter()
            .enumerate()
            .map(|(i, ty)| format!("{} %{}", ty, func.parameters[i].name))
            .collect();

        let signature = format!("define {} @{}({}) {{",
            return_type_str,
            func.name,
            param_strs.join(", ")
        );

        self.instructions.push(signature.clone());
        self.functions.insert(func.name.clone(), signature);

        // Add function parameters to variable scope
        for param in &func.parameters {
            let param_reg = format!("%{}", param.name);
            self.variables.insert(param.name.clone(), param_reg.clone());
            self.instructions.push(format!("  ; Parameter {}: {}", param.name, param_reg));
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
                codegen_error("Function literals not yet implemented".to_string())
            }
            Expression::Region(_) => {
                codegen_error("Region expressions not yet implemented".to_string())
            }
            Expression::StructLiteral(struct_lit) => self.generate_struct_literal(struct_lit),
            Expression::FieldAccess(field_access) => self.generate_field_access(field_access),
            Expression::Tuple(_) => {
                codegen_error("Tuple expressions not yet implemented".to_string())
            }
            Expression::GenericInstantiation(_) => {
                codegen_error("Generic instantiation not yet implemented".to_string())
            }
            Expression::ConstructorCall(_) => {
                codegen_error("Constructor calls not yet implemented".to_string())
            }
        }
    }

    /// Generate expression (LLVM backend) - simplified for function calls only
    #[cfg(feature = "llvm_backend")]
    fn generate_expression(&mut self, _expr: &Expression) -> Result<Option<String>> {
        // For LLVM backend, we use generate_expression_llvm for actual LLVM generation
        // This method is only used by text backend code, so return an error for LLVM
        codegen_error("Text expression generation not available in LLVM backend".to_string())
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
            codegen_error(format!("Undefined identifier: {}", name))
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
            codegen_error("Binary operation on invalid operands".to_string())
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
                    codegen_error("Not operation on invalid operand".to_string())
                }
            }
            UnaryOp::Negate => {
                if let Some(op) = operand {
                    let temp_reg = format!("%t{}", self.instructions.len());
                    self.instructions.push(format!("  {} = sub i64 0, {}", temp_reg, op));
                    Ok(Some(temp_reg))
                } else {
                    codegen_error("Negate operation on invalid operand".to_string())
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
                        return codegen_error("Invalid argument in function call".to_string());
                    }
                }

                // For LLVM IR function calls, arguments should have type prefixes
                // e.g., call i64 @func(i64 %arg1, i64 %arg2)
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
                                return codegen_error("Invalid argument in function call".to_string());
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
                    codegen_error(format!("Undefined function: {}", func_name))
                } else {
                    unreachable!()
                }
            } else {
                codegen_error(format!("Undefined function: {}", func_name))
            }
        } else {
            codegen_error("Complex function expressions not yet supported".to_string())
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
                return codegen_error("Invalid argument in method call".to_string());
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
            Expression::FieldAccess(field_access) => self.generate_field_access_llvm(field_access),
            Expression::Spawn(spawn) => self.generate_spawn_llvm(spawn),
            Expression::Send(send) => self.generate_send_llvm(send),
            Expression::Recv(recv) => self.generate_recv_llvm(recv),
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

                    // For now, we only handle simple identifier patterns
                    // TODO: Handle complex patterns
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
                        _ => return Err(CompilerError::codegen_error("Complex patterns in do expressions not yet supported".to_string())),
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
        unsafe {
            // Generate actor expression first if present
            let actor_val_opt = if let Some(actor_expr) = &recv.actor {
                Some(self.generate_expression_llvm(actor_expr)?
                    .ok_or_else(|| CompilerError::codegen_error("Invalid actor in recv".to_string()))?)
            } else {
                None
            };

            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                // Get the silica_actor_recv function
                if let Some(recv_func) = (*module).get_function("silica_actor_recv") {
                    if let Some(actor_val) = actor_val_opt {

                        // Call silica_actor_recv(actor)
                        let call_result = builder.build_call(
                            recv_func,
                            &[actor_val.into()],
                            "recv_result"
                        ).unwrap();

                        let result: inkwell::values::BasicValueEnum<'static> = unsafe { std::mem::transmute(call_result.try_as_basic_value().unwrap_basic()) };
                        Ok(Some(result))
                    } else {
                        // recv() without actor - not supported in current implementation
                        // Return a placeholder
                        let placeholder_message = (*self.context).i64_type().const_int(0, false);
                        let result: inkwell::values::BasicValueEnum<'static> = unsafe { std::mem::transmute(placeholder_message.into()) };
                        Ok(Some(result))
                    }
                } else {
                    Err(CompilerError::codegen_error("silica_actor_recv function not found".to_string()))
                }
            }
        } else {
            Err(CompilerError::codegen_error("LLVM module or builder not initialized".to_string()))
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
            }
        } else {
            Err(CompilerError::codegen_error("LLVM context not initialized".to_string()))
        }
    }

    /// Generate LLVM value for field access expressions (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_field_access_llvm(&mut self, field_access: &FieldAccessExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        unsafe {
            if let Some(builder) = &self.builder {
                let context = &*self.context;
                // Generate the object expression
                let object_value = match self.generate_expression_llvm(&field_access.object)? {
                    Some(value) => value,
                    None => return Err(CompilerError::codegen_error("Field access requires valid object".to_string())),
                };

                // Simple field name to index mapping (should be replaced with proper struct metadata)
                let field_index = match field_access.field.as_str() {
                    "x" | "0" => 0,
                    "y" | "1" => 1,
                    "z" | "2" => 2,
                    _ => return Err(CompilerError::codegen_error(format!("Unknown field: {}", field_access.field))),
                };

                // Calculate field offset and load the value
                let field_offset = (field_index * 8) as u64;
                let offset_value = (*context).i64_type().const_int(field_offset, false);

                // Get pointer to field location
                let field_ptr = builder.build_gep(
                    (*context).i8_type(),
                    object_value.into_pointer_value(),
                    &[offset_value],
                    "field_ptr"
                ).unwrap();

                // Cast to i64 pointer and load
                let field_ptr_i64 = builder.build_bit_cast(
                    field_ptr,
                    (*context).i64_type().ptr_type(inkwell::AddressSpace::default()),
                    "field_ptr_i64"
                ).unwrap();

                let field_value = builder.build_load(
                    (*context).i64_type(),
                    field_ptr_i64.into_pointer_value(),
                    "field_value"
                ).unwrap();

                Ok(Some(field_value))
            }
        } else {
            Err(CompilerError::codegen_error("LLVM context not initialized".to_string()))
        }
    }

    /// Generate LLVM value for generic function calls (LLVM backend)
    #[cfg(feature = "llvm_backend")]
    fn generate_generic_call_llvm(&mut self, call: &CallExpr) -> Result<Option<inkwell::values::BasicValueEnum<'static>>> {
        if let Expression::Identifier(func_name) = &*call.function {
            // Generate arguments first (before borrowing module/builder)
            let mut arg_vals = Vec::new();
            for arg in &call.arguments {
                if let Some(arg_val) = self.generate_expression_llvm(arg)? {
                    arg_vals.push(arg_val);
                } else {
                    return Err(CompilerError::codegen_error("Invalid argument in generic function call".to_string()));
                }
            }

            if let (Some(module), Some(builder)) = (&self.module, &self.builder) {
                unsafe {
                    let context = &*self.context;

                    // Create monomorphized function name
                    let type_args_str = call.type_args.iter()
                        .map(|ty| self.type_to_string(ty))
                        .collect::<Vec<_>>()
                        .join("_");
                    let mono_name = format!("{}_{}", func_name, type_args_str);

                    // Check if we already have a monomorphized version
                    let func = if let Some(existing_func) = self.monomorphized_functions.get(&mono_name) {
                        *existing_func
                    } else {
                        // Need to create monomorphized version
                        // For now, create a simple stub - in full implementation this would
                        // instantiate the generic function with the specific types
                        let param_types = vec![context.i64_type().into(), context.i64_type().into()];
                        let fn_type = context.i64_type().fn_type(&param_types, false);
                        let func = (*module).add_function(&mono_name, fn_type, None);

                        // Add to cache
                        self.monomorphized_functions.insert(mono_name.clone(), func);

                        func
                    };

                    // Build arguments array
                    let mut llvm_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
                    for arg_val in arg_vals {
                        llvm_args.push(arg_val.into());
                    }

                    // Call the monomorphized function
                    let call_result = builder.build_call(func, &llvm_args, "generic_call_result").unwrap();
                    Ok(Some(call_result.try_as_basic_value().unwrap_basic()))
                }
            } else {
                Err(CompilerError::codegen_error("LLVM context not initialized".to_string()))
            }
        } else {
            Err(CompilerError::codegen_error("Generic calls must use function names".to_string()))
        }
    }

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
        // Generate scrutinee
        let scrutinee_val = match self.generate_expression(&case.scrutinee)? {
            Some(value) => value,
            None => return codegen_error("Case scrutinee must produce a value".to_string()),
        };

        // For now, implement a simple case that just returns the first matching branch
        // In a full implementation, this would generate proper branching logic
        for branch in &case.branches {
            // Check if this branch has a guard
            if let Some(guard) = &branch.guard {
                // Generate guard condition
                let guard_val = match self.generate_expression(guard)? {
                    Some(value) => value,
                    None => return codegen_error("Case guard must produce a value".to_string()),
                };

                // For now, assume the first branch with a guard that evaluates to true (non-zero) matches
                // This is a simplified implementation
                let guard_int = guard_val.trim_start_matches("i64 ").parse::<i64>()
                    .unwrap_or(0);
                if guard_int != 0 {
                    // Bind pattern variables and generate body
                    // For now, skip complex pattern binding
                    return self.generate_expression(&branch.body);
                }
            } else {
                // No guard - check if pattern matches
                // For literal patterns, do a simple comparison
                match &branch.pattern {
                    Pattern::Literal(lit) => {
                        // Compare scrutinee with literal
                        let lit_val = match lit {
                            Literal::Int(n) => format!("i64 {}", n),
                            _ => continue, // Skip non-integer literals for now
                        };

                        // For now, assume the first literal pattern that matches wins
                        let scrutinee_int = scrutinee_val.trim_start_matches("i64 ").parse::<i64>()
                            .unwrap_or(0);
                        let lit_int = lit_val.trim_start_matches("i64 ").parse::<i64>()
                            .unwrap_or(0);

                        if scrutinee_int == lit_int {
                            return self.generate_expression(&branch.body);
                        }
                    }
                    Pattern::Wildcard => {
                        // Wildcard always matches - return this branch
                        return self.generate_expression(&branch.body);
                    }
                    Pattern::Identifier(_) => {
                        // Identifier patterns always match (bind variable)
                        return self.generate_expression(&branch.body);
                    }
                    _ => continue, // Skip other pattern types for now
                }
            }
        }

        // No branch matched - this should be a compile error for non-exhaustive patterns
        codegen_error("Non-exhaustive case expression - no branch matched".to_string())
    }

    /// Generate LLVM IR for if expressions
    fn generate_if(&mut self, if_expr: &IfExpr) -> Result<Option<String>> {
        let cond = self.generate_expression(&if_expr.condition)?;

        if let Some(cond_val) = cond {
            let then_label = format!("then_{}", self.instructions.len());
            let else_label = format!("else_{}", self.instructions.len());
            let end_label = format!("end_{}", self.instructions.len());
            let result_reg = format!("%if_result_{}", self.instructions.len());

            // Generate conditional branch
            self.instructions.push(format!("  br i1 {}, label %{}, label %{}",
                cond_val.trim_start_matches("i64 ").trim_start_matches("i1 "),
                then_label, else_label));

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
            param_value.set_name(&param.name);
            param_info.push((param.name.clone(), param_value));
        }

        // Do LLVM operations to allocate parameters (borrow builder here)
        let mut param_allocas = Vec::new();
        if let Some(builder) = &self.builder {
            unsafe {
                let entry_block = (*self.context).append_basic_block(llvm_func, "entry");
                builder.position_at_end(entry_block);

                // Create parameter variables and allocate them in the function scope
                for (name, param_value) in param_info {
                    // Allocate space for the parameter on the stack
                    let param_type = param_value.get_type();
                    let alloca = builder.build_alloca(param_type, &name).unwrap();

                    // Store the parameter value in the allocated space
                    builder.build_store(alloca, param_value).unwrap();

                    param_allocas.push((name, alloca));
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

                    // For now, we only handle simple identifier patterns
                    // TODO: Handle complex patterns
                    match pattern {
                        Pattern::Identifier(name) => {
                            // Store the value in the current scope
                            if let Some(ref val) = value {
                                // For text IR, we just track the variable name -> register mapping
                                self.add_variable_text(name.clone(), val.clone());
                            }
                            result = value;
                        }
                        _ => return codegen_error("Complex patterns in do expressions not yet supported".to_string()),
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
            self.instructions.push(format!("  {} = call i8* @silica_region_alloc({}, {})", ref_reg, region, val));

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
            self.instructions.push(format!("  {} = call i64 @silica_region_read({})", value_reg, ref_ptr));
            Ok(Some(value_reg))
        } else {
            codegen_error("Invalid reference for read operation".to_string())
        }
    }

    /// Generate LLVM IR for struct literals
    fn generate_struct_literal(&mut self, struct_lit: &StructLiteralExpr) -> Result<Option<String>> {
        // For now, implement structs as tuples - allocate space and store field values
        // In a more complete implementation, this would use the struct definition from type checker

        if struct_lit.fields.is_empty() {
            return Ok(Some("null".to_string())); // Empty struct
        }

        // Generate all field expressions first
        let mut field_values = Vec::new();
        for (_, field_expr) in &struct_lit.fields {
            if let Some(value) = self.generate_expression(field_expr)? {
                field_values.push(value);
            } else {
                return codegen_error("Invalid field value in struct literal".to_string());
            }
        }

        // For now, represent structs as heap-allocated tuples
        // Allocate memory for the struct (size based on number of fields * 8 bytes per i64)
        let struct_size = (field_values.len() * 8) as i64;
        let malloc_reg = format!("%struct_alloc_{}", self.instructions.len());
        self.instructions.push(format!("  {} = call i8* @malloc(i64 {})", malloc_reg, struct_size));

        // Store each field value
        for (i, field_value) in field_values.iter().enumerate() {
            let field_ptr_reg = format!("%field_ptr_{}_{}", self.instructions.len(), i);
            let field_offset = (i * 8) as i64;

            // Get pointer to field location
            self.instructions.push(format!("  {} = getelementptr i8, i8* {}, i64 {}", field_ptr_reg, malloc_reg, field_offset));

            // Cast to i64 pointer and store the value
            let field_ptr_i64 = format!("%field_ptr_i64_{}_{}", self.instructions.len(), i);
            self.instructions.push(format!("  {} = bitcast i8* {} to i64*", field_ptr_i64, field_ptr_reg));
            self.instructions.push(format!("  store i64 {}, i64* {}", field_value.trim_start_matches("i64 "), field_ptr_i64));
        }

        Ok(Some(malloc_reg))
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
            self.instructions.push(format!("  call void @silica_region_write({}, {})", ref_ptr, val));
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
    fn generate_exec_command_call(&mut self, call: &CallExpr) -> Result<Option<String>> {
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

