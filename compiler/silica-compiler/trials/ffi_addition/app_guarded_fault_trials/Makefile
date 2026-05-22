THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
APP_TRIAL_DIR := $(abspath $(THIS_DIR))
APP_LABEL := app_guarded_fault_trials
APP_EXECUTABLES := \
	dangerous_guarded_null_deref_fault \
	dangerous_guarded_arena_bounds_fault \
	dangerous_guarded_sigbus_fault \
	dangerous_handler_altstack_stack_overflow_smoke \
	dangerous_unguarded_null_deref_abort

FFI_TRIAL_DIR := $(abspath $(THIS_DIR)/..)
FIXTURES_DIR := $(FFI_TRIAL_DIR)/fixtures
SILICA_COMPILER := $(abspath $(THIS_DIR)/../../../src/silica-compiler)
SILICA_LINK_SH := $(abspath $(FFI_TRIAL_DIR)/../silica_link.sh)
UNGUARDED_PROBE := $(THIS_DIR)/unguarded_null_deref_probe

include $(FFI_TRIAL_DIR)/fixtures.mk

ARCH := arm64
MACOS_MIN_VERSION := 26.0
ASFLAGS := -mmacosx-version-min=$(MACOS_MIN_VERSION)
FFI_CFLAGS := -std=c11 -Wall -Wextra -O0 -arch $(ARCH) -mmacosx-version-min=$(MACOS_MIN_VERSION) \
	-I$(FIXTURES_DIR)/dangerous_exposure_source/legacy

RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)
RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")
RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld
MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)
LDFLAGS_rust-lld := -arch $(ARCH) -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) -syslibroot $(MACOS_SDK) -lSystem -e main
LDFLAGS_clang := -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)

.PHONY: all integrate clean fixtures unguarded_probe altstack_check

all: integrate

fixtures: ffi-wrapper-archives

$(UNGUARDED_PROBE): $(FIXTURES_DIR)/src/silica_ffi_fault.c
	clang $(FFI_CFLAGS) -c $(FIXTURES_DIR)/src/silica_ffi_fault.c -o $(THIS_DIR)/silica_ffi_fault.o
	clang $(FFI_CFLAGS) $(THIS_DIR)/unguarded_null_deref_main.c $(THIS_DIR)/silica_ffi_fault.o -o $@

unguarded_probe: $(UNGUARDED_PROBE)

altstack_check: dangerous_guarded_null_deref_fault
	@nm "$(APP_TRIAL_DIR)/dangerous_guarded_null_deref_fault" 2>/dev/null | grep -q '_silica_rt_ffi_altstack_mem' || { echo "  ❌ altstack symbol missing"; exit 1; }
	@echo "  ✅ altstack memory symbol present"

integrate: fixtures unguarded_probe
	@ln -sf "$(FIXTURES_DIR)/dangerous_exposure_source" "$(APP_TRIAL_DIR)/dangerous_exposure_source"
	@test -f "$(FFI_LEGACY_ARCHIVE)" || { echo "  ❌ $(APP_LABEL): missing legacy archive"; exit 1; }
	@cd "$(APP_TRIAL_DIR)" && rm -f *.sams *.o $(APP_EXECUTABLES) silica.config silica.link .integrate_counts
	@cd "$(APP_TRIAL_DIR)" && find "$(APP_TRIAL_DIR)" -maxdepth 1 -name '*.silica' ! -name 'dangerous_unguarded_null_deref_abort.silica' | sed "s|^$(APP_TRIAL_DIR)/||" | sort > silica.config
	@cd "$(APP_TRIAL_DIR)" && if ! "$(SILICA_COMPILER)"; then printf '%d %d\n' 0 1 > .integrate_counts; exit 1; fi
	@failed=0; ok=0; ko=0; \
	cd "$(APP_TRIAL_DIR)"; \
	link_archives=$$("$(SILICA_LINK_SH)" .); \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		echo "Assembling $$sams..."; \
		clang $(ASFLAGS) -c -x assembler "$$sams" -o "$$base.o" || { failed=1; break; }; \
	done; \
	for trial in dangerous_guarded_null_deref_fault dangerous_guarded_arena_bounds_fault dangerous_guarded_sigbus_fault dangerous_handler_altstack_stack_overflow_smoke; do \
		[ "$$failed" -ne 0 ] && break; \
		runtime_obj=""; [ -f __silica_runtime.o ] && runtime_obj=__silica_runtime.o; \
		if test -x "$(RUST_LLD)"; then \
			"$(RUST_LLD)" -flavor darwin -o "$$trial" "$$trial.o" $$runtime_obj $$link_archives $(LDFLAGS_rust-lld) || failed=1; \
		else \
			clang "$$trial.o" $$runtime_obj $$link_archives -o "$$trial" $(LDFLAGS_clang) || failed=1; \
		fi; \
		[ "$$failed" -ne 0 ] && break; \
		{ ./"$$trial" 2>&1; echo $$?; } > "$$trial.sout" || failed=1; \
		[ "$$failed" -ne 0 ] && break; \
		if ! diff -Bw -q "$$trial.sout" "$$trial.scout" > /dev/null 2>&1; then \
			echo "  ❌ $(APP_LABEL)/$$trial .sout differs from .scout"; diff -Bw "$$trial.sout" "$$trial.scout" || true; \
			ko=$$((ko + 1)); failed=1; \
		else \
			echo "  ✅✅ $(APP_LABEL)/$$trial output matches .scout"; ok=$$((ok + 1)); \
		fi; \
	done; \
	[ "$$failed" -eq 0 ] && $(MAKE) -C "$(APP_TRIAL_DIR)" altstack_check || failed=1; \
	[ "$$failed" -eq 0 ] && python3 "$(FFI_TRIAL_DIR)/run_integration_expect_crash.py" "$(UNGUARDED_PROBE)" && echo "  ✅✅ $(APP_LABEL)/unguarded_null_deref_abort crash probe OK" && ok=$$((ok + 1)) || { ko=$$((ko + 1)); failed=1; }; \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

clean:
	@cd "$(THIS_DIR)" && rm -f *.sams *.o $(APP_EXECUTABLES) silica.config silica.link .integrate_counts *.sout dangerous_exposure_source $(UNGUARDED_PROBE) silica_ffi_fault.o
