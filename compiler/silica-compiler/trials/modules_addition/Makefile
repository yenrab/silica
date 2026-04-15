# Module trials: library sources under lib/ have no main(). Only entry-point units are linked and run.
# Uses checked-in silica.config (lists lib/*.silica + root entry *.silica).
# Default `make` compiles only; `make integrate` runs assembly diffs, link, run, and output diffs.

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

# Basenames that define main() - extracted from silica.config (library .sams under lib/ are compile-only).
# Dynamically extract entry points from silica.config: files without path separators (i.e., in current dir)
ENTRYPOINTS := $(patsubst %.silica,%,$(filter-out lib/%,$(shell cat silica.config | grep -v '^$$')))

.PHONY: all integrate clean help

.DEFAULT_GOAL := all

all:
	@cd "$(THIS_DIR)" && \
	rm -f *.sams lib/*.sams && \
	echo "Compiling with silica-compiler..." && \
	if ! "$(SILICA_COMPILER)"; then \
		echo "FAIL: Compilation failed"; \
		exit 1; \
	fi && \
	echo "✅ Assembly generated"

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
		sams="$$base.sams"; \
		[ -f "$$sams" ] || continue; \
		echo "Assembling $$sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || { failed=1; break; }; \
	done; \
	for sams in lib/*.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$$(basename "$$sams" .sams); \
		obj="lib/$$base.o"; \
		echo "Assembling $$sams..."; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$obj" || { failed=1; break; }; \
	done; \
	for base in $(ENTRYPOINTS); do \
		[ -f "$$base.o" ] || continue; \
		echo "Linking $$base..."; \
		lib_objs=""; \
		for lib_obj in lib/*.o; do \
			[ -f "$$lib_obj" ] && lib_objs="$$lib_objs $$lib_obj"; \
		done; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" $$lib_objs $(LDFLAGS_rust-lld) || failed=1; \
		else \
			$(ASSEMBLER) "$$base.o" $$lib_objs -o "$$base" $(LDFLAGS_clang) || failed=1; \
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
	rm -f *.sout lib/*.sams lib/*.o *.sams && \
	for base in $(ENTRYPOINTS); do \
		rm -f "$$base" "$$base.o"; \
	done && \
	echo "✅ Clean complete"

help:
	@echo "modules_addition: multi-file module trials; lib/*.silica are libraries (no main)."
	@echo "Targets: all (default, compile only), integrate (assembly/output checks + run), clean"
