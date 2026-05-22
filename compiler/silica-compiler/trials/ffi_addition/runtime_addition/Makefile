# Phase 8 runtime e2e: compile, link fixture archives, run .sout vs .scout

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
FFI_TRIAL_DIR := $(abspath $(THIS_DIR)..)
FIXTURES_DIR := $(FFI_TRIAL_DIR)/fixtures
LIB_DIR := $(FIXTURES_DIR)/dangerous_exposure_source/lib

SILICA_COMPILER := $(abspath $(THIS_DIR)../../../src/silica-compiler)
BASE_MAKEFILE := $(abspath $(THIS_DIR)../../base/makefile)

LEGACY_ARCHIVE := $(LIB_DIR)/libsilica_legacy_math.a
TEXT_ARCHIVE := $(LIB_DIR)/libsilica_text.a

ARCH := arm64
MACOS_MIN_VERSION := 26.0
ASFLAGS := -mmacosx-version-min=$(MACOS_MIN_VERSION)
EXTRA_LIBS := -L$(LIB_DIR) -lsilica_legacy_math -lsilica_text

RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)
RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")
RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld
MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)
LDFLAGS_rust-lld := -arch $(ARCH) -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) -syslibroot $(MACOS_SDK) -lSystem $(EXTRA_LIBS) -e main
LDFLAGS_clang := $(EXTRA_LIBS) -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)

RUNTIME_TRIALS := dangerous_scalar_add_e2e dangerous_string_echo_e2e dangerous_test_concat
STUB_OBJECTS := dangerous_legacy_stub.o dangerous_text_stub.o

.PHONY: all integrate clean

all: integrate

integrate: $(LEGACY_ARCHIVE) $(TEXT_ARCHIVE)
	@ln -sf "$(FIXTURES_DIR)/dangerous_exposure_source" "$(THIS_DIR)dangerous_exposure_source"
	@cd "$(THIS_DIR)" && rm -f *.sams *.o $(RUNTIME_TRIALS) silica.config .integrate_counts
	@cd "$(THIS_DIR)" && find "$(THIS_DIR)" -maxdepth 1 -name '*.silica' | sed "s|^$(THIS_DIR)/||" > silica.config
	@cd "$(THIS_DIR)" && if ! "$(SILICA_COMPILER)"; then printf '%d %d\n' 0 1 > .integrate_counts; exit 1; fi
	@failed=0; ok=0; ko=0; \
	cd "$(THIS_DIR)"; \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		echo "Assembling $$sams..."; \
		clang $(ASFLAGS) -c -x assembler "$$sams" -o "$$base.o" || { failed=1; break; }; \
	done; \
	for trial in $(RUNTIME_TRIALS); do \
		[ "$$failed" -ne 0 ] && break; \
		runtime_obj=""; \
		[ -f __silica_runtime.o ] && runtime_obj=__silica_runtime.o; \
		case "$$trial" in \
			dangerous_scalar_add_e2e) stub_obj="" ;; \
			dangerous_string_echo_e2e) stub_obj="" ;; \
			*) stub_obj="" ;; \
		esac; \
		if test -x "$(RUST_LLD)"; then \
			"$(RUST_LLD)" -flavor darwin -o "$$trial" "$$trial.o" "$$stub_obj" $$runtime_obj $(LDFLAGS_rust-lld) || failed=1; \
		else \
			clang "$$trial.o" "$$stub_obj" $$runtime_obj -o "$$trial" $(LDFLAGS_clang) || failed=1; \
		fi; \
		[ "$$failed" -ne 0 ] && break; \
		if [ -f "$$trial.wait_for_exit" ]; then \
			marker=$$(awk 'NF { print $$1; found=1; exit } END { if (!found) print "done" }' "$$trial.wait_for_exit"); \
			marker_count=$$(awk 'NF { print ($$2 == "" ? 1 : $$2); found=1; exit } END { if (!found) print 1 }' "$$trial.wait_for_exit"); \
			python3 "$(THIS_DIR)run_integration_exit_after_marker.py" "$(THIS_DIR)" "$$trial" "$$trial.sout" "$$marker" "$$marker_count" || failed=1; \
		else \
			{ ./"$$trial" 2>&1; echo $$?; } > "$$trial.sout"; \
		fi; \
		[ "$$failed" -ne 0 ] && break; \
		if ! diff -Bw -q "$$trial.sout" "$$trial.scout" > /dev/null 2>&1; then \
			echo "  ❌ runtime_addition/$$trial .sout differs from .scout"; \
			diff -Bw "$$trial.sout" "$$trial.scout" || true; \
			ko=$$((ko + 1)); failed=1; \
		else \
			echo "  ✅✅ runtime_addition/$$trial output matches .scout"; \
			ok=$$((ok + 1)); \
		fi; \
	done; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

clean:
	@cd "$(THIS_DIR)" && rm -f *.sams *.o *.sout $(RUNTIME_TRIALS) silica.config .integrate_counts dangerous_exposure_source
