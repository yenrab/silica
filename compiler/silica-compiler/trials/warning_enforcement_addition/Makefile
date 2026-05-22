# Makefile for Silica warning enforcement trials.
# Captures compiler warning output, normalizes temp paths, and diffs against
# checked-in .golden_warn files. Warnings must not make compilation fail.

THIS_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))
SILICA_COMPILER := $(THIS_DIR)../../src/silica-compiler

TRIALS := dangerous_legacy_stub

.DEFAULT_GOAL := all
.PHONY: all integrate clean help

all: integrate

integrate: clean
	@failed=0; ok=0; ko=0; rm -f "$(THIS_DIR).integrate_counts"; \
	for trial in $(TRIALS); do \
		tmpdir=$$(mktemp -d); \
		mkdir -p "$$tmpdir/dangerous_exposure_source/legacy"; \
		ln -s "$(THIS_DIR)$$trial.silica" "$$tmpdir/$$trial.silica"; \
		ln -s "$(THIS_DIR)dangerous_exposure_source/legacy/silica_legacy_math_wrapper.meta" "$$tmpdir/dangerous_exposure_source/legacy/silica_legacy_math_wrapper.meta"; \
		echo "$$trial.silica" > "$$tmpdir/silica.config"; \
		out="$$tmpdir/out.txt"; \
		(cd "$$tmpdir" && "$(SILICA_COMPILER)") > "$$out" 2>&1; status=$$?; \
		if [ "$$status" -ne 0 ]; then \
			echo "❌❌ warning_enforcement_addition/$$trial: compilation failed"; \
			cat "$$out"; \
			ko=$$((ko + 1)); failed=1; rm -rf "$$tmpdir"; continue; \
		fi; \
		awk 'BEGIN { keep=0 } /^WARNING:/ { keep=1 } keep { print } /^-->$$/ && keep { keep=0 }' "$$out" \
			| sed "s|$$tmpdir/||g" > "$(THIS_DIR)$$trial.cur_warn"; \
		rm -rf "$$tmpdir"; \
		if diff -q "$(THIS_DIR)$$trial.cur_warn" "$(THIS_DIR)$$trial.golden_warn" > /dev/null 2>&1; then \
			echo "✅✅ warning_enforcement_addition/$$trial"; \
			ok=$$((ok + 1)); \
		else \
			echo "❌❌ warning_enforcement_addition/$$trial: .cur_warn differs from .golden_warn"; \
			diff "$(THIS_DIR)$$trial.cur_warn" "$(THIS_DIR)$$trial.golden_warn" || true; \
			ko=$$((ko + 1)); failed=1; \
		fi; \
	done; \
	printf '%d %d\n' "$$ok" "$$ko" > "$(THIS_DIR).integrate_counts"; \
	exit $$failed

clean:
	@rm -f "$(THIS_DIR)"*.cur_warn "$(THIS_DIR).integrate_counts"
	@echo "✅ warning_enforcement_addition/Clean complete"

help:
	@echo "warning_enforcement_addition Makefile"
	@echo "====================================="
	@echo "Runs warning-producing trials and diffs normalized warning output"
	@echo "against .golden_warn files. Compilation itself must still succeed."
