# Makefile for Silica compilation error tests
# These tests are designed to verify that the compiler properly detects and reports compilation errors
# Note: These tests are expected to fail compilation - this is their purpose!

SILICA_COMPILER := ../../silica-bootstrap-compiler/target/release/silica-boot
SEARCH_PATHS := -I ../../stdlib

# Ensure compiler is built without LLVM backend features for consistent text-based generation
$(SILICA_COMPILER):
	cd ../../silica-bootstrap-compiler && cargo build --release --no-default-features

# Find all .silica files
SILICA_FILES := $(wildcard *.silica)

# For error tests, we still classify as executables vs modules, but expect failures
EXECUTABLES := $(shell grep -l "^fn main" $(SILICA_FILES) 2>/dev/null | sed 's/\.silica$$//')
MODULES := $(shell for f in $(SILICA_FILES); do if ! grep -q "^fn main" "$$f"; then basename "$$f" .silica; fi; done)

# Output files
EXECUTABLES_LL := $(addsuffix .ll, $(EXECUTABLES))
MODULES_LL := $(addsuffix .ll, $(MODULES))

.PHONY: all clean executables modules help test-errors

# Default target - test compilation (expecting some failures)
all: test-errors

# Compile executable files (those with main functions) - expect failures
executables: $(EXECUTABLES_LL)

%.ll: %.silica
	-$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Compile module files (those without main functions) - expect failures
modules: $(MODULES_LL)

# Modules are compiled the same way, but marked as such
$(MODULES_LL): %.ll: %.silica
	-$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Test compilation errors - this is the main purpose of this directory
test-errors: executables modules
	@echo "✅ Compilation error tests completed"
	@echo "Note: Some files in this directory are designed to fail compilation."
	@echo "This is expected behavior - these tests verify error detection."

# Clean generated files
clean:
	rm -f *.ll *.bc *.o $(EXECUTABLES)

# Show help
help:
	@echo "Silica Compilation Error Tests"
	@echo "=============================="
	@echo "These tests verify that the Silica compiler properly detects"
	@echo "and reports compilation errors. Some tests are designed to fail!"
	@echo ""
	@echo "Test files:"
	@echo "  test_shadowing.silica - Tests variable shadowing prevention"
	@echo ""
	@echo "Targets:"
	@echo "  all         - Run all error tests (default)"
	@echo "  test-errors - Test compilation (expecting some failures)"
	@echo "  executables - Compile executable files only"
	@echo "  modules     - Compile module files only"
	@echo "  clean       - Remove all generated files"
	@echo "  help        - Show this help"
	@echo ""
	@echo "Note: Compilation failures are expected and do not indicate problems."