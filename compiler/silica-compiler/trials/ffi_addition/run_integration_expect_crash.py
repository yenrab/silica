#!/usr/bin/env python3
"""Run a trial binary and expect a crash (non-zero exit or fatal signal)."""

import subprocess
import sys


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: run_integration_expect_crash.py EXECUTABLE", file=sys.stderr)
        return 2
    exe = sys.argv[1]
    try:
        result = subprocess.run([f"./{exe}"], capture_output=True, text=True, timeout=10)
    except subprocess.TimeoutExpired:
        print(f"FAIL: {exe} timed out (expected crash)")
        return 1
    code = result.returncode
    if code == 0:
        print(f"FAIL: {exe} exited 0 (expected crash)")
        if result.stdout:
            print(result.stdout, end="")
        if result.stderr:
            print(result.stderr, end="", file=sys.stderr)
        return 1
    if code > 0 and code < 128:
        print(f"OK: {exe} exited {code} (abnormal termination)")
        return 0
    print(f"OK: {exe} terminated with code {code} (likely signal)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
