# Phase 1 standard data structures — dependency-ordered trial root (Layer 0 §6.2)
#
# First-time golden capture (after binaries/silica-compiler is available):
#   make record-golden
#
# Build only (default):
#   make
#   make all
#
# Normal integration:
#   make integrate
#
# Each `make all` / `make integrate` / `make record-golden` compiles the shared
# stdlib data structures once into .stdlib_cache/lib/*.o and reuses those
# objects (and their .iface files) for every trial in every leaf.

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
include $(THIS_DIR)../silica_compiler.mk

SUB_TRIALS := \
	compiler_substrate \
	wbt_core \
	skew_ral_core \
	brodal_okasaki_core \
	ordered_collections \
	self_host_maps \
	live_graphs \
	terminal_structures \
	snapshot_graphs \
	cross_representation

STDLIB_SRC := $(abspath $(THIS_DIR)../../compiler/silica-compiler/stdlib/data_structures)
SDS_STDLIB_CACHE := $(THIS_DIR).stdlib_cache
SDS_STDLIB_MODULES := wbt_set wbt_map OrderedMap OrderedSet

ASSEMBLER := clang
MACOS_MIN_VERSION ?= 26.0
ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)

# Command-line vars are exported into leaf recipes and compile_per_trial.sh.
SDS_LEAF_MAKEFLAGS = \
	SILICA_COMPILER="$(SILICA_COMPILER)" \
	SDS_STDLIB_CACHE="$(SDS_STDLIB_CACHE)" \
	SDS_STDLIB_MODULES="$(SDS_STDLIB_MODULES)"

.PHONY: all integrate record-golden clean help stdlib-objs gate-stdlib $(SUB_TRIALS)

.DEFAULT_GOAL := all

