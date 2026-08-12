# Makefile for Silica experiments in codegen_regressions
# Uses silica-compiler (Phase 2) to generate assembly, then assembles and links
# No Rust runtime — string helpers are emitted as AArch64 in the compiler output.
#
# silica-compiler reads silica.config and outputs .sams (assembly) files.
# Assembler: clang -c (Rust toolchain does not ship a standalone assembler)
# Linker: rust-lld if available, else clang

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
include $(THIS_DIR)../silica_compiler.mk

TRIAL_DIR := $(notdir $(patsubst %/,%,$(THIS_DIR)))
MSG_PREFIX := $(TRIAL_DIR)/
RUN_TIMEOUT_SECS := 5

# Rust toolchain linker (rust-lld) - optional override
RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)
RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")
RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld

# Use rust-lld if it exists, otherwise clang for linking
LINKER := $(shell test -x "$(RUST_LLD)" && echo "$(RUST_LLD)" || echo "clang")
ASSEMBLER := clang

# macOS: linker flags for minimal executable (when using rust-lld)
MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)
MACOS_MIN_VERSION := 26.0
ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)
LDFLAGS_rust-lld := -arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) -syslibroot $(MACOS_SDK) -lSystem -e main
LDFLAGS_clang := -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)

.DEFAULT_GOAL := all
.PHONY: all clean assembly objects executables integrate help silica.config

all: executables

# Stage 1: Rebuild silica.config (all *.silica under this directory, relative paths)
silica.config:
	@cd "$(THIS_DIR)" && find "$$PWD" -type f -name '*.silica' | sed "s|^$$PWD/||" > silica.config

# Stage 2: silica-compiler generates .sams (reads silica.config)
assembly: clean silica.config
	@echo "Compiling with silica-compiler..."
	@rm -f *.sams silica.compile.order silica.needs_runtime silica.link __silica_runtime.sams; \
	$(RUN_SILICA_COMPILER)
	@echo "✅ $(MSG_PREFIX)Assembly generated"

# Stage 3: Assemble .sams -> .o (no Rust code, clang assembler only)
objects: assembly
	@for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
	base=$${sams%.sams}; \
	echo "Assembling: $$sams -> $$base.o"; \
	$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler $$sams -o $$base.o; \
	done

# Stage 4: Link .o -> executable (rust-lld or clang)
executables: objects
	@for o in *.o; do \
	[ -f "$$o" ] || continue; \
	base=$${o%.o}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
	if test -x "$(RUST_LLD)"; then \
			$(RUST_LLD) -flavor darwin -o $$base $$o $$runtime_obj $(LDFLAGS_rust-lld) && echo "  ✅ $(MSG_PREFIX)$$base (rust-lld)"; \
		else \
			$(ASSEMBLER) $$o $$runtime_obj -o $$base $(LDFLAGS_clang) && echo "  ✅ $(MSG_PREFIX)$$base (clang)"; \
		fi; \
	done

