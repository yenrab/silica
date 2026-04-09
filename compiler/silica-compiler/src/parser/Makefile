# Makefile for Silica parser modules
# Parser .silica files are compiled by parent Makefile (parser/*.ll)
# This Makefile provides subdir target for parent; no separate compilation here

.DEFAULT_GOAL := all
.PHONY: all clean help

all:
	@echo "✅ Parser (compiled by parent)"

clean:
	@echo "Parser clean (handled by parent)"

help:
	@echo "Parser modules are compiled by parent src/Makefile"
	@echo "Files: parser_ast, constraint_core, constraint_runner, constraint_extract, parser_core, parser_runner"
	@echo "       capabilities/capability_fn_decl, capabilities/capability_expr_literal"
