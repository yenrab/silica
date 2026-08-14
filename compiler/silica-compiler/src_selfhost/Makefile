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
# seed-compiler, not silica-compiler: this tree must be built by a known-good seed, and
# silica-compiler tracks the newest build, which may be a selfhost that cannot build itself.
SILICA_COMPILER ?= $(BINARIES_DIR)/seed-compiler
SILICA_LINK_SH := $(abspath $(THIS_DIR)../trials/silica_link.sh)
STDLIB_DS := $(abspath $(THIS_DIR)../stdlib/data_structures)
BOOTSTRAP_DIR := $(abspath $(THIS_DIR)../../silica-bootstrap-compiler)
RUNTIME_LIB := $(BOOTSTRAP_DIR)/target/release/libsilica_compiler.a

EXECUTABLE := silica-compiler

# A successful build installs itself as the new selfhost: binaries/silica-<NNNNNN>-<platform>
# with binaries/silica-compiler pointing at it, which is what compiles base and the trials.
# Only the default EXECUTABLE is published; the per-emit-target builds from `make all-targets`
# produce silica-compiler-<target> binaries that do not run on this host.
# Set INSTALL_SELFHOST=0 to build without publishing.
INSTALL_COMPILER_SH := $(BINARIES_DIR)/install_compiler.bash
INSTALL_SELFHOST ?= 1

ASSEMBLER := clang
MACOS_MIN_VERSION ?= 26.0
ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)
LINKER_CLANG := $(or $(LINKER_CLANG), $(shell test -x /opt/homebrew/opt/llvm/bin/clang && echo "/opt/homebrew/opt/llvm/bin/clang" || echo "clang"))
# Self-host lexer/parser recurse over whole files; raise main-thread stack (macOS default 8MB).
LDFLAGS_STACK := -Wl,-stack_size,0x10000000

# Parallel .sams → .o assembly (Stage 2). Silica emission (Stage 1) stays serial
# (leaf-to-root + process-per-unit). Override: make build JOBS=4
JOBS ?= $(shell sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4)

# Stdlib modules pulled in by compiler_maps (staged under lib/ like trial leaves).
LIB_STDLIB_MODULES := wbt_map OrderedMap

.DEFAULT_GOAL := build

.PHONY: all clean build assembly objects executables lib-sources \
	silica.config.regen assembly-selfhost build-selfhost \
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
		echo "Missing seed host: $(SILICA_COMPILER)"; \
		echo "Link it to a known-good seed, newest first (lowest NNNNNN):"; \
		echo "  (cd $(BINARIES_DIR) && ln -sfn silica-<NNNNNN>-seed-<platform> seed-compiler)"; \
		echo "Available seeds:"; \
		ls "$(BINARIES_DIR)" | grep -E '^silica-[0-9]{6}-seed-[a-z0-9-]+$$' | sort | sed 's/^/  /' || true; \
		exit 1; \
	fi

# Stage stdlib WBT/OrderedMap sources into lib/ for config membership (module name = basename).
LIB_STAGED := $(addprefix $(THIS_DIR)lib/,$(addsuffix .silica,$(LIB_STDLIB_MODULES)))

$(THIS_DIR)lib/%.silica: $(STDLIB_DS)/%.silica
	@mkdir -p "$(THIS_DIR)lib"
	@if [ ! -f "$<" ]; then echo "FAIL: missing $<"; exit 1; fi
	@cp "$<" "$@"

lib-sources: $(LIB_STAGED)

# Canonical batch list (§12). Seed reads the basename `silica.config`.
# Order is a topological sort of module-level `use` edges (topo_silica_config.sh);
# main.silica is forced last. Regenerates only when membership, a unit, TARGET,
# or the topo script changes — not on every make.
TOPO_SILICA_CONFIG := $(THIS_DIR)topo_silica_config.sh
SILICA_CONFIG_UNITS_FILE := $(THIS_DIR).silica.config.units

