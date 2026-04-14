/*
   Copyright 2026 Lee Scott Barney

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

pub mod errors;
pub mod ast;
pub mod lexer;
pub mod parser;
pub mod types;
pub mod effects;
pub mod codegen;
pub mod runtime;
pub mod io;
pub mod module_resolver;

use errors::{CompilerError, Result};
use lexer::Lexer;
use parser::Parser;
use types::TypeChecker;
use effects::EffectAnalyzer;
use codegen::{CodeGenerator, OptimizationLevel};
use module_resolver::{ModuleResolver, SymbolTable};

/// Result of compilation
#[derive(Debug)]
pub enum CompileResult {
    /// Compilation produced output
    Success,
    /// Compilation was skipped (e.g., file has no declarations)
    Skipped,
}

pub struct Compiler {
    codegen: CodeGenerator,
    module_resolver: ModuleResolver,
    symbol_table: SymbolTable,
    #[cfg(feature = "llvm_backend")]
    context: Box<inkwell::context::Context>,
}

impl Compiler {
    pub fn new() -> Self {
        Self::with_optimization(OptimizationLevel::None)
    }

    pub fn with_optimization(optimization_level: OptimizationLevel) -> Self {
        Self::with_optimization_and_search_paths(optimization_level, vec![std::path::PathBuf::from(".")])
    }

    pub fn with_optimization_and_search_paths(optimization_level: OptimizationLevel, search_paths: Vec<std::path::PathBuf>) -> Self {
        #[cfg(feature = "llvm_backend")]
        {
            let context = Box::new(inkwell::context::Context::create());
            let module_resolver = ModuleResolver::new(search_paths);
            let symbol_table = SymbolTable::new();
            let mut codegen = CodeGenerator::new_with_optimization("silica_module", optimization_level);
            // Symbol table will be set later after it's populated
            Compiler {
                codegen,
                module_resolver,
                symbol_table,
                context,
            }
        }

        #[cfg(not(feature = "llvm_backend"))]
        {
            let module_resolver = ModuleResolver::new(search_paths);
            let symbol_table = SymbolTable::new();
        Compiler {
            codegen: CodeGenerator::new_with_optimization("silica_module", optimization_level),
                module_resolver,
                symbol_table,
            }
        }
    }

    pub fn compile(&mut self, source: &str, input_file: &str, output_file: &str) -> Result<CompileResult> {
        // Phase 1: Lexical analysis
        // println!("Phase 1: Lexical analysis...");
        let mut lexer = Lexer::new(source.to_string(), input_file.to_string());
        let tokens = lexer.tokenize()?;
        // println!("Successfully tokenized {} tokens", tokens.len());

        // Phase 2: Parsing
        // println!("Phase 2: Parsing...");
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;
        // println!("Successfully parsed program with {} declarations", program.declarations.len());

        // Skip compilation if file contains no declarations (e.g., only comments)
        if program.declarations.is_empty() {
            // println!("⚠️  File contains no declarations - skipping compilation");
            // Create an empty output file so Makefile dependencies are satisfied
            std::fs::write(output_file, "; Empty file - no declarations to compile\n")?;
            return Ok(CompileResult::Skipped);
        }

        // Phase 2.5: Module resolution and combination
        // println!("Phase 2.5: Module resolution...");
        let combined_program = self.resolve_imports_and_combine(&program, input_file)?;

        // Set symbol table in code generator
        let symbol_table_clone = Box::new(self.symbol_table.clone());
        self.codegen.set_symbol_table(symbol_table_clone);

        // Phase 3: Type checking
        // println!("Phase 3: Type checking happening...");
        // Clone symbol table for type checker to avoid borrowing issues
        let symbol_table_clone = self.symbol_table.clone();
        let mut type_checker = TypeChecker::with_symbol_table(Some(&symbol_table_clone));
        // eprintln!("DEBUG LIB: About to call check_program");
        type_checker.check_program(&combined_program)?;
        // println!("DEBUG LIB: check_program completed successfully");
        // println!("Type checking passed");

        // Update symbol table with actual types from type checking
        self.update_symbol_table_types(&type_checker)?;

        // Phase 4: Effect analysis
        // println!("Phase 4: Effect analysis...");
        // Pass type information and effect aliases from type checker to effect analyzer
        let mut effect_analyzer = EffectAnalyzer::with_types_and_effect_aliases(
            type_checker.expression_types.clone(),
            type_checker.actor_mailbox_types.clone(),
            type_checker.get_effect_aliases().clone(),
        );
        effect_analyzer.analyze_program(&combined_program)?;
        // println!("Effect analysis passed");

        // TODO: Pass struct definitions and generic instantiations when supported

        // Phase 5: Code generation
        // println!("Phase 5: LLVM code generation...");
        self.codegen.set_expression_types(type_checker.expression_types.clone());
        self.codegen.set_type_aliases(type_checker.get_type_aliases().clone());
        self.codegen.set_struct_defs(type_checker.get_struct_defs().clone());
        self.codegen.set_trait_impls(type_checker.get_trait_impls().clone());
        self.codegen.generate_program(&combined_program)?;
        // println!("Code generation completed");

        // Print the LLVM IR for verification
        // println!("\nGenerated LLVM IR (Text Representation):");
        // println!("=========================================");
        //self.codegen.print_ir();

        // Write the generated code to file
        self.codegen.write_to_file(output_file)?;
        // println!("📄 LLVM text IR written to {}", output_file);

        // println!("\nFull compilation pipeline completed successfully!");
        // println!("Program structure: {} declarations", program.declarations.len());

        // Optional: Print LLVM IR for debugging
        // codegen.print_ir();

        Ok(CompileResult::Success)
    }

    /// Resolve imports and load modules recursively, returning combined program with all declarations
    /// Update symbol table with actual types after type checking completes
    fn update_symbol_table_types(&mut self, type_checker: &TypeChecker) -> Result<()> {
        // Update symbol table with actual computed types from the type checker
        let env = type_checker.get_env();
        for (_module_name, module_symbols) in &mut self.symbol_table.modules {
            for (symbol_name, symbol_info) in module_symbols {
                // Look up the function name in the type checker's environment
                // Function names are stored without module prefixes
                if let Some(type_scheme) = env.get(symbol_name) {
                    // Update the symbol info with the actual computed type
                    symbol_info.ty = type_scheme.ty.clone();
                }
                // If not found, leave the placeholder type (Type::Unit)
                // This might happen for built-in functions or other special cases
            }
        }
        Ok(())
    }

    fn resolve_imports_and_combine(&mut self, program: &crate::ast::Program, input_file: &str) -> Result<crate::ast::Program> {
        let mut all_declarations = Vec::new();
        let mut all_declaration_modules = Vec::new();
        let mut processed_modules = std::collections::HashSet::new();
        let mut modules_to_process = Vec::new();

        // Main module name from input file path (e.g. "terms/terms.silica" -> "terms")
        let main_module_name = std::path::Path::new(input_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main")
            .to_string();

        // Collect main program declarations (in original order, excluding imports)
        let mut main_declarations = Vec::new();
        for decl in &program.declarations {
            if let crate::ast::Declaration::Import(import_decl) = decl {
                for module_name in &import_decl.modules {
                    if !processed_modules.contains(module_name) {
                        modules_to_process.push(module_name.clone());
                        processed_modules.insert(module_name.clone());
                    }
                }
            } else {
                main_declarations.push(decl.clone());
            }
        }

        // First pass: collect ALL modules that need to be loaded (recursive dependencies)
        // Build a dependency graph to ensure proper ordering
        let mut module_dependencies: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        let mut i = 0;
        while i < modules_to_process.len() {
            let module_name = modules_to_process[i].clone(); // Clone to avoid borrowing issues

            // Load the module to check its dependencies
            self.module_resolver.load_module(&module_name)?;
            let module = self.module_resolver.get_module(&module_name).unwrap();

            // Add symbols to type checker
            self.symbol_table.add_module_symbols(module)?;

            // Collect this module's dependencies
            let mut deps = Vec::new();
            for decl in &module.ast {
                if let crate::ast::Declaration::Import(import_decl) = decl {
                    for dep_module_name in &import_decl.modules {
                        if !processed_modules.contains(dep_module_name) {
                            modules_to_process.push(dep_module_name.clone());
                            processed_modules.insert(dep_module_name.clone());
                        }
                        deps.push(dep_module_name.clone());
                    }
                }
            }
            module_dependencies.insert(module_name.clone(), deps);

            i += 1;
        }

        // Ensure all modules are in the dependency graph (even if they have no dependencies)
        for module_name in &modules_to_process {
            module_dependencies.entry(module_name.clone()).or_insert_with(Vec::new);
        }
        
        // Topological sort to ensure dependencies come before dependents
        let mut sorted_modules = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();
        
        fn visit_module(
            module: &str,
            dependencies: &std::collections::HashMap<String, Vec<String>>,
            visited: &mut std::collections::HashSet<String>,
            temp_visited: &mut std::collections::HashSet<String>,
            result: &mut Vec<String>,
        ) -> Result<()> {
            if temp_visited.contains(module) {
                return Err(CompilerError::codegen_error(
                    format!("Circular dependency detected involving module '{}'", module)
                ));
            }
            if visited.contains(module) {
                return Ok(());
            }
            
            temp_visited.insert(module.to_string());
            if let Some(deps) = dependencies.get(module) {
                for dep in deps {
                    visit_module(dep, dependencies, visited, temp_visited, result)?;
                }
            }
            temp_visited.remove(module);
            visited.insert(module.to_string());
            result.push(module.to_string());
            Ok(())
        }
        
        for module_name in &modules_to_process {
            if !visited.contains(module_name) {
                visit_module(module_name, &module_dependencies, &mut visited, &mut temp_visited, &mut sorted_modules)?;
            }
        }
        
        modules_to_process = sorted_modules;

        // Second pass: add all declarations in dependency order
        // Process modules in topological order (dependencies before dependents)
        for (idx, module_name) in modules_to_process.iter().enumerate() {
            let module = self.module_resolver.get_module(module_name).unwrap();

            // Add all non-import declarations from this module
            let mut type_aliases_in_module = Vec::new();
            for decl in &module.ast {
                if !matches!(decl, crate::ast::Declaration::Import(_)) {
                    if let crate::ast::Declaration::TypeAlias(alias) = decl {
                        type_aliases_in_module.push(alias.name.clone());
                    }
                    all_declarations.push(decl.clone());
                    all_declaration_modules.push(module_name.clone());
                }
            }
        }


        // Add main program declarations (preserving original order)
        // Functions must be defined before they're used
        for _ in &main_declarations {
            all_declaration_modules.push(main_module_name.clone());
        }
        all_declarations.extend(main_declarations);

        // Create combined program
        Ok(crate::ast::Program {
            declarations: all_declarations,
            declaration_modules: all_declaration_modules,
            location: program.location.clone(),
        })
    }
}