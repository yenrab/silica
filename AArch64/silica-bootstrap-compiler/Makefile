# Master Makefile for Silica AArch64 Architecture
# Builds the bootstrap compiler and all experiments

# Compiler settings (we're now in the compiler directory)
COMPILER_BINARY := target/release/silica-boot

# Experiments directory (one level up)
EXPERIMENTS_DIR := ../experiments

.DEFAULT_GOAL := all
.PHONY: all clean clean-compiler clean-experiments compiler experiments help

# Default target - build everything
all: compiler experiments
	@echo "✅ Silica AArch64 build complete!"

# Build the bootstrap compiler
compiler: $(COMPILER_BINARY)
	@echo "✅ Silica bootstrap compiler built"

$(COMPILER_BINARY):
	@echo "🏗️  Building Silica bootstrap compiler..."
	cargo build --release --no-default-features

# Build all experiments
experiments:
	@echo "🏗️  Building all Silica experiments..."
	$(MAKE) -C $(EXPERIMENTS_DIR) all

# Clean everything
clean: clean-compiler clean-experiments
	@echo "✅ All components cleaned"

# Clean just the compiler
clean-compiler:
	@echo "🧹 Cleaning Silica bootstrap compiler..."
	cargo clean

# Clean just the experiments
clean-experiments:
	@echo "🧹 Cleaning Silica experiments..."
	$(MAKE) -C $(EXPERIMENTS_DIR) clean

# Force rebuild compiler
rebuild-compiler:
	@echo "🔨 Rebuilding Silica bootstrap compiler..."
	cargo clean && cargo build --release --no-default-features

# Rebuild everything from scratch
rebuild-all: rebuild-compiler
	$(MAKE) -C $(EXPERIMENTS_DIR) rebuild-all

# Quick test - just build basic functionality
test-basic:
	$(MAKE) compiler
	$(MAKE) -C $(EXPERIMENTS_DIR) test-basic

# Show build status
status:
	@echo "Silica AArch64 Build Status"
	@echo "==========================="
	@echo "Compiler: $(COMPILER_BINARY)"
	@if [ -f "$(COMPILER_BINARY)" ]; then \
		echo "  ✅ Built"; \
		ls -lh $(COMPILER_BINARY); \
	else \
		echo "  ❌ Not built"; \
	fi
	@echo ""
	$(MAKE) -C $(EXPERIMENTS_DIR) status

# Help target
help:
	@echo "Silica AArch64 Master Makefile"
	@echo "=============================="
	@echo ""
	@echo "This Makefile builds the complete Silica ecosystem for AArch64:"
	@echo "  - Silica bootstrap compiler (Rust)"
	@echo "  - All language experiments and tests"
	@echo ""
	@echo "Targets:"
	@echo "  all              - Build compiler and all experiments (default)"
	@echo "  compiler         - Build only the bootstrap compiler"
	@echo "  experiments      - Build only the experiments"
	@echo "  clean            - Clean everything"
	@echo "  clean-compiler   - Clean only the compiler"
	@echo "  clean-experiments - Clean only the experiments"
	@echo "  rebuild-compiler - Force rebuild of compiler"
	@echo "  rebuild-all      - Clean and rebuild everything from scratch"
	@echo "  test-basic       - Quick test of basic functionality"
	@echo "  status           - Show build status and file counts"
	@echo "  help             - Show this help"
	@echo ""
	@echo "Directory structure:"
	@echo "  .                 - Rust compiler implementation (current directory)"
	@echo "  ../experiments/   - Language feature tests"
	@echo "  ../stdlib/        - Standard library modules"
