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
    let optimization_level = parse_optimization_level(&args);

    println!("Starting Phase 2D: LLVM Optimization Integration Test...");
    println!("Optimization level: {:?}", optimization_level);

    // Test the full compilation pipeline with advanced types
    let source = r#"
use module my_module;

import std::io;
export add;

// Type alias
type MyInt = int;

// Struct definition
struct Point {
    x: int,
    y: int,
}

// Enum definition
enum Result {
    Ok(int),
    Err(string),
}

// Trait definition
trait Display {
    fn display(self) -> string;
}

// Implementation commented out for stability
// impl Display for int {
//     fn display(self) -> string {
//         self
//     }
// }

fn add(x: int, y: int) -> int {
    x + y
}

fn test_memory() -> int {
    read_ref(alloc_ref(region(), 42))
}

fn test_actors() -> int {
    spawn(100, add) + recv()
}

fn main() -> int {
    42
}
"#;

    println!("Source code:\n{}", source);

    let mut compiler = Compiler::with_optimization(optimization_level);
    match compiler.compile(source, "test.silica", "test.bc") {
        Ok(()) => {
            println!("✅ Full compilation pipeline completed successfully!");
            println!("Generated LLVM bitcode in test.bc");
        }
        Err(err) => {
            eprintln!("❌ Compilation error: {}", err);
        }
    }

    println!("Done!");
}