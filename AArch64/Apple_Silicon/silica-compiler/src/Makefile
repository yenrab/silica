# Makefile for Silica Phase 2 Compiler
# Compiles the compiler source code in src/, lexer/, and parser/ using the Silica bootstrap compiler

SILICA_COMPILER := ../../silica-bootstrap-compiler/target/release/silica-boot
RUNTIME_LIB := ../../silica-bootstrap-compiler/target/release/libsilica_compiler.a
SEARCH_PATHS := -I lexer -I parser -I parser/capabilities -I parser/declarations -I parser/expressions -I parser/patterns -I parser/types -I parser/effects -I type_checker -I type_checker/declarations -I type_checker/expressions -I type_checker/traits -I effect_checker -I sir_generator -I sir_generator/declarations -I sir_generator/terms -I sir_generator/types -I sir_generator/patterns -I sir_generator/effects -I emitter -I emitter/atoms -I emitter/constraints -I emitter/terms -I emitter/terms/prims -I emitter/terms/const -I emitter/terms/let -I emitter/terms/call -I emitter/shared -I emitter/control -I emitter/aarch64 -I emitter/effects -I emitter/recursion -I data_structures -I build_output -I diagnostics -I ../stdlib

# LLVM toolchain (for .ll -> .o and linking)
LLVM_AS := $(or $(LLVM_AS), $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo "llvm-as"))
LLC := $(or $(LLVM_LLC), $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo "llc"))
LINKER_CLANG := $(or $(LINKER_CLANG), $(shell test -x /opt/homebrew/opt/llvm/bin/clang && echo "/opt/homebrew/opt/llvm/bin/clang" || echo "clang"))

# Ensure compiler is built without LLVM backend features for consistent text-based generation
$(SILICA_COMPILER):
	@echo "Building Silica bootstrap compiler..."
	cd ../../silica-bootstrap-compiler && cargo build --release --no-default-features

# Runtime library is built together with silica-boot
$(RUNTIME_LIB): $(SILICA_COMPILER)
	@true

# Find all .silica files in lexer, parser, type_checker, effect_checker, sir_generator, emitter (including subdirs)
LEXER_FILES := $(wildcard lexer/*.silica)
LEXER_MODULES := $(LEXER_FILES:lexer/%.silica=lexer/%)
PARSER_FILES := $(wildcard parser/*.silica parser/*/*.silica)
PARSER_MODULES := $(patsubst %.silica,%,$(PARSER_FILES))
TYPE_CHECKER_FILES := $(wildcard type_checker/*.silica type_checker/*/*.silica)
TYPE_CHECKER_MODULES := $(patsubst %.silica,%,$(TYPE_CHECKER_FILES))
EFFECT_CHECKER_FILES := $(wildcard effect_checker/*.silica)
EFFECT_CHECKER_MODULES := $(patsubst %.silica,%,$(EFFECT_CHECKER_FILES))

# Main executable file
MAIN_FILE := main.silica
MAIN_LL := main.ll
MAIN_BC := main.bc
MAIN_OBJ := main.o
EXECUTABLE := silica-compiler

# Output files for lexer, parser, type_checker, and effect_checker modules
LEXER_LL := $(addsuffix .ll, $(LEXER_MODULES))
PARSER_LL := $(addsuffix .ll, $(PARSER_MODULES))
TYPE_CHECKER_LL := $(addsuffix .ll, $(TYPE_CHECKER_MODULES))
EFFECT_CHECKER_LL := $(addsuffix .ll, $(EFFECT_CHECKER_MODULES))

# Subdirectories with Makefiles (build order respects dependencies)
SUBDIRS := lexer parser type_checker effect_checker sir_generator emitter

.PHONY: all clean executables modules subdirs help $(SUBDIRS)

.DEFAULT_GOAL := all

# Default target - ensure bootstrap compiler exists, then compile subdirs, modules, and main
all: $(SILICA_COMPILER) subdirs modules executables

# Build all subdirectories with Makefiles (before compiling main.silica)
# subdirs requires bootstrap compiler to exist first
subdirs: $(SILICA_COMPILER) $(SUBDIRS)

$(SUBDIRS):
	@echo "Building $@..."
	@$(MAKE) -C $@ QUIET=1

# Compile lexer, parser, type_checker, and effect_checker modules
modules: $(LEXER_LL) $(PARSER_LL) $(TYPE_CHECKER_LL) $(EFFECT_CHECKER_LL)

# Compile lexer module files
lexer/%.ll: lexer/%.silica $(SILICA_COMPILER)
	@$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Compile parser module files (depend on lexer for token types; parser/%.ll covers subdirs)
parser/%.ll: parser/%.silica $(LEXER_LL) $(SILICA_COMPILER)
	@$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Compile type_checker module files (type_checker/%.ll covers subdirs)
type_checker/%.ll: type_checker/%.silica $(LEXER_LL) $(PARSER_LL) $(SILICA_COMPILER)
	@$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Compile main executable (LLVM IR -> object -> linked binary)
executables: $(EXECUTABLE)

# Main LLVM IR depends on subdirs, lexer, parser, type_checker, effect_checker, and emitter (main.silica imports emitter_core)
EMITTER_FILES := $(wildcard emitter/*.silica emitter/*/*.silica emitter/*/*/*.silica)
$(MAIN_LL): $(MAIN_FILE) subdirs $(LEXER_LL) $(PARSER_LL) $(TYPE_CHECKER_LL) $(EFFECT_CHECKER_LL) $(EMITTER_FILES) $(SILICA_COMPILER)
	@$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# main.ll -> main.bc (bitcode)
