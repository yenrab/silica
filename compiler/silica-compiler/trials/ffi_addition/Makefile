# FFI trial harness — Phase 0 builds prebuilt C wrapper static libraries and sidecar fixtures.
# No Silica compiler changes required for this phase.
#
# Run: make -C compiler/silica-compiler/trials/ffi_addition phase-0

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

FIXTURES_DIR := $(THIS_DIR)fixtures
SRC_DIR := $(FIXTURES_DIR)/src
LIB_DIR := $(FIXTURES_DIR)/dangerous_exposure_source/lib
BUILD_DIR := $(FIXTURES_DIR)/build

CC := clang
ARCH := arm64
CFLAGS := -std=c11 -Wall -Wextra -O2 -arch $(ARCH) -mmacosx-version-min=26.0 \
	-I$(FIXTURES_DIR)/dangerous_exposure_source/legacy \
	-I$(FIXTURES_DIR)/dangerous_exposure_source/text

LEGACY_OBJ := $(BUILD_DIR)/silica_legacy_math.o
TEXT_OBJ := $(BUILD_DIR)/silica_text.o

LEGACY_ARCHIVE := $(LIB_DIR)/libsilica_legacy_math.a
TEXT_ARCHIVE := $(LIB_DIR)/libsilica_text.a

NM := $(shell command -v llvm-nm 2>/dev/null || command -v nm)

LEGACY_SYMBOLS := silica_legacy_math_add_int64 _silica_legacy_math_add_int64
TEXT_SYMBOLS := silica_text_echo _silica_text_echo

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(abspath $(THIS_DIR)../../src/silica-compiler)

MODULE_VALID_FILE := module_addition/dangerous_naming_valid.silica

METADATA_VALID_FILE := metadata_addition/dangerous_sidecar_match.silica

PLACEMENT_VALID_FILES := \
	placement_addition/dangerous_legacy_stub.silica \
	placement_addition/dangerous_ffi_worker_valid.silica

TAINT_VALID_FILES := \
	taint_addition/dangerous_legacy_stub.silica \
	taint_addition/dangerous_taint_worker_valid.silica

ABI_VALID_FILES := \
	abi_addition/dangerous_scalar_abi_valid.silica \
	abi_addition/dangerous_string_two_layer_valid.silica

.PHONY: all integrate fixtures phase-0 phase-a phase-3 phase-4 phase-5 phase-6 phase-7 phase-8 module-valid metadata-valid placement-valid taint-valid abi-valid verify-symbols clean clean-fixtures help

.DEFAULT_GOAL := phase-0

all: phase-0

integrate:
	@rm -f .integrate_counts
	@if $(MAKE) phase-8; then \
		if [ -f runtime_addition/.integrate_counts ]; then \
			cat runtime_addition/.integrate_counts > .integrate_counts; \
		else \
			printf '%d %d\n' 0 0 > .integrate_counts; \
		fi; \
	else \
		if [ -f runtime_addition/.integrate_counts ]; then \
			read ok ko < runtime_addition/.integrate_counts || true; \
			[ "$${ko:-0}" -eq 0 ] && ko=1; \
			printf '%d %d\n' "$${ok:-0}" "$${ko:-1}" > .integrate_counts; \
		else \
			printf '%d %d\n' 0 1 > .integrate_counts; \
		fi; \
		exit 1; \
	fi

fixtures: $(LEGACY_ARCHIVE) $(TEXT_ARCHIVE)
	@echo "✅ ffi_addition/fixtures: static libraries built"

$(BUILD_DIR):
	@mkdir -p $(BUILD_DIR)

$(LIB_DIR):
	@mkdir -p $(LIB_DIR)

$(LEGACY_OBJ): $(SRC_DIR)/silica_legacy_math.c | $(BUILD_DIR)
	$(CC) $(CFLAGS) -c $< -o $@

$(TEXT_OBJ): $(SRC_DIR)/silica_text.c | $(BUILD_DIR)
	$(CC) $(CFLAGS) -c $< -o $@

$(LEGACY_ARCHIVE): $(LEGACY_OBJ) | $(LIB_DIR)
	ar rcs $@ $^

$(TEXT_ARCHIVE): $(TEXT_OBJ) | $(LIB_DIR)
	ar rcs $@ $^

verify-symbols: fixtures
	@test -n "$(NM)" || { echo "FAIL: neither llvm-nm nor nm found"; exit 1; }
	@failed=0; \
	for archive in "$(LEGACY_ARCHIVE)" "$(TEXT_ARCHIVE)"; do \
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

