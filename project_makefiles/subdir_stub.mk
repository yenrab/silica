# Shared stub for project leaf directories.
# Set COMPONENT_NAME before including. The parent root Makefile owns the build.

.DEFAULT_GOAL := help
.PHONY: all clean help --help

all: help
	@echo ""
	@echo "$(COMPONENT_NAME): compiled by the parent Makefile (config-driven)."
	@echo "Run: make -C .. assembly"
	@echo "     make -C .. build"

clean:
	@find . \( -name '*.ll' -o -name '*.bc' -o -name '*.o' -o -name '*.sams' \) -delete 2>/dev/null || true
	@echo "$(COMPONENT_NAME) clean complete"

--help: help
help:
	@echo "$(COMPONENT_NAME)/ — sources built by the parent Makefile"
	@echo ""
	@echo "Usage:"
	@echo "  make -C .. help"
	@echo "  make -C .. build"
	@echo "  make -C $(COMPONENT_NAME) help"
	@echo "  make -C $(COMPONENT_NAME) -- --help"
	@echo ""
	@echo "These modules are listed in ../silica.config and compiled by"
	@echo "the parent seed batch. Configure and build only at the parent:"
	@echo "  make -C .. help"
