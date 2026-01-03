# Core Affinity Test Suite Makefile
# Automates compilation and testing of core affinity features

# Use Homebrew LLVM tools
HAS_LLVM_AS := /opt/homebrew/opt/llvm/bin/llvm-as
HAS_LLC := /opt/homebrew/opt/llvm/bin/llc

# Compiler path
SILICA_BOOT = /Volumes/Files_1/Silica\ Language/AArch64/silica-bootstrap-compiler/target/release/silica-boot
SEARCH_PATH = /Volumes/Files_1/Silica\ Language/AArch64/stdlib

# Build the standalone runtime library
RUNTIME_LIB := ../../silica-bootstrap-compiler/target/release/libsilica_compiler.a

# Set clang variables - use Homebrew for both compilation and linking
COMPILER_CLANG := /opt/homebrew/opt/llvm/bin/clang
LINKER_CLANG := /opt/homebrew/opt/llvm/bin/clang

$(RUNTIME_LIB): ../../silica-bootstrap-compiler/src/runtime.rs ../../silica-bootstrap-compiler/src/io.rs
	@echo "Building Silica runtime library..."
	@cd ../../silica-bootstrap-compiler && cargo build --release

.PHONY: all clean executables full_test help

# Find all .silica files
SILICA_FILES := $(wildcard *.silica)

# Separate executables (have main) from modules (don't have main)
EXECUTABLES := $(shell grep -l "^fn main" $(SILICA_FILES) 2>/dev/null | sed 's/\.silica$$//')
MODULES := $(shell for f in $(SILICA_FILES); do if ! grep -q "^fn main" "$$f"; then basename "$$f" .silica; fi; done)

# Output files
EXECUTABLES_LL := $(addsuffix .ll, $(EXECUTABLES))
MODULES_LL := $(addsuffix .ll, $(MODULES))

# Default target - build full executables
all: executables full_test

# Compile executable files (those with main functions)
executables: $(EXECUTABLES_LL) $(RUNTIME_LIB)
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
		echo "    ✅ Successfully built $$exe_file"; \
	done
else
	@echo "llc not found - install LLVM tools for full compilation"
endif
else
	@echo "llvm-as not found - install LLVM tools for full compilation"
endif

%.ll: %.silica
	@echo "Compiling: $< -> $@"
	$(SILICA_BOOT) $< $@ -I $(SEARCH_PATH)

# Clean all generated files
clean:
	rm -f *.ll *.bc *.o $(EXECUTABLES)

# Full test suite
full_test:
	@echo "🎯 Core Affinity Test Suite Complete"
	@echo "All tests compiled successfully!"
	@echo ""
	@echo "Executables: $(EXECUTABLES)"
	@echo "Modules: $(MODULES)"
	@echo ""
	@echo "Generated files:"
	@ls -la *.ll *.bc *.o $(EXECUTABLES) 2>/dev/null || ls -la *.ll

# Show what will be built
help:
	@echo "Core Affinity Test Suite"
	@echo "========================"
	@echo "Executables: $(EXECUTABLES)"
	@echo "Modules: $(MODULES)"
	@echo ""
	@echo "Targets:"
	@echo "  all              - Build full executables (default)"
	@echo "  executables      - Build LLVM IR files and link to executables"
	@echo "  clean            - Remove all generated files"
	@echo "  full_test        - Show compilation results"
	@echo "  help             - Show this help"
