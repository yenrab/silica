# Harness work queued for after the lane merge (one lane, main checkout)

1. Retire the rust linker. 64 tracked makefiles carry the sysroot probe, RUST_LLD,
   LDFLAGS_rust-lld and two-branch link recipes. clang is the only supported linker;
   the GNU linker variant stays for Linux. Tested transform ready at
   programmer_tools/defect_batch/drop_rust_lld.py (run it, then check every file it flags by hand, then
   ask VERIFY to run a few suites). Also delete any remaining *.golden_link_fail.rust-lld.

2. RUN_SILICA_COMPILER_QUIET_WITH ends its unit loop with `break`, so the loop's exit
   status is 0 even when the compiler failed. The harness's compile-failure branch then
   never fires, and a unit that should be reported as a compile failure surfaces as some
   other symptom, e.g. "silica.link not emitted". Found by the diagnostics lane 2026-09-24
   while tracing that exact message. Fix the status propagation and check how many trials
   change verdict when it is fixed.

3. Reclaim the pre-rename compiler tree on the two Linux machines once the new layout has
   been synced there: ~/silica/compiler/silica-compiler on admin@pix.local (157M) and
   lee@nix.local (276M). Lee removes these himself; the build script prints the notice.

4. update_goldens.sh creates goldens nothing compares. Run with --create-missing inside a suite that
   has lib/ units, it wrote about 100 .ascomp files for library units the harness never compares,
   which had to be deleted again by hand (ordered_data_structures, 2026-09-25). Teach it the same
   rule the harness uses about which units are compared, so --create-missing cannot invent goldens.

5. An empty trial suite fails the harness. codegen_defects_addition is a holding pen for defects whose
   failure mode is a broken assemble or link step. Its last trial was fixed and moved on 2026-09-25,
   leaving the directory with only a README, a Makefile and runtime goldens, and the harness reported
   "silica.config missing after generation" as a failure. The directory was removed so the tree can be
   green. Teach the harness that a suite with no trials passes with zero of zero, then recreate the
   holding pen with its README so the pattern works for the next defect of that kind.
