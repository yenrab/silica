# Master Makefile for all Silica experiments
# Builds all test directories in dependency order

# Subdirectories in testing order (dependencies must be built first)
SUBDIRS := 01_basic 02_expressions 03_data_types 04_patterns 05_memory 06_advanced_features 07_concurrency 08_stdlib 09_binding 10_experimental

# LLVM tool paths (including Homebrew location on macOS)
LLVM_AS := $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo "")
LLVM_LLC := $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo "")
COMPILER_CLANG := $(shell test -x /opt/homebrew/opt/llvm/bin/clang && echo "/opt/homebrew/opt/llvm/bin/clang" || echo "clang")
LINKER_CLANG := clang
HAS_LLVM_TOOLS := $(and $(LLVM_AS),$(LLVM_LLC))

# Export variables for sub-Makefiles
export COMPILER_CLANG LINKER_CLANG LLVM_AS LLVM_LLC

.PHONY: all clean $(SUBDIRS) help

# Default target - build full executables in order
all: $(SUBDIRS)
	@echo "✅ All Silica experiments compiled with full executables!"

# Build each subdirectory
$(SUBDIRS):
	@echo "🏗️  Building $@ experiments..."
	$(MAKE) -C $@ all
	@echo "✅ $@ completed"

# Clean all subdirectories
clean:
	@echo "🧹 Cleaning all experiments..."
	for dir in $(SUBDIRS); do \
		$(MAKE) -C $$dir clean; \
	done
	@echo "✅ All experiments cleaned"

# Build only executables across all directories
executables:
	@echo "🏗️  Building all executables..."
	for dir in $(SUBDIRS); do \
		echo "Building executables in $$dir..."; \
		$(MAKE) -C $$dir executables; \
	done
	@echo "✅ All executables built"

# Build only modules across all directories
modules:
	@echo "🏗️  Building all modules..."
	for dir in $(SUBDIRS); do \
		echo "Building modules in $$dir..."; \
		$(MAKE) -C $$dir modules; \
	done
	@echo "✅ All modules built"

# Build full executables across all directories (requires LLVM tools)
executables-full:
	@echo "🏗️  Building all full executables..."
	if [ -z "$(HAS_LLVM_TOOLS)" ]; then \
		echo "❌ LLVM tools not found. Install LLVM (brew install llvm on macOS)"; \
		exit 1; \
	fi
	for dir in $(SUBDIRS); do \
		echo "Building full executables in $$dir..."; \
		$(MAKE) -C $$dir executables-full; \
	done
	@echo "✅ All full executables built"

# Quick test - build just the basic functionality
test-basic:
	$(MAKE) -C 01_basic all

# Show comprehensive build status
status:
	@echo "Silica Experiments Build Status"
	@echo "==============================="
	@echo "Directories to build in order:"
	@i=1; for dir in $(SUBDIRS); do \
		echo "$$i. $$dir - $$(ls $$dir/*.silica 2>/dev/null | wc -l) files"; \
		i=$$((i+1)); \
	done
	@echo ""
	@echo "Total files: $$(find $(SUBDIRS) -name "*.silica" | wc -l)"

# Help target
help:
	@echo "Silica Experiments Master Makefile"
	@echo "=================================="
	@echo ""
	@echo "This Makefile builds all Silica test experiments in dependency order."
	@echo "Each subdirectory contains tests for specific language features."
	@echo ""
	@echo "Targets:"
	@echo "  all              - Build full native executables across all directories (default)"
	@echo "  clean            - Clean all generated files"
	@echo "  executables      - Build only executable LLVM IR files across all directories"
	@echo "  modules          - Build only module files across all directories"
	@echo "  test-basic       - Quick test of basic functionality only"
	@echo "  status           - Show build status and file counts"
	@echo "  help             - Show this help"
	@echo ""
	@echo "Subdirectories (build in this order):"
	@echo "  01_basic           - Language fundamentals"
	@echo "  02_expressions     - Operators & control flow"
	@echo "  03_data_types      - Tuples, structs, enums"
	@echo "  04_patterns        - Case expressions & matching"
	@echo "  05_memory          - Region-based memory management"
	@echo "  06_advanced_features - Generics & traits"
	@echo "  07_concurrency     - Actor system & messaging"
	@echo "  08_stdlib          - Standard library integration"
	@echo "  09_binding         - Variable binding & sequencing"
	@echo "  10_experimental    - Advanced & research features"
