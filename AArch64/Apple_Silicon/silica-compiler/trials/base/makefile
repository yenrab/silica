# smakefile for Silica experiments in 01_basic
# Uses silica-compiler (Phase 2) to generate assembly, then assembles and links
# No Rust code - only silica-compiler + assembler + linker
#
# silica-compiler reads silica.config and outputs .sams (assembly) files.
# Assembler: clang -c (Rust toolchain does not ship a standalone assembler)
# Linker: rust-lld if available, else clang

SILICA_COMPILER := ../../src/silica-compiler

# Files to build: read from silica.config (one .silica per line)
CONFIG_FILES := $(shell grep -v '^\#' silica.config 2>/dev/null | grep '\.silica$$' || true)
EXECUTABLES := $(CONFIG_FILES:%.silica=%)
SAMS_FILES := $(addsuffix .sams, $(EXECUTABLES))
OBJ_FILES := $(addsuffix .o, $(EXECUTABLES))

# Rust toolchain linker (rust-lld) - optional override
RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)
RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")
RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld

# Use rust-lld if it exists, otherwise clang for linking
LINKER := $(shell test -x "$(RUST_LLD)" && echo "$(RUST_LLD)" || echo "clang")
ASSEMBLER := clang

# macOS: linker flags for minimal executable (when using rust-lld)
MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)
MACOS_MIN_VERSION ?= 26.0
ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)
LDFLAGS_rust-lld := -arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) -syslibroot $(MACOS_SDK) -lSystem -e main
LDFLAGS_clang := -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)

.DEFAULT_GOAL := all
.PHONY: all clean assembly objects executables help

all: executables

# Stage 1: silica-compiler generates .sams from .silica (reads silica.config)
assembly: $(SAMS_FILES)

$(SAMS_FILES): $(CONFIG_FILES) silica.config
	@echo "Compiling with silica-compiler..."
	@$(SILICA_COMPILER)
	@echo "✅ Assembly generated"

# Stage 2: Assemble .sams -> .o (no Rust code, clang assembler only)
objects: $(OBJ_FILES)

%.o: %.sams
	@echo "Assembling: $< -> $@"
	@$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler $< -o $@

# Stage 3: Link .o -> executable (rust-lld or clang)
executables: assembly $(OBJ_FILES)
	@echo "Linking..."
	@for exe in $(EXECUTABLES); do \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o $$exe $$exe.o $(LDFLAGS_rust-lld) && echo "  ✅ Built $$exe (rust-lld)"; \
		else \
			$(ASSEMBLER) $$exe.o -o $$exe $(LDFLAGS_clang) && echo "  ✅ Built $$exe (clang)"; \
		fi; \
	done

clean:
	rm -f *.sams *.o $(EXECUTABLES)
	@echo "✅ Clean complete"

help:
	@echo "smakefile - Silica assembly build (no Rust code)"
	@echo "================================================"
	@echo "Uses: silica-compiler -> .sams -> .o -> executable"
	@echo ""
	@echo "Targets:"
	@echo "  all         - Build executables (default)"
	@echo "  assembly    - Run silica-compiler (generates .sams)"
	@echo "  objects     - Assemble .sams to .o"
	@echo "  executables - Full build: assembly + link"
	@echo "  clean       - Remove .sams, .o, executables"
	@echo "  help        - Show this help"
	@echo ""
	@echo "Config: silica.config (one .silica file per line)"
	@echo "Linker: $(LINKER)"
