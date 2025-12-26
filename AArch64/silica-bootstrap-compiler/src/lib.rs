pub mod errors;
pub mod ast;
pub mod lexer;
pub mod parser;
pub mod types;
pub mod effects;
pub mod codegen;
pub mod runtime;

use errors::{CompilerError, Result};
use lexer::Lexer;
use parser::Parser;
use types::TypeChecker;
use effects::EffectAnalyzer;
use codegen::{CodeGenerator, OptimizationLevel};

pub struct Compiler {
    codegen: CodeGenerator,
}

impl Compiler {
    pub fn new() -> Self {
        Self::with_optimization(OptimizationLevel::None)
    }

    pub fn with_optimization(optimization_level: OptimizationLevel) -> Self {
        Compiler {
            codegen: CodeGenerator::new_with_optimization("silica_module", optimization_level),
        }
    }

    pub fn compile(&mut self, source: &str, input_file: &str, output_file: &str) -> Result<()> {
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

        // Phase 3: Type checking
        println!("Phase 3: Type checking...");
        let mut type_checker = TypeChecker::new();
        type_checker.check_program(&program)?;
        println!("Type checking passed");

        // Phase 4: Effect analysis
        println!("Phase 4: Effect analysis...");
        let mut effect_analyzer = EffectAnalyzer::new();
        effect_analyzer.analyze_program(&program)?;
        println!("Effect analysis passed");

        // Phase 5: Code generation
        println!("Phase 5: LLVM code generation...");
        self.codegen.generate_program(&program)?;
        println!("Code generation completed");

        // Print the LLVM IR for verification
        println!("\nGenerated LLVM IR:");
        println!("==================");
        self.codegen.print_ir();

        // Write the generated LLVM bitcode to file
        self.codegen.write_to_file(output_file)?;
        println!("\nGenerated LLVM bitcode written to {}", output_file);

        println!("\nFull compilation pipeline completed successfully!");
        println!("Program structure: {} declarations", program.declarations.len());

        // Optional: Print LLVM IR for debugging
        // codegen.print_ir();

        Ok(())
    }
}
