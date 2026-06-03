THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(THIS_DIR)../../../src/silica-compiler
MSG_PREFIX := error_enforcement_addition/standard_data_structures_phase04/
LIB_SRC := ../../../src/standard_data_structures

.PHONY: all integrate check clean silica.config lib-sources help

all: integrate

lib-sources:
	@mkdir -p "$(THIS_DIR)lib"
	@ln -sf "$(abspath $(THIS_DIR)$(LIB_SRC)/btree_set_phase04.silica)" "$(THIS_DIR)lib/btree_set_phase04.silica"

silica.config: clean lib-sources
	@cd "$(THIS_DIR)" && { \
		echo 'lib/btree_set_phase04.silica'; \
		echo 'constructor_record_wrong_return.silica'; \
	} > silica.config

integrate: silica.config check

check:
	@cd "$(THIS_DIR)" && \
	if $(SILICA_COMPILER) > constructor_record_wrong_return.cur_fail 2>&1; then \
		echo "❌❌ $(MSG_PREFIX)constructor_record_wrong_return unexpectedly compiled"; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi; \
	if diff -Bw -q constructor_record_wrong_return.cur_fail constructor_record_wrong_return.golden_fail > /dev/null 2>&1; then \
		echo "✅✅ $(MSG_PREFIX)constructor_record_wrong_return"; \
		printf '%d %d\n' 1 0 > .integrate_counts; \
	else \
		echo "❌❌ $(MSG_PREFIX)constructor_record_wrong_return .cur_fail differs from .golden_fail"; \
		diff -Bw constructor_record_wrong_return.cur_fail constructor_record_wrong_return.golden_fail || true; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi

clean:
	@rm -f "$(THIS_DIR)"*.cur_fail "$(THIS_DIR)"silica.config "$(THIS_DIR)".integrate_counts "$(THIS_DIR)"*.sams "$(THIS_DIR)"lib/*.sams "$(THIS_DIR)"lib/*.silica

help:
	@echo "standard_data_structures_phase04 negative constructor-record trials"