# Detect add/delete: refresh membership list at parse time; mtime bumps only on change.
$(shell \
	if [ -x "$(TOPO_SILICA_CONFIG)" ]; then \
		"$(TOPO_SILICA_CONFIG)" --list-units "$(TARGET)" > "$(SILICA_CONFIG_UNITS_FILE).tmp" 2>/dev/null || true; \
		if [ -f "$(SILICA_CONFIG_UNITS_FILE).tmp" ]; then \
			if ! cmp -s "$(SILICA_CONFIG_UNITS_FILE).tmp" "$(SILICA_CONFIG_UNITS_FILE)" 2>/dev/null; then \
				mv "$(SILICA_CONFIG_UNITS_FILE).tmp" "$(SILICA_CONFIG_UNITS_FILE)"; \
			else \
				rm -f "$(SILICA_CONFIG_UNITS_FILE).tmp"; \
			fi; \
		fi; \
	fi \
)

SILICA_CONFIG_UNITS := $(addprefix $(THIS_DIR),$(shell cat "$(SILICA_CONFIG_UNITS_FILE)" 2>/dev/null))

# Membership list (real file). Built when missing; parse-time shell above refreshes on add/delete.
$(SILICA_CONFIG_UNITS_FILE): $(TOPO_SILICA_CONFIG) $(LIB_STAGED) silica.target
	@"$(TOPO_SILICA_CONFIG)" --list-units "$(TARGET)" > "$@"

silica.config.compiler: silica.target $(TOPO_SILICA_CONFIG) $(LIB_STAGED) \
		$(SILICA_CONFIG_UNITS_FILE) $(SILICA_CONFIG_UNITS)
	@echo "Regenerating silica.config.compiler (TARGET=$(TARGET))..."
	@cd "$(THIS_DIR)" && "$(TOPO_SILICA_CONFIG)" --list-units "$(TARGET)" > "$(SILICA_CONFIG_UNITS_FILE)"
	@cd "$(THIS_DIR)" && "$(TOPO_SILICA_CONFIG)" "$(TARGET)" > silica.config.compiler.tmp
	@mv "$(THIS_DIR)silica.config.compiler.tmp" "$(THIS_DIR)silica.config.compiler"
	@test -s "$(THIS_DIR)silica.config.compiler" || (echo "FAIL: silica.config.compiler empty"; exit 1)
	@test -n "$$(find "$(THIS_DIR)emitter/$(TARGET)" -type f -name '*.silica' 2>/dev/null | head -n 1)" \
		|| (echo "FAIL: no .silica files under emitter/$(TARGET)/"; exit 1)
	@test "$$(tail -n 1 "$(THIS_DIR)silica.config.compiler")" = "main.silica" \
		|| (echo "FAIL: main.silica must be last in silica.config.compiler (mangler/other deps first)"; exit 1)
	@echo "  $$(wc -l < "$(THIS_DIR)silica.config.compiler" | tr -d ' ') units (emit target: $(TARGET))"

silica.config: silica.config.compiler
	@cmp -s "$(THIS_DIR)silica.config.compiler" "$(THIS_DIR)silica.config" 2>/dev/null \
		|| cp "$(THIS_DIR)silica.config.compiler" "$(THIS_DIR)silica.config"

# Force rebuild of the topo-sorted unit list (ignores incremental timestamps).
silica.config.regen:
	@rm -f "$(THIS_DIR)silica.config.compiler" "$(THIS_DIR)silica.config" "$(SILICA_CONFIG_UNITS_FILE)"
	@$(MAKE) -f "$(THIS_DIR)Makefile" silica.config TARGET="$(TARGET)"

