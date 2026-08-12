# Shared Makefile fragment for Phase 1 standard-data-structure trial leaves.
# Each leaf Makefile must set LEAF_DIR before including this file:
#   LEAF_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
#   include ../leaf.mk

ifndef LEAF_DIR
$(error LEAF_DIR must be set before including leaf.mk)
endif

THIS_DIR        := $(LEAF_DIR)
include $(LEAF_DIR)../../silica_compiler.mk
TRIAL_DIR       := $(notdir $(patsubst %/,%,$(THIS_DIR)))
MSG_PREFIX      := standard_data_structures_phase1/$(TRIAL_DIR)/

ASSEMBLER       := clang
MACOS_MIN_VERSION ?= 26.0
ASFLAGS_macos   := -mmacosx-version-min=$(MACOS_MIN_VERSION)
LDFLAGS_clang   := -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)

# Positive trials: compile, link, run, diff .sams/.ascomp and .sout/.scout.
# Pure-make discovery (no $(shell find)) so `make` starts immediately on slow/network volumes.
# Excluded from silica.config (handled by other harness paths):
#   trial_negative_*            isolated negative compiles
#   trial_compile_fail_*        expected compile failure (error_enforcement_addition/standard_data_structures)
#   trial_collection_error_*    expected deterministic runtime collection error
_ALL_LEAF_SILICA := $(wildcard $(THIS_DIR)*.silica)
_EXCLUDED_SILICA := $(wildcard $(THIS_DIR)trial_negative_*.silica) \
	$(wildcard $(THIS_DIR)trial_compile_fail_*.silica) \
	$(wildcard $(THIS_DIR)trial_collection_error_*.silica)
POSITIVE_SILICA := $(sort $(patsubst $(THIS_DIR)%,%,$(filter-out $(_EXCLUDED_SILICA),$(_ALL_LEAF_SILICA))))

