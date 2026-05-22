# FFI trial harness — Phase 0 builds prebuilt C wrapper static libraries and sidecar fixtures.
#
# Success-path app trials live under app_*/ (each a runnable program with .scout goldens).
# Run: make -C compiler/silica-compiler/trials/ffi_addition phase-0

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

include $(THIS_DIR)fixtures.mk

FIXTURES_DIR := $(FFI_FIXTURES_DIR)
SRC_DIR := $(FFI_SRC_DIR)
LIB_DIR := $(FFI_LIB_DIR)
BUILD_DIR := $(FFI_BUILD_DIR)
CC := $(FFI_CC)
ARCH := $(FFI_ARCH)
CFLAGS := $(FFI_CFLAGS)
LEGACY_OBJ := $(FFI_LEGACY_OBJ)
TEXT_OBJ := $(FFI_TEXT_OBJ)
NET_OBJ := $(FFI_NET_OBJ)
LEGACY_ARCHIVE := $(FFI_LEGACY_ARCHIVE)
TEXT_ARCHIVE := $(FFI_TEXT_ARCHIVE)
NET_ARCHIVE := $(FFI_NET_ARCHIVE)

NM := $(shell command -v llvm-nm 2>/dev/null || command -v nm)

APP_SIDECAR_DIR := app_sidecar_legacy_math_add
APP_CAST_WORKER_DIR := app_cast_worker_legacy_add
APP_FFI_RESULT_CAST_DIR := app_ffi_result_cast_add
APP_FOREIGN_ABI_DIR := app_foreign_abi_valid
APP_E2E_DIR := app_e2e_scalar_string_echo
APP_GUARDED_ONCE_DIR := app_legacy_math_add_guarded
APP_GUARDED_TWICE_DIR := app_legacy_math_add_twice

APP_DIRS := \
	$(APP_SIDECAR_DIR) \
	$(APP_CAST_WORKER_DIR) \
	$(APP_FFI_RESULT_CAST_DIR) \
	$(APP_FOREIGN_ABI_DIR) \
	$(APP_E2E_DIR) \
	$(APP_GUARDED_ONCE_DIR) \
	$(APP_GUARDED_TWICE_DIR)

GUARDED_APP_DIRS := \
	$(APP_GUARDED_ONCE_DIR) \
	$(APP_GUARDED_TWICE_DIR)

SILICA_COMPILER := $(abspath $(THIS_DIR)../../src/silica-compiler)
MODULE_VALID_FILE := module_addition/dangerous_naming_valid.silica

.PHONY: all integrate fixtures phase-0 phase-a phase-3 phase-4 phase-5 phase-6 phase-7 phase-8 phase-9 phase-11 \
	ffi-wrapper-archives module-valid verify-symbols clean clean-fixtures clean-apps help

.DEFAULT_GOAL := phase-0

# Success-path regression gate: fixtures + all app integrates (manifest-driven link via .scout goldens).
all: phase-9
	@echo "✅ ffi_addition all complete"

integrate: fixtures
	@rm -f .integrate_counts; ok=0; ko=0; failed=0; \
	for dir in $(APP_DIRS); do \
		echo "=== ffi_addition/$$dir ==="; \
		if $(MAKE) -C "$(THIS_DIR)$$dir" integrate; then st=0; else st=1; failed=1; fi; \
		cf="$(THIS_DIR)$$dir/.integrate_counts"; \
		if [ -f "$$cf" ]; then \
			read n1 n2 < "$$cf" || true; \
			ok=$$((ok + $${n1:-0})); ko=$$((ko + $${n2:-0})); \
		elif [ "$$st" -ne 0 ]; then ko=$$((ko + 1)); fi; \
	done; \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	test "$$failed" -eq 0 && test "$$ko" -eq 0

fixtures: ffi-wrapper-archives
	@echo "✅ ffi_addition/fixtures: static libraries built"