# Stage 1: seed reads silica.config → .sams (and optional __silica_runtime.sams / silica.link).
# Multi-unit batches use process-per-unit hygiene: seed exits 75 between units so the OS
# reclaims host heap (required for large modules such as lib/wbt_map.silica).
# Seed failures cover compilation and emitter errors.
#
# Incremental: keep .sams / .iface whose mtime is not older than their .silica source AND not
# older than the seed that produced them. If either artifact is missing or outdated, remove both
# and recompile that unit. Orphan .sams/.iface (no matching .silica) are removed.
# silica.compile.order lists only units that still need a .sams after this prune.
#
# The seed is a staleness input because it decides layout, not just instruction selection: a seed
# fix that moves a record field affects every unit it compiles, so keeping .sams built by an older
# seed links writers using the old offsets against readers using the new ones. That is a silent
# wrong-answer build, and it is exactly what a record-field-offset fix produced here -- the emitter
# modules were recompiled and the rest were not. Comparing against $(SILICA_COMPILER) follows the
# seed-compiler symlink to the binary it currently names, so publishing a new seed invalidates the
# tree on the next build without anyone having to remember to clean.
assembly: check-seed silica.config record-compile-start
	@cd "$(THIS_DIR)" && \
	  SEED_BIN="$(SILICA_COMPILER)"; \
	  removed=0; \
	  for art in $$(find . \( -name '*.sams' -o -name '*.iface' \) ! -path './_wd_probe/*' -type f | sed 's|^\./||' | sort); do \
	    case "$$art" in \
	      __silica_runtime.sams) continue ;; \
	    esac; \
	    case "$$art" in \
	      *.sams) src="$${art%.sams}.silica" ;; \
	      *.iface) src="$${art%.iface}.silica" ;; \
	      *) continue ;; \
	    esac; \
	    if [ ! -f "$$src" ]; then \
	      echo "  orphan: $$art"; \
	      rm -f "$$art"; \
	      removed=$$((removed + 1)); \
	    fi; \
	  done; \
	  while IFS= read -r unit || [ -n "$$unit" ]; do \
	    [ -z "$$unit" ] && continue; \
	    case "$$unit" in \#*) continue ;; esac; \
	    [ -f "$$unit" ] || continue; \
	    base=$${unit%.silica}; \
	    sams="$$base.sams"; \
	    iface="$$base.iface"; \
	    stale=0; \
	    reason="$$unit"; \
	    if [ ! -f "$$sams" ] || [ "$$unit" -nt "$$sams" ]; then stale=1; fi; \
	    if [ ! -f "$$iface" ] || [ "$$unit" -nt "$$iface" ]; then stale=1; fi; \
	    if [ "$$SEED_BIN" -nt "$$sams" ] || [ "$$SEED_BIN" -nt "$$iface" ]; then stale=1; reason="$$SEED_BIN"; fi; \
	    if [ "$$stale" -eq 1 ]; then \
	      if [ -f "$$sams" ] || [ -f "$$iface" ]; then \
	        echo "  stale: $$base.{sams,iface} (vs $$reason)"; \
	      fi; \
	      rm -f "$$sams" "$$iface"; \
	      removed=$$((removed + 1)); \
	    fi; \
	  done < silica.config; \
	  echo "Invalidated $$removed unit artifact set(s)"; \
	  rm -f __silica_runtime.sams silica.link silica.needs_runtime; \
	  : > silica.compile.order; \
	  while IFS= read -r unit || [ -n "$$unit" ]; do \
	    [ -z "$$unit" ] && continue; \
	    case "$$unit" in \#*) continue ;; esac; \
	    base=$${unit%.silica}; \
	    if [ ! -f "$$base.sams" ]; then \
	      echo "$$unit" >> silica.compile.order; \
	    fi; \
	  done < silica.config; \
	  need=$$(wc -l < silica.compile.order | tr -d ' '); \
	  if [ "$$need" -eq 0 ]; then \
	    rm -f silica.compile.order; \
	    echo "All units up to date (.sams/.iface newer than both .silica and the seed); skipping seed"; \
	    echo "✅ Assembly generated (.sams)"; \
	    $(call MAYBE_REPORT_COMPILE_TIME_SUCCESS,assembly); \
	    exit 0; \
	  fi; \
	  echo "Compiling with seed host: $(SILICA_COMPILER) ($$need unit(s) to compile)"; \
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

# Stage 2: .sams → .o (clang assembler), including nested paths and lib/.
# Invoked in parallel via sub-make -j$(JOBS) after Stage 1 finishes.
%.o: %.sams
	@echo "Assembling: $< -> $@"
	@$(ASSEMBLER) $(ASFLAGS_macos) -c -x assembler "$<" -o "$@"

objects: assembly
	@cd "$(THIS_DIR)" && \
	obj_targets=$$(find . -type f -name '*.sams' ! -path './_wd_probe/*' \
		| sed 's|^\./||' | sed 's|\.sams$$|.o|' | LC_ALL=C sort | tr '\n' ' '); \
	if [ -z "$$obj_targets" ]; then \
		echo "FAIL: no .sams files found after assembly"; \
		$(REPORT_COMPILE_TIME); exit 1; \
	fi; \
	nobj=$$(find . -type f -name '*.sams' ! -path './_wd_probe/*' | wc -l | tr -d ' '); \
	echo "Assembling $$nobj .sams file(s) with -j$(JOBS)..."; \
	if ! $(MAKE) -C "$(THIS_DIR)" -f "$(THIS_DIR)Makefile" -j$(JOBS) $$obj_targets; then \
		$(REPORT_COMPILE_TIME); exit 1; \
	fi; \
	$(call MAYBE_REPORT_COMPILE_TIME_SUCCESS,objects)

