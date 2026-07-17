# Lexer sources are compiled by the parent config-driven batch (silica.config → seed).

.DEFAULT_GOAL := all
.PHONY: all clean help

all:
	@echo "Lexer: compiled by parent src_selfhost/Makefile (config-driven)."
	@echo "Run: make -C .. assembly"

clean:
	@find . \( -name '*.ll' -o -name '*.bc' -o -name '*.o' -o -name '*.sams' \) -delete 2>/dev/null || true
	@echo "Lexer clean complete"

help:
	@echo "Lexer modules are included in ../silica.config.compiler and built by the parent."