verify-symbols: fixtures
	@test -n "$(NM)" || { echo "FAIL: neither llvm-nm nor nm found"; exit 1; }
	@failed=0; \
	for archive in "$(LEGACY_ARCHIVE)" "$(TEXT_ARCHIVE)" "$(NET_ARCHIVE)"; do \
		echo "Checking symbols in $$archive..."; \
		syms=$$($(NM) -g "$$archive" 2>/dev/null || $(NM) "$$archive"); \
		case "$$archive" in \
			*libsilica_legacy_math.a) \
				if echo "$$syms" | grep -Eq '(_)?silica_legacy_math_add_int64($$|[[:space:]])'; then \
					echo "  ✅ silica_legacy_math_add_int64"; \
				else \
					echo "  ❌ missing silica_legacy_math_add_int64"; failed=1; \
				fi ;; \
			*libsilica_text.a) \
				if echo "$$syms" | grep -Eq '(_)?silica_text_echo($$|[[:space:]])'; then \
					echo "  ✅ silica_text_echo"; \
				else \
					echo "  ❌ missing silica_text_echo"; failed=1; \
				fi ;; \
			*libsilica_net.a) \
				if echo "$$syms" | grep -Eq '(_?)silica_net_parse_port($$|[[:space:]])'; then \
					echo "  ✅ silica_net_parse_port"; \
				else \
					echo "  ❌ missing silica_net_parse_port"; failed=1; \
				fi ;; \
		esac; \
	done; \
	test "$$failed" -eq 0

phase-0: verify-symbols
	@test -f "$(FIXTURES_DIR)/dangerous_exposure_source/legacy/silica_legacy_math_wrapper.meta"
	@test -f "$(FIXTURES_DIR)/dangerous_exposure_source/text/silica_text_wrapper.meta"
	@echo "✅ ffi_addition phase-0 complete"

module-valid: $(SILICA_COMPILER)
	@test -x "$(SILICA_COMPILER)" || { echo "FAIL: build compiler first (make -C compiler/silica-compiler/src build)"; exit 1; }
	@silica="$(MODULE_VALID_FILE)"; \
	base=$$(basename "$$silica" .silica); \
	tmpdir=$$(mktemp -d); \
	ln -s "$(THIS_DIR)$$silica" "$$tmpdir/$$base.silica"; \
	ln -s "$(FIXTURES_DIR)/dangerous_exposure_source" "$$tmpdir/dangerous_exposure_source"; \
	echo "$$base.silica" > "$$tmpdir/silica.config"; \
	out="$$tmpdir/out.txt"; \
	(cd "$$tmpdir" && "$(SILICA_COMPILER)") > "$$out" 2>&1 || true; \
	if grep -qE '❌|Error error|ParseError|TypeError|EffectError|ModuleError' "$$out"; then \
		echo "  ❌ $$silica: expected compile success"; cat "$$out"; rm -rf "$$tmpdir"; exit 1; \
	elif ! grep -q "Type checking..." "$$out"; then \
		echo "  ❌ $$silica: did not reach type checking"; cat "$$out"; rm -rf "$$tmpdir"; exit 1; \
	else \
		echo "  ✅ $$silica: module rules OK through type check"; \
	fi; \
	rm -rf "$$tmpdir"

phase-3: module-valid
	@echo "✅ ffi_addition phase-3 complete"

phase-4: phase-3
	@$(MAKE) -C "$(THIS_DIR)$(APP_SIDECAR_DIR)" integrate
	@echo "✅ ffi_addition phase-4 complete ($(APP_SIDECAR_DIR))"

phase-5: phase-4
	@$(MAKE) -C "$(THIS_DIR)$(APP_CAST_WORKER_DIR)" integrate
	@echo "✅ ffi_addition phase-5 complete ($(APP_CAST_WORKER_DIR))"