# Compile each shared stdlib module once for this all/integrate/record-golden run.
stdlib-objs:
	@echo "=== standard_data_structures: compile stdlib data structures once ==="
	@$(ENSURE_SILICA_COMPILER)
	@rm -rf "$(SDS_STDLIB_CACHE)"
	@mkdir -p "$(SDS_STDLIB_CACHE)/lib"
	@for m in $(SDS_STDLIB_MODULES); do \
		if [ ! -f "$(STDLIB_SRC)/$$m.silica" ]; then \
			echo "FAIL: missing $(STDLIB_SRC)/$$m.silica"; \
			exit 1; \
		fi; \
		ln -sf "$(STDLIB_SRC)/$$m.silica" "$(SDS_STDLIB_CACHE)/lib/$$m.silica"; \
	done
	@{ \
		for m in $(SDS_STDLIB_MODULES); do \
			printf 'lib/%s.silica\n' "$$m"; \
		done; \
	} > "$(SDS_STDLIB_CACHE)/silica.config"
	@cp "$(SDS_STDLIB_CACHE)/silica.config" "$(SDS_STDLIB_CACHE)/silica.compile.order"
	@cd "$(SDS_STDLIB_CACHE)" && $(RUN_SILICA_COMPILER)
	@cd "$(SDS_STDLIB_CACHE)" && for sams in lib/*.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$$(basename "$$sams" .sams); \
		echo "Assembling $$sams -> lib/$$base.o"; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "lib/$$base.o" || exit 1; \
	done
	@missing=0; \
	for m in $(SDS_STDLIB_MODULES); do \
		if [ ! -f "$(SDS_STDLIB_CACHE)/lib/$$m.o" ]; then \
			echo "FAIL: stdlib compile did not produce $(SDS_STDLIB_CACHE)/lib/$$m.o"; \
			missing=1; \
		fi; \
	done; \
	[ "$$missing" -eq 0 ]
	@echo "✅ standard_data_structures: reusing .stdlib_cache/lib/*.o for every trial"

# Fast assembly-drift report for bringing up a new compiler.
#
# .ascomp is canonical: `integrate` fails on any mismatch, and so does this target.
# It answers the same question far faster and with better triage -- it normalises the
# o<N> label counters and separates code drift from comment drift -- so run it FIRST
# against a new compiler and let the trial fan-out confirm rather than debug.
#
# The four shared stdlib goldens live in stdlib_goldens/ and are symlinked into each
# leaf's lib/, so there is exactly one copy to regenerate.
#
# The four shared stdlib modules are 13,776 lines producing ~779KB of recorded
# assembly, and wbt_map/wbt_set goldens are byte-identical in both leaves that
# carry them -- emission does not depend on the surrounding silica.config. So
# these four units exercise far more of the compiler than any single trial while
# costing a tiny fraction of the 107-trial fan-out.
#
# Run this FIRST against a new compiler; run `integrate` once it passes, so the
# trial fan-out confirms rather than debugs.
#
#   make gate-stdlib
#   make gate-stdlib SILICA_COMPILER=/path/to/self-hosted-compiler
gate-stdlib: stdlib-objs
	@echo "=== standard_data_structures: stdlib golden gate ==="
	@ok=0; ko=0; \
	for m in $(SDS_STDLIB_MODULES); do \
		sams="$(SDS_STDLIB_CACHE)/lib/$$m.sams"; \
		case "$$m" in \
			OrderedMap|OrderedSet) golden="$(THIS_DIR)ordered_collections/lib/$$m.ascomp";; \
			*) golden="$(THIS_DIR)wbt_core/lib/$$m.ascomp";; \
		esac; \
		if [ ! -f "$$sams" ]; then \
			echo "❌❌ gate-stdlib: $$m produced no assembly"; ko=$$((ko + 1)); continue; \
		fi; \
		if [ ! -f "$$golden" ]; then \
			echo "❌❌ gate-stdlib: $$m has no golden at $$golden"; ko=$$((ko + 1)); continue; \
		fi; \
		if diff -Bw -q "$$sams" "$$golden" > /dev/null 2>&1; then \
			echo "✅✅ gate-stdlib: $$m matches its golden exactly"; ok=$$((ok + 1)); continue; \
		fi; \
		raw=$$(diff -Bw "$$sams" "$$golden" | grep -c '^[<>]'); \
		sed -E 's/o[0-9]+/oN/g' "$$sams"   > "$$sams.norm"; \
		sed -E 's/o[0-9]+/oN/g' "$$golden" > "$$sams.gnorm"; \
		norm=$$(diff -Bw "$$sams.norm" "$$sams.gnorm" | grep -c '^[<>]'); \
		subst=$$(diff -Bw "$$sams.norm" "$$sams.gnorm" | grep '^[<>]' | grep -vc '^[<>][[:space:]]*;'); \
		echo "❌❌ gate-stdlib: $$m differs -- $$raw diff lines; $$norm after normalising o<N> counters; $$subst of those are code, not comments"; \
		if [ "$$norm" -eq 0 ]; then \
			echo "        (drift is ENTIRELY SIR/label counter numbering -- generated code is identical)"; \
		else \
			echo "        first real differences (counters normalised):"; \
			diff -Bw "$$sams.norm" "$$sams.gnorm" | head -30 | sed 's/^/        /' || true; \
		fi; \
		rm -f "$$sams.norm" "$$sams.gnorm"; \
		ko=$$((ko + 1)); \
	done; \
	echo "gate-stdlib: $$ok matched, $$ko differed"; \
	[ "$$ko" -eq 0 ]

all: stdlib-objs
	@failed=0; \
	for sub in $(SUB_TRIALS); do \
		echo "=== standard_data_structures_phase1/$$sub (all) ==="; \
		$(MAKE) -C "$(THIS_DIR)$$sub" all $(SDS_LEAF_MAKEFLAGS) || failed=1; \
	done; \
	exit $$failed

integrate: stdlib-objs
	@failed=0; ok=0; ko=0; rm -f "$(THIS_DIR).integrate_counts"; \
	for sub in $(SUB_TRIALS); do \
		echo "=== standard_data_structures_phase1/$$sub ==="; \
		if $(MAKE) -C "$(THIS_DIR)$$sub" integrate $(SDS_LEAF_MAKEFLAGS); then \
			st=0; \
		else \
			st=1; failed=1; \
		fi; \
		if [ -f "$(THIS_DIR)$$sub/.integrate_counts" ]; then \
			read s1 s2 < "$(THIS_DIR)$$sub/.integrate_counts" || true; \
			ok=$$((ok + $${s1:-0})); ko=$$((ko + $${s2:-0})); \
		elif [ "$$st" -ne 0 ]; then \
			echo "❌❌ standard_data_structures_phase1/$$sub integrate failed (no .integrate_counts)"; \
			ko=$$((ko + 1)); \
		fi; \
	done; \
	printf '%d %d\n' "$$ok" "$$ko" > "$(THIS_DIR).integrate_counts"; \
	exit $$failed

record-golden: stdlib-objs
	@failed=0; \
	for sub in $(SUB_TRIALS); do \
		echo "=== record-golden: $$sub ==="; \
		$(MAKE) -C "$(THIS_DIR)$$sub" record-golden $(SDS_LEAF_MAKEFLAGS) || failed=1; \
	done; \
	exit $$failed

clean:
	@for sub in $(SUB_TRIALS); do \
		$(MAKE) -C "$(THIS_DIR)$$sub" clean SILICA_COMPILER="$(SILICA_COMPILER)" || true; \
	done
	@rm -rf "$(SDS_STDLIB_CACHE)"
	@rm -f "$(THIS_DIR).integrate_counts"
	@echo "✅ standard_data_structures_phase1/ clean complete"

help:
	@echo "Phase 1 standard data structures trial root"
	@echo "  all             - compile stdlib once, then build all leaves (default)"
	@echo "  integrate       - compile stdlib once, then run all dependency leaves"
	@echo "  record-golden   - compile stdlib once, then capture .ascomp/.scout/.golden_fail"
	@echo "  stdlib-objs     - compile shared stdlib .o files into .stdlib_cache/"
	@echo "  gate-stdlib     - FAST gate: 4 stdlib modules must match their .ascomp goldens"
	@echo "  clean           - clean all leaves and the stdlib object cache"
	@echo "  Subdirs: $(SUB_TRIALS)"
