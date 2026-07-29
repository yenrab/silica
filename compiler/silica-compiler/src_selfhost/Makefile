# Parallel self-host tree (`src_selfhost/`)
# Config-driven like trials/: generate silica.config, run seed host (no argv),
# then .sams → .o → link. Does not use silica-boot for Silica→assembly.
#
# Emit target (TARGET) selects emitter/<TARGET>/ for silica.config and writes
# silica.target. Defaults to the current host platform when unset.
#
# Usage:
#   make help                   # descriptive help (project --help equivalent)
#   make                        # assembly + objects + link silica-compiler
#   make assembly               # seed compile only (.sams)
#   make TARGET=apple_silicon_mac   # explicit emit / emitter backend

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
include $(THIS_DIR)emit_target.mk

BINARIES_DIR := $(abspath $(THIS_DIR)../../../binaries)
SILICA_COMPILER ?= $(BINARIES_DIR)/silica-compiler
UPDATE_SILICA_COMPILER_LINK := $(BINARIES_DIR)/update_silica_compiler_link.bash
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
	silica.config silica.config.compiler assembly-selfhost build-selfhost \
	help --help all-targets check-seed record-compile-start

# Wall-clock compile timing: record epoch seconds at compile start; report
# elapsed time on success or on compilation / emitter / linker failure.
COMPILE_START_FILE := $(THIS_DIR).compile_start

record-compile-start:
	@date +%s > "$(COMPILE_START_FILE)"

define REPORT_COMPILE_TIME
	if [ -f "$(COMPILE_START_FILE)" ]; then \
		_start=$$(cat "$(COMPILE_START_FILE)"); \
		_end=$$(date +%s); \
		_elapsed=$$((_end - _start)); \
		if [ "$$_elapsed" -ge 60 ]; then \
			printf 'Compile time: %dm %02ds\n' $$((_elapsed / 60)) $$((_elapsed % 60)); \
		else \
			printf 'Compile time: %ds\n' "$$_elapsed"; \
		fi; \
		rm -f "$(COMPILE_START_FILE)"; \
	fi
endef

# Report elapsed time only when this stage is a top-level goal (not an intermediate).
# Empty MAKECMDGOALS means the default goal `build`.
define MAYBE_REPORT_COMPILE_TIME_SUCCESS
	case " $(if $(MAKECMDGOALS),$(MAKECMDGOALS),build) " in \
	*" $(1) "*|*" $(1)-selfhost "*) $(REPORT_COMPILE_TIME) ;; \
	esac
endef

# Completion chime for this root Makefile only (not the compiler, not subdir Makefiles).
# Tone: Ping — distinct from src (Glass).
define SELFHOST_COMPLETION_BING
	if command -v afplay >/dev/null 2>&1 && [ -f /System/Library/Sounds/Ping.aiff ]; then \
		afplay /System/Library/Sounds/Ping.aiff >/dev/null 2>&1 || true; \
	else \
		printf '\a'; \
	fi
endef

all: clean build

