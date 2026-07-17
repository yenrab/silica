# Parallel self-host tree (`src_selfhost/`)
# Config-driven like trials/: generate silica.config, run seed host (no argv),
# then .sams → .o → link. Does not use silica-boot for Silica→assembly.
#
# Usage:
#   make                        # assembly + objects + link silica-compiler
#   make assembly               # seed compile only (.sams)
#   make TARGET=apple_silicon   # which emitter/* tree to include in the config

TARGET ?= apple_silicon

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER ?= $(abspath $(THIS_DIR)../src/silica-compiler)
SILICA_LINK_SH := $(abspath $(THIS_DIR)../trials/silica_link.sh)
STDLIB_DS := $(abspath $(THIS_DIR)../stdlib/data_structures)
BOOTSTRAP_DIR := $(abspath $(THIS_DIR)../../silica-bootstrap-compiler)
RUNTIME_LIB := $(BOOTSTRAP_DIR)/target/release/libsilica_compiler.a

EXECUTABLE := silica-compiler
ASSEMBLER := clang
MACOS_MIN_VERSION ?= 26.0
ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)
LINKER_CLANG := $(or $(LINKER_CLANG), $(shell test -x /opt/homebrew/opt/llvm/bin/clang && echo "/opt/homebrew/opt/llvm/bin/clang" || echo "clang"))
# Self-host lexer/parser recurse over whole files; raise main-thread stack (macOS default 8MB).
LDFLAGS_STACK := -Wl,-stack_size,0x10000000

# Stdlib modules pulled in by compiler_maps (staged under lib/ like trial leaves).
LIB_STDLIB_MODULES := wbt_map OrderedMap

.DEFAULT_GOAL := build

.PHONY: all clean build assembly objects executables lib-sources \
	silica.config silica.config.compiler assembly-selfhost build-selfhost help \
	check-seed

all: clean build

check-seed:
	@test -x "$(SILICA_COMPILER)" || (echo "Missing seed host: $(SILICA_COMPILER)"; echo "Build frozen src first: make -C $(abspath $(THIS_DIR)../src)"; exit 1)

# Stage stdlib WBT/OrderedMap sources into lib/ for config membership (module name = basename).
lib-sources:
	@mkdir -p "$(THIS_DIR)lib"
	@for mod in $(LIB_STDLIB_MODULES); do \
		src="$(STDLIB_DS)/$$mod.silica"; \
		dst="$(THIS_DIR)lib/$$mod.silica"; \
		if [ ! -f "$$src" ]; then echo "FAIL: missing $$src"; exit 1; fi; \
		cp "$$src" "$$dst"; \
	done

# Canonical batch list (§12). Seed reads the basename `silica.config`.
silica.config.compiler: lib-sources
	@echo "Regenerating silica.config.compiler (TARGET=$(TARGET))..."
	@cd "$(THIS_DIR)" && { \
		find . \( -path './_wd_probe' -o -path './emitter' \) -prune -o -type f -name '*.silica' -print; \
		find ./emitter -maxdepth 1 -type f -name '*.silica' -print 2>/dev/null; \
		find ./emitter/$(TARGET) -type f -name '*.silica' -print 2>/dev/null; \
		find ./lib -type f -name '*.silica' -print 2>/dev/null; \
	} | sed 's|^\./||' \
	  | grep -v '_struct_parse_test' \
	  | grep -v 'btree_set_nodeid\.silica' \
	  | sort -u > silica.config.compiler
	@test -s "$(THIS_DIR)silica.config.compiler" || (echo "FAIL: silica.config.compiler empty"; exit 1)
	@echo "  $$(wc -l < "$(THIS_DIR)silica.config.compiler" | tr -d ' ') units"

silica.config: silica.config.compiler
	@cp "$(THIS_DIR)silica.config.compiler" "$(THIS_DIR)silica.config"

# Stage 1: seed reads silica.config → .sams (and optional __silica_runtime.sams / silica.link)
assembly: check-seed silica.config
	@cd "$(THIS_DIR)" && \
	  find . -name '*.sams' -print | while read -r f; do rm -f "$$f"; done; \
	  rm -f __silica_runtime.sams silica.link; \
	  echo "Compiling with seed host: $(SILICA_COMPILER)"; \
	  if ! "$(SILICA_COMPILER)"; then \
	    echo "FAIL: seed compilation failed"; \
	    exit 1; \
	  fi; \
	  echo "✅ Assembly generated (.sams)"

# Stage 2: .sams → .o (clang assembler), including nested paths and lib/
objects: assembly
	@cd "$(THIS_DIR)" && \
	failed=0; \
	for sams in $$(find . -type f -name '*.sams' | sed 's|^\./||' | sort); do \
		[ -f "$$sams" ] || continue; \
		base=$${sams%.sams}; \
		echo "Assembling: $$sams -> $$base.o"; \
		$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$$sams" -o "$$base.o" || failed=1; \
	done; \
	exit $$failed

# Stage 3: link entry point `main` with all module objects + runtime
executables: objects
	@cd "$(THIS_DIR)" && \
	if [ ! -f main.o ]; then echo "FAIL: main.o missing"; exit 1; fi; \
	objs=$$(find . -type f -name '*.o' ! -path './_wd_probe/*' | sed 's|^\./||' | sort | tr '\n' ' '); \
	link_archives=""; \
	if [ -x "$(SILICA_LINK_SH)" ]; then link_archives=$$("$(SILICA_LINK_SH)" .); fi; \
	runtime_extra=""; \
	if [ -f "__silica_runtime.o" ]; then \
		runtime_extra=""; \
	elif [ -f "$(RUNTIME_LIB)" ]; then \
		runtime_extra="$(RUNTIME_LIB)"; \
	elif [ -d "$(BOOTSTRAP_DIR)" ]; then \
		echo "Building Silica runtime library (bootstrap)..."; \
		(cd "$(BOOTSTRAP_DIR)" && cargo build --release --no-default-features); \
		runtime_extra="$(RUNTIME_LIB)"; \
	fi; \
	echo "Linking: $(EXECUTABLE)"; \
	$(LINKER_CLANG) $$objs $$runtime_extra $$link_archives $(LDFLAGS_STACK) -o $(EXECUTABLE); \
	echo "✅ Built $(EXECUTABLE)"

build: executables

assembly-selfhost: assembly
build-selfhost: build

clean:
	@echo "Cleaning generated files..."
	@cd "$(THIS_DIR)" && \
	  find . \( -name '*.sams' -o -name '*.o' -o -name '*.ll' -o -name '*.bc' \) ! -path './_wd_probe/*' -delete; \
	  rm -f $(EXECUTABLE) silica.config silica.config.compiler silica.link __silica_runtime.sams __silica_runtime.o; \
	  rm -f lib/wbt_map.silica lib/OrderedMap.silica lib/wbt_map.sams lib/OrderedMap.sams lib/*.o 2>/dev/null || true
	@echo "✅ Clean complete"

help:
	@echo "src_selfhost — config-driven seed build"
	@echo "  Seed: $(SILICA_COMPILER)"
	@echo "  TARGET=$(TARGET) (emitter subtree included in silica.config.compiler)"
	@echo ""
	@echo "  make / build     - silica.config → seed → .sams → .o → $(EXECUTABLE)"
	@echo "  assembly         - seed compile only"
	@echo "  objects          - assemble .sams"
	@echo "  clean            - remove generated artifacts"
	@echo "  silica.config.compiler - regenerate unit list"
