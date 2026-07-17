# Emitter sources are compiled by the parent config-driven batch (silica.config → seed).
TARGET ?= apple_silicon

.DEFAULT_GOAL := all
.PHONY: all clean help

all:
	@echo "Emitter: compiled by parent src_selfhost/Makefile (config-driven, TARGET=$(TARGET))."
	@echo "Run: make -C .. assembly TARGET=$(TARGET)"

clean:
	@find . \( -name '*.ll' -o -name '*.bc' -o -name '*.o' -o -name '*.sams' \) -delete
	@echo "Emitter clean complete"

help:
	@echo "Emitter modules under $$TARGET are included in ../silica.config.compiler and built by the parent."
