# Multi-file trial: library exports one function; main uses it then calls a non-exported sibling (ParseError E1011, spec §19.2).
# Run from this directory or via parent `make integrate` / `make all` (parent runs `make integrate` here).

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
# Same binary as parent SILICA_COMPILER; parent passes COMPILER= when invoking.
SILICA_COMPILER ?= $(THIS_DIR)../../../src/silica-compiler
COMPILER ?= $(SILICA_COMPILER)

TRIAL_DIR := $(notdir $(patsubst %/,%,$(THIS_DIR)))
MSG_PREFIX := $(TRIAL_DIR)/


.DEFAULT_GOAL := all
.PHONY: all integrate check clean help silica.config

# Parent: integrate -> silica.config all; sub: all == integrate == silica.config + golden check.
all: integrate

integrate: clean silica.config check

check:
	@cd "$(THIS_DIR)" && $(COMPILER) > main_use_partial.cur_fail 2>&1 || true
	@if diff -q "$(THIS_DIR)main_use_partial.cur_fail" "$(THIS_DIR)main_use_partial.golden_fail" > /dev/null 2>&1; then \
		echo "✅✅ $(MSG_PREFIX)module_nonexported_callee_trial"; \
		printf '%d %d\n' 1 0 > "$(THIS_DIR).integrate_counts"; \
	else \
		echo "❌❌ $(MSG_PREFIX)module_nonexported_callee_trial: .cur_fail differs from .golden_fail"; \
		diff "$(THIS_DIR)main_use_partial.cur_fail" "$(THIS_DIR)main_use_partial.golden_fail" || true; \
		printf '%d %d\n' 0 1 > "$(THIS_DIR).integrate_counts"; \
		exit 1; \
	fi

clean:
	@rm -f "$(THIS_DIR)main_use_partial.cur_fail" "$(THIS_DIR)silica.config" "$(THIS_DIR).integrate_counts"

silica.config: | clean
	@cd "$(dir $(abspath $(lastword $(MAKEFILE_LIST))))" && find "$$PWD" -type f -name '*.silica' | sed "s|^$$PWD/||" > silica.config

help:
	@echo "module_nonexported_callee_trial (multi-file error enforcement)"
	@echo "=============================================================="
	@echo "Targets:"
	@echo "  all       - Same as integrate (default)"
	@echo "  integrate - Regenerate silica.config, run compiler, diff vs .golden_fail"
	@echo "  check     - Run compiler + diff (assumes silica.config is current)"
	@echo "  clean     - Remove .cur_fail, silica.config, .integrate_counts"
	@echo "  help      - Show this help"
