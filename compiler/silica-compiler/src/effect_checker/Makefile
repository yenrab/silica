# Makefile for Silica effect checker modules
# Compiles effect_checker .silica files to object files (.o) using the bootstrap compiler and LLVM toolchain
# Depends on lexer, parser, type_checker; build those first if needed.

SILICA_COMPILER := ../../../silica-bootstrap-compiler/target/release/silica-boot
SEARCH_PATHS := -I . -I .. -I ../lexer -I ../parser -I ../parser/capabilities -I ../parser/declarations -I ../parser/expressions -I ../parser/patterns -I ../parser/types -I ../parser/effects -I ../type_checker -I ../type_checker/declarations -I ../type_checker/expressions -I ../type_checker/traits -I ../data_structures -I ../diagnostics -I ../../stdlib

# Find LLVM tools (including Homebrew paths)
LLVM_AS := $(or $(LLVM_AS), $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo "llvm-as"))
LLC := $(or $(LLVM_LLC), $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo "llc"))

# When QUIET=1 (from parent make), suppress progress messages
ifdef QUIET
  ECHO := @true
else
  ECHO := @echo
endif

# All .silica files in this directory
SILICA_FILES := $(wildcard *.silica)

# Intermediate and output files
LL_FILES := $(SILICA_FILES:.silica=.ll)
BC_FILES := $(SILICA_FILES:.silica=.bc)
OBJ_FILES := $(SILICA_FILES:.silica=.o)

# Keep .ll and .bc files; otherwise make removes them as "intermediate" and echoes rm
.SECONDARY: $(LL_FILES) $(BC_FILES)

.DEFAULT_GOAL := all
.PHONY: all clean help

# Default target - compile all .silica files to object files
all: $(OBJ_FILES)
	@echo "✅ effect_checker modules compiled to object files"

# Step 1: .silica -> .ll (LLVM IR) using bootstrap compiler
%.ll: %.silica
	$(ECHO) "Compiling: $< -> $@"
	@-$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Step 2: .ll -> .bc (LLVM bitcode)
%.bc: %.ll
	$(ECHO) "Assembling: $< -> $@"
	@-$(LLVM_AS) $< -o $@

# Step 3: .bc -> .o (object file)
%.o: %.bc
	$(ECHO) "Linking: $< -> $@"
	@-$(LLC) -filetype=obj $< -o $@

# Clean all generated files
clean:
	@echo "Cleaning generated files..."
	@rm -f *.ll *.bc *.o
	@echo "✅ Clean complete"

# Show help
help:
	@echo "Silica effect_checker Makefile"
	@echo "=============================="
	@echo ""
	@echo "This Makefile compiles effect_checker .silica files to object files (.o) using the bootstrap compiler."
	@echo "effect_checker depends on lexer, parser, type_checker; ensure they are built first if needed."
	@echo ""
	@echo "Compilation pipeline:"
	@echo "  1. .silica -> .ll  (LLVM IR via bootstrap compiler)"
	@echo "  2. .ll -> .bc      (LLVM bitcode via llvm-as)"
	@echo "  3. .bc -> .o       (Object file via llc)"
	@echo ""
	@echo "Targets:"
	@echo "  all   - Compile all .silica files to .o object files (default)"
	@echo "  clean - Remove all generated .ll, .bc, and .o files"
	@echo "  help  - Show this help"
	@echo ""
	@echo "Files to compile:"
	@for f in $(SILICA_FILES); do echo "  $$f"; done
	@echo ""
	@echo "Bootstrap compiler: $(SILICA_COMPILER)"
	@echo "Search paths: $(SEARCH_PATHS)"
