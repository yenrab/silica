# Language Creation Templates

## Direct Silica Runtime Connections - No AST Translation

---

## Template-Based Language Development

**One-Command Language Foundation:**
```bash
# Generate complete language starter kit
silica-lang-template mylang \
  --paradigm=functional \
  --concurrency=actor \
  --effects=io,filesystem \
  --target=aarch64

# Templates generate complete compilers that use Silica runtime to compile language source files to AArch64 libraries
# Including standard libraries and user code - full compilation pipeline from .ex/.erl to native AArch64 libraries
```

**What Templates Generate:**

**Complete Compiler Toolchain:**
- **Source file parser** - processes .ex, .erl, .gleam files
- **AST transformer** - converts language AST to Silica code generation
- **Library compiler** - produces AArch64 library files using Silica runtime
- **Interactive REPL** - read-eval-print loop with Silica runtime integration
- **Build integration** - works with existing language build tools
- **Runtime bindings** - connects to Silica runtime APIs

**Generated File Structure:**
```
mylang-silica-compiler/
├── src/
│   ├── compiler.rs          # Main compiler entry point
│   ├── parser.rs            # Language source file parser
│   ├── codegen.rs           # Silica code generation
│   ├── stdlib/              # Standard library in Silica
│   │   ├── io.silica        # Silica source for stdlib
│   │   ├── lists.silica
│   │   ├── processes.silica
│   │   └── ...
│   └── bindings.rs          # Silica runtime API bindings
├── bin/
│   ├── mylangc              # Compiler executable
│   └── mylang-repl          # Interactive REPL
└── lib/
    └── libstdlib.a          # Compiled AArch64 standard library
```

**Template Categories:**

**Functional Language Template:**
```silica
// Template generates complete Silica compiler for functional languages

effect io_eff = [device_io]
effect file_io_eff = [device_io, mem(normal)]

// Generated main compiler function
fn compile_mylang_to_silica(source_file: string, output_lib: string) -> proc[file_io_eff] result<unit, string> {
    // 1. Read and parse MyLang source file
    let source_code <- read_file(source_file)?;
    let ast <- parse_mylang_source(source_code);

    // 2. Transform to Silica code
    let silica_code <- transform_mylang_to_silica(ast);

    // 3. Write Silica library file
    write_file(output_lib, silica_code)?;

    // 4. Use Silica compiler to generate AArch64 library
    compile_silica_to_aarch64(silica_code, output_lib ++ ".a")?;

    println("Compiled {} to {}", source_file, output_lib);
    Ok(unit)
}

// Generated transformation from MyLang to Silica
fn transform_mylang_to_silica(ast: mylang_ast) -> proc[] string {
    let mut silica_code = "# Generated Silica code from MyLang\n";

    case ast.items of
        [] -> silica_code
        [Function{name, params, body} | rest] -> {
            let param_list = params |> map(fn(p) -> "#{p.name}: #{map_type_to_silica(p.type)}") |> join(", ");
            let body_code = generate_silica_expr(body);

            silica_code = silica_code ++ "fn #{name}(#{param_list}) -> unit {\n    #{body_code}\n}\n\n";
            transform_mylang_to_silica({items: rest})
        }
        [Module{name, functions} | rest] -> {
            silica_code = silica_code ++ "mod #{name} {\n#{transform_functions_to_silica(functions)}\n}\n\n";
            transform_mylang_to_silica({items: rest})
        }
    end
}

fn generate_silica_expr(expr: mylang_expr) -> string {
    case expr of
        Literal(value) -> to_string(value)
        Variable(name) -> name
        Call{function: func, arguments: args} -> {
            let arg_list = args |> map(generate_silica_expr) |> join(", ");
            "#{generate_silica_expr(func)}(#{arg_list})"
        }
        _ -> "// TODO: implement other expressions"
    end
}
```

