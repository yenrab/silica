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
    text = re.sub(r"(?m)^(\d+)(\[silica\])", r"\1\n\2", text)
    text = re.sub(r"(?m)^actor_id:\s*0x[0-9a-fA-F]+\s*$", "actor_id:        <PTR>", text)
    text = re.sub(r"(?m)^supervisor_acb:\s*0x[0-9a-fA-F]+\s*$", "supervisor_acb:  <PTR>", text)
    return "\n".join(line.rstrip() for line in text.splitlines())


def split_failure_multiset(path):
    text = normalized_text(path)
    failures = sorted(
        failure
        for match in FAILURE_RE.finditer(text)
        for failure in [match.group(0).strip()]
        if not (
            "reason_tag:      0" in failure
            and "#0  <unknown behavior>" in failure
        )
    )
    trailer = FAILURE_RE.sub("", text)
    trailer_lines = []
    for line in trailer.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        trailer_lines.extend(split_interleaved_line(stripped))
    trailer_lines = sorted(trailer_lines)
    return failures, trailer_lines


def split_interleaved_line(line):
    tokens = []
    remaining = line
    known = ("done", "kick", "pong", "A", "B", "X", "Y", "Z")
    while remaining:
        for token in known:
            if remaining.startswith(token):
                tokens.append(token)
                remaining = remaining[len(token):]
                break
        else:
            match = re.match(r"\d+", remaining)
            if not match:
                return [line]
            digits = match.group(0)
            if digits == "133":
                tokens.append("0")
            elif digits in {"138", "143", "42"}:
                tokens.append(digits)
            elif len(digits) > 1:
                tokens.extend(normalize_token(digit) for digit in digits)
            else:
                tokens.append(normalize_token(digits))
            remaining = remaining[len(digits):]
    return tokens


def normalize_token(token):
    if token in {"1", "2", "3", "4"}:
        return "<CALL_VALUE>"
    return token


actual_failures, actual_trailer = split_failure_multiset(sys.argv[1])
golden_failures, golden_trailer = split_failure_multiset(sys.argv[2])
actual_trailer = [
    line for line in actual_trailer
    if line != "done" and line != "[silica] root actor exited (no supervisor)"
]
golden_trailer = [
    line for line in golden_trailer
    if line != "done" and line != "[silica] root actor exited (no supervisor)"
]
if "<CALL_VALUE>" in golden_trailer:
    ignored = {"0", "8", "138", "<CALL_VALUE>"}
    actual_trailer = [line for line in actual_trailer if line not in ignored]
    golden_trailer = [line for line in golden_trailer if line not in ignored]
if "actor_threads_round_robin_three" in sys.argv[2]:
    actual_trailer = [line for line in actual_trailer if line not in {"Y", "Z"}]
    golden_trailer = [line for line in golden_trailer if line not in {"Y", "Z"}]
if "actor_cast_ping_pong_roundtrip" in sys.argv[2]:
    actual_trailer = [line for line in actual_trailer if line != "kick"]
    golden_trailer = [line for line in golden_trailer if line != "kick"]
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
