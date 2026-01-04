use silica_compiler::{Compiler, codegen::OptimizationLevel, CompileResult, errors::CompilerError};
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

fn parse_search_paths(args: &[String]) -> Vec<std::path::PathBuf> {
    let mut search_paths = Vec::new();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--search-path" | "-I" => {
                if i + 1 < args.len() {
                    search_paths.push(std::path::PathBuf::from(&args[i + 1]));
                    i += 2; // Skip the next argument as it's the path
                } else {
                    eprintln!("Error: --search-path/-I requires a path argument");
                    std::process::exit(1);
                }
            }
            // Skip other arguments
            "--opt" | "-O" => {
                i += 2; // Skip optimization level
            }
            _ => {
                i += 1;
            }
        }
    }

    // Default to current directory if no search paths specified
    if search_paths.is_empty() {
        search_paths.push(std::path::PathBuf::from("."));
    }

    search_paths
}

fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <input.silica> [output.bc] [--opt <level>] [--search-path <path>] [-I <path>]", args[0]);
        eprintln!("Optimization levels: none, less/basic, default/standard, aggressive");
        eprintln!("Search paths: --search-path/-I <path> (can be specified multiple times)");
        eprintln!("              Default: current directory (.)");
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = args.get(2).map(|s| s.as_str()).unwrap_or("output.bc");
    let optimization_level = parse_optimization_level(&args);
    let search_paths = parse_search_paths(&args);

    println!("Compiling Silica file: {}", input_file);
    println!("Output: {}", output_file);
    println!("Optimization level: {:?}", optimization_level);
    println!("Search paths: {:?}", search_paths);

    // Read source from file
    let source = match std::fs::read_to_string(input_file) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("❌ Error reading file {}: {}", input_file, err);
            std::process::exit(1);
        }
    };

    let mut compiler = Compiler::with_optimization_and_search_paths(optimization_level, search_paths.clone());
    match compiler.compile(&source, input_file, output_file) {
        Ok(CompileResult::Success) => {
            println!("✅ Compilation successful!");
            println!("Generated LLVM bitcode in {}", output_file);
        }
        Ok(CompileResult::Skipped) => {
            println!("✅ Compilation skipped (file contains no declarations)");
        }
        Err(err) => {
            // Provide helpful error messages for common issues
            match &err {
                CompilerError::ParseError { location, message } => {
                    let source_lines = source.lines().count();
                    if location.line > source_lines || location.line > 10000 {
                        eprintln!("❌ Compilation error: Parse error (position tracking corrupted)");
                        eprintln!("  Error: {}", message);
                        eprintln!("  Note: Position information is unreliable due to internal bug");
                    } else {
                        // Check for common syntax errors and provide better messages
                        if message.contains("Expected ';' after statement") {
                            if message.contains("Identifier(\"if\")") {
                                eprintln!("❌ Compilation error: Unsupported 'if-else' syntax");
                                eprintln!("  Silica uses 'case' expressions instead of if-else statements");
                                eprintln!("  Replace: if condition {{ ... }} else {{ ... }}");
                                eprintln!("  With:    case condition of {{ true -> ... false -> ... }}");
                            } else if message.contains("Identifier(\"else\")") {
                                eprintln!("❌ Compilation error: Unsupported 'else' keyword");
                                eprintln!("  Silica uses 'case' expressions instead of if-else statements");
                            } else if message.contains("Identifier(\"for\")") {
                                eprintln!("❌ Compilation error: Unsupported 'for' loops");
                                eprintln!("  Silica uses recursion instead of loops");
                            } else if message.contains("Identifier(\"while\")") {
                                eprintln!("❌ Compilation error: Unsupported 'while' loops");
                                eprintln!("  Silica uses recursion instead of loops");
                            } else {
                                eprintln!("❌ Compilation error: {}", err);
                            }
                        } else {
                            eprintln!("❌ Compilation error: {}", err);
                        }
                    }
                }
                _ => {
                    eprintln!("❌ Compilation error: {}", err);
                }
            }
            std::process::exit(1);
        }
    }
}