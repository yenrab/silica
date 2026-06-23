THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(THIS_DIR)../../../src/silica-compiler
MSG_PREFIX := error_enforcement_addition/standard_data_structures_phase04/
LIB_SRC := ../../../stdlib/data_structures

STDLIB_PREREQ_MK := $(firstword $(wildcard $(THIS_DIR)../stdlib_prereq.mk $(THIS_DIR)../../stdlib_prereq.mk $(THIS_DIR)../../../stdlib_prereq.mk))
include $(STDLIB_PREREQ_MK)

TRIALS := \
	constructor_record_wrong_return \
	constructor_record_missing_field \
	constructor_record_missing_compare_item \
	constructor_record_wrong_arity \
	constructor_comparator_invalid_atom_literal \
	constructor_record_wide_atom_return \
	constructor_witness_item_mismatch \
	trait_impl_return_mismatch \
	trait_call_item_type_mismatch

.PHONY: all integrate check clean silica.config lib-sources help check-one stdlib-data-structures $(TRIALS)

all: integrate

lib-sources: stdlib-data-structures
	@mkdir -p "$(THIS_DIR)lib"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_set_phase04.silica)" "$(THIS_DIR)lib/btree_set_phase04.silica"

silica.config: clean lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/btree_set_phase04.silica'; \
		for trial in $(TRIALS); do echo "$$trial.silica"; done; \
	} > silica.config

integrate: lib-sources check

check:
	@failed=0; ok=0; ko=0; \
	for trial in $(TRIALS); do \
		if $(MAKE) -f "$(THIS_DIR)Makefile" check-one TRIAL=$$trial SILICA_COMPILER="$(SILICA_COMPILER)"; then \
			ok=$$((ok + 1)); \
		else \
			ko=$$((ko + 1)); failed=1; \
		fi; \
	done; \
	printf '%d %d\n' "$$ok" "$$ko" > "$(THIS_DIR).integrate_counts"; \
	exit $$failed

check-one:
	@cd "$(THIS_DIR)" && { \
		echo 'lib/btree_set_phase04.silica'; \
		echo '$(TRIAL).silica'; \
	} > silica.config && \
	if $(SILICA_COMPILER) > $(TRIAL).cur_fail 2>&1; then \
		echo "❌❌ $(MSG_PREFIX)$(TRIAL) unexpectedly compiled"; \
		exit 1; \
	fi; \
	if diff -Bw -q $(TRIAL).cur_fail $(TRIAL).golden_fail > /dev/null 2>&1; then \
		echo "✅✅ $(MSG_PREFIX)$(TRIAL)"; \
	else \
		echo "❌❌ $(MSG_PREFIX)$(TRIAL) .cur_fail differs from .golden_fail"; \
		diff -Bw $(TRIAL).cur_fail $(TRIAL).golden_fail || true; \
		exit 1; \
	fi

clean:
	@rm -f "$(THIS_DIR)"*.cur_fail "$(THIS_DIR)"silica.config "$(THIS_DIR)".integrate_counts "$(THIS_DIR)"*.sams "$(THIS_DIR)"lib/*.sams "$(THIS_DIR)"lib/*.silica

help:
	@echo "standard_data_structures_phase04 negative constructor-record trials"
	@echo "Trials: $(TRIALS)"