**Actor Language Template:**
```silica
// Generated actor/message passing framework in Silica
effect concurrency_eff = [concurrency]

fn generate_actor_behavior(behavior: mylang_behavior) -> proc[concurrency_eff] actor_ref<term> {
    // Template creates Silica actor with message handling
    spawn_actor(fn() -> unit {
        loop {
            let message <- recv();
            case behavior.handle_message(message) of
                Reply{response} -> send(response)
                Become{new_behavior} -> {
                    behavior = new_behavior;
                    continue
                }
                Terminate -> break
            end
        }
    })
}

// Example: Template generates actor spawn code
fn compile_actor_spawn(actor_def: mylang_actor) -> silica_expr {
    let behavior_code = generate_actor_behavior(actor_def.behavior);
    quote! {
        let actor = #{behavior_code};
        actor  // Return actor reference
    }
}
```

---

## Template Components

**1. AST Transformation Framework:**
- Parser generator templates
- AST node definitions
- Transformation pipelines
- Error handling patterns

**2. Effect System Integration:**
```silica
// Template-generated effect declarations
effect io_eff = [device_io, mem(normal)]
effect network_eff = [networking, io_eff]
effect filesystem_eff = [filesystem, io_eff]

// Language-specific effect combinators
fn with_file_io<F, R>(operation: F) -> proc[filesystem_eff] R {
    // Template implements resource management
}
```

**3. Runtime Library Templates:**
```silica
// Generated standard library bindings
module mylang.std {
    // Basic types
    type Result<T, E> = silica::result::Result<T, E>
    type Option<T> = silica::option::Option<T>

    // Actor primitives
    fn spawn_actor = silica::actor::spawn
    fn send_message = silica::actor::send
    fn receive_message = silica::actor::recv

    // Effect helpers
    fn with_io<F, R>(f: F) -> proc[io_eff] R { ... }
}
```

---

## Template Patterns

**Pattern Matching Template:**
```silica
// Generated pattern matching compiler in Silica
fn compile_pattern(pattern: mylang_pattern, value: silica_expr) -> silica_expr {
    case pattern of
        LiteralPattern{lit} -> {
            // Generate Silica literal match
            quote! {
                case #{value} of
                    #{lit} -> true
                    _ -> false
                end
            }
        }
        VariablePattern{name} -> {
            // Generate Silica variable binding
            quote! {
                let #{name} = #{value};
                true
            }
        }
        TuplePattern{elements} -> {
            // Generate Silica tuple pattern matching
            let mut conditions = [];
            let element_checks = elements |> map(fn(elem, index) -> {
                let elem_value = quote!{#{value}.#{index}};
                compile_pattern(elem, elem_value)
            });

            quote! {
                case #{value} of
                    {#{elements |> map(fn(e) -> "_") |> join(", ")}} -> {
                        #{element_checks |> join(" && ")}
                    }
                    _ -> false
                end
            }
        }
    end
}

// Example usage in case expression compilation
fn compile_case_expr(case_expr: mylang_case) -> silica_expr {
    let match_expr = compile_expr(case_expr.value);
    let branches = case_expr.branches |> map(fn(branch) -> {
        let pattern_match = compile_pattern(branch.pattern, match_expr);
        let body = compile_expr(branch.body);
        quote! {
            #{pattern_match} -> #{body}
        }
    }) |> join("\n    ");

    quote! {
        case #{match_expr} of
            #{branches}
        end
    }
}
```

**Macro System Template:**
```silica
// Generated compile-time macro expansion in Silica
fn expand_macro(invocation: mylang_macro_invocation) -> mylang_ast {
    case invocation.name of
        "if_let" -> expand_if_let_macro(invocation)
        "match_all" -> expand_match_all_macro(invocation)
        "async" -> expand_async_macro(invocation)
        _ -> invocation  // Pass through unchanged
    end
}

// Example macro expansions
fn expand_if_let_macro(invocation: mylang_macro_invocation) -> mylang_ast {
    // if_let pattern = expr { body } → case expr { pattern -> body; _ -> () }
    let pattern = invocation.args[0];
    let expr = invocation.args[1];
    let body = invocation.args[2];

    CaseExpr{
        value: expr,
        branches: [
            CaseBranch{pattern: pattern, body: body},
            CaseBranch{pattern: Wildcard, body: Literal{value: unit}}
        ]
    }
}

fn expand_async_macro(invocation: mylang_macro_invocation) -> mylang_ast {
    // async { body } → spawn(fn() -> body end)
    let body = invocation.args[0];

    SpawnExpr{
        function: Function{
            params: [],
            body: body,
            effects: [concurrency]
        }
    }
}
```