$(MAIN_BC): $(MAIN_LL)
	@echo "Assembling: $< -> $@"
	@-$(LLVM_AS) $< -o $@

# main.bc -> main.o (object file)
$(MAIN_OBJ): $(MAIN_BC)
	@echo "Compiling: $< -> $@"
	@-$(LLC) -filetype=obj $< -o $@

# Link main.o + runtime into silica-compiler executable
# (main.silica is compiled as a single unit with all imports, so main.o contains everything)
$(EXECUTABLE): $(MAIN_OBJ) $(RUNTIME_LIB)
	@echo "Linking: $(EXECUTABLE)"
	@-$(LINKER_CLANG) $(MAIN_OBJ) $(RUNTIME_LIB) -o $@
	@echo "✅ Built $(EXECUTABLE)"

# Clean generated files (including subdirectories)
clean:
	@echo "Cleaning generated files..."
	@for d in $(SUBDIRS); do $(MAKE) -C $$d clean 2>/dev/null || true; done
	rm -f *.ll *.bc *.o $(EXECUTABLE)
	rm -f lexer/*.ll
	rm -f parser/*.ll parser/*/*.ll
	rm -f type_checker/*.ll type_checker/*/*.ll
	rm -f effect_checker/*.ll
	@echo "✅ Clean complete"

# Help target
help:
	@echo "Silica Phase 2 Compiler Makefile"
	@echo "================================="
	@echo ""
	@echo "This Makefile compiles the Phase 2 Silica compiler source code using the bootstrap compiler."
	@echo ""
	@echo "Targets:"
	@echo "  all              - Compile all files (default)"
	@echo "  subdirs          - Build lexer, parser, type_checker, sir_generator, emitter"
	@echo "  executables      - Compile main.silica to LLVM IR"
	@echo "  modules          - Compile lexer and parser module files to LLVM IR"
	@echo "  clean            - Remove all generated .ll files"
	@echo "  help             - Show this help"
	@echo ""
	@echo "Files:"
	@echo "  main.silica      - Main compiler entry point"
	@echo "  lexer/*.silica   - Lexer module files"
	@echo "  parser/*.silica  - Parser core and subdirs (declarations/, expressions/, patterns/, types/, effects/)"
	@echo "  type_checker/*.silica - Type checker core and subdirs (declarations/, expressions/, traits/)"
	@echo ""
	@echo "Output:"
	@echo "  main.ll          - Main executable LLVM IR"
	@echo "  main.o           - Main object file"
	@echo "  silica-compiler  - Linked executable (requires llvm-as, llc, clang)"
	@echo "  lexer/*.ll       - Lexer module LLVM IR files"
	@echo "  parser/*.ll      - Parser module LLVM IR files (including subdirectories)"
	@echo "  type_checker/*.ll - Type checker module LLVM IR files (including subdirectories)"
