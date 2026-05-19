# smakefile for Silica experiments in float64_addition
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

TRIAL_DIR := $(notdir $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST))))))
MSG_PREFIX := $(TRIAL_DIR)/


.DEFAULT_GOAL := all
.PHONY: all clean assembly objects executables integrate help silica.config

silica.config: | clean
	@cd "$(dir $(abspath $(lastword $(MAKEFILE_LIST))))" && find "$$PWD" -type f -name '*.silica' | sed "s|^$$PWD/||" > silica.config

all: executables

# Stage 1: silica-compiler generates .sams (reads silica.config internally)
assembly: clean silica.config
	@echo "Compiling with silica-compiler..."
	@$(SILICA_COMPILER)
	@echo "✅ $(MSG_PREFIX)Assembly generated"

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
		[ "$$base" = "__silica_runtime" ] && continue; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
	if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o $$base $$o $$runtime_obj $(LDFLAGS_rust-lld) && echo "  ✅ $(MSG_PREFIX)$$base (rust-lld)"; \
		else \
			$(ASSEMBLER) $$o $$runtime_obj -o $$base $(LDFLAGS_clang) && echo "  ✅ $(MSG_PREFIX)$$base (clang)"; \
		fi; \
	done

# integrate: compile .silica per silica.config, diff .sams vs .ascomp, run binaries, diff .sout vs .scout
integrate: clean silica.config
	@rm -f .integrate_counts; ok=0; ko=0; failed=0; \
	if [ ! -s silica.config ]; then \
		echo "SKIP: no silica.config"; \
		printf '%d %d\n' 0 0 > .integrate_counts; \
		exit 0; \
	fi; \
	rm -f *.sams; \
	echo "Compiling with silica-compiler..."; \
	if ! $(abspath $(SILICA_COMPILER)); then \
		echo "FAIL: Compilation failed"; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi; \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
	if [ ! -f "$$base.ascomp" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base has no .ascomp file"; \
			ko=$$((ko + 1)); failed=1; \
		elif ! diff -Bw -q "$$sams" "$$base.ascomp" > /dev/null 2>&1; then \
			echo "❌❌ $(MSG_PREFIX)$$base .sams differs from .ascomp"; \
			diff -Bw "$$sams" "$$base.ascomp" || true; \
			ko=$$((ko + 1)); failed=1; \
		else \
			echo "✅✅ $(MSG_PREFIX)$$base assembly matches .ascomp"; \
			ok=$$((ok + 1)); \
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
		[ "$$base" = "__silica_runtime" ] && continue; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
	echo "Linking $$base..."; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o $$base $$o $$runtime_obj $(LDFLAGS_rust-lld) || failed=1; \
		else \
			$(ASSEMBLER) $$o $$runtime_obj -o $$base $(LDFLAGS_clang) || failed=1; \
		fi; \
	done; \
	for exe in *.sams; do \
		[ -f "$$exe" ] || continue; \
		base=$${exe%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
	if [ -x "$$base" ]; then \
			{ ./$$base 2>&1; echo $$?; } > "$$base.sout"; \
			if [ -f "$$base.scout" ]; then \
				if ! diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
					echo "❌❌ $(MSG_PREFIX)$$base .sout differs from .scout"; \
					diff -Bw "$$base.sout" "$$base.scout" || true; \
					ko=$$((ko + 1)); failed=1; \
				else \
					echo "✅✅ $(MSG_PREFIX)$$base output matches .scout"; \
					ok=$$((ok + 1)); \
				fi; \
			else \
				echo "❌❌ $(MSG_PREFIX)$$base has no .scout file"; \
				ko=$$((ko + 1)); failed=1; \
			fi; \
		fi; \
	done; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

clean:
	@for s in *.sams; do \
		[ -f "$$s" ] || continue; \
		rm -f $${s%.sams}; \
	done
	rm -f *.sams *.o silica.config .integrate_counts
	@echo "✅ $(MSG_PREFIX)Clean complete"

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
