# Makefile for Silica trials - integrate target compiles all .silica files,
# diffs .sams vs .ascomp, runs executables, and compares output to .scout files.
#
# Each subdir regenerates silica.config from: ls -1 *.silica | grep '\.silica$'
# To refresh configs without compiling: ./rebuild-silica-configs.sh
#
# integrate: compile all .silica in subdirs, diff .sams vs .ascomp, run each binary, diff .sout vs .scout
# clean: remove executables, .sams, .o (keeps .scout and .ascomp)

SILICA_COMPILER := ../src/silica-compiler

# Dynamically discover all immediate subdirectories (will pick up new trials as they are added)
SUBDIRS := $(sort $(dir $(wildcard */)))

# Rust toolchain linker (rust-lld) - optional override
RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)
RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")
RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld
LINKER := $(shell test -x "$(RUST_LLD)" && echo "$(RUST_LLD)" || echo "clang")
ASSEMBLER := clang

MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)
MACOS_MIN_VERSION ?= 26.0
ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)
LDFLAGS_rust-lld := -arch arm64 -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) -syslibroot $(MACOS_SDK) -lSystem -e main
LDFLAGS_clang := -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)

.PHONY: integrate clean help

.DEFAULT_GOAL := integrate

# integrate: compile all .silica in subdirs, diff .sams vs .ascomp, run each binary, diff .sout vs .scout
integrate:
	@failed=0; \
	for dir in $(SUBDIRS); do \
		dir=$${dir%/}; \
		echo "=== $$dir ==="; \
		if [ "$$dir" = "error_enforcement_addition" ]; then \
			$(MAKE) -C $$dir || failed=1; \
			continue; \
		fi; \
		if [ "$$dir" = "modules_addition" ] && [ -f "modules_addition/INTEGRATE_PENDING" ]; then \
			echo "SKIP: modules_addition (INTEGRATE_PENDING — see modules_addition/INTEGRATE_PENDING)"; \
			continue; \
		fi; \
		if [ "$$dir" = "modules_addition" ]; then \
			$(MAKE) -C $$dir integrate || failed=1; \
			continue; \
		fi; \
		if [ "$$dir" = "cpu_discovery" ] && [ -f "cpu_discovery/INTEGRATE_PENDING" ]; then \
			echo "SKIP: cpu_discovery (INTEGRATE_PENDING — see cpu_discovery/README.md)"; \
			continue; \
		fi; \
		cd $$dir && \
		ls -1 *.silica 2>/dev/null | grep '\.silica$$' > silica.config || true; \
		if [ ! -s silica.config ]; then \
			echo "SKIP: $$dir has no silica.config"; \
			cd ..; \
			continue; \
		fi; \
		rm -f *.sams; \
		echo "Compiling with silica-compiler..."; \
		if ! $(abspath $(SILICA_COMPILER)); then \
			echo "FAIL: Compilation failed in $$dir"; \
			failed=1; \
			cd ..; \
			continue; \
		fi; \
		for sams in *.sams; do \
			[ -f "$$sams" ] || continue; \
			base=$${sams%.sams}; \
			if [ ! -f "$$base.ascomp" ]; then \
				echo "❌❌ $$dir/$$base has no .ascomp file"; \
				failed=1; \
			elif ! diff -Bw -q "$$sams" "$$base.ascomp" > /dev/null 2>&1; then \
				echo "❌❌ $$dir/$$base .sams differs from .ascomp"; \
				diff -Bw "$$sams" "$$base.ascomp" || true; \
				failed=1; \
			else \
				echo "✅✅ $$dir/$$base assembly matches .ascomp"; \
			fi; \
		done; \
		for sams in *.sams; do \
			[ -f "$$sams" ] || continue; \
			base=$${sams%.sams}; \
			echo "Assembling $$sams..."; \
			$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler $$sams -o $$base.o || { failed=1; break; }; \
		done; \
		for o in *.o; do \
			[ -f "$$o" ] || continue; \
			base=$${o%.o}; \
			echo "Linking $$base..."; \
			if test -x "$(RUST_LLD)"; then \
				$(RUST_LLD) -flavor darwin -o $$base $$o $(LDFLAGS_rust-lld) || failed=1; \
			else \
				$(ASSEMBLER) $$o -o $$base $(LDFLAGS_clang) || failed=1; \
			fi; \
		done; \
		for exe in *.sams; do \
			[ -f "$$exe" ] || continue; \
			base=$${exe%.sams}; \
			if [ -x "$$base" ]; then \
				{ ./$$base 2>&1; echo $$?; } > "$$base.sout"; \
				if [ -f "$$base.scout" ]; then \
					if ! diff -Bw -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
						echo "❌❌ $$dir/$$base .sout differs from .scout"; \
						diff -Bw "$$base.sout" "$$base.scout" || true; \
						failed=1; \
					else \
						echo "✅✅ $$dir/$$base output matches .scout"; \
					fi; \
				else \
					echo "❌❌ $$dir/$$base has no .scout file"; \
					failed=1; \
				fi; \
			fi; \
		done; \
		cd ..; \
	done; \
	exit $$failed

# clean: remove executables, .sams, .o, .sout (not .scout, .ascomp)
clean:
	@for dir in $(SUBDIRS); do \
		dir=$${dir%/}; \
		echo "Cleaning $$dir..."; \
		if [ "$$dir" = "error_enforcement_addition" ]; then \
			$(MAKE) -C $$dir clean; \
			continue; \
		fi; \
		if [ "$$dir" = "modules_addition" ]; then \
			$(MAKE) -C $$dir clean; \
			continue; \
		fi; \
		cd $$dir && \
		rm -f *.sout && \
		for f in *.sams; do \
			[ -f "$$f" ] || continue; \
			base=$${f%.sams}; \
			rm -f "$$base" "$$base.o" "$$f"; \
		done && \
		cd ..; \
	done
	@echo "Clean complete"

help:
	@echo "Silica trials Makefile"
	@echo "====================="
	@echo ""
	@echo "Targets:"
	@echo "  integrate  - Regenerate silica.config (ls *.silica), compile, diff .sams vs .ascomp,"
	@echo "               run each binary, diff .sout vs .scout (report failures)"
	@echo "  clean      - Remove executables, .sams, .o, .sout (keeps .scout, .ascomp)"
	@echo "  help       - Show this help"
	@echo ""
	@echo "Also: ./rebuild-silica-configs.sh — write silica.config in each subdir without compiling."
	@echo ""
	@echo "Subdirs: $(SUBDIRS)"
