# Makefile for Silica type checker modules
# Type checker .silica files are compiled by parent Makefile (type_checker/*.ll)
# This Makefile provides subdir target for parent; no separate compilation here

.DEFAULT_GOAL := all
.PHONY: all clean help

all:
	@echo "✅ Type checker (compiled by parent)"

clean:
	@echo "Type checker clean (handled by parent)"

help:
	@echo "Type checker modules are compiled by parent src/Makefile"
	@echo "Files: type_checker_core, type_checker_runner, declarations/type_checker_declarations_functions,"
	@echo "       expressions/type_checker_expressions_literals, expressions/type_checker_expressions_identifiers"
