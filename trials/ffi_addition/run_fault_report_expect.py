"""Run a binary that faults outside any actor and assert the runtime's contract.

The pre-existing run_integration_expect_crash.py accepts ANY non-zero exit, so it
cannot tell a clean reported exit from a silent signal death -- nor notice if the
fault-site diagnostic stops being emitted. This asserts both halves of the rule:
a non-actor hard fault must (1) print a fault-site line naming the faulting symbol
and (2) terminate with exit 70, never hang and never die by bare signal.
"""
import re
import subprocess
import sys

EXPECTED_EXIT = 70
PATTERN = re.compile(r"\[silica\] fault at 0x[0-9a-f]{16} in \S+")


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: run_fault_report_expect.py EXECUTABLE", file=sys.stderr)
        return 2
    exe = sys.argv[1]
    try:
        r = subprocess.run([exe], capture_output=True, text=True, timeout=60)
    except subprocess.TimeoutExpired:
        print(f"FAIL: {exe} timed out -- the fault path must never hang")
        return 1
    blob = (r.stdout or "") + (r.stderr or "")
    if not PATTERN.search(blob):
        print(f"FAIL: {exe} printed no '[silica] fault at 0x... in <symbol>' line")
        print(blob[-400:], file=sys.stderr)
        return 1
    if r.returncode != EXPECTED_EXIT:
        print(f"FAIL: {exe} exited {r.returncode}, expected {EXPECTED_EXIT}")
        return 1
    print(f"OK: {exe} reported the fault site and exited {EXPECTED_EXIT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
