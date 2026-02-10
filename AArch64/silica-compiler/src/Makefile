# Makefile for Silica Phase 2 Compiler
# Compiles the compiler source code in src/, lexer/, and parser/ using the Silica bootstrap compiler

SILICA_COMPILER := ../silica-bootstrap-compiler/target/release/silica-boot
SEARCH_PATHS := -I lexer -I parser -I parser/declarations -I parser/expressions -I parser/patterns -I parser/types -I parser/effects -I ../stdlib

# Ensure compiler is built without LLVM backend features for consistent text-based generation
$(SILICA_COMPILER):
	@echo "Building Silica bootstrap compiler..."
	cd ../../silica-bootstrap-compiler && cargo build --release --no-default-features

# Find all .silica files in lexer and parser directories (including parser subdirs)
LEXER_FILES := $(wildcard lexer/*.silica)
LEXER_MODULES := $(LEXER_FILES:lexer/%.silica=lexer/%)
PARSER_FILES := $(wildcard parser/*.silica parser/*/*.silica)
PARSER_MODULES := $(patsubst %.silica,%,$(PARSER_FILES))

# Main executable file
MAIN_FILE := main.silica
MAIN_LL := main.ll

# Output files for lexer and parser modules
LEXER_LL := $(addsuffix .ll, $(LEXER_MODULES))
PARSER_LL := $(addsuffix .ll, $(PARSER_MODULES))

.PHONY: all clean executables modules help

.DEFAULT_GOAL := all

# Default target - compile all files
all: modules executables

# Compile lexer and parser modules (dependencies of main.silica)
modules: $(LEXER_LL) $(PARSER_LL)

# Compile lexer module files
lexer/%.ll: lexer/%.silica $(SILICA_COMPILER)
	@echo "Compiling module: $<"
	-$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Compile parser module files (depend on lexer for token types; parser/%.ll covers subdirs)
parser/%.ll: parser/%.silica $(LEXER_LL) $(SILICA_COMPILER)
	@echo "Compiling module: $<"
	-$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Compile main executable
executables: $(MAIN_LL)

# Main executable depends on lexer and parser modules
$(MAIN_LL): $(MAIN_FILE) $(LEXER_LL) $(PARSER_LL) $(SILICA_COMPILER)
	@echo "Compiling main executable: $<"
	-$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Clean generated files
clean:
	@echo "Cleaning generated files..."
	rm -f *.ll
	rm -f lexer/*.ll
	rm -f parser/*.ll parser/*/*.ll
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
	@echo "  executables      - Compile main.silica to LLVM IR"
	@echo "  modules          - Compile lexer and parser module files to LLVM IR"
	@echo "  clean            - Remove all generated .ll files"
	@echo "  help             - Show this help"
	@echo ""
	@echo "Files:"
	@echo "  main.silica      - Main compiler entry point"
	@echo "  lexer/*.silica   - Lexer module files"
	@echo "  parser/*.silica  - Parser core and subdirs (declarations/, expressions/, patterns/, types/, effects/)"
	@echo ""
	@echo "Output:"
	@echo "  main.ll          - Main executable LLVM IR"
	@echo "  lexer/*.ll       - Lexer module LLVM IR files"
	@echo "  parser/*.ll      - Parser module LLVM IR files (including subdirectories)"
