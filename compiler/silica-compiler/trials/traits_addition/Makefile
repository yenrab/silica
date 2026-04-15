# Trait trials: trait sources under traits/ have no main(). Only entry-point units are linked and run.
# Uses checked-in silica.config (lists traits/*.silica + root entry *.silica).
# Default `make` / `make all`: compile, assemble with $(ASSEMBLER), link with rust-lld or clang.
# `make compile`: silica-compiler only (emit .sams).
# `make integrate`: depends on `all`, then assembly vs .ascomp diffs, run binaries, .sout vs .scout diffs.

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(THIS_DIR)../../src/silica-compiler

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

# Entry points: root-level files in silica.config (not under traits/)
ENTRYPOINTS := $(patsubst %.silica,%,$(filter-out traits/%,$(shell cat silica.config | grep -v '^$$')))

.PHONY: all compile integrate clean help

.DEFAULT_GOAL := all

compile:
	@cd "$(THIS_DIR)" && \
	rm -f *.sams traits/*.sams && \
	echo "Compiling with silica-compiler..." && \
	if ! "$(SILICA_COMPILER)"; then \
		echo "FAIL: Compilation failed"; \
		exit 1; \
	fi && \
	echo "✅ Assembly generated (.sams)"

all: compile
	@failed=0; \
	cd "$(THIS_DIR)" && \
	for base in $(ENTRYPOINTS); do \
		sams="$$base.sams"; \
		if [ ! -f "$$sams" ]; then \
			echo "❌ $$base: missing $$sams"; \
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
	for base in $(ENTRYPOINTS); do \
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
	echo "✅ Linked entrypoints: $(ENTRYPOINTS)"

integrate: all
	@failed=0; \
	cd "$(THIS_DIR)" && \
	for base in $(ENTRYPOINTS); do \
		sams="$$base.sams"; \
		if [ ! -f "$$sams" ]; then \
			echo "❌❌ $$base: missing $$sams"; \
			failed=1; \
			continue; \
		fi; \
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
	for base in $(ENTRYPOINTS); do \
		if [ -x "$$base" ]; then \
			{ ./"$$base" 2>&1; echo $$?; } > "$$base.sout"; \
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
	@cd "$(THIS_DIR)" && \
	rm -f *.sout traits/*.sams traits/*.o *.sams && \
	for base in $(ENTRYPOINTS); do \
		rm -f "$$base" "$$base.o"; \
	done && \
	echo "✅ Clean complete"

help:
	@echo "traits_addition: multi-file trait trials; traits/*.silica are trait definitions (no main)."
	@echo "Targets: all (default: compile+assemble+link), compile (.sams only), integrate (checks+run), clean"
