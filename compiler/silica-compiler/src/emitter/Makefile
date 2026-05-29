# Makefile for Silica emitter modules
# Compiles emitter .silica files to object files (.o)
# Emitter: SIR -> target-specific assembly
#
# Usage:
#   make                        # builds for default target (apple_silicon)
#   make TARGET=esp32-s3        # builds for ESP32-S3
#   make TARGET=apple_silicon   # builds for Apple Silicon

# Target chip directory under emitter/ (override from parent or command line)
TARGET ?= apple_silicon

# Validate that the target directory exists
ifeq ($(wildcard $(TARGET)/),)
  $(error Unknown TARGET '$(TARGET)'. Available targets: $(patsubst %/,%,$(wildcard */)))
endif

SILICA_COMPILER := ../../../silica-bootstrap-compiler/target/release/silica-boot
SEARCH_PATHS := -I . -I .. -I ../parser -I ../sir_generator -I ../data_structures -I $(TARGET) -I $(TARGET)/terms -I $(TARGET)/terms/prims -I $(TARGET)/terms/const -I $(TARGET)/terms/let -I $(TARGET)/terms/call -I $(TARGET)/recursion -I $(TARGET)/control -I $(TARGET)/effects -I $(TARGET)/atoms -I $(TARGET)/shared -I $(TARGET)/constraints -I ../../stdlib

# Find LLVM tools
LLVM_AS := $(or $(LLVM_AS), $(shell command -v llvm-as 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llvm-as && echo "/opt/homebrew/opt/llvm/bin/llvm-as") || echo "llvm-as"))
LLC := $(or $(LLVM_LLC), $(shell command -v llc 2> /dev/null || (test -x /opt/homebrew/opt/llvm/bin/llc && echo "/opt/homebrew/opt/llvm/bin/llc") || echo "llc"))

# When QUIET=1 (from parent make), suppress progress messages
ifdef QUIET
  ECHO := @true
else
  ECHO := @echo
endif

SILICA_FILES := $(wildcard $(TARGET)/*.silica) $(wildcard $(TARGET)/*/*.silica) $(wildcard $(TARGET)/*/*/*.silica)
LL_FILES := $(SILICA_FILES:.silica=.ll)
BC_FILES := $(SILICA_FILES:.silica=.bc)
OBJ_FILES := $(SILICA_FILES:.silica=.o)

.SECONDARY: $(LL_FILES) $(BC_FILES)

.DEFAULT_GOAL := all
.PHONY: all clean help

all: $(OBJ_FILES)
	@echo "✅ emitter modules compiled to object files (TARGET=$(TARGET))"

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
	@rm -f $(TARGET)/*.ll $(TARGET)/*.bc $(TARGET)/*.o
	@rm -f $(TARGET)/*/*.ll $(TARGET)/*/*.bc $(TARGET)/*/*.o
	@rm -f $(TARGET)/*/*/*.ll $(TARGET)/*/*/*.bc $(TARGET)/*/*/*.o
	@echo "✅ Clean complete (TARGET=$(TARGET))"

help:
	@echo "Silica emitter Makefile"
	@echo "======================="
	@echo "Emitter: SIR -> target-specific assembly"
	@echo "Depends on sir_generator (sir_ast)"
	@echo ""
	@echo "Usage:"
	@echo "  make                        # default target (apple_silicon)"
	@echo "  make TARGET=esp32-s3        # build for ESP32-S3"
	@echo ""
	@echo "Available targets: $(patsubst %/,%,$(wildcard */))"
	@echo "Current target:    $(TARGET)"
	@echo ""
	@echo "Targets: all, clean, help"
