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

.PHONY: all integrate record-golden clean help stdlib-objs $(SUB_TRIALS)

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
	@echo "  clean           - clean all leaves and the stdlib object cache"
	@echo "  Subdirs: $(SUB_TRIALS)"