metadata-valid: $(SILICA_COMPILER)
	@test -x "$(SILICA_COMPILER)" || { echo "FAIL: build compiler first (make -C compiler/silica-compiler/src build)"; exit 1; }
	@silica="$(METADATA_VALID_FILE)"; \
	base=$$(basename "$$silica" .silica); \
	tmpdir=$$(mktemp -d); \
	ln -s "$(THIS_DIR)$$silica" "$$tmpdir/$$base.silica"; \
	ln -s "$(FIXTURES_DIR)/dangerous_exposure_source" "$$tmpdir/dangerous_exposure_source"; \
	echo "$$base.silica" > "$$tmpdir/silica.config"; \
	out="$$tmpdir/out.txt"; \
	(cd "$$tmpdir" && "$(SILICA_COMPILER)") > "$$out" 2>&1 || true; \
	if grep -qE '❌|Error error|ParseError|TypeError|EffectError|ModuleError' "$$out"; then \
		echo "  ❌ $$silica: expected compile success"; cat "$$out"; rm -rf "$$tmpdir"; exit 1; \
	elif ! grep -q "FFI metadata..." "$$out"; then \
		echo "  ❌ $$silica: did not reach FFI metadata pass"; cat "$$out"; rm -rf "$$tmpdir"; exit 1; \
	elif ! grep -q "Type checking..." "$$out"; then \
		echo "  ❌ $$silica: did not reach type checking"; cat "$$out"; rm -rf "$$tmpdir"; exit 1; \
	else \
		echo "  ✅ $$silica: sidecar metadata OK through type check"; \
	fi; \
	rm -rf "$$tmpdir"

phase-4: phase-3 metadata-valid
	@echo "✅ ffi_addition phase-4 complete"

placement-valid: $(SILICA_COMPILER)
	@test -x "$(SILICA_COMPILER)" || { echo "FAIL: build compiler first (make -C compiler/silica-compiler/src build)"; exit 1; }
	@tmpdir=$$(mktemp -d); \
	ln -s "$(THIS_DIR)placement_addition/dangerous_legacy_stub.silica" "$$tmpdir/dangerous_legacy_stub.silica"; \
	ln -s "$(THIS_DIR)placement_addition/dangerous_ffi_worker_valid.silica" "$$tmpdir/dangerous_ffi_worker_valid.silica"; \
	ln -s "$(FIXTURES_DIR)/dangerous_exposure_source" "$$tmpdir/dangerous_exposure_source"; \
	printf "dangerous_legacy_stub.silica\ndangerous_ffi_worker_valid.silica\n" > "$$tmpdir/silica.config"; \
	out="$$tmpdir/out.txt"; \
	(cd "$$tmpdir" && "$(SILICA_COMPILER)") > "$$out" 2>&1 || true; \
	if grep -qE '❌|Error error|ParseError|TypeError|EffectError|ModuleError' "$$out"; then \
		echo "  ❌ placement_addition/dangerous_ffi_worker_valid: expected compile success"; cat "$$out"; rm -rf "$$tmpdir"; exit 1; \
	elif ! grep -q "FFI placement..." "$$out"; then \
		echo "  ❌ placement_addition/dangerous_ffi_worker_valid: did not reach FFI placement pass"; cat "$$out"; rm -rf "$$tmpdir"; exit 1; \
	elif ! grep -q "Successfully built dangerous_ffi_worker_valid" "$$out"; then \
		echo "  ❌ placement_addition/dangerous_ffi_worker_valid: did not finish emit"; cat "$$out"; rm -rf "$$tmpdir"; exit 1; \
	else \
		echo "  ✅ placement_addition/dangerous_ffi_worker_valid: cast/worker model OK"; \
	fi; \
	rm -rf "$$tmpdir"

phase-5: phase-4 placement-valid
	@echo "✅ ffi_addition phase-5 complete"

taint-valid: $(SILICA_COMPILER)
	@test -x "$(SILICA_COMPILER)" || { echo "FAIL: build compiler first (make -C compiler/silica-compiler/src build)"; exit 1; }
	@tmpdir=$$(mktemp -d); \
	ln -s "$(THIS_DIR)taint_addition/dangerous_legacy_stub.silica" "$$tmpdir/dangerous_legacy_stub.silica"; \
	ln -s "$(THIS_DIR)taint_addition/dangerous_taint_worker_valid.silica" "$$tmpdir/dangerous_taint_worker_valid.silica"; \
	ln -s "$(FIXTURES_DIR)/dangerous_exposure_source" "$$tmpdir/dangerous_exposure_source"; \
	printf "dangerous_legacy_stub.silica\ndangerous_taint_worker_valid.silica\n" > "$$tmpdir/silica.config"; \
	out="$$tmpdir/out.txt"; \
	(cd "$$tmpdir" && "$(SILICA_COMPILER)") > "$$out" 2>&1 || true; \
	if grep -qE '❌|Error error|ParseError|TypeError|EffectError|ModuleError' "$$out"; then \
		echo "  ❌ taint_addition/dangerous_taint_worker_valid: expected compile success"; cat "$$out"; rm -rf "$$tmpdir"; exit 1; \
	elif ! grep -q "FFI taint..." "$$out"; then \
		echo "  ❌ taint_addition/dangerous_taint_worker_valid: did not reach FFI taint pass"; cat "$$out"; rm -rf "$$tmpdir"; exit 1; \
	elif ! grep -q "Successfully built dangerous_taint_worker_valid" "$$out"; then \
		echo "  ❌ taint_addition/dangerous_taint_worker_valid: did not finish emit"; cat "$$out"; rm -rf "$$tmpdir"; exit 1; \
	else \
		echo "  ✅ taint_addition/dangerous_taint_worker_valid: structural taint OK"; \
	fi; \
	rm -rf "$$tmpdir"

