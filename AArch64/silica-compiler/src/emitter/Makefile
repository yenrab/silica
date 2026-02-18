# Makefile for Silica emitter modules
# Compiles emitter .silica files to object files (.o)
# Emitter: SIR -> AArch64 assembly

SILICA_COMPILER := ../../../silica-bootstrap-compiler/target/release/silica-boot
SEARCH_PATHS := -I . -I .. -I ../sir_generator -I constraints -I terms -I terms/prims -I control -I aarch64 -I ../../stdlib

# Find LLVM tools
LLVM_AS := $(or $(LLVM_AS), $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo "llvm-as"))
LLC := $(or $(LLVM_LLC), $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo "llc"))

# When QUIET=1 (from parent make), suppress progress messages
ifdef QUIET
  ECHO := @true
else
  ECHO := @echo
endif

SILICA_FILES := $(wildcard *.silica) $(wildcard */*.silica) $(wildcard */*/*.silica)
LL_FILES := $(SILICA_FILES:.silica=.ll)
BC_FILES := $(SILICA_FILES:.silica=.bc)
OBJ_FILES := $(SILICA_FILES:.silica=.o)

.SECONDARY: $(LL_FILES) $(BC_FILES)

.DEFAULT_GOAL := all
.PHONY: all clean help

all: $(OBJ_FILES)
	@echo "✅ emitter modules compiled to object files"

%.ll: %.silica
	$(ECHO) "Compiling: $< -> $@"
	@-$(SILICA_COMPILER) $< $@ $(SEARCH_PATHS)

%.bc: %.ll
	$(ECHO) "Assembling: $< -> $@"
	@-$(LLVM_AS) $< -o $@

%.o: %.bc
	$(ECHO) "Linking: $< -> $@"
	@-$(LLC) -filetype=obj $< -o $@

clean:
	@echo "Cleaning generated files..."
	@rm -f *.ll *.bc *.o
	@rm -f */*.ll */*.bc */*.o
	@echo "✅ Clean complete"

help:
	@echo "Silica emitter Makefile"
	@echo "======================="
	@echo "Emitter: SIR -> AArch64 assembly"
	@echo "Depends on sir_generator (sir_ast)"
	@echo ""
	@echo "Targets: all, clean, help"
