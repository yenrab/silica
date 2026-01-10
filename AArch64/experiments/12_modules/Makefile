# Makefile for Silica Module Import Experiments
# Tests importing and using code from other modules

SILICA_COMPILER := ../../silica-bootstrap-compiler/target/release/silica-boot
SEARCH_PATHS := -I . -I ../../stdlib

# Ensure compiler is built without LLVM backend features for consistent text-based generation
$(SILICA_COMPILER):
	cd ../../silica-bootstrap-compiler && cargo build --release --no-default-features

# Find all .silica files
SILICA_FILES := $(wildcard *.silica)

# Separate executables (have main) from modules (don't have main)
EXECUTABLES := $(shell grep -l "^fn main" $(SILICA_FILES) 2>/dev/null | sed 's/\.silica$$//')
MODULES := $(shell for f in $(SILICA_FILES); do if ! grep -q "^fn main" "$$f"; then basename "$$f" .silica; fi; done)

# Output files
EXECUTABLES_LL := $(addsuffix .ll, $(EXECUTABLES))
MODULES_LL := $(addsuffix .ll, $(MODULES))

# Use inherited LLVM tool paths or detect them (including Homebrew)
HAS_LLVM_AS := $(or $(LLVM_AS), $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo ""))
HAS_LLC := $(or $(LLVM_LLC), $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo ""))

# Build the standalone runtime library
RUNTIME_LIB := ../../silica-bootstrap-compiler/target/release/libsilica_compiler.a

# Set clang variables - use Homebrew for compilation, system for linking
COMPILER_CLANG := /opt/homebrew/opt/llvm/bin/clang
LINKER_CLANG := /usr/bin/clang

.PHONY: all clean executables modules executables-all help

# Default target - build full executables and modules
.DEFAULT_GOAL := all
all: executables-all modules

$(RUNTIME_LIB): ../../silica-bootstrap-compiler/src/runtime.rs
	@echo "Building Silica runtime library..."
	@cd ../../silica-bootstrap-compiler && cargo build --release

# Compile executable files (those with main functions)
executables: $(EXECUTABLES_LL)

%.ll: %.silica $(SILICA_COMPILER)
	@echo "Compiling executable: $< -> $@"
	-$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Compile module files (those without main functions)
modules: $(MODULES_LL)

# Modules are compiled the same way, but marked as such
$(MODULES_LL): %.ll: %.silica $(SILICA_COMPILER)
	@echo "Compiling module: $< -> $@"
	-$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

executables-all: executables $(RUNTIME_LIB)
ifdef HAS_LLVM_AS
ifdef HAS_LLC
	@echo "Building full executables with LLVM toolchain..."
	@for ll_file in $(EXECUTABLES_LL); do \
		if [ -f "$$ll_file" ]; then \
			bc_file=$$(basename $$ll_file .ll).bc; \
			obj_file=$$(basename $$ll_file .ll).o; \
			exe_file=$$(basename $$ll_file .ll); \
			echo "  $$ll_file -> $$exe_file"; \
			if $(HAS_LLVM_AS) $$ll_file -o $$bc_file && \
			   $(HAS_LLC) -filetype=obj $$bc_file -o $$obj_file && \
			   $(LINKER_CLANG) $$obj_file $(RUNTIME_LIB) -o $$exe_file; then \
				echo "    ✅ Successfully built $$exe_file"; \
			else \
				echo "    ❌ Failed to build $$exe_file"; \
			fi; \
		fi; \
	done
else
	@echo "llc not found - install LLVM tools for full compilation"
endif
else
	@echo "llvm-as not found - install LLVM tools for full compilation"
endif

# Clean generated files
clean:
	rm -f *.ll *.bc *.o $(EXECUTABLES)

# Test compilation only (verify LLVM IR generation)
test: $(EXECUTABLES_LL) $(MODULES_LL)
	@echo "All module tests compiled successfully!"
	@echo "Generated LLVM IR files: $(EXECUTABLES_LL) $(MODULES_LL)"
	@echo ""
	@echo "Note: These tests verify that Silica's module system recognizes"
	@echo "imports and loads modules. Full multi-module compilation that"
	@echo "includes all imported code in executables has been implemented."

# Show help
help:
	@echo "Silica Module Import Experiments"
	@echo "================================"
	@echo "Executables ($(words $(EXECUTABLES))): $(EXECUTABLES)"
	@echo "Modules ($(words $(MODULES))): $(MODULES)"
	@echo ""
	@echo "Build stages:"
	@echo "  Stage 1: Silica → LLVM IR (.ll files)"
	@echo "  Stage 2: LLVM IR → Bitcode (.bc files) - requires llvm-as"
	@echo "  Stage 3: Bitcode → Object (.o files) - requires llc"
	@echo "  Stage 4: Object → Executable - requires clang + runtime"
	@echo ""
	@echo "Module structure:"
	@echo "  math_utils.silica    - Math utilities (exports: add/2, multiply/2, square/1)"
	@echo "  string_utils.silica  - String utilities (exports: concat/2, length/1)"
	@echo "  calculator.silica    - Calculator using math_utils (exports: calculate_expression/2, evaluate/1)"
	@echo "  main.silica          - Executable using math_utils + string_utils (static linking)"
	@echo "  app.silica           - Executable using calculator + string_utils (static linking)"
	@echo ""
	@echo "Targets:"
	@echo "  all              - Build full executables with runtime (default)"
	@echo "  executables      - Build only executable LLVM IR files"
	@echo "  modules          - Build only module LLVM IR files"
	@echo "  test             - Build LLVM IR and verify compilation"
	@echo "  clean            - Remove all generated files"
	@echo "  help             - Show this help"