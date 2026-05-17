#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: compare_scout_multiset.sh <actual.sout> <golden.scout>" >&2
  exit 2
fi

python3 - "$1" "$2" <<'PY'
import re
import sys


FAILURE_RE = re.compile(
    r"=== Silica Actor Failure ===\n.*?=== End Silica Actor Failure ===",
    re.DOTALL,
)


def normalized_text(path):
    text = open(path, "r", encoding="utf-8", errors="replace").read()
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    text = re.sub(r"(?m)^actor_id:\s*0x[0-9a-fA-F]+\s*$", "actor_id:        <PTR>", text)
    text = re.sub(r"(?m)^supervisor_acb:\s*0x[0-9a-fA-F]+\s*$", "supervisor_acb:  <PTR>", text)
    return "\n".join(line.rstrip() for line in text.splitlines())


def split_failure_multiset(path):
    text = normalized_text(path)
    failures = sorted(match.group(0).strip() for match in FAILURE_RE.finditer(text))
    trailer = FAILURE_RE.sub("", text)
    trailer_lines = [line for line in trailer.splitlines() if line.strip()]
    return failures, trailer_lines


actual_failures, actual_trailer = split_failure_multiset(sys.argv[1])
golden_failures, golden_trailer = split_failure_multiset(sys.argv[2])
if actual_failures == golden_failures and actual_trailer == golden_trailer:
    raise SystemExit(0)

import difflib

golden = ["[failure banners]"] + golden_failures + ["[trailer]"] + golden_trailer
actual = ["[failure banners]"] + actual_failures + ["[trailer]"] + actual_trailer
for line in difflib.unified_diff(
    golden,
    actual,
    fromfile=sys.argv[2],
    tofile=sys.argv[1],
    lineterm="",
):
    print(line)
raise SystemExit(1)
PY
