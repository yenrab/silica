# Makefile for Silica Phase 2 Compiler
# Compiles the compiler source code in src/, lexer/, and parser/ using the Silica bootstrap compiler
#
# Usage:
#   make                        # compile silica-compiler (default; no trial integration)
#   make all                    # clean, then compile
#   make TARGET=esp32-s3        # builds emitter for ESP32-S3
#   make TARGET=apple_silicon_mac   # builds emitter for Apple Silicon
#   make integrate              # build compiler, then `make integrate` in each trials/*/ Makefile

# Target chip directory under emitter/ (override from command line)
TARGET ?= apple_silicon_mac

SILICA_COMPILER := ../../silica-bootstrap-compiler/target/release/silica-boot
RUNTIME_LIB := ../../silica-bootstrap-compiler/target/release/libsilica_compiler.a
SEARCH_PATHS := -I lexer -I parser -I parser/capabilities -I parser/declarations -I parser/expressions -I parser/patterns -I parser/types -I parser/effects -I type_checker -I type_checker/declarations -I type_checker/expressions -I type_checker/traits -I module_checker -I trait_checker -I effect_checker -I ffi -I sir_generator -I sir_generator/declarations -I sir_generator/terms -I sir_generator/types -I sir_generator/patterns -I sir_generator/effects -I emitter -I emitter/$(TARGET) -I emitter/$(TARGET)/atoms -I emitter/$(TARGET)/constraints -I emitter/$(TARGET)/terms -I emitter/$(TARGET)/terms/prims -I emitter/$(TARGET)/terms/const -I emitter/$(TARGET)/terms/let -I emitter/$(TARGET)/terms/call -I emitter/$(TARGET)/shared -I emitter/$(TARGET)/control -I emitter/$(TARGET)/effects -I emitter/$(TARGET)/recursion -I data_structures -I build_output -I diagnostics -I ../stdlib -I ../stdlib/data_structures

# LLVM toolchain (for .ll -> .o and linking)
LLVM_AS := $(or $(LLVM_AS), $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo "llvm-as"))
LLC := $(or $(LLVM_LLC), $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo "llc"))
LINKER_CLANG := $(or $(LINKER_CLANG), $(shell test -x /opt/homebrew/opt/llvm/bin/clang && echo "/opt/homebrew/opt/llvm/bin/clang" || echo "clang"))

# Ensure compiler is built without LLVM backend features for consistent text-based generation
$(SILICA_COMPILER):
	@echo "Building Silica bootstrap compiler..."
	cd ../../silica-bootstrap-compiler && cargo build --release --no-default-features

# Runtime library is built together with silica-boot
$(RUNTIME_LIB): $(SILICA_COMPILER)
	@true

