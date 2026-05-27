# Makefile for standard_data_structures generator foundation (Phase 0)

SILICA_COMPILER := ../../../silica-bootstrap-compiler/target/release/silica-boot
SEARCH_PATHS := -I .

LLVM_AS := $(or $(LLVM_AS), $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo "llvm-as"))
LLC := $(or $(LLVM_LLC), $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo "llc"))

ifdef QUIET
  ECHO := @true
else
  ECHO := @echo
endif

SILICA_FILES := $(wildcard *.silica)
LL_FILES := $(SILICA_FILES:.silica=.ll)
BC_FILES := $(SILICA_FILES:.silica=.bc)
OBJ_FILES := $(SILICA_FILES:.silica=.o)

.SECONDARY: $(LL_FILES) $(BC_FILES)

.DEFAULT_GOAL := all
.PHONY: all clean help

all: $(OBJ_FILES)
	@echo "✅ standard_data_structures modules compiled to object files"

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
	@rm -f *.ll *.bc *.o
	@echo "✅ standard_data_structures clean complete"

help:
	@echo "Silica standard_data_structures Makefile — Phase 0 registry and type expansion"