---

## Rapid Language Prototyping

**Domain-Specific Language Example:**
```bash
# Create DSL for financial contracts
silica-lang-template finlang \
  --paradigm=dsl \
  --domain=finance \
  --effects=audit,persistence

# Generated in seconds:
# - Contract AST with financial primitives
# - Audit trail effect system
# - Persistence with ACID semantics
# - Financial standard library
```

**Educational Language Example:**
```bash
# Create teaching language for concurrency
silica-lang-template learn-concurrency \
  --paradigm=educational \
  --focus=concurrency \
  --visualization=true

# Includes:
# - Visual actor/message diagrams
# - Step-through debugging
# - Race condition detection
# - Performance visualization
```

---

## Template Benefits

**For Language Creators:**
- **No AST translation complexity** - direct Silica runtime connections
- **Templates handle all Silica integration** - focus on language semantics
- **Immediate native compilation** - bypasses bytecode entirely
- **Hardware acceleration included** - AArch64 features built-in

---

## How Direct Silica Runtime Connections Work

**Template Generation Process:**
```bash
# 1. Analyze target language semantics
silica-lang-template elixir --analyze-semantics

# 2. Generate complete compiler toolchain
# Creates: elixir_compiler/ (complete language toolchain)
# ├── src/compiler.rs      - Main compiler
# ├── src/parser.rs        - Elixir source parser
# ├── src/codegen.rs       - Silica code generation
# ├── src/repl.rs          - Interactive REPL implementation
# ├── src/stdlib/          - Elixir stdlib in Silica
# ├── bin/elixir-to-silica - Compiler executable
# ├── bin/elixir-repl      - Interactive REPL executable
# └── lib/libelixir.so     - Compiled AArch64 stdlib

# 3. Use generated tools
./elixir-to-silica my_module.ex libmy_module.a        # Compile to library
./elixir-to-silica --stdlib libelixir_stdlib.so       # Compile stdlib
./elixir-repl                                        # Launch interactive REPL
```

**Generated Complete Compiler Toolchain:**
```silica
// Auto-generated by silica-lang-template
// bin/elixir-to-silica.silica - Complete compiler executable in Silica

effect io_eff = [device_io]
effect file_io_eff = [device_io, mem(normal)]

// Main compiler entry point
fn main() -> proc[io_eff] unit {
    let args = get_command_line_args();

    case args of
        [_, "--stdlib", output_path] -> {
            // Compile entire Elixir standard library to AArch64 shared library
            let compiler = ElixirCompiler{};
            compiler.compile_stdlib_to_aarch64_library(output_path)?;
            println("Compiled Elixir stdlib to {}", output_path);
        }
        [_, source_path, output_path] -> {
            // Compile individual source file to AArch64 static library
            let compiler = ElixirCompiler{};
            compiler.compile_file_to_aarch64_library(source_path, output_path)?;
            println("Compiled {} to {}", source_path, output_path);
        }
        _ -> {
            println("Usage: elixir-to-silica <source.ex> <output.a>");
            println("       elixir-to-silica --stdlib <libstdlib.so>");
        }
    end
}

// Generated compiler implementation
type ElixirCompiler = {
    parser: ElixirParser,
    codegen: SilicaCodeGenerator,
    stdlib: list<string>
}

fn ElixirCompiler::new() -> ElixirCompiler {
    ElixirCompiler{
        parser: ElixirParser{},
        codegen: SilicaCodeGenerator{},
        stdlib: load_stdlib_functions()
    }
}

fn ElixirCompiler::compile_file_to_aarch64_library(self, source_path: string, output_path: string) -> proc[file_io_eff] result<unit, string> {
    // 1. Read and parse Elixir source
    let source <- read_file(source_path)?;
    let ast <- self.parser.parse(source);

    // 2. Transform to Silica-compatible form
    let silica_ast <- self.codegen.transform_ast(ast);

    // 3. Use Silica runtime to compile to AArch64 library
    silica::compiler::compile_to_aarch64_library(silica_ast, output_path)?;

    Ok(unit)
}

fn ElixirCompiler::compile_stdlib_to_aarch64_library(self, output_path: string) -> proc[file_io_eff] result<unit, string> {
    // Compile all stdlib functions using Silica runtime to AArch64 library
    let stdlib_ast <- self.codegen.generate_stdlib_ast(self.stdlib);
    silica::compiler::compile_to_aarch64_shared_library(stdlib_ast, output_path)?;
    Ok(unit)
}
```