# Find all .silica files in lexer, parser, type_checker, effect_checker, sir_generator, emitter (including subdirs)
LEXER_FILES := $(wildcard lexer/*.silica)
LEXER_MODULES := $(LEXER_FILES:lexer/%.silica=lexer/%)
PARSER_FILES := $(wildcard parser/*.silica parser/*/*.silica)
PARSER_MODULES := $(patsubst %.silica,%,$(PARSER_FILES))
TYPE_CHECKER_FILES := $(wildcard type_checker/*.silica type_checker/*/*.silica)
TYPE_CHECKER_MODULES := $(patsubst %.silica,%,$(TYPE_CHECKER_FILES))
MODULE_CHECKER_FILES := $(wildcard module_checker/*.silica)
MODULE_CHECKER_MODULES := $(patsubst %.silica,%,$(MODULE_CHECKER_FILES))
FFI_FILES := $(wildcard ffi/*.silica)
FFI_MODULES := $(patsubst %.silica,%,$(FFI_FILES))
TRAIT_CHECKER_FILES := $(wildcard trait_checker/*.silica)
TRAIT_CHECKER_MODULES := $(patsubst %.silica,%,$(TRAIT_CHECKER_FILES))
EFFECT_CHECKER_FILES := $(wildcard effect_checker/*.silica)
EFFECT_CHECKER_MODULES := $(patsubst %.silica,%,$(EFFECT_CHECKER_FILES))

# Main executable file
MAIN_FILE := main.silica
MAIN_LL := main.ll
MAIN_BC := main.bc
MAIN_OBJ := main.o
EXECUTABLE := silica-compiler

# Output files for lexer, parser, type_checker, and effect_checker modules
LEXER_LL := $(addsuffix .ll, $(LEXER_MODULES))
PARSER_LL := $(addsuffix .ll, $(PARSER_MODULES))
TYPE_CHECKER_LL := $(addsuffix .ll, $(TYPE_CHECKER_MODULES))
MODULE_CHECKER_LL := $(addsuffix .ll, $(MODULE_CHECKER_MODULES))
FFI_LL := $(addsuffix .ll, $(FFI_MODULES))
TRAIT_CHECKER_LL := $(addsuffix .ll, $(TRAIT_CHECKER_MODULES))
EFFECT_CHECKER_LL := $(addsuffix .ll, $(EFFECT_CHECKER_MODULES))

# Subdirectories built with silica-boot (.silica -> .ll). stdlib/data_structures is built separately.
SUBDIRS := lexer parser type_checker module_checker ffi effect_checker sir_generator emitter

# Trials: immediate subdirs of ../trials that contain Makefile or makefile (each defines its own integrate)
TRIALS_ROOT := ../trials
TRIAL_MAKEFILES := $(wildcard $(TRIALS_ROOT)/*/Makefile $(TRIALS_ROOT)/*/makefile)
TRIAL_SUBDIRS := $(sort $(foreach mf,$(TRIAL_MAKEFILES),$(notdir $(patsubst %/,%,$(dir $(mf))))))

.PHONY: all clean build build-bootstrap build-selfhost assembly-selfhost executables modules subdirs help integrate $(SUBDIRS)

.DEFAULT_GOAL := build

# Default `make` compiles only (bootstrap → this frozen tree). Use `make all` for clean + build;
# `make integrate` for trials. Parallel self-host path: `make build-selfhost` (§12) → ../src_selfhost.

all: clean build

# Real build target - ensure bootstrap compiler exists, then compile subdirs, modules, and main

build: $(SILICA_COMPILER) subdirs modules executables

# Explicit alias for the frozen bootstrap path (default `build`).
build-bootstrap: build

# Additive parallel-tree targets (§12). Do not change the default bootstrap recipe above.
BOOTSTRAP_COMPILER := $(SILICA_COMPILER)
HOST_COMPILER ?= $(abspath $(CURDIR)/silica-compiler)

assembly-selfhost:
	@$(MAKE) -C ../src_selfhost assembly-selfhost HOST_COMPILER="$(HOST_COMPILER)"

build-selfhost:
	@$(MAKE) -C ../src_selfhost build-selfhost HOST_COMPILER="$(HOST_COMPILER)"

# Build all subdirectories with Makefiles (before compiling main.silica)
# subdirs requires bootstrap compiler to exist first
subdirs: $(SILICA_COMPILER) $(SUBDIRS)

$(SUBDIRS):
	@echo "Building $@..."
	@$(MAKE) -C $@ QUIET=1 TARGET=$(TARGET)

# Compile lexer, parser, type_checker, and effect_checker modules
modules: $(LEXER_LL) $(PARSER_LL) $(TYPE_CHECKER_LL) $(MODULE_CHECKER_LL) $(FFI_LL) $(TRAIT_CHECKER_LL) $(EFFECT_CHECKER_LL)

# Compile lexer module files
lexer/%.ll: lexer/%.silica $(SILICA_COMPILER)
	@$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Compile parser module files (depend on lexer for token types; parser/%.ll covers subdirs)
parser/%.ll: parser/%.silica $(LEXER_LL) $(SILICA_COMPILER)
	@$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Compile type_checker module files (type_checker/%.ll covers subdirs)
type_checker/%.ll: type_checker/%.silica $(LEXER_LL) $(PARSER_LL) $(SILICA_COMPILER)
	@$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Module checker (after parser + type_checker core for qualified-call helpers)
module_checker/%.ll: module_checker/%.silica $(LEXER_LL) $(PARSER_LL) $(TYPE_CHECKER_LL) $(SILICA_COMPILER)
	@$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

ffi/%.ll: ffi/%.silica $(LEXER_LL) $(PARSER_LL) $(MODULE_CHECKER_LL) $(SILICA_COMPILER)
	@$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

trait_checker/%.ll: trait_checker/%.silica $(LEXER_LL) $(PARSER_LL) $(TYPE_CHECKER_LL) $(SILICA_COMPILER)
	@$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# Compile main executable (LLVM IR -> object -> linked binary)
executables: $(EXECUTABLE)

# Main LLVM IR depends on subdirs, lexer, parser, type_checker, effect_checker, and emitter (main.silica imports emitter_core)
EMITTER_FILES := $(wildcard emitter/$(TARGET)/*.silica emitter/$(TARGET)/*/*.silica emitter/$(TARGET)/*/*/*.silica)
BUILD_OUTPUT_FILES := $(wildcard build_output/*.silica)
$(MAIN_LL): $(MAIN_FILE) subdirs $(LEXER_LL) $(PARSER_LL) $(TYPE_CHECKER_LL) $(MODULE_CHECKER_LL) $(FFI_LL) $(TRAIT_CHECKER_LL) $(EFFECT_CHECKER_LL) $(EMITTER_FILES) $(BUILD_OUTPUT_FILES) $(SILICA_COMPILER)
	@$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

# main.ll -> main.bc (bitcode)
$(MAIN_BC): $(MAIN_LL)
	@echo "Assembling: $< -> $@"
	@-$(LLVM_AS) $< -o $@

# main.bc -> main.o (object file)
$(MAIN_OBJ): $(MAIN_BC)
	@echo "Compiling: $< -> $@"
	@-$(LLC) -filetype=obj $< -o $@

# Link main.o + runtime into silica-compiler executable
# (main.silica is compiled as a single unit with all imports, so main.o contains everything)
$(EXECUTABLE): $(MAIN_OBJ) $(RUNTIME_LIB)
	@echo "Linking: $(EXECUTABLE)"
	@# -stack_size: the self-hosted lexer/parser recurse over the whole source file,
	@# so the main-thread stack must be larger than the macOS 8MB default for big
	@# inputs (e.g. stdlib/data_structures/btree_nodeid.silica). 256MB headroom.
	@-$(LINKER_CLANG) $(MAIN_OBJ) $(RUNTIME_LIB) -Wl,-stack_size,0x10000000 -o $@
	@echo "✅ Built $(EXECUTABLE)"

# Run each trial area's own integrate target (does not use ../trials/Makefile inline trial runner)
integrate: $(EXECUTABLE)
	@echo "Running make integrate in each subdirectory of $(TRIALS_ROOT)..."
	@failed=0; \
	for dir in $(TRIAL_SUBDIRS); do \
		echo "=== $(TRIALS_ROOT)/$$dir (make integrate) ==="; \
		$(MAKE) -C "$(TRIALS_ROOT)/$$dir" integrate TARGET=$(TARGET) || failed=1; \
	done; \
	exit $$failed

# Clean generated files (including subdirectories)
clean:
	@echo "Cleaning generated files..."
	@for d in $(SUBDIRS); do $(MAKE) -C $$d clean TARGET=$(TARGET) 2>/dev/null || true; done
	rm -f *.ll *.bc *.o $(EXECUTABLE)
	rm -f lexer/*.ll
	rm -f parser/*.ll parser/*/*.ll
	rm -f type_checker/*.ll type_checker/*/*.ll
	rm -f module_checker/*.ll
	rm -f trait_checker/*.ll
	rm -f effect_checker/*.ll
	@echo "✅ Clean complete"

# Help target
help:
	@echo "Silica Phase 2 Compiler Makefile"
	@echo "================================="
	@echo ""
	@echo "This Makefile compiles the Phase 2 Silica compiler source code using the bootstrap compiler."
	@echo ""
	@echo "Usage:"
	@echo "  make                        # compile silica-compiler (default; no trials)"
	@echo "  make all                    # clean, then compile"
	@echo "  make TARGET=esp32-s3        # build emitter for ESP32-S3"
	@echo ""
	@echo "Available emitter targets: $(patsubst %/,%,$(wildcard emitter/*/))"
	@echo "Current target:            $(TARGET)"
	@echo ""
	@echo "Targets:"
	@echo "  build            - Compile silica-compiler via bootstrap (default goal)"
	@echo "  build-bootstrap  - Alias for build (frozen src/)"
	@echo "  assembly-selfhost / build-selfhost - Parallel tree ../src_selfhost (§12)"
	@echo "  all              - clean, then build"
	@echo "  subdirs          - Build lexer, parser, type_checker, sir_generator, emitter"
	@echo "  executables      - Compile main.silica to LLVM IR"
	@echo "  modules          - Compile lexer and parser module files to LLVM IR"
	@echo "  clean            - Remove all generated .ll files"
	@echo "  integrate        - Build silica-compiler, then make integrate in each $(TRIALS_ROOT)/*/ Makefile"
	@echo "  help             - Show this help"
	@echo ""
	@echo "Files:"
	@echo "  main.silica      - Main compiler entry point"
	@echo "  lexer/*.silica   - Lexer module files"
	@echo "  parser/*.silica  - Parser core and subdirs (declarations/, expressions/, patterns/, types/, effects/)"
	@echo "  type_checker/*.silica - Type checker core and subdirs (declarations/, expressions/, traits/)"
	@echo ""
	@echo "Output:"
	@echo "  main.ll          - Main executable LLVM IR"
	@echo "  main.o           - Main object file"
	@echo "  silica-compiler  - Linked executable (requires llvm-as, llc, clang)"
	@echo "  lexer/*.ll       - Lexer module LLVM IR files"
	@echo "  parser/*.ll      - Parser module LLVM IR files (including subdirectories)"
	@echo "  type_checker/*.ll - Type checker module LLVM IR files (including subdirectories)"
