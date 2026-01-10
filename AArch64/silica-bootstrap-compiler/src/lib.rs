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
        println!("Phase 1: Lexical analysis...");
        let mut lexer = Lexer::new(source.to_string(), input_file.to_string());
        let tokens = lexer.tokenize()?;
        println!("Successfully tokenized {} tokens", tokens.len());

        // Phase 2: Parsing
        println!("Phase 2: Parsing...");
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;
        println!("Successfully parsed program with {} declarations", program.declarations.len());

        // Skip compilation if file contains no declarations (e.g., only comments)
        if program.declarations.is_empty() {
            println!("⚠️  File contains no declarations - skipping compilation");
            // Create an empty output file so Makefile dependencies are satisfied
            std::fs::write(output_file, "; Empty file - no declarations to compile\n")?;
            return Ok(CompileResult::Skipped);
        }

        // Phase 2.5: Module resolution and combination
        println!("Phase 2.5: Module resolution...");
        let combined_program = self.resolve_imports_and_combine(&program)?;

        // Set symbol table in code generator
        let symbol_table_clone = Box::new(self.symbol_table.clone());
        self.codegen.set_symbol_table(symbol_table_clone);

        // Phase 3: Type checking
        println!("Phase 3: Type checking happening...");
        let mut type_checker = TypeChecker::with_symbol_table(Some(&self.symbol_table));
        // eprintln!("DEBUG LIB: About to call check_program");
        type_checker.check_program(&combined_program)?;
        // println!("DEBUG LIB: check_program completed successfully");
        println!("Type checking passed");

        // Phase 4: Effect analysis
        println!("Phase 4: Effect analysis...");
        let mut effect_analyzer = EffectAnalyzer::new();
        effect_analyzer.analyze_program(&combined_program)?;
        println!("Effect analysis passed");

        // TODO: Pass struct definitions and generic instantiations when supported

        // Phase 5: Code generation
        println!("Phase 5: LLVM code generation...");
        self.codegen.set_expression_types(type_checker.expression_types.clone());
        self.codegen.set_type_aliases(type_checker.get_type_aliases().clone());
        self.codegen.set_struct_defs(type_checker.get_struct_defs().clone());
        self.codegen.set_trait_impls(type_checker.get_trait_impls().clone());
        self.codegen.generate_program(&combined_program)?;
        println!("Code generation completed");

        // Print the LLVM IR for verification
        println!("\nGenerated LLVM IR (Text Representation):");
        println!("=========================================");
        self.codegen.print_ir();

        // Write the generated code to file
        self.codegen.write_to_file(output_file)?;
        println!("📄 LLVM text IR written to {}", output_file);

        println!("\nFull compilation pipeline completed successfully!");
        println!("Program structure: {} declarations", program.declarations.len());

        // Optional: Print LLVM IR for debugging
        // codegen.print_ir();

        Ok(CompileResult::Success)
    }

    /// Resolve imports and load modules recursively, returning combined program with all declarations
    fn resolve_imports_and_combine(&mut self, program: &crate::ast::Program) -> Result<crate::ast::Program> {
        let mut all_declarations = Vec::new();
        let mut processed_modules = std::collections::HashSet::new();
        let mut modules_to_process = Vec::new();

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
        let mut i = 0;
        while i < modules_to_process.len() {
            let module_name = &modules_to_process[i];

            // Load the module to check its dependencies
            self.module_resolver.load_module(module_name)?;
            let module = self.module_resolver.get_module(module_name).unwrap();

            // Add symbols to type checker
            self.symbol_table.add_module_symbols(module)?;

            // Check this module's imports and add any new dependencies
            for decl in &module.ast {
                if let crate::ast::Declaration::Import(import_decl) = decl {
                    for dep_module_name in &import_decl.modules {
                        if !processed_modules.contains(dep_module_name) {
                            modules_to_process.push(dep_module_name.clone());
                            processed_modules.insert(dep_module_name.clone());
                        }
                    }
                }
            }

            i += 1;
        }

        // Second pass: add all declarations in dependency order
        // Process modules in reverse order (dependencies first)
        for module_name in modules_to_process.iter().rev() {
            let module = self.module_resolver.get_module(module_name).unwrap();
            println!("Loading module: {}", module_name);
            println!("Loaded module '{}' with {} exports", module.name, module.exports.len());

            // Add all non-import declarations from this module
            for decl in &module.ast {
                if !matches!(decl, crate::ast::Declaration::Import(_)) {
                    all_declarations.push(decl.clone());
                }
            }
        }


        // Add main program declarations (preserving original order)
        // Functions must be defined before they're used
        all_declarations.extend(main_declarations);

        println!("Module resolution completed - combined {} declarations from {} modules",
                 all_declarations.len(), processed_modules.len());

        // Create combined program
        Ok(crate::ast::Program {
            declarations: all_declarations,
            location: program.location.clone(),
        })
    }
}