# Stage 3: link entry point `main` with all module objects + runtime.
# Always link bootstrap libsilica_compiler.a for DeviceIO helpers (_silica_*_path).
# deviceio_link_thunks.s covers seeds that still emit DeviceIO as module_user calls.
# main_entry_alias.s provides _main for Apple ld (asm emits `main` without underscore).
executables: objects
	@cd "$(THIS_DIR)" && \
	if [ ! -f main.o ]; then echo "FAIL: main.o missing"; $(REPORT_COMPILE_TIME); exit 1; fi; \
	if [ ! -f "$(RUNTIME_LIB)" ]; then \
		echo "Building Silica runtime library (bootstrap)..."; \
		(cd "$(BOOTSTRAP_DIR)" && cargo build --release --no-default-features) || { $(REPORT_COMPILE_TIME); exit 1; }; \
	fi; \
	$(ASSEMBLER) $(ASFLAGS_macos) -c deviceio_link_thunks.s -o deviceio_link_thunks.o || { $(REPORT_COMPILE_TIME); exit 1; }; \
	$(ASSEMBLER) $(ASFLAGS_macos) -c main_entry_alias.s -o main_entry_alias.o || { $(REPORT_COMPILE_TIME); exit 1; }; \
	objs=$$(find . -type f -name '*.o' ! -path './_wd_probe/*' | sed 's|^\./||' | sort | tr '\n' ' '); \
	link_archives=""; \
	if [ -x "$(SILICA_LINK_SH)" ]; then link_archives=$$("$(SILICA_LINK_SH)" .); fi; \
	echo "Linking: $(EXECUTABLE)"; \
	if ! $(LINKER_CLANG) $$objs "$(RUNTIME_LIB)" $$link_archives $(LDFLAGS_STACK) -o $(EXECUTABLE); then \
		echo "FAIL: linker failed"; \
		$(REPORT_COMPILE_TIME); \
		exit 1; \
	fi; \
	echo "✅ Built $(EXECUTABLE)"; \
	if [ "$(INSTALL_SELFHOST)" = "1" ] && [ "$(EXECUTABLE)" = "silica-compiler" ]; then \
		if [ -x "$(INSTALL_COMPILER_SH)" ]; then \
			"$(INSTALL_COMPILER_SH)" selfhost "$(THIS_DIR)$(EXECUTABLE)" || { $(REPORT_COMPILE_TIME); exit 1; }; \
		else \
			echo "WARN: $(INSTALL_COMPILER_SH) missing; binaries/silica-compiler still points at the previous selfhost"; \
		fi; \
	fi; \
	$(call MAYBE_REPORT_COMPILE_TIME_SUCCESS,executables)

# Plain `make` / `build`: if silica.config (topo unit list) is missing or stale,
# build it first; then run the normal assembly → objects → link pipeline.
build: silica.config executables
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
	    silica.compile.order \
	    .silica.config.units .silica.config.units.tmp silica.config.compiler.tmp \
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
	@echo "  make / build              create silica.config if missing/stale, then seed → .o → $(EXECUTABLE)"
	@echo "  assembly                  seed compile only (.sams); keeps up-to-date .sams/.iface"
	@echo "  objects                   assemble .sams → .o (parallel -j$(JOBS); override with JOBS=N)"
	@echo "  executables               link $(EXECUTABLE)"
	@echo "  silica.config             create/update topo-sorted unit list if missing or stale"
	@echo "  silica.config.regen       force-regenerate unit list (ignores timestamps)"
	@echo "  silica.target             rewrite emit_target declaration"
	@echo "  all-targets               build silica-compiler-<TARGET> for each allowable TARGET"
	@echo "  clean                     remove generated artifacts (*.sams/*.o, silica.target, silica.compile.order, *.iface, .cache/ifaces)"
	@echo "  check-target              validate TARGET against emitter/*/"
	@echo "  check-seed                ensure binaries/seed-compiler exists"
	@echo ""
	@echo "Paths"
	@echo "  Seed compiler:  $(SILICA_COMPILER)"
	@echo "  Emitter root:   $(EMITTER_ROOT)"
	@echo "  This tree:      $(THIS_DIR)"