check-seed:
	@if [ ! -x "$(SILICA_COMPILER)" ]; then \
		echo "Missing $(SILICA_COMPILER); running update_silica_compiler_link.bash..."; \
		"$(UPDATE_SILICA_COMPILER_LINK)" || exit 1; \
		if [ ! -x "$(SILICA_COMPILER)" ]; then \
			echo "Missing seed host after update: $(SILICA_COMPILER)"; \
			exit 1; \
		fi; \
	fi

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
# Append constraint_extract_* impl units in dependency order so plain sort does
# not place them before parser_tuples / other parser deps.
# Reorder emitter prims with cross-module `use` deps (record → tuple → memory/list/actors).
silica.config.compiler: check-target silica.target lib-sources
	@echo "Regenerating silica.config.compiler (TARGET=$(TARGET))..."
	@cd "$(THIS_DIR)" && { \
		find . \( -path './_wd_probe' -o -path './emitter' \) -prune -o -type f -name '*.silica' -print; \
		find ./emitter -maxdepth 1 -type f -name '*.silica' -print 2>/dev/null; \
		find ./emitter/$(TARGET) -type f -name '*.silica' -print 2>/dev/null; \
		find ./lib -type f -name '*.silica' -print 2>/dev/null; \
	} | sed 's|^\./||' \
	  | grep -v '_struct_parse_test' \
	  | grep -v 'btree_set_nodeid\.silica' \
	  | sort -u > silica.config.compiler.raw
	@cd "$(THIS_DIR)" && \
	  grep -v '^parser/constraint_extract_' silica.config.compiler.raw \
	    | grep -Ev '^emitter/[^/]+/terms/prims/prims(_actors_child_table_asm|_actors_runtime_asm|_record|_tuple|_memory|_list|_actors)?\.silica$$' \
	    > silica.config.compiler && \
	  awk -v target="$(TARGET)" ' \
	    $$0 == "emitter/" target "/terms/prims/prims_atom.silica" && !inserted { \
	      print "emitter/" target "/terms/prims/prims_record.silica"; \
	      print "emitter/" target "/terms/prims/prims_tuple.silica"; \
	      print "emitter/" target "/terms/prims/prims_memory.silica"; \
	      print "emitter/" target "/terms/prims/prims_list.silica"; \
	      print "emitter/" target "/terms/prims/prims_actors.silica"; \
	      print "emitter/" target "/terms/prims/prims_actors_child_table_asm.silica"; \
	      print "emitter/" target "/terms/prims/prims_actors_runtime_asm.silica"; \
	      inserted = 1; \
	    } \
	    $$0 ~ ("^emitter/" target "/terms/prims/prims_uint64\\.silica$$") { \
	      print; \
	      print "emitter/" target "/terms/prims/prims.silica"; \
	      next; \
	    } \
	    { print } \
	  ' silica.config.compiler > silica.config.compiler.prims && \
	  mv silica.config.compiler.prims silica.config.compiler && \
	  for u in \
	    parser/constraint_extract_slots.silica \
	    parser/constraint_extract_params.silica \
	    parser/constraint_extract_expr.silica \
	    parser/constraint_extract_decls.silica \
	    parser/constraint_extract_lambda_lift.silica \
	    parser/constraint_extract_hof_wrap.silica \
	    parser/constraint_extract_validate.silica \
	  ; do \
	    grep -qx "$$u" silica.config.compiler.raw && echo "$$u" >> silica.config.compiler; \
	  done && \
	  rm -f silica.config.compiler.raw
	@test -s "$(THIS_DIR)silica.config.compiler" || (echo "FAIL: silica.config.compiler empty"; exit 1)
	@test -n "$$(find "$(THIS_DIR)emitter/$(TARGET)" -type f -name '*.silica' 2>/dev/null | head -n 1)" \
		|| (echo "FAIL: no .silica files under emitter/$(TARGET)/"; exit 1)
	@echo "  $$(wc -l < "$(THIS_DIR)silica.config.compiler" | tr -d ' ') units (emit target: $(TARGET))"

silica.config: silica.config.compiler
	@cp "$(THIS_DIR)silica.config.compiler" "$(THIS_DIR)silica.config"

# Stage 1: seed reads silica.config → .sams (and optional __silica_runtime.sams / silica.link).
# Multi-unit batches use process-per-unit hygiene: seed exits 75 between units so the OS
# reclaims host heap (required for large modules such as lib/wbt_map.silica).
# Seed failures cover compilation and emitter errors.
assembly: check-seed silica.config record-compile-start
	@cd "$(THIS_DIR)" && \
	  find . -name '*.sams' -print | while read -r f; do rm -f "$$f"; done; \
	  rm -f __silica_runtime.sams silica.link silica.compile.order silica.needs_runtime; \
	  echo "Compiling with seed host: $(SILICA_COMPILER)"; \
	  echo "Emit target: $(TARGET) (see silica.target)"; \
	  while true; do \
	    "$(SILICA_COMPILER)"; \
	    ec=$$?; \
	    if [ $$ec -eq 0 ]; then break; fi; \
	    if [ $$ec -eq 75 ]; then \
	      echo "  (reclaiming memory; continuing next unit)"; \
	      continue; \
	    fi; \
	    echo "FAIL: seed compilation failed (exit $$ec)"; \
	    $(REPORT_COMPILE_TIME); \
	    exit 1; \
	  done; \
	  echo "✅ Assembly generated (.sams)"; \
	  $(call MAYBE_REPORT_COMPILE_TIME_SUCCESS,assembly)

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
	if [ $$failed -ne 0 ]; then \
		$(REPORT_COMPILE_TIME); \
		exit 1; \
	fi; \
	$(call MAYBE_REPORT_COMPILE_TIME_SUCCESS,objects)

# Stage 3: link entry point `main` with all module objects + runtime
executables: objects
	@cd "$(THIS_DIR)" && \
	if [ ! -f main.o ]; then echo "FAIL: main.o missing"; $(REPORT_COMPILE_TIME); exit 1; fi; \
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
	if ! $(LINKER_CLANG) $$objs $$runtime_extra $$link_archives $(LDFLAGS_STACK) -o $(EXECUTABLE); then \
		echo "FAIL: linker failed"; \
		$(REPORT_COMPILE_TIME); \
		exit 1; \
	fi; \
	echo "✅ Built $(EXECUTABLE)"; \
	$(call MAYBE_REPORT_COMPILE_TIME_SUCCESS,executables)

build: executables
	@$(REPORT_COMPILE_TIME)
	@$(SELFHOST_COMPLETION_BING)

