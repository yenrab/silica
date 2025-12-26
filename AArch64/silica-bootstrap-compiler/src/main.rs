use silica_compiler::{Compiler, codegen::OptimizationLevel};
use std::env;

fn parse_optimization_level(args: &[String]) -> OptimizationLevel {
    for i in 1..args.len() {
        if args[i] == "--opt" || args[i] == "-O" {
            if i + 1 < args.len() {
                match args[i + 1].as_str() {
                    "none" | "0" => return OptimizationLevel::None,
                    "basic" | "1" | "less" => return OptimizationLevel::Less,
                    "standard" | "2" | "default" => return OptimizationLevel::Default,
                    "aggressive" | "3" => return OptimizationLevel::Aggressive,
                    _ => {
                        eprintln!("Invalid optimization level: {}", args[i + 1]);
                        eprintln!("Valid levels: none/0, basic/1/less, standard/2/default, aggressive/3");
                        return OptimizationLevel::None;
                    }
                }
            }
        }
    }
    OptimizationLevel::None
}

fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <input.silica> [output.bc] [--opt <level>]", args[0]);
        eprintln!("Optimization levels: none, less/basic, default/standard, aggressive");
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = args.get(2).map(|s| s.as_str()).unwrap_or("output.bc");
    let optimization_level = parse_optimization_level(&args);

    println!("Compiling Silica file: {}", input_file);
    println!("Output: {}", output_file);
    println!("Optimization level: {:?}", optimization_level);

    // Read source from file
    let source = match std::fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("❌ Error reading file {}: {}", input_file, err);
            std::process::exit(1);
        }
    };

    let mut compiler = Compiler::with_optimization(optimization_level);
    match compiler.compile(&source, input_file, output_file) {
        Ok(()) => {
            println!("✅ Compilation successful!");
            println!("Generated LLVM bitcode in {}", output_file);
        }
        Err(err) => {
            eprintln!("❌ Compilation error: {}", err);
            std::process::exit(1);
        }
    }
}