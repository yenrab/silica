# Supervisor trials: entrypoint *.silica at repo root; trait modules under traits/
# (same linking pattern as traits_addition — traits/*.o linked into each entrypoint).
#
# silica-compiler reads silica.config (all *.silica under this directory).
# Assembler: clang -c; linker: rust-lld if available, else clang.

THIS_DIR        := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(abspath $(THIS_DIR)../../src/silica-compiler)

TRIAL_DIR := $(notdir $(patsubst %/,%,$(THIS_DIR)))
MSG_PREFIX := $(TRIAL_DIR)/

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

.PHONY: all compile clean integrate help silica.config

silica.config:
	@cd "$(dir $(abspath $(lastword $(MAKEFILE_LIST))))" && find "$$PWD" -type f -name '*.silica' | sed "s|^$$PWD/||" > silica.config

.DEFAULT_GOAL := all

compile: silica.config
	@cd "$(THIS_DIR)" && \
	rm -f *.sams traits/*.sams && \
	echo "Compiling with silica-compiler..." && \
	if ! "$(SILICA_COMPILER)"; then \
		echo "FAIL: Compilation failed"; \
		exit 1; \
	fi && \
	echo "✅ $(MSG_PREFIX)Assembly generated (.sams)"

all: compile
	@failed=0; \
	cd "$(THIS_DIR)" && \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^traits/'); do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
		if [ ! -f "$$sams" ]; then \
			echo "❌ $(MSG_PREFIX)$$base: missing $$sams"; \
			failed=1; \
			continue; \
		fi; \
		echo "Assembling $$sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || { failed=1; break; }; \
	done; \
	for sams in traits/*.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$$(basename "$$sams" .sams); \
		obj="traits/$$base.o"; \
		echo "Assembling $$sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$obj" || { failed=1; break; }; \
	done; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^traits/'); do \
		base=$${line%.silica}; \
		[ -f "$$base.o" ] || continue; \
		echo "Linking $$base..."; \
		trait_objs=""; \
		for trait_obj in traits/*.o; do \
			[ -f "$$trait_obj" ] && trait_objs="$$trait_objs $$trait_obj"; \
		done; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" $$trait_objs $(LDFLAGS_rust-lld) || failed=1; \
		else \
			$(ASSEMBLER) "$$base.o" $$trait_objs -o "$$base" $(LDFLAGS_clang) || failed=1; \
		fi; \
	done; \
	if [ $$failed -ne 0 ]; then \
		echo "FAIL: Assemble/link failed"; \
		exit 1; \
	fi; \
	echo "✅ $(MSG_PREFIX)Linked entrypoints (root *.silica + traits/*.o)"

integrate: silica.config compile
	@cd "$(THIS_DIR)" || exit 1; \
	rm -f .integrate_counts; ok=0; ko=0; failed=0; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^traits/'); do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
		if [ ! -f "$$sams" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base: missing $$sams"; \
			ko=$$((ko + 1)); failed=1; \
			continue; \
		fi; \
		echo "Assembling $$sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || { failed=1; break; }; \
	done; \
	for sams in traits/*.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$$(basename "$$sams" .sams); \
		obj="traits/$$base.o"; \
		echo "Assembling $$sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$obj" || { failed=1; break; }; \
	done; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$'); do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
		if [ -f "$$base.ascomp" ]; then \
			if ! diff -Bw -q "$$sams" "$$base.ascomp" > /dev/null 2>&1; then \
				echo "❌❌ $(MSG_PREFIX)$$base .sams differs from .ascomp"; \
				diff -Bw "$$sams" "$$base.ascomp" || true; \
				ko=$$((ko + 1)); failed=1; \
			else \
				echo "✅✅ $(MSG_PREFIX)$$base assembly matches .ascomp"; \
				ok=$$((ok + 1)); \
			fi; \
		fi; \
	done; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^traits/'); do \
		base=$${line%.silica}; \
		[ -f "$$base.o" ] || continue; \
		echo "Linking $$base..."; \
		trait_objs=""; \
		for trait_obj in traits/*.o; do \
			[ -f "$$trait_obj" ] && trait_objs="$$trait_objs $$trait_obj"; \
		done; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" $$trait_objs $(LDFLAGS_rust-lld) || failed=1; \
		else \
			$(ASSEMBLER) "$$base.o" $$trait_objs -o "$$base" $(LDFLAGS_clang) || failed=1; \
		fi; \
	done; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^traits/'); do \
		base=$${line%.silica}; \
		if [ -x "$$base" ]; then \
			if [ -f "$$base.scout" ]; then \
				{ ./"$$base" 2>&1; echo $$?; } > "$$base.sout"; \
				if ! diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
					echo "❌❌ $(MSG_PREFIX)$$base .sout differs from .scout"; \
					diff -Bw "$$base.sout" "$$base.scout" || true; \
					ko=$$((ko + 1)); failed=1; \
				else \
					echo "✅✅ $(MSG_PREFIX)$$base output matches .scout"; \
					ok=$$((ok + 1)); \
				fi; \
			else \
				echo "✅✅ $(MSG_PREFIX)$$base built (no .scout; skipped run check)"; \
				ok=$$((ok + 1)); \
			fi; \
		fi; \
	done; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

clean:
	@cd "$(THIS_DIR)" && \
	rm -f *.sout traits/*.sams traits/*.o *.sams && \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^traits/'); do \
		base=$${line%.silica}; \
		rm -f "$$base" "$$base.o"; \
	done && \
	rm -f silica.config .integrate_counts && \
	echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "supervisors_addition: Phase A supervisor trial + local trait copy under traits/"
	@echo "Targets: all (compile+link), compile (.sams), integrate (optional .ascomp/.scout), clean"