assembly-selfhost: assembly
build-selfhost: build

# Build once per allowable emit target (distinct binaries). Requires an
# emitter/<name>/ tree for each entry in ALLOWED_TARGETS.
all-targets:
	@if [ -z "$(ALLOWED_TARGETS)" ]; then \
		echo "FAIL: no emitter/*/ targets found"; \
		exit 1; \
	fi
	@for t in $(ALLOWED_TARGETS); do \
		echo ""; \
		echo "======== all-targets: TARGET=$$t ========"; \
		$(MAKE) -C "$(THIS_DIR)" clean build TARGET=$$t EXECUTABLE=silica-compiler-$$t || exit 1; \
	done
	@echo ""
	@echo "✅ all-targets complete: $(ALLOWED_TARGETS)"

clean:
	@echo "Cleaning generated files (including *.iface and .cache/ifaces)..."
	@cd "$(THIS_DIR)" && \
	  find . \( -name '*.sams' -o -name '*.o' -o -name '*.ll' -o -name '*.bc' \) ! -path './_wd_probe/*' -delete; \
	  rm -f $(EXECUTABLE) silica.config silica.config.compiler silica.target silica.link \
	    __silica_runtime.sams __silica_runtime.o "$(COMPILE_START_FILE)"; \
	  rm -f silica-compiler-* 2>/dev/null || true; \
	  rm -f lib/wbt_map.silica lib/OrderedMap.silica lib/wbt_map.sams lib/OrderedMap.sams lib/*.o 2>/dev/null || true; \
	  rm -rf .cache/ifaces; \
	  find . -name '*.iface' -type f ! -path './_wd_probe/*' -delete 2>/dev/null || true
	@echo "✅ Clean complete"

# Project help. Prefer: make help
# Note: plain `make --help` is GNU Make's own help; use `make help` or `make -- --help`.
--help: help
help:
	@echo "src_selfhost — config-driven seed build for silica-compiler"
	@echo ""
	@echo "Usage:"
	@echo "  make [target] [TARGET=<emit-target>] [EXECUTABLE=<name>]"
	@echo "  make help"
	@echo "  make -- --help"
	@echo ""
	@echo "Emit target (TARGET)"
	@echo "  Selects which emitter/<TARGET>/ backend is included in silica.config"
	@echo "  and writes ./silica.target (emit_target: <TARGET>)."
	@echo "  This is the code-generation / emit backend baked into this build —"
	@echo "  not a runtime cross-compile switch inside an already-built binary."
	@echo "  It is also not the local host binary tag used by binaries/"
	@echo "  (e.g. macos-applesilicon); those name where the compiler runs."
	@echo ""
	@echo "  Default: current host platform when TARGET is unset."
	@echo "  Host:    $(HOST_UNAME_S) $(HOST_UNAME_M)"
	@echo "  Active:  TARGET=$(TARGET)  (source: $(TARGET_ORIGIN))"
	@echo "  File:    silica.target → emit_target: $(TARGET)"
	@echo ""
	@echo "  Allowable emit targets (from emitter/*/):"
	@for t in $(ALLOWED_TARGETS); do \
		if [ "$$t" = "$(TARGET)" ]; then echo "    * $$t  (selected)"; \
		else echo "      $$t"; fi; \
	done
	@if [ -z "$(ALLOWED_TARGETS)" ]; then echo "      (none found)"; fi
	@echo ""
	@echo "  Examples:"
	@echo "    make                          # host-default TARGET + full build"
	@echo "    make TARGET=apple_silicon_mac     # explicit emit backend"
	@echo "    make assembly TARGET=apple_silicon_mac"
	@echo "    make all-targets              # loop clean/build for each allowable TARGET"
	@echo ""
	@echo "Build targets"
	@echo "  make / build              silica.config → seed → .sams → .o → $(EXECUTABLE)"
	@echo "  assembly                  seed compile only (.sams)"
	@echo "  objects                   assemble .sams → .o"
	@echo "  executables               link $(EXECUTABLE)"
	@echo "  silica.config.compiler    regenerate unit list for TARGET"
	@echo "  silica.target             rewrite emit_target declaration"
	@echo "  all-targets               build silica-compiler-<TARGET> for each allowable TARGET"
	@echo "  clean                     remove generated artifacts (*.sams/*.o, silica.target, *.iface, .cache/ifaces)"
	@echo "  check-target              validate TARGET against emitter/*/"
	@echo "  check-seed                ensure binaries/silica-compiler exists"
	@echo ""
	@echo "Paths"
	@echo "  Seed compiler:  $(SILICA_COMPILER)"
	@echo "  Emitter root:   $(EMITTER_ROOT)"
	@echo "  This tree:      $(THIS_DIR)"
