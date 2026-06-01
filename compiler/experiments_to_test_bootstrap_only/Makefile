# Master Makefile for all Silica experiments
# Automatically discovers and builds all subdirectories containing Makefiles

# Automatically discover all subdirectories that contain Makefiles
# Exclude projected/future directories: 10_data_structures, 11_patterns, 13_schedular
# Exclude directories without Makefiles: 08.1_actors, 08.2_core_affinity
SUBDIRS := $(shell find . -maxdepth 1 -type d -name "[0-9][0-9][._]*" | grep -v -E "(10_data_structures|11_patterns|13_schedular|08\.1_actors|08\.2_core_affinity)$$" | sort | sed 's|^\./||')

# LLVM tool paths (including Homebrew location on macOS)
LLVM_AS := $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo "")
LLVM_LLC := $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo "")
COMPILER_CLANG := $(shell test -x /opt/homebrew/opt/llvm/bin/clang && echo "/opt/homebrew/opt/llvm/bin/clang" || echo "clang")
LINKER_CLANG := clang
HAS_LLVM_TOOLS := $(and $(LLVM_AS),$(LLVM_LLC))

# On Raspbian/Debian, llc defaults to a generic ARM triple without VFP register
# arguments; Rust and clang use gnueabihf. macOS Homebrew llc already matches.
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
LLC_FLAGS := -mtriple=arm-linux-gnueabihf
else
LLC_FLAGS :=
endif

# Export variables for sub-Makefiles
export COMPILER_CLANG LINKER_CLANG LLVM_AS LLVM_LLC LLC_FLAGS

.PHONY: all clean $(SUBDIRS) help

# Default target - build full executables in order
all: $(SUBDIRS)
	@echo "✅ All Silica experiments compiled with full executables!"

# Build each subdirectory
$(SUBDIRS):
	@echo "🏗️  Building $@ experiments..."
	@if [ -f "$@/Makefile" ]; then \
		$(MAKE) -C $@ all || true; \
	else \
		echo "⚠️  No Makefile found in $@ - skipping"; \
	fi
	@echo "✅ $@ completed"

# Clean all subdirectories
clean:
	@echo "🧹 Cleaning all experiments..."
	@for dir in $(SUBDIRS); do \
		if [ -f "$$dir/Makefile" ]; then \
			$(MAKE) -C $$dir clean; \
		fi; \
	done
	@echo "✅ All experiments cleaned"

# Force rebuild compiler across all directories
rebuild-compiler:
	@echo "🔨 Rebuilding Silica compiler..."
	cd ../silica-bootstrap-compiler && cargo build --release --no-default-features
	@echo "✅ Compiler rebuilt"

# Clean everything including compiler
clean-all: clean rebuild-compiler

# Rebuild everything from scratch
rebuild-all: clean-all all

# Build only executables across all directories
executables:
	@echo "🏗️  Building all executables..."
	@for dir in $(SUBDIRS); do \
		if [ -f "$$dir/Makefile" ]; then \
			echo "Building executables in $$dir..."; \
			$(MAKE) -C $$dir executables || true; \
		fi; \
	done
	@echo "✅ All executables built"

# Build only modules across all directories
modules:
	@echo "🏗️  Building all modules..."
	@for dir in $(SUBDIRS); do \
		if [ -f "$$dir/Makefile" ]; then \
			echo "Building modules in $$dir..."; \
			$(MAKE) -C $$dir modules || true; \
		fi; \
	done
	@echo "✅ All modules built"

# Build full executables across all directories (requires LLVM tools)
executables-full:
	@echo "🏗️  Building all full executables..."
	@if [ -z "$(HAS_LLVM_TOOLS)" ]; then \
		echo "❌ LLVM tools not found. Install LLVM (brew install llvm on macOS)"; \
		exit 1; \
	fi
	@for dir in $(SUBDIRS); do \
		if [ -f "$$dir/Makefile" ]; then \
			echo "Building full executables in $$dir..."; \
			$(MAKE) -C $$dir executables-full || true; \
		fi; \
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
	@echo "This Makefile automatically discovers and builds all Silica test experiments."
	@echo "Each subdirectory contains tests for specific language features."
	@echo ""
	@echo "Targets:"
	@echo "  all              - Build full native executables across all directories (default)"
	@echo "  clean            - Clean all generated files"
	@echo "  clean-all        - Clean everything including compiler rebuild"
	@echo "  rebuild-all      - Clean and rebuild everything from scratch"
	@echo "  executables      - Build only executable LLVM IR files across all directories"
	@echo "  modules          - Build only module files across all directories"
	@echo "  executables-full - Build native executables (requires LLVM tools)"
	@echo "  test-basic       - Quick test of basic functionality only"
	@echo "  status           - Show build status and file counts"
	@echo "  help             - Show this help"
	@echo ""
	@echo "Subdirectories (automatically discovered):"
	@echo "  01_basic           - Language fundamentals"
	@echo "  02_expressions     - Operators & control flow"
	@echo "  03_data_types      - Tuples, structs, enums"
	@echo "  04_patterns        - Case expressions & matching"
	@echo "  05_memory          - Region-based memory management"
	@echo "  06_traits          - Traits & polymorphism"
	@echo "  08_stdlib          - Standard library integration"
	@echo "  09_binding         - Variable binding & sequencing"
	@echo "  10_experimental    - Advanced & research features"
