# Supervisor trials: entrypoint *.silica at repo root; trait modules under traits/
# (same linking pattern as traits_addition — traits/*.o linked into each entrypoint).
#
# silica-compiler reads silica.config (all *.silica under this directory).
# Assembler: clang -c; linker: rust-lld if available, else clang.
#
# integrate uses bash (arithmetic, run_integration_binary.sh). /bin/sh is not required to support this recipe.
SHELL := /bin/bash

THIS_DIR        := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
include $(THIS_DIR)../silica_compiler.mk

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

.PHONY: all compile clean integrate stress-j2 help silica.config

silica.config:
	@cd "$(THIS_DIR)" && { \
		find "$$PWD" -type f -name '*.silica' ! -name 'trial_negative_*.silica' | sed "s|^$$PWD/||"; \
		echo "../../compiler/silica-compiler/stdlib/Supervisor.silica"; \
	} > silica.config

.DEFAULT_GOAL := all

compile: clean silica.config
	@cd "$(THIS_DIR)" && \
	rm -f *.sams *.iface silica.compile.order silica.needs_runtime silica.link __silica_runtime.sams traits/*.sams __silica_runtime.o && \
	echo "Compiling with silica-compiler..." && \
	if ! { $(RUN_SILICA_COMPILER); }; then \
		echo "❌❌ $(MSG_PREFIX)compilation failed"; \
		exit 1; \
	fi && \
	echo "✅ $(MSG_PREFIX)Assembly generated (.sams)"

all: compile
	@failed=0; \
	cd "$(THIS_DIR)" && \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^traits/' | grep -v '^\.\./\.\./compiler/silica-compiler/stdlib/'); do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
		if [ ! -f "$$sams" ]; then \
			echo "❌ $(MSG_PREFIX)$$base: missing $$sams"; \
			failed=1; \
			continue; \
		fi; \
		echo "Assembling $$sams..."; \
		if ! $(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o"; then \
		echo "❌❌ $(MSG_PREFIX)$$base assemble failed"; \
		ko=$$((ko + 1)); failed=1; \
	fi; \
	done; \
	for sams in traits/*.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$$(basename "$$sams" .sams); \
		obj="traits/$$base.o"; \
		echo "Assembling $$sams..."; \
		if ! $(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$obj"; then \
		echo "❌❌ $(MSG_PREFIX)$$obj assemble failed"; \
		ko=$$((ko + 1)); failed=1; \
	fi; \
	done; \
	if [ -f "__silica_runtime.sams" ]; then \
		echo "Assembling __silica_runtime.sams..."; \
		if ! $(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "__silica_runtime.sams" -o "__silica_runtime.o"; then \
		echo "❌❌ $(MSG_PREFIX)__silica_runtime assemble failed"; \
		ko=$$((ko + 1)); failed=1; \
	fi; \
	fi; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^traits/' | grep -v '^\.\./\.\./compiler/silica-compiler/stdlib/'); do \
		base=$${line%.silica}; \
		[ -f "$$base.o" ] || continue; \
		echo "Linking $$base..."; \
		trait_objs=""; \
		for trait_obj in traits/*.o; do \
			[ -f "$$trait_obj" ] && trait_objs="$$trait_objs $$trait_obj"; \
		done; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		if test -x "$(RUST_LLD)"; then \
			if ! $(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" $$trait_objs $$runtime_obj $(LDFLAGS_rust-lld); then \
			echo "❌❌ $(MSG_PREFIX)$$base link failed"; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
		else \
			if ! $(ASSEMBLER) "$$base.o" $$trait_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang); then \
			echo "❌❌ $(MSG_PREFIX)$$base link failed"; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
		fi; \
	done; \
	if [ $$failed -ne 0 ]; then \
		echo "❌❌ $(MSG_PREFIX)assemble/link failed"; \
		exit 1; \
	fi; \
	echo "✅ $(MSG_PREFIX)Linked entrypoints (root *.silica + traits/*.o)"

# integrate: For each linked entrypoint with a .scout golden, run via
# run_integration_exit_after_marker.py and wait for a line containing "done" before writing "exit\n".
# The helper captures stdout + stderr and appends trailing exit code into <base>.sout.
# Compare: default diff -Bw <base>.sout <base>.scout. If <base>.scout.normalized exists, use
# compare_scout_normalized.sh (ordered compare with dynamic runtime fields normalized). If <base>.scout.multiset
# exists, use compare_scout_multiset.sh (or compare_scout_multiset_oneof.sh when <base>.scout.alt exists —
# multiset matches any listed golden).
integrate:
	@$(MAKE) -f "$(THIS_DIR)Makefile" clean
	@$(MAKE) -f "$(THIS_DIR)Makefile" silica.config
	@$(MAKE) -f "$(THIS_DIR)Makefile" compile
	@cd "$(THIS_DIR)" || exit 1; \
	rm -f .integrate_counts; ok=0; ko=0; failed=0; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^traits/' | grep -v '^\.\./\.\./compiler/silica-compiler/stdlib/'); do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
		if [ ! -f "$$sams" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base: missing $$sams"; \
			ko=$$((ko + 1)); failed=1; \
			continue; \
		fi; \
		echo "Assembling $$sams..."; \
		if ! $(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o"; then \
		echo "❌❌ $(MSG_PREFIX)$$base assemble failed"; \
		ko=$$((ko + 1)); failed=1; \
	fi; \
	done; \
	for sams in traits/*.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$$(basename "$$sams" .sams); \
		obj="traits/$$base.o"; \
		echo "Assembling $$sams..."; \
		if ! $(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$obj"; then \
		echo "❌❌ $(MSG_PREFIX)$$obj assemble failed"; \
		ko=$$((ko + 1)); failed=1; \
	fi; \
	done; \
	if [ -f "__silica_runtime.sams" ]; then \
		echo "Assembling __silica_runtime.sams..."; \
		if ! $(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "__silica_runtime.sams" -o "__silica_runtime.o"; then \
		echo "❌❌ $(MSG_PREFIX)__silica_runtime assemble failed"; \
		ko=$$((ko + 1)); failed=1; \
	fi; \
	fi; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$'); do \
		base=$${line%.silica}; \
		sams="$$base.sams"; \
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
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^traits/' | grep -v '^\.\./\.\./compiler/silica-compiler/stdlib/'); do \
		base=$${line%.silica}; \
		[ -f "$$base.o" ] || continue; \
		echo "Linking $$base..."; \
		trait_objs=""; \
		for trait_obj in traits/*.o; do \
			[ -f "$$trait_obj" ] && trait_objs="$$trait_objs $$trait_obj"; \
		done; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		if test -x "$(RUST_LLD)"; then \
			if ! $(RUST_LLD) -flavor darwin -o "$$base" "$$base.o" $$trait_objs $$runtime_obj $(LDFLAGS_rust-lld); then \
			echo "❌❌ $(MSG_PREFIX)$$base link failed"; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
		else \
			if ! $(ASSEMBLER) "$$base.o" $$trait_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang); then \
			echo "❌❌ $(MSG_PREFIX)$$base link failed"; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
		fi; \
	done; \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^traits/' | grep -v '^\.\./\.\./compiler/silica-compiler/stdlib/'); do \
		base=$${line%.silica}; \
		if [ ! -x "$$base" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base missing executable (assemble/link failed earlier)"; \
			ko=$$((ko + 1)); failed=1; \
		elif [ -x "$$base" ]; then \
			if [ -f "$$base.scout" ]; then \
				if [ -f "$(THIS_DIR)run_integration_exit_after_marker.py" ]; then \
					python3 "$(THIS_DIR)run_integration_exit_after_marker.py" "$(THIS_DIR)" "$$base" "$$base.sout" "done"; \
				else \
					echo "❌❌ $(MSG_PREFIX)$$base missing run_integration_exit_after_marker.py"; \
					ko=$$((ko + 1)); failed=1; \
					continue; \
				fi; \
				if [ -f "$$base.scout.normalized" ]; then \
					if [ -f "$(THIS_DIR)compare_scout_normalized.sh" ] && ! "$(SHELL)" "$(THIS_DIR)compare_scout_normalized.sh" "$$base.sout" "$$base.scout"; then \
						echo "❌❌ $(MSG_PREFIX)$$base .sout normalized-compare differs from .scout"; \
						ko=$$((ko + 1)); failed=1; \
					elif [ ! -f "$(THIS_DIR)compare_scout_normalized.sh" ] && ! diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
						echo "❌❌ $(MSG_PREFIX)$$base .sout differs from .scout (missing compare_scout_normalized.sh; used diff -Bw)"; \
						diff -Bw "$$base.sout" "$$base.scout" || true; \
						ko=$$((ko + 1)); failed=1; \
					else \
						echo "✅✅ $(MSG_PREFIX)$$base output normalized-matches .scout"; \
						ok=$$((ok + 1)); \
					fi; \
				elif [ -f "$$base.scout.multiset" ]; then \
					if [ -f "$$base.scout.alt" ]; then \
						if [ -f "$(THIS_DIR)compare_scout_multiset_oneof.sh" ] && ! "$(SHELL)" "$(THIS_DIR)compare_scout_multiset_oneof.sh" "$$base.sout" "$$base.scout" "$$base.scout.alt"; then \
							echo "❌❌ $(MSG_PREFIX)$$base .sout multiset-compare differs from all of .scout .scout.alt"; \
							ko=$$((ko + 1)); failed=1; \
						elif [ ! -f "$(THIS_DIR)compare_scout_multiset_oneof.sh" ] && ! diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
							echo "❌❌ $(MSG_PREFIX)$$base .sout differs from .scout (missing compare_scout_multiset_oneof.sh; used diff -Bw)"; \
							diff -Bw "$$base.sout" "$$base.scout" || true; \
							ko=$$((ko + 1)); failed=1; \
						else \
							echo "✅✅ $(MSG_PREFIX)$$base output multiset-matches a golden (.scout or .scout.alt)"; \
							ok=$$((ok + 1)); \
						fi; \
					elif [ -f "$(THIS_DIR)compare_scout_multiset.sh" ] && ! "$(SHELL)" "$(THIS_DIR)compare_scout_multiset.sh" "$$base.sout" "$$base.scout"; then \
						echo "❌❌ $(MSG_PREFIX)$$base .sout multiset-compare differs from .scout"; \
						ko=$$((ko + 1)); failed=1; \
					elif [ ! -f "$(THIS_DIR)compare_scout_multiset.sh" ] && ! diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
						echo "❌❌ $(MSG_PREFIX)$$base .sout differs from .scout (missing compare_scout_multiset.sh; used diff -Bw)"; \
						diff -Bw "$$base.sout" "$$base.scout" || true; \
						ko=$$((ko + 1)); failed=1; \
					else \
						echo "✅✅ $(MSG_PREFIX)$$base output multiset-matches .scout"; \
						ok=$$((ko + 1)); \
					fi; \
				elif ! diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
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

# Phase J2: full integrate gate + e4e/e4f stress loop (see design_documents/supervisors_j2_runtime_hardening.md)
stress-j2:
	@cd "$(THIS_DIR)" && bash ./stress_j2_supervision_harness.sh

# clean removes build artifacts only. Preserves golden/compare inputs: *.scout, *.scout.normalized,
# *.scout.multiset, *.scout.alt, *.ascomp.
clean:
	@cd "$(THIS_DIR)" && \
	rm -f *.sout traits/*.sams traits/*.o *.iface *.sams __silica_runtime.o && \
	for line in $$(grep -v '^#' silica.config 2>/dev/null | grep -v '^$$' | grep -v '^traits/' | grep -v '^\.\./\.\./compiler/silica-compiler/stdlib/'); do \
		base=$${line%.silica}; \
		rm -f "$$base" "$$base.o"; \
	done && \
	rm -f silica.config .integrate_counts silica.compile.order silica.needs_runtime silica.link && \
	echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "supervisors_addition: Phase A supervisor trial + local trait copy under traits/"
	@echo "Targets: all (compile+link), compile (.sams), integrate (optional .ascomp/.scout; capture → diff, normalized / .scout.normalized, or multiset / .scout.multiset + optional .scout.alt one-of), stress-j2 (Phase J2 harness), clean (keeps .scout / .scout.normalized / .scout.multiset / .ascomp)"