phase-6: phase-5 taint-valid
	@echo "✅ ffi_addition phase-6 complete"

abi-valid: $(SILICA_COMPILER)
	@test -x "$(SILICA_COMPILER)" || { echo "FAIL: build compiler first (make -C compiler/silica-compiler/src build)"; exit 1; }
	@failed=0; \
	for silica in $(ABI_VALID_FILES); do \
		base=$$(basename "$$silica" .silica); \
		tmpdir=$$(mktemp -d); \
		ln -s "$(THIS_DIR)$$silica" "$$tmpdir/$$base.silica"; \
		ln -s "$(FIXTURES_DIR)/dangerous_exposure_source" "$$tmpdir/dangerous_exposure_source"; \
		echo "$$base.silica" > "$$tmpdir/silica.config"; \
		out="$$tmpdir/out.txt"; \
		(cd "$$tmpdir" && "$(SILICA_COMPILER)") > "$$out" 2>&1 || true; \
		if grep -qE '❌|Error error|ParseError|TypeError|EffectError|ModuleError' "$$out"; then \
			echo "  ❌ $$silica: expected compile success"; cat "$$out"; failed=1; \
		elif ! grep -q "FFI ABI..." "$$out"; then \
			echo "  ❌ $$silica: did not reach FFI ABI pass"; cat "$$out"; failed=1; \
		elif ! grep -q "Successfully built $$base" "$$out"; then \
			echo "  ❌ $$silica: did not finish emit"; cat "$$out"; failed=1; \
		else \
			echo "  ✅ $$silica: Silica-side ABI OK"; \
		fi; \
		rm -rf "$$tmpdir"; \
	done; \
	test "$$failed" -eq 0

phase-7: phase-6 abi-valid
	@echo "✅ ffi_addition phase-7 complete"

phase-8: phase-7 fixtures
	@test -x "$(SILICA_COMPILER)" || { echo "FAIL: build compiler first (make -C compiler/silica-compiler/src build)"; exit 1; }
	@$(MAKE) -C "$(THIS_DIR)runtime_addition" integrate
	@echo "✅ ffi_addition phase-8 complete"

# Alias for callers who refer to this as "phase a".
phase-a: phase-0

clean-fixtures:
	@rm -rf "$(BUILD_DIR)"
	@rm -f "$(LEGACY_ARCHIVE)" "$(TEXT_ARCHIVE)"

clean: clean-fixtures
	@rm -f .integrate_counts
	@echo "✅ ffi_addition clean complete"

help:
	@echo "ffi_addition Makefile"
	@echo "=================="
	@echo ""
	@echo "Targets:"
	@echo "  integrate          - Run the full FFI trial chain used by top-level trials/integrate"
	@echo "  phase-0 / phase-a  - Build fixture archives and verify exported symbols"
	@echo "  phase-3            - Module naming, exports, wrapper_meta path rules (module_addition/)"
	@echo "  phase-4            - Sidecar metadata loader (metadata_addition/)"
	@echo "  phase-5            - Cast/worker placement rules (placement_addition/)"
	@echo "  phase-6            - Structural taint and message boundaries (taint_addition/)"
	@echo "  phase-7            - Silica-side ABI checker (abi_addition/)"
	@echo "  phase-8            - Emitter/runtime foreign calls (runtime_addition/)"
	@echo "  module-valid       - Compile dangerous_naming_valid through type check"
	@echo "  metadata-valid     - Compile dangerous_sidecar_match through type check"
	@echo "  placement-valid     - Compile dangerous_ffi_worker_valid through emit"
	@echo "  taint-valid          - Compile dangerous_taint_worker_valid through emit"
	@echo "  abi-valid            - Compile scalar_abi_valid and string_two_layer_valid through emit"
	@echo "  fixtures           - Build libsilica_legacy_math.a and libsilica_text.a"
	@echo "  verify-symbols     - Re-check nm/llvm-nm symbol exports"
	@echo "  clean              - Remove build artifacts and archives"
	@echo "  help               - Show this help"