phase-6: phase-5
	@$(MAKE) -C "$(THIS_DIR)$(APP_FFI_RESULT_CAST_DIR)" integrate
	@echo "✅ ffi_addition phase-6 complete ($(APP_FFI_RESULT_CAST_DIR))"

phase-7: phase-6
	@$(MAKE) -C "$(THIS_DIR)$(APP_FOREIGN_ABI_DIR)" integrate
	@echo "✅ ffi_addition phase-7 complete ($(APP_FOREIGN_ABI_DIR))"

phase-8: phase-7 fixtures
	@test -x "$(SILICA_COMPILER)" || { echo "FAIL: build compiler first (make -C compiler/silica-compiler/src build)"; exit 1; }
	@$(MAKE) -C "$(THIS_DIR)$(APP_E2E_DIR)" integrate
	@echo "✅ ffi_addition phase-8 complete ($(APP_E2E_DIR))"

phase-9: fixtures
	@test -x "$(SILICA_COMPILER)" || { echo "FAIL: build compiler first (make -C compiler/silica-compiler/src build)"; exit 1; }
	@$(MAKE) integrate
	@echo "✅ ffi_addition phase-9 complete (manifest-driven link via app integrates)"

phase-11: fixtures
	@test -x "$(SILICA_COMPILER)" || { echo "FAIL: build compiler first (make -C compiler/silica-compiler/src build)"; exit 1; }
	@failed=0; \
	for dir in $(GUARDED_APP_DIRS); do \
		echo "=== ffi_addition/$$dir (guarded boundary) ==="; \
		$(MAKE) -C "$(THIS_DIR)$$dir" integrate || failed=1; \
	done; \
	test "$$failed" -eq 0
	@echo "✅ ffi_addition phase-11 complete (guarded FFI runtime boundary)"

phase-a: phase-0

clean-fixtures:
	@rm -rf "$(BUILD_DIR)"
	@rm -f "$(LEGACY_ARCHIVE)" "$(TEXT_ARCHIVE)" "$(NET_ARCHIVE)"

clean-apps:
	@for dir in $(APP_DIRS); do $(MAKE) -C "$(THIS_DIR)$$dir" clean; done

clean: clean-fixtures clean-apps
	@rm -f .integrate_counts
	@echo "✅ ffi_addition clean complete"

help:
	@echo "ffi_addition Makefile"
	@echo "====================="
	@echo ""
	@echo "App trials (runnable programs with .scout goldens):"
	@echo "  $(APP_SIDECAR_DIR)      - sidecar metadata + legacy math add via FFI worker"
	@echo "  $(APP_CAST_WORKER_DIR)  - cast-only client and external_danger FFI worker"
	@echo "  $(APP_FFI_RESULT_CAST_DIR) - FFI result cast delivers external-danger-touched int64"
	@echo "  $(APP_FOREIGN_ABI_DIR)  - Silica-side ABI declarations compile and run"
	@echo "  $(APP_E2E_DIR) - scalar add and string echo end-to-end foreign calls"
	@echo ""
	@echo "Targets:"
	@echo "  all         - Success-path suite: phase-9 (all app integrates)"
	@echo "  integrate   - Run all app trial integrates and sum .integrate_counts"
	@echo "  phase-0     - Build fixture archives and verify exported symbols"
	@echo "  phase-3     - Module naming / export rules (module_addition/)"
	@echo "  phase-4..8  - One app trial integrate per phase (see app dirs above)"
	@echo "  phase-9     - All app integrates with silica.link-driven link (.scout goldens)"
	@echo "  phase-11    - Guarded FFI boundary app trials (legacy math add via enter/exit wrapper)"
	@echo "  (compile/link failure goldens: error_enforcement_addition/ffi_addition integrate)"
	@echo "  fixtures    - Build libsilica_legacy_math.a, libsilica_text.a, and libsilica_net.a"
	@echo "  clean       - Remove fixture and app trial build artifacts"
