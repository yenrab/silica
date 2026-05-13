# Trial: atoms merged across every TU in silica.config — one executable (app_wide_atom_main), not cross-app sharing.
# layout mirrors modules_addition: lib/*.silica libraries + root main TU.

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(abspath $(THIS_DIR)../../../src/silica-compiler)

TRIAL_DIR := $(notdir $(patsubst %/,%,$(THIS_DIR)))
MSG_PREFIX := $(TRIAL_DIR)/

RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)
RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")
RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld
ASSEMBLER := clang

MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)
MACOS_MIN_VERSION ?= 26.0
ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)
LDFLAGS_rust-lld := -arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) -syslibroot $(MACOS_SDK) -lSystem -e main
LDFLAGS_clang := -Wl,-e,main -Wl,-macosx_version_min,$(MACOS_MIN_VERSION)

MAIN_BASE := app_wide_atom_main
LIB_SAM := lib/app_wide_atom_lib.sams
LIB_O := lib/app_wide_atom_lib.o

.DEFAULT_GOAL := all
.PHONY: all clean integrate help assembly

all:
	@$(MAKE) -C "$(THIS_DIR)" assembly

assembly:
	@cd "$(THIS_DIR)" && \
	find . \( -maxdepth 1 -name '*.sams' \) ! -path './.*' -delete 2>/dev/null ; \
	find ./lib \( -maxdepth 1 -name '*.sams' \) ! -path './.*' -delete 2>/dev/null ; \
	echo "Compiling with silica-compiler (app-wide atom merge)..." && \
	if ! "$(SILICA_COMPILER)"; then \
		echo "FAIL: Compilation failed"; \
		exit 1; \
	fi && \
	echo "✅ $(MSG_PREFIX)Assembly generated"

integrate:
	@cd "$(THIS_DIR)" || exit 1; \
	ok=0; ko=0; failed=0; \
	if [ ! -s silica.config ]; then \
		echo "SKIP $(MSG_PREFIX): empty silica.config"; \
		printf '%d %d\n' 0 0 > .integrate_counts; \
		exit 0; \
	fi; \
	$(MAKE) -s assembly || exit 1; \
	echo "=== $(MSG_PREFIX) (merged atoms across lib + main TU) ==="; \
	check_needles () { \
		s="$$1"; exp="$$2"; lab="$$3"; \
		r_ok=1; \
		if [ ! -f "$$s" ]; then echo "❌❌ $(MSG_PREFIX)missing $$s"; ko=$$((ko+1)); failed=1; return 1; fi; \
		if [ ! -f "$$exp" ]; then echo "❌❌ $(MSG_PREFIX)missing $$exp"; ko=$$((ko+1)); failed=1; return 1; fi; \
		while IFS= read -r needle || [ -n "$$needle" ]; do \
			case "$$needle" in ''|'#'*) continue ;; esac; \
			grep -Fq "$$needle" "$$s" > /dev/null || { echo "❌❌ $(MSG_PREFIX)$$lab missing rodata needle: $$needle"; r_ok=0; }; \
		done < "$$exp"; \
		if [ $$r_ok -eq 1 ]; then echo "✅✅ $(MSG_PREFIX)$$lab merged rodata (neighbor literals present)"; else ko=$$((ko+1)); failed=1; fi; \
	}; \
	check_needles "$(MAIN_BASE).sams" "$(MAIN_BASE).atom_rotext.expect" "$(MAIN_BASE)"; \
	check_needles "$(LIB_SAM)" lib/app_wide_atom_lib.atom_rotext.expect lib/app_wide_atom_lib; \
	if cmp -s "$(MAIN_BASE).sams" "$(LIB_SAM)"; then \
		echo "❌❌ $(MSG_PREFIX).sams are identical unexpectedly"; ko=$$((ko+1)); failed=1; \
	else echo "✅✅ $(MSG_PREFIX)main vs lib codegen differs"; fi; \
	if [ ! -f "$(MAIN_BASE).ascomp" ]; then echo "❌❌ $(MSG_PREFIX)$(MAIN_BASE) missing .ascomp"; ko=$$((ko+1)); failed=1; \
	elif ! diff -Bw -q "$(MAIN_BASE).sams" "$(MAIN_BASE).ascomp" > /dev/null 2>&1; then \
		echo "❌❌ $(MSG_PREFIX)$(MAIN_BASE) .sams differs from .ascomp"; diff -Bw "$(MAIN_BASE).sams" "$(MAIN_BASE).ascomp" || true; \
		ko=$$((ko+1)); failed=1; \
	else echo "✅✅ $(MSG_PREFIX)$(MAIN_BASE) assembly golden OK"; ok=$$((ok+1)); fi; \
	echo "Assembling $(MAIN_BASE).sams..."; \
	$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$(MAIN_BASE).sams" -o "$(MAIN_BASE).o" || { ko=$$((ko+1)); failed=1; }; \
	echo "Assembling $(LIB_SAM)..."; \
	$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$(LIB_SAM)" -o "$(LIB_O)" || { ko=$$((ko+1)); failed=1; }; \
	if [ "$$failed" -eq 0 ]; then \
		echo "Linking $(MAIN_BASE) (+ lib objs)..."; \
		if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o "$(MAIN_BASE)" "$(MAIN_BASE).o" "$(LIB_O)" $(LDFLAGS_rust-lld) || { ko=$$((ko+1)); failed=1; }; \
		else \
			$(ASSEMBLER) "$(MAIN_BASE).o" "$(LIB_O)" -o "$(MAIN_BASE)" $(LDFLAGS_clang) || { ko=$$((ko+1)); failed=1; }; \
		fi; \
	fi; \
	if [ "$$failed" -eq 0 ] && [ -x "$(MAIN_BASE)" ]; then \
		{ ./"$(MAIN_BASE)" 2>&1; echo $$?; } > "$(MAIN_BASE).sout"; \
		if [ ! -f "$(MAIN_BASE).scout" ]; then \
			echo "❌❌ $(MSG_PREFIX)$(MAIN_BASE) missing .scout"; ko=$$((ko+1)); failed=1; \
		elif ! diff -Bw -q "$(MAIN_BASE).sout" "$(MAIN_BASE).scout" > /dev/null 2>&1; then \
			echo "❌❌ $(MSG_PREFIX)$(MAIN_BASE) output differs"; diff -Bw "$(MAIN_BASE).sout" "$(MAIN_BASE).scout" || true; \
			ko=$$((ko+1)); failed=1; \
		else echo "✅✅ $(MSG_PREFIX)$(MAIN_BASE) run OK"; ok=$$((ok+1)); fi; \
	elif [ "$$failed" -eq 0 ]; then ko=$$((ko+1)); failed=1; fi; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko+1)); \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

clean:
	@cd "$(THIS_DIR)" && \
	find . \( -maxdepth 1 -name '*.sams' \) ! -path './.*' -delete 2>/dev/null ; \
	find ./lib \( -maxdepth 1 -name '*.sams' \) ! -path './.*' -delete 2>/dev/null ; \
	rm -f "$(MAIN_BASE).o" "$(MAIN_BASE)" "$(MAIN_BASE).sout" "$(LIB_O)" \
	.integrate_counts 2>/dev/null ; \
	true; \
	echo "✅ $(MSG_PREFIX)clean"

help:
	@echo "app_wide_atom trial: merged atom table inside one exe. Targets: integrate, clean."
