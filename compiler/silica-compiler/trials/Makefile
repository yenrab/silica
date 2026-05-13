# Makefile for Silica trials — `integrate` runs each subdirectory's own Makefile
# (messages and pass/fail rules come from those Makefiles, not from here).
#
# integrate / clean: `make -C <trial> integrate|clean` for each immediate child with Makefile or makefile
# To refresh only silica.config in a trial: `make -C <trial> silica.config`

# Immediate trial subdirs that contain Makefile or makefile (GNU make finds makefile in -C dir)
TRIAL_MAKEFILES := $(wildcard */Makefile */makefile)
TRIAL_SUBDIRS := $(sort $(foreach mf,$(TRIAL_MAKEFILES),$(notdir $(patsubst %/,%,$(dir $(mf))))))

.PHONY: integrate clean help

.DEFAULT_GOAL := integrate

# integrate: delegate; each trial writes .integrate_counts ("<ok> <fail>" one line) for the parent to sum
# Runs full tree `clean` first, then each trial's integrate (each trial lists `clean` before `silica.config` and
# uses `silica.config: | clean` so regeneration never runs after a wipe that deletes the config).
integrate: clean
integrate:
	@ok=0; ko=0; failed=0; \
	for dir in $(TRIAL_SUBDIRS); do \
		echo "=== $$dir ==="; \
		if [ "$$dir" = "modules_addition" ] && [ -f "modules_addition/INTEGRATE_PENDING" ]; then \
			echo "SKIP: modules_addition (INTEGRATE_PENDING — see modules_addition/INTEGRATE_PENDING)"; \
			continue; \
		fi; \
		if [ "$$dir" = "cpu_discovery_and_spawn_pinning" ] && [ -f "cpu_discovery_and_spawn_pinning/INTEGRATE_PENDING" ]; then \
			echo "SKIP: cpu_discovery_and_spawn_pinning (INTEGRATE_PENDING — see cpu_discovery_and_spawn_pinning/README.md)"; \
			continue; \
		fi; \
		if $(MAKE) -C $$dir integrate; then st=0; else st=1; failed=1; fi; \
		cf="$$dir/.integrate_counts"; \
		if [ -f "$$cf" ]; then \
			read n1 n2 < "$$cf" || true; \
			ok=$$((ok + $${n1:-0})); ko=$$((ko + $${n2:-0})); \
		elif [ "$$st" -ne 0 ]; then \
			ko=$$((ko + 1)); \
		fi; \
	done; \
	echo "success: ✅✅ $$ok"; \
	echo "fail: ❌❌ $$ko"; \
	test "$$ko" -eq 0 && test "$${failed:-0}" -eq 0

# clean: delegate so each trial removes what its Makefile defines (same dirs as integrate)
clean:
	@failed=0; \
	for dir in $(TRIAL_SUBDIRS); do \
		echo "Cleaning $$dir..."; \
		$(MAKE) -C $$dir clean || failed=1; \
	done; \
	if [ "$${failed:-0}" -eq 0 ]; then echo "Clean complete"; else echo "❌ clean: one or more trial directories failed"; fi; \
	test "$${failed:-0}" -eq 0

help:
	@echo "Silica trials Makefile"
	@echo "====================="
	@echo ""
	@echo "Targets:"
	@echo "  integrate  - Full tree clean, then for each trial subdir: make -C <dir> integrate"
	@echo "               (compiler output and pass/fail rules are defined in that subdir)"
	@echo "               Ends with totals: success: ✅✅ <n>  fail: ❌❌ <m>"
	@echo "               (<n>/<m> sum each trial's .integrate_counts: ok fail)"
	@echo "  clean      - For each trial subdir: make -C <dir> clean (rules live in that subdir)"
	@echo "  help       - Show this help"
	@echo ""
	@echo "Per-trial silica.config: run \`make -C <trial_dir> silica.config\` (see that Makefile)."
	@echo ""
	@echo "Trial dirs (integrate + clean): $(TRIAL_SUBDIRS)"
