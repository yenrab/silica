# Makefile for Silica experiments in this directory
# Automatically detects executable files (with main) vs modules (without main)

SILICA_COMPILER := ../../silica-bootstrap-compiler/target/release/silica-boot

# Ensure compiler is built without LLVM backend features for consistent text-based generation
$(SILICA_COMPILER):
	cd ../../silica-bootstrap-compiler && cargo build --release --no-default-features
SEARCH_PATHS := -I ../../stdlib

# Find all .silica files
SILICA_FILES := $(wildcard *.silica)

# Tests that are expected to fail compilation (negative tests)
EXPECTED_FAILURES := test_shadowing

# Separate executables (have main) from modules (don't have main)
ALL_EXECUTABLES := $(shell grep -l "^fn main" $(SILICA_FILES) 2>/dev/null | sed 's/\.silica$$//')
EXECUTABLES := $(filter-out $(EXPECTED_FAILURES), $(ALL_EXECUTABLES))
EXPECTED_FAILURE_EXECUTABLES := $(filter $(EXPECTED_FAILURES), $(ALL_EXECUTABLES))
MODULES := $(shell for f in $(SILICA_FILES); do if ! grep -q "^fn main" "$$f"; then basename "$$f" .silica; fi; done)

# Output files
EXECUTABLES_LL := $(addsuffix .ll, $(EXECUTABLES))
EXPECTED_FAILURE_LL := $(addsuffix .ll, $(EXPECTED_FAILURE_EXECUTABLES))
MODULES_LL := $(addsuffix .ll, $(MODULES))

# Use inherited LLVM tool paths or detect them (including Homebrew)
HAS_LLVM_AS := $(or $(LLVM_AS), $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo ""))
HAS_LLC := $(or $(LLVM_LLC), $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo ""))

# Build the standalone runtime library
RUNTIME_LIB := ../../silica-bootstrap-compiler/target/release/libsilica_compiler.a

# Set clang variables - use Homebrew for compilation, system for linking
COMPILER_CLANG := /opt/homebrew/opt/llvm/bin/clang
LINKER_CLANG := /usr/bin/clang

$(RUNTIME_LIB): ../../silica-bootstrap-compiler/src/runtime.rs
	@echo "Building Silica runtime library..."
	@cd ../../silica-bootstrap-compiler && cargo build --release

.DEFAULT_GOAL := all
.PHONY: all clean executables modules executables-all help

# Default target - build full executables
all: executables-all modules

# Compile executable files (those with main functions)
executables: $(EXECUTABLES_LL) expected-failures

# Compile expected failure tests (should fail but don't stop build)
expected-failures: $(EXPECTED_FAILURE_LL)

$(EXPECTED_FAILURE_LL): %.ll: %.silica
	@echo "Compiling expected failure test: $< -> $@ (expected to fail)"
	-@$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS) && echo "    ⚠️  Test $@ was expected to fail but succeeded!" || echo "    ✅ Test $@ failed as expected"

%.ll: %.silica
	$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Compile module files (those without main functions)
modules: $(MODULES_LL)

# Modules are compiled the same way, but marked as such
$(MODULES_LL): %.ll: %.silica
	$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Build executables if LLVM tools are available
executables-all: executables $(RUNTIME_LIB)
ifdef HAS_LLVM_AS
ifdef HAS_LLC
	@echo "Building full executables with LLVM toolchain..."
	@for ll_file in $(EXECUTABLES_LL); do \
		bc_file=$$(basename $$ll_file .ll).bc; \
		obj_file=$$(basename $$ll_file .ll).o; \
		exe_file=$$(basename $$ll_file .ll); \
		echo "  $$ll_file -> $$exe_file"; \
		$(HAS_LLVM_AS) $$ll_file -o $$bc_file && \
		$(HAS_LLC) -filetype=obj $$bc_file -o $$obj_file && \
		$(LINKER_CLANG) $$obj_file $(RUNTIME_LIB) -o $$exe_file && \
		echo "    ✅ Successfully built $$exe_file" || echo "    ❌ Failed to build $$exe_file"; \
	done
else
	@echo "llc not found - install LLVM tools for full compilation"
endif
else
	@echo "llvm-as not found - install LLVM tools for full compilation"
endif

# Clean all generated files
clean:
	rm -f *.ll *.bc *.o $(EXECUTABLES) $(EXPECTED_FAILURE_EXECUTABLES) runtime.o

# Show what will be built
help:
	@echo "Silica Experiments Makefile"
	@echo "=========================="
	@echo "Executables ($(words $(EXECUTABLES))): $(EXECUTABLES)"
	@echo "Modules ($(words $(MODULES))): $(MODULES)"
	@echo ""
	@echo "Targets:"
	@echo "  all              - Build full executables with runtime (default)"
	@echo "  executables      - Build only executable LLVM IR files"
	@echo "  modules          - Build only module LLVM IR files"
	@echo "  clean            - Remove all generated files"
	@echo "  help             - Show this help"
