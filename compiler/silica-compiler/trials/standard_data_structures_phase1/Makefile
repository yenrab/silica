# Phase 1 standard data structures — dependency-ordered trial root (Layer 0 §6.2)
#
# First-time golden capture (after building the compiler):
#   make -C ../../src
#   make record-golden
#
# Normal integration:
#   make integrate

THIS_DIR        := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(abspath $(THIS_DIR)../../src/silica-compiler)

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
	error_enforcement \
	cross_representation

.PHONY: all integrate record-golden clean help $(SUB_TRIALS)

.DEFAULT_GOAL := integrate

integrate:
	@failed=0; ok=0; ko=0; rm -f "$(THIS_DIR).integrate_counts"; \
	for sub in $(SUB_TRIALS); do \
		echo "=== standard_data_structures_phase1/$$sub ==="; \
		if $(MAKE) -C "$(THIS_DIR)$$sub" integrate SILICA_COMPILER="$(SILICA_COMPILER)"; then \
			:; \
		else \
			failed=1; \
		fi; \
		if [ -f "$(THIS_DIR)$$sub/.integrate_counts" ]; then \
			read s1 s2 < "$(THIS_DIR)$$sub/.integrate_counts" || true; \
			ok=$$((ok + $${s1:-0})); ko=$$((ko + $${s2:-0})); \
		fi; \
	done; \
	printf '%d %d\n' "$$ok" "$$ko" > "$(THIS_DIR).integrate_counts"; \
	exit $$failed

record-golden:
	@failed=0; \
	for sub in $(SUB_TRIALS); do \
		echo "=== record-golden: $$sub ==="; \
		$(MAKE) -C "$(THIS_DIR)$$sub" record-golden SILICA_COMPILER="$(SILICA_COMPILER)" || failed=1; \
	done; \
	exit $$failed

clean:
	@for sub in $(SUB_TRIALS); do \
		$(MAKE) -C "$(THIS_DIR)$$sub" clean SILICA_COMPILER="$(SILICA_COMPILER)" || true; \
	done
	@rm -f "$(THIS_DIR).integrate_counts"
	@echo "✅ standard_data_structures_phase1/ clean complete"

help:
	@echo "Phase 1 standard data structures trial root"
	@echo "  integrate       - run all dependency leaves (default)"
	@echo "  record-golden   - capture .ascomp/.scout/.golden_fail from current compiler"
	@echo "  clean           - clean all leaves"
	@echo "  Subdirs: $(SUB_TRIALS)"