# Remove stale assembly/link artifacts before compile (used inline — not via `clean` prerequisite).
INTEGRATE_PRE_CLEAN = cd "$(THIS_DIR)" && \
	rm -f .integrate_counts *.sams lib/*.sams *.o lib/*.o *.iface lib/*.iface __silica_runtime.o *.sout \
		silica.compile.order silica.needs_runtime silica.link && \
	for s in *.sams; do \
		[ -f "$$s" ] || continue; \
		rm -f "$${s%.sams}"; \
	done

.PHONY: all clean assembly objects executables integrate positive-integrate record-positive-golden record-golden help silica.config

.DEFAULT_GOAL := all

silica.config:
	@echo "$(MSG_PREFIX)regenerating silica.config..."
	@cd "$(THIS_DIR)" && { \
		printf '%s\n' $(POSITIVE_SILICA); \
		if [ -d lib ]; then find lib \( -type f -o -type l \) -name '*.silica' | sort; fi; \
	} > silica.config

all: executables

assembly: silica.config
	@if [ ! -s "$(THIS_DIR)silica.config" ]; then \
		echo "SKIP: $(MSG_PREFIX)no positive .silica trials"; \
		exit 0; \
	fi
	@$(ENSURE_SILICA_COMPILER)
	@$(INTEGRATE_PRE_CLEAN)
	@echo "Compiling with silica-compiler..."
	@cd "$(THIS_DIR)" && $(RUN_SILICA_COMPILER)
	@echo "✅ $(MSG_PREFIX)Assembly generated"

objects: assembly
	@cd "$(THIS_DIR)" && for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		echo "Assembling: $$sams -> $$base.o"; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o"; \
	done
	@if [ -f "$(THIS_DIR)__silica_runtime.sams" ]; then \
		echo "Assembling: __silica_runtime.sams -> __silica_runtime.o"; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$(THIS_DIR)__silica_runtime.sams" -o "$(THIS_DIR)__silica_runtime.o"; \
	fi
	@cd "$(THIS_DIR)" && for sams in lib/*.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$$(basename "$$sams" .sams); \
		echo "Assembling: $$sams -> lib/$$base.o"; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "lib/$$base.o"; \
	done

executables: objects
	@cd "$(THIS_DIR)" || exit 1; \
	lib_objs=""; \
	for obj in lib/*.o; do [ -f "$$obj" ] && lib_objs="$$lib_objs $$obj"; done; \
	rust_sysroot=$$(rustc --print sysroot 2>/dev/null); \
	rust_target=$$(rustc -vV 2>/dev/null | sed -n 's/^host: //p'); \
	rust_lld="$$rust_sysroot/lib/rustlib/$$rust_target/bin/rust-lld"; \
	macos_sdk=$$(xcrun --sdk macosx --show-sdk-path 2>/dev/null); \
	for trial in $(POSITIVE_SILICA); do \
		base=$${trial%.silica}; \
		[ -f "$$base.o" ] || continue; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		if test -x "$$rust_lld"; then \
			$$rust_lld -flavor darwin -o "$$base" "$$base.o" $$lib_objs $$runtime_obj \
				-arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) \
				-syslibroot "$$macos_sdk" -lSystem -e main && echo "  ✅ $(MSG_PREFIX)$$base (rust-lld)"; \
		else \
			$(ASSEMBLER) "$$base.o" $$lib_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang) && echo "  ✅ $(MSG_PREFIX)$$base (clang)"; \
		fi; \
	done

positive-integrate: silica.config
	@echo "$(MSG_PREFIX)positive-integrate: starting..."
	@cd "$(THIS_DIR)" || exit 1; \
	$(INTEGRATE_PRE_CLEAN); \
	ok=0; ko=0; failed=0; \
	if [ ! -s silica.config ]; then \
		echo "SKIP: $(MSG_PREFIX)no positive trials"; \
		printf '%d %d\n' 0 0 > .integrate_counts; \
		exit 0; \
	fi; \
	$(ENSURE_SILICA_COMPILER); \
	echo "Compiling with silica-compiler..."; \
	ec=0; \
	{ $(RUN_SILICA_COMPILER); } || ec=$$?; \
	if [ "$$ec" -ne 0 ]; then \
		if [ "$$ec" -eq 137 ] || [ "$$ec" -eq 9 ]; then \
			echo "❌❌ $(MSG_PREFIX)compilation killed (exit $$ec; likely OOM while compiling large silica.config)"; \
		else \
			echo "❌❌ $(MSG_PREFIX)compilation failed (exit $$ec)"; \
		fi; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi; \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		if [ ! -f "$$base.ascomp" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base has no .ascomp file (run make record-golden from trial root)"; \
			ko=$$((ko + 1)); failed=1; \
		elif ! diff -Bw -q "$$sams" "$$base.ascomp" > /dev/null 2>&1; then \
			echo "❌❌ $(MSG_PREFIX)$$base .sams differs from .ascomp"; \
			diff -Bw "$$sams" "$$base.ascomp" || true; \
			ko=$$((ko + 1)); failed=1; \
		else \
			echo "✅✅ $(MSG_PREFIX)$$base assembly matches .ascomp"; \
			ok=$$((ok + 1)); \
		fi; \
	done; \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		echo "Assembling $$sams..."; \
		if ! $(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o"; then \
			echo "❌❌ $(MSG_PREFIX)$$base assemble failed"; \
			ko=$$((ko + 1)); failed=1; break; \
		fi; \
	done; \
	if [ -f "__silica_runtime.sams" ]; then \
		echo "Assembling __silica_runtime.sams..."; \
		if ! $(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "__silica_runtime.sams" -o "__silica_runtime.o"; then \
			echo "❌❌ $(MSG_PREFIX)__silica_runtime assemble failed"; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
	fi; \
	for sams in lib/*.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$$(basename "$$sams" .sams); \
		echo "Assembling $$sams..."; \
		if ! $(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "lib/$$base.o"; then \
			echo "❌❌ $(MSG_PREFIX)lib/$$base assemble failed"; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
	done; \
	lib_objs=""; \
	for obj in lib/*.o; do [ -f "$$obj" ] && lib_objs="$$lib_objs $$obj"; done; \
	rust_sysroot=$$(rustc --print sysroot 2>/dev/null); \
	rust_target=$$(rustc -vV 2>/dev/null | sed -n 's/^host: //p'); \
	rust_lld="$$rust_sysroot/lib/rustlib/$$rust_target/bin/rust-lld"; \
	macos_sdk=$$(xcrun --sdk macosx --show-sdk-path 2>/dev/null); \
	for trial in $(POSITIVE_SILICA); do \
		base=$${trial%.silica}; \
		[ -f "$$base.o" ] || continue; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		echo "Linking $$base..."; \
		if test -x "$$rust_lld"; then \
			if ! $$rust_lld -flavor darwin -o "$$base" "$$base.o" $$lib_objs $$runtime_obj \
				-arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) \
				-syslibroot "$$macos_sdk" -lSystem -e main; then \
				echo "❌❌ $(MSG_PREFIX)$$base link failed"; \
				ko=$$((ko + 1)); failed=1; \
			fi; \
		else \
			if ! $(ASSEMBLER) "$$base.o" $$lib_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang); then \
				echo "❌❌ $(MSG_PREFIX)$$base link failed"; \
				ko=$$((ko + 1)); failed=1; \
			fi; \
		fi; \
	done; \
	for trial in $(POSITIVE_SILICA); do \
		base=$${trial%.silica}; \
		if [ ! -x "$$base" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base missing executable (assemble/link failed earlier)"; \
			ko=$$((ko + 1)); failed=1; \
			continue; \
		fi; \
		echo "Running $$base..."; \
		{ ./$$base 2>&1; echo $$?; } > "$$base.sout"; \
		if [ -f "$$base.scout" ]; then \
			if ! diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
				echo "❌❌ $(MSG_PREFIX)$$base .sout differs from .scout"; \
				diff -Bw "$$base.sout" "$$base.scout" || true; \
				ko=$$((ko + 1)); failed=1; \
			else \
				echo "✅✅ $(MSG_PREFIX)$$base output matches .scout"; \
				ok=$$((ok + 1)); \
			fi; \
		else \
			echo "❌❌ $(MSG_PREFIX)$$base has no .scout file (run make record-golden from trial root)"; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
	done; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

integrate: positive-integrate

record-positive-golden: silica.config
	@echo "$(MSG_PREFIX)record-positive-golden: starting..."
	@$(ENSURE_SILICA_COMPILER)
	@$(INTEGRATE_PRE_CLEAN)
	@cd "$(THIS_DIR)" && $(RUN_SILICA_COMPILER)
	@cd "$(THIS_DIR)" && for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		cp "$$sams" "$$base.ascomp"; \
		echo "Recorded $$base.ascomp"; \
	done
	@cd "$(THIS_DIR)" && for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o"; \
	done
	@if [ -f "$(THIS_DIR)__silica_runtime.sams" ]; then \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$(THIS_DIR)__silica_runtime.sams" -o "$(THIS_DIR)__silica_runtime.o"; \
	fi
	@cd "$(THIS_DIR)" && for sams in lib/*.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$$(basename "$$sams" .sams); \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "lib/$$base.o"; \
	done
	@cd "$(THIS_DIR)" || exit 1; \
	lib_objs=""; \
	for obj in lib/*.o; do [ -f "$$obj" ] && lib_objs="$$lib_objs $$obj"; done; \
	rust_sysroot=$$(rustc --print sysroot 2>/dev/null); \
	rust_target=$$(rustc -vV 2>/dev/null | sed -n 's/^host: //p'); \
	rust_lld="$$rust_sysroot/lib/rustlib/$$rust_target/bin/rust-lld"; \
	macos_sdk=$$(xcrun --sdk macosx --show-sdk-path 2>/dev/null); \
	for trial in $(POSITIVE_SILICA); do \
		base=$${trial%.silica}; \
		[ -f "$$base.o" ] || continue; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		if test -x "$$rust_lld"; then \
			$$rust_lld -flavor darwin -o "$$base" "$$base.o" $$lib_objs $$runtime_obj \
				-arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) \
				-syslibroot "$$macos_sdk" -lSystem -e main; \
		else \
			$(ASSEMBLER) "$$base.o" $$lib_objs $$runtime_obj -o "$$base" $(LDFLAGS_clang); \
		fi; \
		{ ./$$base 2>&1; echo $$?; } > "$$base.scout"; \
		echo "Recorded $$base.scout"; \
	done
	@cd "$(THIS_DIR)" && for trial in $(POSITIVE_SILICA); do \
		base=$${trial%.silica}; \
		rm -f "$$base" "$$base.o"; \
	done
	@cd "$(THIS_DIR)" && rm -f __silica_runtime.o lib/*.o *.iface lib/*.iface *.sams lib/*.sams *.sout .integrate_counts
	@echo "✅ $(MSG_PREFIX)record-positive-golden complete"

record-golden: record-positive-golden

clean:
	@echo "$(MSG_PREFIX)clean: removing prior artifacts..."
	@cd "$(THIS_DIR)" && for s in *.sams; do \
		[ -f "$$s" ] || continue; \
		rm -f "$${s%.sams}"; \
	done
	@cd "$(THIS_DIR)" && rm -f *.sams lib/*.sams *.o lib/*.o *.iface lib/*.iface __silica_runtime.o smoke_harness_ready *.sout silica.config .integrate_counts silica.compile.order silica.needs_runtime silica.link
	@echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "Phase 1 leaf trial Makefile ($(TRIAL_DIR))"
	@echo "  all / executables  - build positive trials"
	@echo "  integrate          - compile, diff .ascomp, run, diff .scout"
	@echo "  record-positive-golden - capture .ascomp and .scout from current compiler"
	@echo "  record-golden          - alias for record-positive-golden"
	@echo "  clean              - remove build artifacts"