**Generated Interactive REPL:**
```silica
// bin/elixir-repl.silica - Template-generated interactive REPL in Silica

effect io_eff = [device_io]

fn main() -> proc[io_eff] unit {
    println("Elixir REPL (Silica Backend)");
    println("Type expressions to evaluate, 'quit' to exit");

    let compiler = ElixirCompiler::new();
    let mut context = silica::runtime::ExecutionContext::new();

    loop {
        print("iex> ");

        let input <- read_line();
        let input = input |> trim();

        if input == "quit" {
            break;
        }

        case compiler.parse_and_compile_expr(input) of
            Ok{silica_code} -> {
                case context.execute(silica_code) of
                    Ok{result} -> println("{:?}", result)
                    Err{e} -> println("Runtime error: {}", e)
                end
            }
            Err{e} -> println("Parse error: {}", e)
        end
    }
}

// Compiler extension for REPL
fn ElixirCompiler::parse_and_compile_expr(self, input: string) -> result<silica::runtime::Code, string> {
    // Parse as expression
    let ast <- self.parser.parse_expression(input);

    // Transform to Silica
    let silica_ast <- self.codegen.transform_expr(ast);

    // Generate executable code (not library)
    let silica_code <- self.codegen.generate_executable(silica_ast);

    Ok(silica_code)
}
```

**REPL Features Generated by Templates:**
- **Interactive expression evaluation** using Silica runtime
- **State persistence** between evaluations
- **Error handling** with language-specific error messages
- **Auto-completion** for language constructs
- **Multi-line input** support
- **History** and navigation

---

**Integration in Language Compiler:**
```silica
// elixir_compiler.silica - uses generated bindings
use elixir_bindings::*;

fn compile_elixir_spawn(expr: elixir_ast) -> silica_code {
    case expr of
        Spawn{function, args} -> {
            // Generate direct binding call
            quote! {
                spawn_elixir_process(
                    #{compile_elixir_function(function)}
                )
            }
        }
    end
}

fn compile_elixir_send(expr: elixir_ast) -> silica_code {
    case expr of
        Send{pid, message} -> {
            // Generate direct binding call
            quote! {
                send_elixir_message(
                    #{compile_elixir_expr(pid)},
                    #{compile_elixir_expr(message)}
                )
            }
        }
    end
}
```

**Runtime Connection Flow:**
```
Elixir Code
    ↓
Language Compiler (with template-generated bindings)
    ↓
Direct Silica Runtime API Calls
    ↓
Silica Actor System + Memory Manager + Effect System
    ↓
AArch64 Hardware Acceleration
```

**Benefits of Direct Connections:**
- **Zero abstraction layers** - language calls Silica directly
- **Optimized performance** - no intermediate representations
- **Type safety** - compile-time guarantees across language boundary
- **Hardware acceleration** - direct access to all Silica features

**For the Ecosystem:**
- **Diverse language landscape** - easier to experiment with new paradigms
- **Cross-language interoperability** - all compile to common Silica runtime
- **Innovation acceleration** - rapid prototyping of language features
- **Knowledge sharing** - common patterns across language implementations

**For Users:**
- **Rich language selection** - domain-specific languages for specialized needs
- **Performance consistency** - all languages benefit from Silica optimizations
- **Seamless integration** - languages work together through Silica runtime
- **Future migration** - easy transition between compatible languages
