# sir_generator sources are compiled by the parent config-driven batch (silica.config → seed).

.DEFAULT_GOAL := all
.PHONY: all clean help

all:
	@echo "sir_generator: compiled by parent src_selfhost/Makefile (config-driven)."
	@echo "Run: make -C .. assembly"

clean:
	@find . \( -name '*.ll' -o -name '*.bc' -o -name '*.o' -o -name '*.sams' \) -delete 2>/dev/null || true
	@echo "sir_generator clean complete"

help:
	@echo "sir_generator modules are included in ../silica.config.compiler and built by the parent."
