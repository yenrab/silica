# Three isolated cycle cases. Each writes its own silica.config and diffs .golden_fail.
# Run from this directory or via parent `make integrate` / `make all`.

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
include $(THIS_DIR)../../silica_compiler.mk
COMPILER ?= $(SILICA_COMPILER)

TRIAL_DIR := $(notdir $(patsubst %/,%,$(THIS_DIR)))
MSG_PREFIX := $(TRIAL_DIR)/

CASES := short few many


.DEFAULT_GOAL := all
.PHONY: all integrate check clean help silica.config

all: integrate

integrate: clean silica.config check

silica.config:
	@printf '%s\n' 'cycle_a.silica' 'cycle_b.silica' > "$(THIS_DIR)short/silica.config"
	@printf '%s\n' 'few_a.silica' 'few_b.silica' 'few_c.silica' 'few_leaf.silica' > "$(THIS_DIR)few/silica.config"
	@printf '%s\n' 'many_a.silica' 'many_b.silica' 'many_c.silica' 'many_d.silica' 'many_e.silica' 'many_f.silica' 'many_l1.silica' 'many_l2.silica' 'many_l3.silica' > "$(THIS_DIR)many/silica.config"

check:
	@ok=0; ko=0; \
	for case in $(CASES); do \
		(cd "$(THIS_DIR)$$case" && { $(call RUN_SILICA_COMPILER_QUIET_WITH,$(COMPILER)); }) > "$(THIS_DIR)$$case/$$case.cur_fail" 2>&1 || true; \
		if [ ! -f "$(THIS_DIR)$$case/$$case.golden_fail" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$case: no .golden_fail"; \
			ko=$$((ko + 1)); \
		elif diff -q "$(THIS_DIR)$$case/$$case.cur_fail" "$(THIS_DIR)$$case/$$case.golden_fail" > /dev/null 2>&1; then \
			echo "✅✅ $(MSG_PREFIX)$$case"; \
			ok=$$((ok + 1)); \
		else \
			echo "❌❌ $(MSG_PREFIX)$$case: .cur_fail differs from .golden_fail"; \
			diff "$(THIS_DIR)$$case/$$case.cur_fail" "$(THIS_DIR)$$case/$$case.golden_fail" || true; \
			ko=$$((ko + 1)); \
		fi; \
	done; \
	printf '%d %d\n' "$$ok" "$$ko" > "$(THIS_DIR).integrate_counts"; \
	[ "$$ko" -eq 0 ]

clean:
	@rm -f "$(THIS_DIR).integrate_counts"
	@for case in $(CASES); do \
		rm -f "$(THIS_DIR)$$case/$$case.cur_fail" "$(THIS_DIR)$$case/silica.config"; \
	done

help:
	@echo "module_cycle_trial (short / few / many use-cycle error enforcement)"
	@echo "=================================================================="
	@echo "Targets:"
	@echo "  all       - Same as integrate (default)"
	@echo "  integrate - Write each case silica.config, run compiler, diff vs .golden_fail"
	@echo "  check     - Run compiler + diff (assumes silica.config files are current)"
	@echo "  clean     - Remove .cur_fail, silica.config, and .integrate_counts"
	@echo "  help      - Show this help"
