# Makefile for Silica trials - integrate target compiles all .silica files,
# diffs .sams vs .ascomp, runs executables, and compares output to .scout files.
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
		cd $$dir && \
		if [ ! -f silica.config ]; then \
			ls -1 *.silica 2>/dev/null | grep '\.silica$$' > silica.config || true; \
		fi; \
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
			elif ! diff -q "$$sams" "$$base.ascomp" > /dev/null 2>&1; then \
				echo "❌❌ $$dir/$$base .sams differs from .ascomp"; \
				diff "$$sams" "$$base.ascomp" || true; \
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
				echo "Running $$dir/$$base..."; \
				{ ./$$base 2>&1; echo $$?; } > "$$base.sout"; \
				if [ -f "$$base.scout" ]; then \
					if ! diff -q "$$base.sout" "$$base.scout" > /dev/null 2>&1; then \
						echo "❌❌ $$dir/$$base .sout differs from .scout"; \
						diff "$$base.sout" "$$base.scout" || true; \
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
	@echo "  integrate  - Compile all .silica in subdirs, diff .sams vs .ascomp,"
	@echo "               run each binary, diff .sout vs .scout (report failures)"
	@echo "  clean      - Remove executables, .sams, .o, .sout (keeps .scout, .ascomp)"
	@echo "  help       - Show this help"
	@echo ""
	@echo "Subdirs: $(SUBDIRS)"