# integrate: compile .silica per silica.config; diff .sams vs .ascomp, run, diff .sout vs .scout.
integrate:
	@$(MAKE) -f "$(THIS_DIR)Makefile" clean
	@$(MAKE) -f "$(THIS_DIR)Makefile" silica.config
	@cd "$(THIS_DIR)" || exit 1; \
	rm -f .integrate_counts; ok=0; ko=0; failed=0; \
	if [ ! -s silica.config ]; then \
		echo "❌❌ $(MSG_PREFIX)silica.config missing after generation"; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi; \
	rm -f *.sams silica.compile.order silica.needs_runtime silica.link __silica_runtime.sams; \
	echo "Compiling with silica-compiler..."; \
	if ! { $(RUN_SILICA_COMPILER); }; then \
		echo "❌❌ $(MSG_PREFIX)compilation failed"; \
		printf '%d %d\n' 0 1 > .integrate_counts; \
		exit 1; \
	fi; \
	for sams in *.sams; do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		if [ ! -f "$$base.ascomp" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base has no .ascomp file"; \
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
		echo "Assembling $$sams..."; \
		if ! $(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler $$sams -o $$base.o; then \
			echo "❌❌ $(MSG_PREFIX)$$base assemble failed"; \
			ko=$$((ko + 1)); failed=1; break; \
		fi; \
	done; \
	for o in *.o; do \
		[ -f "$$o" ] || continue; \
		base=$${o%.o}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		runtime_obj=""; \
		[ -f "__silica_runtime.o" ] && runtime_obj="__silica_runtime.o"; \
		echo "Linking $$base..."; \
		if test -x "$(RUST_LLD)"; then \
			if ! $(RUST_LLD) -flavor darwin -o $$base $$o $$runtime_obj $(LDFLAGS_rust-lld); then \
				echo "❌❌ $(MSG_PREFIX)$$base link failed"; \
				ko=$$((ko + 1)); failed=1; \
			fi; \
		else \
			if ! $(ASSEMBLER) $$o $$runtime_obj -o $$base $(LDFLAGS_clang); then \
				echo "❌❌ $(MSG_PREFIX)$$base link failed"; \
				ko=$$((ko + 1)); failed=1; \
			fi; \
		fi; \
	done; \
	for exe in *.sams; do \
		[ -f "$$exe" ] || continue; \
		base=$${exe%.sams}; \
		[ "$$base" = "__silica_runtime" ] && continue; \
		if [ ! -x "$$base" ]; then \
			echo "❌❌ $(MSG_PREFIX)$$base missing executable (assemble/link failed earlier)"; \
			ko=$$((ko + 1)); failed=1; \
		elif [ -x "$$base" ]; then \
			./$$base >"$$base.sout.tmp" 2>&1 & \
			pid=$$!; \
			timed_out=0; \
			elapsed=0; \
			while kill -0 $$pid 2>/dev/null; do \
				if [ "$$elapsed" -ge "$(RUN_TIMEOUT_SECS)" ]; then \
					kill -9 $$pid 2>/dev/null || true; \
					wait $$pid 2>/dev/null || true; \
					timed_out=1; \
					break; \
				fi; \
				sleep 1; \
				elapsed=$$((elapsed + 1)); \
			done; \
			if [ "$$timed_out" -eq 0 ]; then \
				wait $$pid; \
				rc=$$?; \
			fi; \
			if [ "$$timed_out" -eq 1 ]; then \
				echo "TIMEOUT" > "$$base.sout"; \
				echo "❌❌ $(MSG_PREFIX)$$base timed out ($(RUN_TIMEOUT_SECS)s)"; \
				ko=$$((ko + 1)); failed=1; \
			else \
				{ cat "$$base.sout.tmp"; echo "$$rc"; } > "$$base.sout"; \
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
					echo "❌❌ $(MSG_PREFIX)$$base has no .scout file"; \
					ko=$$((ko + 1)); failed=1; \
				fi; \
			fi; \
			rm -f "$$base.sout.tmp"; \
		fi; \
	done; \
	[ "$$failed" -ne 0 ] && [ "$$ko" -eq 0 ] && ko=$$((ko + 1)); \
	printf '%d %d\n' "$$ok" "$$ko" > .integrate_counts; \
	exit $$failed

clean:
	@for s in *.sams; do \
		[ -f "$$s" ] || continue; \
		rm -f $${s%.sams}; \
	done
	rm -f *.sams *.o silica.config silica.compile.order silica.needs_runtime silica.link .integrate_counts
	@echo "✅ $(MSG_PREFIX)Clean complete"

help:
	@echo "Makefile - Silica assembly build (no Rust code)"
	@echo "=============================================="
	@echo "Uses: silica-compiler -> .sams -> .o -> executable"
	@echo ""
	@echo "Targets:"
	@echo "  all         - Build executables (default)"
	@echo "  assembly    - Run silica-compiler (generates .sams)"
	@echo "  objects     - Assemble .sams to .o"
	@echo "  executables - Full build: assembly + link"
	@echo "  integrate   - Compile; diff .sams vs .ascomp, run, diff .sout vs .scout"
	@echo "  clean       - Remove .sams, .o, executables"
	@echo "  help        - Show this help"
	@echo ""
	@echo "Config: silica.config (auto-generated from all .silica files in this directory)"
	@echo "Linker: $(LINKER)"
