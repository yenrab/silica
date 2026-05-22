# FFI expected-failure trials live under error_enforcement_addition.
# This harness preserves the FFI-specific temp project setup these cases need.

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(THIS_DIR)../../../src/silica-compiler
FIXTURES_DIR := $(THIS_DIR)../../ffi_addition/fixtures
FIXTURE_ROOT := $(FIXTURES_DIR)/dangerous_exposure_source
FFI_SUCCESS_DIR := $(THIS_DIR)../../ffi_addition

TRIAL_DIR := error_enforcement_addition/ffi_addition
MSG_PREFIX := $(TRIAL_DIR)/

FAIL_CASES := \
	module_addition/dangerous_export_raw \
	module_addition/dangerous_missing_wrapper_meta \
	module_addition/dangerous_wrapper_meta_outside_tree \
	module_addition/dangerous_dependency_naming_fail_trial/app_uses_dangerous \
	metadata_addition/dangerous_sidecar_missing_file \
	metadata_addition/dangerous_sidecar_missing_entry \
	metadata_addition/dangerous_sidecar_missing_link_library \
	placement_addition/dangerous_external_danger_in_main \
	placement_addition/dangerous_direct_call_fail \
	placement_addition/dangerous_client_not_cast_only \
	taint_addition/dangerous_taint_in_produces \
	taint_addition/dangerous_taint_ordinary_cast \
	taint_addition/dangerous_taint_device_io \
	taint_addition/dangerous_taint_region_escape \
	abi_addition/dangerous_raw_string_param_fail \
	abi_addition/dangerous_too_many_args_fail

.DEFAULT_GOAL := integrate
.PHONY: all integrate clean help

all: integrate

integrate: clean
	@failed=0; ok=0; ko=0; \
	for case in $(FAIL_CASES); do \
		base=$$(basename "$$case"); \
		golden="$(THIS_DIR)$$case.golden_fail"; \
		cur="$(THIS_DIR)$$case.cur_fail"; \
		tmpdir=$$(mktemp -d); \
		mkdir -p "$$tmpdir/dangerous_exposure_source"; \
		ln -s "$(FIXTURE_ROOT)/legacy" "$$tmpdir/dangerous_exposure_source/legacy"; \
		ln -s "$(FIXTURE_ROOT)/text" "$$tmpdir/dangerous_exposure_source/text"; \
		ln -s "$(FIXTURE_ROOT)/net" "$$tmpdir/dangerous_exposure_source/net"; \
		ln -s "$(FIXTURE_ROOT)/other" "$$tmpdir/dangerous_exposure_source/other"; \
		ln -s "$(FIXTURE_ROOT)/lib" "$$tmpdir/dangerous_exposure_source/lib"; \
		ln -s "$(THIS_DIR)metadata_addition/dangerous_exposure_source/bad_wrong_symbol.meta" "$$tmpdir/dangerous_exposure_source/bad_wrong_symbol.meta"; \
		ln -s "$(THIS_DIR)metadata_addition/dangerous_exposure_source/bad_no_link_library.meta" "$$tmpdir/dangerous_exposure_source/bad_no_link_library.meta"; \
		case "$$case" in \
			module_addition/dangerous_dependency_naming_fail_trial/app_uses_dangerous) \
				ln -s "$(THIS_DIR)module_addition/dangerous_dependency_naming_fail_trial/dangerous_legacy_stub.silica" "$$tmpdir/dangerous_legacy_stub.silica"; \
				ln -s "$(THIS_DIR)module_addition/dangerous_dependency_naming_fail_trial/app_uses_dangerous.silica" "$$tmpdir/app_uses_dangerous.silica"; \
				printf "dangerous_legacy_stub.silica\napp_uses_dangerous.silica\n" > "$$tmpdir/silica.config"; \
				;; \
			placement_addition/*) \
				ln -s "$(FFI_SUCCESS_DIR)/placement_addition/dangerous_legacy_stub.silica" "$$tmpdir/dangerous_legacy_stub.silica"; \
				ln -s "$(THIS_DIR)$$case.silica" "$$tmpdir/$$base.silica"; \
				printf "dangerous_legacy_stub.silica\n$$base.silica\n" > "$$tmpdir/silica.config"; \
				;; \
			taint_addition/*) \
				ln -s "$(FFI_SUCCESS_DIR)/taint_addition/dangerous_legacy_stub.silica" "$$tmpdir/dangerous_legacy_stub.silica"; \
				ln -s "$(THIS_DIR)$$case.silica" "$$tmpdir/$$base.silica"; \
				printf "dangerous_legacy_stub.silica\n$$base.silica\n" > "$$tmpdir/silica.config"; \
				;; \
			*) \
				ln -s "$(THIS_DIR)$$case.silica" "$$tmpdir/$$base.silica"; \
				printf "$$base.silica\n" > "$$tmpdir/silica.config"; \
				;; \
		esac; \
		(cd "$$tmpdir" && "$(SILICA_COMPILER)") > "$$cur" 2>&1 || true; \
		rm -rf "$$tmpdir"; \
		if [ ! -f "$$golden" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$case: missing .golden_fail"; \
			ko=$$((ko + 1)); failed=1; \
		elif diff -q "$$cur" "$$golden" > /dev/null 2>&1; then \
			echo "✅✅ $(MSG_PREFIX)$$case"; \
			ok=$$((ok + 1)); \
		else \
			echo "❌❌ $(MSG_PREFIX)$$case: .cur_fail differs from .golden_fail"; \
			diff "$$cur" "$$golden" || true; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
	done; \
	printf '%d %d\n' "$$ok" "$$ko" > "$(THIS_DIR).integrate_counts"; \
	exit $$failed

clean:
	@rm -f "$(THIS_DIR).integrate_counts"
	@find "$(THIS_DIR)" -name '*.cur_fail' -delete
	@echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "FFI error-enforcement trials"
	@echo "============================"
	@echo "Runs FFI expected-failure cases and compares .cur_fail to .golden_fail."
