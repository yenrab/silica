# Makefile for Silica actor trials in actors_addition
# Uses silica-compiler to generate assembly, then assembles and links.
#
# silica-compiler reads silica.config in this directory and emits .sams files.
# Assembler: clang -c; linker: rust-lld if available, else clang.
#
# Run from this directory: cd actors_addition && make
# Or from anywhere: make -C /path/to/actors_addition

THIS_DIR        := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
include $(THIS_DIR)../silica_compiler.mk
TRIAL_DIR := $(notdir $(patsubst %/,%,$(THIS_DIR)))
MSG_PREFIX := $(TRIAL_DIR)/

# Rust toolchain linker (rust-lld) - optional override
RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)
RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")
RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld

LINKER := $(shell test -x "$(RUST_LLD)" && echo "$(RUST_LLD)" || echo "clang")
ASSEMBLER := clang

MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)
MACOS_MIN_VERSION ?= 26.0
ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)
LDFLAGS_rust-lld := -arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) -syslibroot $(MACOS_SDK) -lSystem -e main
LDFLAGS_clang := -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)

.DEFAULT_GOAL := all
.PHONY: all clean assembly objects executables integrate help silica.config

silica.config:
	@cd "$(THIS_DIR)" && \
	{ \
		for f in *.silica; do \
			[ -f "$$f" ] && printf '%s\n' "$$f"; \
		done; \
	} | LC_ALL=C sort > silica.config

all: executables

# Stage 1: silica-compiler generates .sams (reads silica.config in THIS_DIR)
assembly: clean silica.config
	@echo "Compiling with silica-compiler..."
	@cd "$(THIS_DIR)" && $(SILICA_COMPILER)
	@echo "✅ $(MSG_PREFIX)Assembly generated"

# Stage 2: Assemble .sams -> .o
objects: assembly
	@cd "$(THIS_DIR)" && for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		echo "Assembling: $$sams -> $$base.o"; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler $$sams -o $$base.o; \
	done

# Stage 3: Link .o -> executable
executables: objects
	@cd "$(THIS_DIR)" && \
	while read -r ln; do \
		[ -n "$$ln" ] || continue; \
		bn=$$(basename "$$ln"); \
		stem=$${bn%.silica}; \
		main_o="$$stem.o"; \
		[ -f "$$main_o" ] || continue; \
		[ "$$stem" = "__silica_runtime" ] && continue; \
		rt_one=""; \
		[ -f "__silica_runtime.o" ] && rt_one="__silica_runtime.o"; \
		echo "Linking $$stem..."; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o $$stem $$main_o $$rt_one $(LDFLAGS_rust-lld) && echo "  ✅ $(MSG_PREFIX)$$stem (rust-lld)"; \
		else \
			$(ASSEMBLER) $$main_o $$rt_one -o $$stem $(LDFLAGS_clang) && echo "  ✅ $(MSG_PREFIX)$$stem (clang)"; \
		fi; \
	done < silica.config

# integrate: compile, diff .sams vs .ascomp, run, diff .sout vs .scout
integrate:
	@$(MAKE) -f "$(THIS_DIR)Makefile" clean
	@$(MAKE) -f "$(THIS_DIR)Makefile" silica.config
	@cd "$(THIS_DIR)" || exit 1; \
	rm -f .integrate_counts; ok=0; ko=0; failed=0; \
	if [ ! -s silica.config ]; then \
		echo "❌❌ $(MSG_PREFIX)silica.config missing after generation"; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi; \
	rm -f *.sams; \
	echo "Compiling with silica-compiler..."; \
	if ! $(SILICA_COMPILER); then \
		echo "❌❌ $(MSG_PREFIX)compilation failed"; \
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
		if ! $(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler $$sams -o $$base.o; then \
		echo "❌❌ $(MSG_PREFIX)$$base assemble failed"; \
		ko=$$((ko + 1)); failed=1; break; \
	fi; \
	done; \
	while read -r ln; do \
		[ -n "$$ln" ] || continue; \
		bn=$$(basename "$$ln"); \
		stem=$${bn%.silica}; \
		main_o="$$stem.o"; \
		[ -f "$$main_o" ] || continue; \
		[ "$$stem" = "__silica_runtime" ] && continue; \
		rt_one=""; \
		[ -f "__silica_runtime.o" ] && rt_one="__silica_runtime.o"; \
		echo "Linking $$stem..."; \
		if test -x "$(RUST_LLD)"; then \
						if ! $(RUST_LLD) -flavor darwin -o $$stem $$main_o $$rt_one $(LDFLAGS_rust-lld); then \
				echo "❌❌ $(MSG_PREFIX)$$stem link failed"; \
				ko=$$((ko + 1)); failed=1; \
			fi; \
		else \
						if ! $(ASSEMBLER) $$main_o $$rt_one -o $$stem $(LDFLAGS_clang); then \
				echo "❌❌ $(MSG_PREFIX)$$stem link failed"; \
				ko=$$((ko + 1)); failed=1; \
			fi; \
		fi; \
	done < silica.config; \
	while read -r ln; do \
		[ -n "$$ln" ] || continue; \
		bn=$$(basename "$$ln"); \
		stem=$${bn%.silica}; \
		[ "$$stem" = "__silica_runtime" ] && continue; \
		if [ ! -x "$$stem" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$stem missing executable (assemble/link failed earlier)"; \
			ko=$$((ko + 1)); failed=1; continue; \
		fi; \
		if [ -f "$$stem.scout" ]; then \
			{ printf 'exit\n' | ./$$stem 2>&1; echo $$?; } > "$$stem.sout"; \
			if ! diff -Bw -q "$$stem.sout" "$$stem.scout" > /dev/null 2>&1; then \
				echo "❌❌ $(MSG_PREFIX)$$stem .sout differs from .scout"; \
				diff -Bw "$$stem.sout" "$$stem.scout" || true; \
				ko=$$((ko + 1)); failed=1; \
			else \
				echo "✅✅ $(MSG_PREFIX)$$stem output matches .scout"; \
				ok=$$((ok + 1)); \
			fi; \
		else \
			echo "❌❌ $(MSG_PREFIX)$$stem has no .scout file"; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
	done < silica.config; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

clean:
	@cd "$(THIS_DIR)" && \
	for s in *.sams; do \
		[ -f "$$s" ] || continue; \
		rm -f $${s%.sams}; \
	done; \
	rm -f *.sams *.o *.sout .integrate_counts silica.config; \
	echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "Makefile - Silica actor trials (actors_addition)"
	@echo "================================================="
	@echo "Uses: silica-compiler -> .sams -> .o -> executable"
	@echo ""
	@echo "Targets:"
	@echo "  all         - Build executables (default)"
	@echo "  assembly    - Run silica-compiler (generates .sams)"
	@echo "  objects     - Assemble .sams to .o"
	@echo "  executables - Full build: assembly + link"
	@echo "  integrate   - Compile; diff .sams vs .ascomp, run, diff .sout vs .scout"
	@echo "  clean       - Remove .sams, .o, executables, .sout"
	@echo "  help        - Show this help"
	@echo ""
	@echo "Linker: $(LINKER)"
