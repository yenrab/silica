# Makefile for Silica lexer modules
# Compiles lexer .silica files to object files (.o) using the bootstrap compiler and LLVM toolchain

SILICA_COMPILER := ../../../silica-bootstrap-compiler/target/release/silica-boot
SEARCH_PATHS := -I . -I ..

# Find LLVM tools (including Homebrew paths)
LLVM_AS := $(or $(LLVM_AS), $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo "llvm-as"))
LLC := $(or $(LLVM_LLC), $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo "llc"))

# When QUIET=1 (from parent make), suppress progress messages
ifdef QUIET
  ECHO := @true
else
  ECHO := @echo
endif

# Find all .silica files in current directory
SILICA_FILES := $(wildcard *.silica)

# Intermediate files
LL_FILES := $(SILICA_FILES:.silica=.ll)
BC_FILES := $(SILICA_FILES:.silica=.bc)

# Final output files (.o object files)
OBJ_FILES := $(SILICA_FILES:.silica=.o)

# Keep .ll and .bc files; otherwise make removes them as "intermediate" and echoes rm
.SECONDARY: $(LL_FILES) $(BC_FILES)

.DEFAULT_GOAL := all
.PHONY: all clean help

# Default target - compile all .silica files to object files
all: $(OBJ_FILES)
	@echo "✅ Lexer modules compiled to object files"

# Step 1: Compile .silica to .ll (LLVM IR) using bootstrap compiler
%.ll: %.silica
	$(ECHO) "Compiling: $< -> $@"
	@-$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Step 2: Convert .ll to .bc (LLVM bitcode)
%.bc: %.ll
	$(ECHO) "Assembling: $< -> $@"
	@-$(LLVM_AS) $< -o $@

# Step 3: Convert .bc to .o (object file)
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
	@echo "Silica Lexer Makefile"
	@echo "===================="
	@echo ""
	@echo "This Makefile compiles lexer .silica files to object files (.o) using the bootstrap compiler."
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
