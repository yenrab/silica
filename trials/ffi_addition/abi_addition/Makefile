# Compile-only success trials for Silica-facing C ABI types (spec §5.4–§6.8, §8–§9).
# Each .silica file is compiled in isolation with dangerous_exposure_source fixtures linked.

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
FFI_TRIAL_DIR := $(abspath $(THIS_DIR)/..)
include $(THIS_DIR)../../silica_compiler.mk
FIXTURES_DIR := $(FFI_TRIAL_DIR)/fixtures

include $(FFI_TRIAL_DIR)/fixtures.mk

ABI_TRIAL_FILES := \
	dangerous_fixed_width_integers_abi_valid.silica \
	dangerous_fixed_width_uintegers_abi_valid.silica \
	dangerous_floats_abi_valid.silica \
	dangerous_boolean_abi_valid.silica \
	dangerous_record_by_value_abi_valid.silica \
	dangerous_tagged_result_abi_valid.silica \
	dangerous_string_ptr_len_in_abi_valid.silica \
	dangerous_string_two_layer_in_abi_valid.silica \
	dangerous_string_two_layer_out_abi_valid.silica

.PHONY: all integrate clean fixtures help

.DEFAULT_GOAL := integrate

integrate: fixtures $(SILICA_COMPILER)
	@failed=0; ok=0; ko=0; \
	for silica in $(ABI_TRIAL_FILES); do \
		base=$$(basename "$$silica" .silica); \
		tmpdir=$$(mktemp -d); \
		ln -s "$(THIS_DIR)$$silica" "$$tmpdir/$$base.silica"; \
		ln -s "$(FIXTURES_DIR)/dangerous_exposure_source" "$$tmpdir/dangerous_exposure_source"; \
		echo "$$base.silica" > "$$tmpdir/silica.config"; \
		out="$$tmpdir/out.txt"; \
		(cd "$$tmpdir" && { $(call RUN_SILICA_COMPILER_QUIET_WITH,"$(SILICA_COMPILER)"); }) > "$$out" 2>&1 || true; \
		if grep -qE '❌|Error error|ParseError|TypeError|EffectError|ModuleError' "$$out"; then \
			echo "  ❌ abi_addition/$$silica: expected compile success"; cat "$$out"; rm -rf "$$tmpdir"; ko=$$((ko + 1)); failed=1; \
		elif ! grep -q "Type checking..." "$$out"; then \
			echo "  ❌ abi_addition/$$silica: did not reach type checking"; cat "$$out"; rm -rf "$$tmpdir"; ko=$$((ko + 1)); failed=1; \
		else \
			echo "  ✅✅ abi_addition/$$silica"; ok=$$((ok + 1)); \
		fi; \
		rm -rf "$$tmpdir"; \
	done; \
	printf '%d %d\n' "$$ok" "$$ko" > "$(THIS_DIR).integrate_counts"; \
	test "$$failed" -eq 0

all: integrate

fixtures: ffi-wrapper-archives

clean:
	@rm -f "$(THIS_DIR).integrate_counts"
	@echo "✅ abi_addition clean complete"

help:
	@echo "abi_addition — compile-success trials for documented FFI C data shapes"
	@echo "  integrate: compile each trial through type checking with fixture sidecars"
