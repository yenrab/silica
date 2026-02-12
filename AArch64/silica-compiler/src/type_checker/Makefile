# Makefile for Silica type checker modules
# Compiles type checker .silica files to object files (.o) using the bootstrap compiler and LLVM toolchain
# Type checker depends on lexer (SourceLocation) and parser (AST types); build lexer and parser first from ../lexer and ../parser if needed.

SILICA_COMPILER := ../../../silica-bootstrap-compiler/target/release/silica-boot
SEARCH_PATHS := -I . -I .. -I ../lexer -I ../parser -I ../parser/declarations -I ../parser/expressions -I ../parser/patterns -I ../parser/types -I ../parser/effects -I declarations -I expressions -I traits -I ../../stdlib

# Find LLVM tools (including Homebrew paths)
LLVM_AS := $(or $(LLVM_AS), $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo "llvm-as"))
LLC := $(or $(LLVM_LLC), $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo "llc"))

# All .silica files in this directory and subdirectories
SILICA_FILES := $(wildcard *.silica) $(wildcard */*.silica)

# Intermediate and output files (same path with different extension)
LL_FILES := $(SILICA_FILES:.silica=.ll)
BC_FILES := $(SILICA_FILES:.silica=.bc)
OBJ_FILES := $(SILICA_FILES:.silica=.o)

# Keep .ll and .bc files; otherwise make removes them as "intermediate" and echoes rm
.SECONDARY: $(LL_FILES) $(BC_FILES)

.DEFAULT_GOAL := all
.PHONY: all clean help

# Default target - compile all .silica files to object files
all: $(OBJ_FILES)
	@echo "✅ Type checker modules compiled to object files"

# Step 1: .silica -> .ll (LLVM IR) using bootstrap compiler
%.ll: %.silica
	@echo "Compiling: $< -> $@"
	@-$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Step 2: .ll -> .bc (LLVM bitcode)
%.bc: %.ll
	@echo "Assembling: $< -> $@"
	@-$(LLVM_AS) $< -o $@

# Step 3: .bc -> .o (object file)
%.o: %.bc
	@echo "Linking: $< -> $@"
	@-$(LLC) -filetype=obj $< -o $@

# Clean all generated files (current dir and subdirs)
clean:
	@echo "Cleaning generated files..."
	@rm -f *.ll *.bc *.o
	@rm -f */*.ll */*.bc */*.o
	@echo "✅ Clean complete"

# Show help
help:
	@echo "Silica Type Checker Makefile"
	@echo "==========================="
	@echo ""
	@echo "This Makefile compiles type checker .silica files to object files (.o) using the bootstrap compiler."
	@echo "Type checker modules depend on lexer (../lexer) and parser (../parser); ensure they are built first if needed."
	@echo ""
	@echo "Directories:"
	@echo "  .                 - type_checker_core.silica"
	@echo "  declarations/     - Function declaration type checking"
	@echo "  expressions/     - Literal, identifier, and expression type checking"
	@echo "  traits/           - Trait implementation and constraint checking"
	@echo ""
	@echo "Compilation pipeline:"
	@echo "  1. .silica -> .ll  (LLVM IR via bootstrap compiler)"
	@echo "  2. .ll -> .bc      (LLVM bitcode via llvm-as)"
	@echo "  3. .bc -> .o       (Object file via llc)"
	@echo ""
	@echo "Targets:"
	@echo "  all              - Compile all .silica files to .o object files (default)"
	@echo "  clean            - Remove all generated .ll, .bc, and .o files"
	@echo "  help             - Show this help"
	@echo ""
	@echo "Files to compile:"
	@for f in $(SILICA_FILES); do echo "  $$f"; done
	@echo ""
	@echo "Bootstrap compiler: $(SILICA_COMPILER)"
	@echo "LLVM assembler (llvm-as): $(LLVM_AS)"
	@echo "LLVM compiler (llc): $(LLC)"
	@echo "Search paths: $(SEARCH_PATHS)"
