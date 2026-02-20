# Makefile for Silica experiments in atoms_addition
# Uses silica-compiler (Phase 2) to generate assembly, then assembles and links
# No Rust code - only silica-compiler + assembler + linker
#
# silica-compiler reads silica.config and outputs .sams (assembly) files.
# Assembler: clang -c (Rust toolchain does not ship a standalone assembler)
# Linker: rust-lld if available, else clang

SILICA_COMPILER := silica-compiler

# Rust toolchain linker (rust-lld) - optional override
RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)
RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")
RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld

# Use rust-lld if it exists, otherwise clang for linking
LINKER := $(shell test -x "$(RUST_LLD)" && echo "$(RUST_LLD)" || echo "clang")
ASSEMBLER := clang

# macOS: linker flags for minimal executable (when using rust-lld)
MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)
MACOS_MIN_VERSION := 26.0
ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)
LDFLAGS_rust-lld := -arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) -syslibroot $(MACOS_SDK) -lSystem -e main
LDFLAGS_clang := -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)

.DEFAULT_GOAL := all
.PHONY: all clean assembly objects executables help silica.config

all: executables

# Stage 1: Update silica.config with all .silica files in this directory (always refresh)
silica.config:
	@echo "Updating silica.config with all .silica files..."
	@ls -1 *.silica 2>/dev/null | grep '\.silica$$' > silica.config || true
	@echo "✅ silica.config updated"

# Stage 2: silica-compiler generates .sams (reads silica.config)
assembly: silica.config
	@echo "Compiling with silica-compiler..."
	@$(SILICA_COMPILER)
	@echo "✅ Assembly generated"

# Stage 3: Assemble .sams -> .o (no Rust code, clang assembler only)
objects: assembly
	@for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
	base=$${sams%.sams}; \
	echo "Assembling: $$sams -> $$base.o"; \
	$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler $$sams -o $$base.o; \
	done

# Stage 4: Link .o -> executable (rust-lld or clang)
executables: objects
	@for o in *.o; do \
	[ -f "$$o" ] || continue; \
	base=$${o%.o}; \
	if test -x "$(RUST_LLD)"; then \
		$(RUST_LLD) -flavor darwin -o $$base $$o $(LDFLAGS_rust-lld) && echo "  ✅ Built $$base (rust-lld)"; \
	else \
		$(ASSEMBLER) $$o -o $$base $(LDFLAGS_clang) && echo "  ✅ Built $$base (clang)"; \
	fi; \
	done

clean:
	@for s in *.sams; do \
		[ -f "$$s" ] || continue; \
		rm -f $${s%.sams}; \
	done
	rm -f *.sams *.o
	@echo "✅ Clean complete"

help:
	@echo "Makefile - Silica assembly build (no Rust code)"
	@echo "=============================================="
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
	@echo "Config: silica.config (auto-generated from all .silica files in this directory)"
	@echo "Linker: $(LINKER)"
