# Shared integrate recipe for ffi_addition app trials.
# Required variables before include:
#   APP_TRIAL_DIR   - absolute or Make-relative path to the app trial directory
#   APP_LABEL       - short label for log messages (e.g. app_cast_worker_legacy_add)
#   APP_LINK_OBJECTS - optional extra .o files to link with each executable (e.g. stub modules)
APP_LINK_OBJECTS ?=

FFI_TRIAL_DIR := $(abspath $(APP_TRIAL_DIR)/..)
include $(dir $(abspath $(lastword $(MAKEFILE_LIST))))../silica_compiler.mk
FIXTURES_DIR := $(FFI_TRIAL_DIR)/fixtures
SILICA_LINK_SH := $(abspath $(FFI_TRIAL_DIR)/../silica_link.sh)

include $(FFI_TRIAL_DIR)/fixtures.mk

.PHONY: fixtures
fixtures: ffi-wrapper-archives

ARCH := arm64
MACOS_MIN_VERSION := 26.0
ASFLAGS := -mmacosx-version-min=$(MACOS_MIN_VERSION)

RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)
RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")
RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld
MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)
LDFLAGS_rust-lld := -arch $(ARCH) -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) -syslibroot $(MACOS_SDK) -lSystem -e main
LDFLAGS_clang := -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)

define APP_INTEGRATE_BODY
	@$(ENSURE_SILICA_COMPILER)
	@ln -sf "$(FIXTURES_DIR)/dangerous_exposure_source" "$(APP_TRIAL_DIR)/dangerous_exposure_source"
	@test -f "$(FFI_LEGACY_ARCHIVE)" || { echo "  ❌ $(APP_LABEL): missing $(FFI_LEGACY_ARCHIVE) (run make fixtures)"; exit 1; }
	@for ar in $(FFI_WRAPPER_ARCHIVES); do \
		test -f "$$ar" || { echo "  ❌ $(APP_LABEL): missing wrapper archive $$ar (run make fixtures)"; exit 1; }; \
	done
	@cd "$(APP_TRIAL_DIR)" && rm -f *.sams *.o *.iface $(APP_EXECUTABLES) silica.config silica.link .integrate_counts silica.compile.order silica.needs_runtime
	@cd "$(APP_TRIAL_DIR)" && find "$(APP_TRIAL_DIR)" -maxdepth 1 -name '*.silica' | sed "s|^$(APP_TRIAL_DIR)/||" | sort > silica.config
	@cd "$(APP_TRIAL_DIR)" && if ! { $(RUN_SILICA_COMPILER); }; then echo "❌❌ $(APP_LABEL): compilation failed"; printf '%d %d\n' 0 1 > .integrate_counts; exit 1; fi
	@failed=0; ok=0; ko=0; \
	cd "$(APP_TRIAL_DIR)"; \
	if [ -f silica.link.scout ]; then \
		if [ ! -f silica.link ]; then \
			echo "  ❌ $(APP_LABEL): silica.link not emitted"; \
			printf '%d %d\n' 0 1 > .integrate_counts; exit 1; \
		elif ! diff -Bw -q silica.link silica.link.scout > /dev/null 2>&1; then \
			echo "  ❌ $(APP_LABEL): silica.link differs from .scout"; \
			diff -Bw silica.link silica.link.scout || true; \
			printf '%d %d\n' 0 1 > .integrate_counts; exit 1; \
		else \
			echo "  ✅✅ $(APP_LABEL)/silica.link matches .scout"; \
			ok=$$((ok + 1)); \
		fi; \
	fi; \
	link_archives=$$("$(SILICA_LINK_SH)" .); \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		if [ ! -f "$$base.ascomp" ]; then \
			echo "  ❌ $(APP_LABEL)/$$base: missing .ascomp"; \
			ko=$$((ko + 1)); failed=1; \
		elif ! diff -Bw -q "$$sams" "$$base.ascomp" > /dev/null 2>&1; then \
			echo "  ❌ $(APP_LABEL)/$$base .sams differs from .ascomp"; \
			diff -Bw "$$sams" "$$base.ascomp" || true; \
			ko=$$((ko + 1)); failed=1; \
		else \
			echo "  ✅✅ $(APP_LABEL)/$$base assembly matches .ascomp"; \
			ok=$$((ok + 1)); \
		fi; \
	done; \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		echo "Assembling $$sams..."; \
		if ! clang $(ASFLAGS) -c -x assembler "$$sams" -o "$$base.o"; then \
		echo "❌❌ $$base assemble failed"; \
		ko=$$((ko + 1)); failed=1; break; \
	fi; \
	done; \
	for trial in $(APP_EXECUTABLES); do \
		[ "$$failed" -ne 0 ] && break; \
		runtime_obj=""; \
		[ -f __silica_runtime.o ] && runtime_obj=__silica_runtime.o; \
		if test -x "$(RUST_LLD)"; then \
			objs="$$trial.o $(APP_LINK_OBJECTS)"; \
			if ! "$(RUST_LLD)" -flavor darwin -o "$$trial" $$objs $$runtime_obj $$link_archives $(LDFLAGS_rust-lld); then \
			echo "❌❌ $(APP_LABEL)/$$trial link failed"; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
		else \
			objs="$$trial.o $(APP_LINK_OBJECTS)"; \
			if ! clang $$objs $$runtime_obj $$link_archives -o "$$trial" $(LDFLAGS_clang); then \
			echo "❌❌ $(APP_LABEL)/$$trial link failed"; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
		fi; \
		[ "$$failed" -ne 0 ] && break; \
		if [ -f "$$trial.wait_for_exit" ]; then \
			marker=$$(awk 'NF { if ($$NF ~ /^[0-9]+$$/) { m=$$0; sub(/[ \t]+[0-9]+$$/, "", m); print m } else print $$0; exit } END { if (!NR) print "done" }' "$$trial.wait_for_exit"); \
			marker_count=$$(awk 'NF { if ($$NF ~ /^[0-9]+$$/) print $$NF; else print 1; exit } END { if (!NR) print 1 }' "$$trial.wait_for_exit"); \
			if ! python3 "$(FFI_TRIAL_DIR)/run_integration_exit_after_marker.py" "$(APP_TRIAL_DIR)" "$$trial" "$$trial.sout" "$$marker" "$$marker_count"; then \
			echo "❌❌ $(APP_LABEL)/$$trial run failed"; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
		else \
			{ ./"$$trial" 2>&1; echo $$?; } > "$$trial.sout"; \
		fi; \
		[ "$$failed" -ne 0 ] && break; \
		if [ ! -f "$$trial.scout" ]; then \
			echo "  ❌ $(APP_LABEL)/$$trial: missing .scout"; ko=$$((ko + 1)); failed=1; \
		elif [ -f "$(FFI_TRIAL_DIR)/compare_scout_normalized.sh" ] && ! "$(SHELL)" "$(FFI_TRIAL_DIR)/compare_scout_normalized.sh" "$$trial.sout" "$$trial.scout" > /dev/null 2>&1; then \
			echo "  ❌ $(APP_LABEL)/$$trial .sout differs from .scout (normalized compare)"; \
			"$(SHELL)" "$(FFI_TRIAL_DIR)/compare_scout_normalized.sh" "$$trial.sout" "$$trial.scout" || true; \
			ko=$$((ko + 1)); failed=1; \
		elif [ ! -f "$(FFI_TRIAL_DIR)/compare_scout_normalized.sh" ] && ! diff -Bw -q "$$trial.sout" "$$trial.scout" > /dev/null 2>&1; then \
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
endef
