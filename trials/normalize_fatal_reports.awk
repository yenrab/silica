# Folds the run- and target-dependent part of the process-fatal report lines (silica-specification
# §15.4.5.5) so that one golden holds on every target. Applied to both the .sout and the .scout.
#
#   [silica] fault at 0x<pc>[ in <symbol>+0x<offset>]  addr=0x..[  actor=0x..  sbase=0x..  ssize=0x..]
#       -> [silica] fault at <PTR>
#   [silica] abort: <reason> at 0x<pc>[ in <symbol>+0x<offset>]
#       -> [silica] abort: <reason> at <PTR>
#
# Every 0x value in a report is opaque (addresses differ from run to run and between targets), and the
# fields after the pc differ by target (no symbol lookup or actor runtime on the ESP32-S3 board), so a
# fault line keeps only its class and an abort line only its class and reason. The report can follow
# program output on the same line (a program that printed no newline), so the match is not anchored.
# A golden is written in the folded form. Used by trials/<suite>/compare_scout_normalized.sh,
# trials/targets/board_suite.sh (through those scripts), the ESP32-S3 board's tools/compare_sout.sh and
# the x86-64 ladder; see compiler/silica-compiler/design_documents/runtime_failure_reporting.md.
{ sub(/\r$/, "") }
{
    i = index($0, "[silica] fault at ")
    if (i > 0) {
        print substr($0, 1, i - 1) "[silica] fault at <PTR>"
        next
    }
    i = index($0, "[silica] abort: ")
    if (i > 0) {
        rest = substr($0, i)
        if (match(rest, / at (0x[0-9a-fA-F]+|<PTR>)/)) {
            print substr($0, 1, i - 1) substr(rest, 1, RSTART - 1) " at <PTR>"
            next
        }
    }
    print
}
