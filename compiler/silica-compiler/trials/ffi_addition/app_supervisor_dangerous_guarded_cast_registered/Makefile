THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
APP_TRIAL_DIR := $(abspath $(THIS_DIR))
APP_LABEL := app_supervisor_dangerous_guarded_cast_registered
APP_EXECUTABLES := dangerous_supervisor_guarded_cast_registered
SIBLING_DIR := $(abspath $(THIS_DIR)/../app_supervisor_dangerous_guarded_restart)

FFI_TRIAL_DIR := $(abspath $(THIS_DIR)/..)
FIXTURES_DIR := $(FFI_TRIAL_DIR)/fixtures
SILICA_COMPILER := $(abspath $(THIS_DIR)/../../../src/silica-compiler)
SILICA_LINK_SH := $(abspath $(FFI_TRIAL_DIR)/../silica_link.sh)

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

TRIAL_FAULT_OBJ := $(THIS_DIR)/silica_supervisor_trial_guarded_fault.o
TRIAL_FAULT_C := $(SIBLING_DIR)/silica_supervisor_trial_guarded_fault.c

.PHONY: all integrate clean fixtures trial_fault_obj

all: integrate

fixtures: ffi-wrapper-archives trial_fault_obj

$(TRIAL_FAULT_OBJ): $(TRIAL_FAULT_C) $(FIXTURES_DIR)/src/silica_ffi_fault.c
	clang $(FFI_CFLAGS) -c $(FIXTURES_DIR)/src/silica_ffi_fault.c -o $(THIS_DIR)/silica_ffi_fault.o
	clang $(FFI_CFLAGS) -c $(TRIAL_FAULT_C) -o $@

trial_fault_obj: $(TRIAL_FAULT_OBJ)

integrate: fixtures
	@ln -sf "$(FIXTURES_DIR)/dangerous_exposure_source" "$(APP_TRIAL_DIR)/dangerous_exposure_source"
	@test -f "$(FFI_LEGACY_ARCHIVE)" || { echo "  ❌ $(APP_LABEL): missing legacy archive"; exit 1; }
	@cd "$(APP_TRIAL_DIR)" && rm -f *.sams *.o $(APP_EXECUTABLES) silica.config silica.link .integrate_counts
	@$(MAKE) trial_fault_obj
	@cd "$(APP_TRIAL_DIR)" && { \
		find "$(APP_TRIAL_DIR)" -maxdepth 1 -name '*.silica' | sed "s|^$(APP_TRIAL_DIR)/||" | sort; \
		echo "../../../stdlib/Supervisor.silica"; \
	} > silica.config
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
	[ -f __silica_runtime.sams ] && clang $(ASFLAGS) -c -x assembler __silica_runtime.sams -o __silica_runtime.o || true; \
	for trial in $(APP_EXECUTABLES); do \
		[ "$$failed" -ne 0 ] && break; \
		runtime_obj=""; \
		[ -f __silica_runtime.o ] && runtime_obj=__silica_runtime.o; \
		if test -x "$(RUST_LLD)"; then \
			"$(RUST_LLD)" -flavor darwin -o "$$trial" "$$trial.o" $$runtime_obj $(TRIAL_FAULT_OBJ) $$link_archives $(LDFLAGS_rust-lld) || failed=1; \
		else \
			clang "$$trial.o" $$runtime_obj $(TRIAL_FAULT_OBJ) $$link_archives -o "$$trial" $(LDFLAGS_clang) || failed=1; \
		fi; \
		[ "$$failed" -ne 0 ] && break; \
		if [ -f "$$trial.wait_for_exit" ]; then \
			marker=$$(awk 'NF { if ($$NF ~ /^[0-9]+$$/) { m=$$0; sub(/[ \t]+[0-9]+$$/, "", m); print m } else print $$0; exit } END { if (!NR) print "done" }' "$$trial.wait_for_exit"); \
			marker_count=$$(awk 'NF { if ($$NF ~ /^[0-9]+$$/) print $$NF; else print 1; exit } END { if (!NR) print 1 }' "$$trial.wait_for_exit"); \
			python3 "$(FFI_TRIAL_DIR)/run_integration_exit_after_marker.py" "$(APP_TRIAL_DIR)" "$$trial" "$$trial.sout" "$$marker" "$$marker_count" || failed=1; \
		else \
			{ ./"$$trial" 2>&1; echo $$?; } > "$$trial.sout" || failed=1; \
		fi; \
		[ "$$failed" -ne 0 ] && break; \
		if [ ! -f "$$trial.scout" ]; then \
			echo "  ❌ $(APP_LABEL)/$$trial: missing .scout"; ko=$$((ko + 1)); failed=1; \
		elif [ -f "$$trial.scout.sorted" ] && [ -f "$(FFI_TRIAL_DIR)/compare_scout_sorted.sh" ] && ! "$(SHELL)" "$(FFI_TRIAL_DIR)/compare_scout_sorted.sh" "$$trial.sout" "$$trial.scout" > /dev/null 2>&1; then \
			echo "  ❌ $(APP_LABEL)/$$trial .sout differs from .scout (sorted compare)"; \
			"$(SHELL)" "$(FFI_TRIAL_DIR)/compare_scout_sorted.sh" "$$trial.sout" "$$trial.scout" || true; \
			ko=$$((ko + 1)); failed=1; \
		elif [ -f "$$trial.scout.sorted" ] && [ ! -f "$(FFI_TRIAL_DIR)/compare_scout_sorted.sh" ] && ! diff -Bw -q "$$trial.sout" "$$trial.scout" > /dev/null 2>&1; then \
			echo "  ❌ $(APP_LABEL)/$$trial .sout differs from .scout (missing compare_scout_sorted.sh; used diff -Bw)"; \
			diff -Bw "$$trial.sout" "$$trial.scout" || true; \
			ko=$$((ko + 1)); failed=1; \
		elif [ ! -f "$$trial.scout.sorted" ] && [ -f "$(FFI_TRIAL_DIR)/compare_scout_normalized.sh" ] && ! "$(SHELL)" "$(FFI_TRIAL_DIR)/compare_scout_normalized.sh" "$$trial.sout" "$$trial.scout" > /dev/null 2>&1; then \
			echo "  ❌ $(APP_LABEL)/$$trial .sout differs from .scout (normalized compare)"; \
			"$(SHELL)" "$(FFI_TRIAL_DIR)/compare_scout_normalized.sh" "$$trial.sout" "$$trial.scout" || true; \
			ko=$$((ko + 1)); failed=1; \
		elif [ ! -f "$$trial.scout.sorted" ] && [ ! -f "$(FFI_TRIAL_DIR)/compare_scout_normalized.sh" ] && ! diff -Bw -q "$$trial.sout" "$$trial.scout" > /dev/null 2>&1; then \
			echo "  ❌ $(APP_LABEL)/$$trial .sout differs from .scout"; \
			diff -Bw "$$trial.sout" "$$trial.scout" || true; \
			ko=$$((ko + 1)); failed=1; \
		else \
			echo "  ✅✅ $(APP_LABEL)/$$trial output matches .scout"; \
			ok=$$((ok + 1)); \
		fi; \
	done; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

clean:
	@cd "$(THIS_DIR)" && rm -f *.sams *.o $(APP_EXECUTABLES) silica.config silica.link .integrate_counts *.sout dangerous_exposure_source silica_ffi_fault.o silica_supervisor_trial_guarded_fault.o
