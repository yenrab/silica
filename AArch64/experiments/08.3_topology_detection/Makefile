# Topology Detection Experiments Makefile
# Tests CPU topology detection functionality across different platforms

# Use inherited LLVM tool paths or detect them (including Homebrew)
HAS_LLVM_AS := $(or $(LLVM_AS), $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo ""))
HAS_LLC := $(or $(LLVM_LLC), $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo ""))

# Find all .silica files
SILICA_FILES := $(wildcard *.silica)

# Separate executables (have main) from modules (don't have main)
EXECUTABLES := $(shell grep -l "^fn main" $(SILICA_FILES) 2>/dev/null | sed 's/\.silica$$//')
MODULES := $(shell for f in $(SILICA_FILES); do if ! grep -q "^fn main" "$$f"; then basename "$$f" .silica; fi; done)

# Output files
EXECUTABLES_LL := $(addsuffix .ll, $(EXECUTABLES))
MODULES_LL := $(addsuffix .ll, $(MODULES))

# Runtime library path
RUNTIME_LIB := ../../silica-bootstrap-compiler/target/release/libsilica_compiler.a

# Set clang variables - use Homebrew for both compilation and linking
COMPILER_CLANG := /opt/homebrew/opt/llvm/bin/clang
LINKER_CLANG := /opt/homebrew/opt/llvm/bin/clang

.PHONY: all clean executables modules executables-all help

# Default target - build full executables
all: executables-all

# Compile executable files (those with main functions) to LLVM IR
executables: $(EXECUTABLES_LL)

%.ll: %.silica
	../../silica-bootstrap-compiler/target/release/silica-boot $< $@ -I ../../stdlib

# Compile module files (those without main functions) to LLVM IR
modules: $(MODULES_LL)

# Modules are compiled the same way, but marked as such
$(MODULES_LL): %.ll: %.silica
	../../silica-bootstrap-compiler/target/release/silica-boot $< $@ -I ../../stdlib

# Build full executables - depends on LLVM IR files being built
executables-all: $(EXECUTABLES_LL)
	@echo "Building LLVM IR files..."
	@echo "HAS_LLVM_AS=$(HAS_LLVM_AS)"
	@echo "HAS_LLC=$(HAS_LLC)"
	@if [ -n "$(HAS_LLVM_AS)" ] && [ -n "$(HAS_LLC)" ]; then \
		echo "Building full executables with LLVM toolchain..."; \
		for ll_file in $(EXECUTABLES_LL); do \
			obj_file=$$(basename $$ll_file .ll).o; \
			exe_file=$$(basename $$ll_file .ll); \
			echo "  $$ll_file -> $$exe_file"; \
			$(HAS_LLC) -filetype=obj $$ll_file -o $$obj_file && \
			$(LINKER_CLANG) $$obj_file $(RUNTIME_LIB) -o $$exe_file && \
			echo "    ✅ Successfully built $$exe_file" || exit 1; \
		done; \
	else \
		echo "LLVM tools not found - cannot build executables"; \
		echo "Install LLVM tools (llvm-as, llc, clang) for full compilation"; \
		exit 1; \
	fi

# Clean all generated files
clean:
	rm -f *.ll *.bc *.o $(EXECUTABLES)

# Show what will be built
help:
	@echo "Silica Topology Detection Tests"
	@echo "==============================="
	@echo "Executables: $(EXECUTABLES)"
	@echo "Modules: $(MODULES)"
	@echo ""
	@echo "Targets:"
	@echo "  all              - Build full executables (default)"
	@echo "  executables      - Build LLVM IR files for executables"
	@echo "  modules          - Build LLVM IR files for modules"
	@echo "  executables-all  - Build full executables (requires LLVM tools)"
	@echo "  clean            - Remove all generated files"
	@echo "  help             - Show this help"
