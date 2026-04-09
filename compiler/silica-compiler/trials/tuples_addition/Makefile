# smakefile for Silica experiments in string_addition
# Uses silica-compiler (Phase 2) to generate assembly, then assembles and links
# No Rust code - only silica-compiler + assembler + linker
#
# silica-compiler reads silica.config internally and outputs .sams (assembly) files.
# The Makefile does not read silica.config - it just runs the compiler.
# Assembler: clang -c (Rust toolchain does not ship a standalone assembler)
# Linker: rust-lld if available, else clang

SILICA_COMPILER := ../../src/silica-compiler

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
.PHONY: all clean assembly objects executables integrate help

all: executables

# Stage 1: silica-compiler generates .sams (reads silica.config internally)
assembly:
	@echo "Compiling with silica-compiler..."
	@$(SILICA_COMPILER)
	@echo "✅ Assembly generated"

# Stage 2: Assemble .sams -> .o (no Rust code, clang assembler only)
objects: assembly
	@for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		echo "Assembling: $$sams -> $$base.o"; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler $$sams -o $$base.o; \
	done

# Stage 3: Link .o -> executable (rust-lld or clang)
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

# integrate: compile .silica per silica.config, diff .sams vs .ascomp, run binaries, diff .sout vs .scout
integrate:
	@failed=0; \
	ls -1 *.silica 2>/dev/null | grep '\.silica$$' > silica.config || true; \
	if [ ! -s silica.config ]; then \
		echo "SKIP: no silica.config"; \
		exit 0; \
	fi; \
	rm -f *.sams; \
	echo "Compiling with silica-compiler..."; \
	if ! $(abspath $(SILICA_COMPILER)); then \
		echo "FAIL: Compilation failed"; \
		exit 1; \
	fi; \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		if [ ! -f "$$base.ascomp" ]; then \
			echo "❌❌ $$base has no .ascomp file"; \
			failed=1; \
		elif ! diff -Bw -q "$$sams" "$$base.ascomp" > /dev/null 2>&1; then \
			echo "❌❌ $$base .sams differs from .ascomp"; \
			diff -Bw "$$sams" "$$base.ascomp" || true; \
			failed=1; \
		else \
			echo "✅✅ $$base assembly matches .ascomp"; \
		fi; \
	done; \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		echo "Assembling $$sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler $$sams -o $$base.o || { failed=1; break; }; \
	done; \
	for o in *.o; do \
		[ -f "$$o" ] || continue; \
		base=$${o%.o}; \
		echo "Linking $$base..."; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o $$base $$o $(LDFLAGS_rust-lld) || failed=1; \
		else \
			$(ASSEMBLER) $$o -o $$base $(LDFLAGS_clang) || failed=1; \
		fi; \
	done; \
	for exe in *.sams; do \
		[ -f "$$exe" ] || continue; \
		base=$${exe%.sams}; \
		if [ -x "$$base" ]; then \
			{ ./$$base 2>&1; echo $$?; } > "$$base.sout"; \
			if [ -f "$$base.scout" ]; then \
				if ! diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
					echo "❌❌ $$base .sout differs from .scout"; \
					diff -Bw "$$base.sout" "$$base.scout" || true; \
					failed=1; \
				else \
					echo "✅✅ $$base output matches .scout"; \
				fi; \
			else \
				echo "❌❌ $$base has no .scout file"; \
				failed=1; \
			fi; \
		fi; \
	done; \
	exit $$failed

clean:
	@for s in *.sams; do \
		[ -f "$$s" ] || continue; \
		rm -f $${s%.sams}; \
	done
	rm -f *.sams *.o
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
	@echo "  integrate   - Compile, diff .sams vs .ascomp, run, diff .sout vs .scout"
	@echo "  clean       - Remove .sams, .o, executables"
	@echo "  help        - Show this help"
	@echo ""
	@echo "Linker: $(LINKER)"
