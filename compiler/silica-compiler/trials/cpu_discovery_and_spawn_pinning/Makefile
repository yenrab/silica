# Makefile for CPU topology / spawn-placement trials (cpu_discovery).
# Same pipeline as actors_addition: silica-compiler -> .sams -> .o -> executable.
#
# Run: cd cpu_discovery && make
# integrate: optional .ascomp / .scout checks (see README).

THIS_DIR        := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(abspath $(THIS_DIR)../../src/silica-compiler)

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
.PHONY: all clean assembly objects executables integrate phase-h help

all: executables

assembly:
	@echo "Compiling with silica-compiler..."
	@cd "$(THIS_DIR)" && $(SILICA_COMPILER)
	@echo "✅ Assembly generated"

objects: assembly
	@cd "$(THIS_DIR)" && for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		echo "Assembling: $$sams -> $$base.o"; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler $$sams -o $$base.o; \
	done

executables: objects
	@cd "$(THIS_DIR)" && for o in *.o; do \
		[ -f "$$o" ] || continue; \
		base=$${o%.o}; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o $$base $$o $(LDFLAGS_rust-lld) && echo "  ✅ Built $$base (rust-lld)"; \
		else \
			$(ASSEMBLER) $$o -o $$base $(LDFLAGS_clang) && echo "  ✅ Built $$base (clang)"; \
		fi; \
	done

integrate:
	@failed=0; \
	cd "$(THIS_DIR)" || exit 1; \
	ls -1 *.silica 2>/dev/null | grep '\.silica$$' > silica.config || true; \
	if [ ! -s silica.config ]; then \
		echo "SKIP: no silica.config"; \
		exit 0; \
	fi; \
	rm -f *.sams; \
	echo "Compiling with silica-compiler..."; \
	if ! $(SILICA_COMPILER); then \
		echo "FAIL: Compilation failed"; \
		exit 1; \
	fi; \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		if [ -f "$$base.ascomp" ]; then \
			if ! diff -Bw -q "$$sams" "$$base.ascomp" > /dev/null 2>&1; then \
				echo "❌❌ $$base .sams differs from .ascomp"; \
				diff -Bw "$$sams" "$$base.ascomp" || true; \
				failed=1; \
			else \
				echo "✅✅ $$base assembly matches .ascomp"; \
			fi; \
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
			if [ -f "$$base.scout" ]; then \
				{ ./$$base 2>&1; echo $$?; } > "$$base.sout"; \
				if ! diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
					echo "❌❌ $$base .sout differs from .scout"; \
					diff -Bw "$$base.sout" "$$base.scout" || true; \
					failed=1; \
				else \
					echo "✅✅ $$base output matches .scout"; \
				fi; \
			else \
				echo "✅✅ $$base built (no .scout; skipped run check)"; \
			fi; \
		fi; \
	done; \
	exit $$failed

# Phase H (cpu_topology_implementation_plan): static checks on emitter runtime asm (no binary run).
phase-h:
	@$(SHELL) "$(THIS_DIR)phase_h_static.sh"

clean:
	@cd "$(THIS_DIR)" && \
	for s in *.sams; do \
		[ -f "$$s" ] || continue; \
		rm -f $${s%.sams}; \
	done; \
	rm -f *.sams *.o *.sout; \
	echo "✅ Clean complete"

help:
	@echo "Makefile - cpu_discovery trials"
	@echo "============================="
	@echo "Uses: silica-compiler -> .sams -> .o -> executable"
	@echo "Phase H: make phase-h  (static grep of prims_actors_runtime_asm.silica)"
	@echo "See README.md for assembly golden (.ascomp) workflow."
