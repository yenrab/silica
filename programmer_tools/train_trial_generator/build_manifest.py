"""Entry point: generate all cases and write the manifest."""
import random
import sys
from collections import Counter
import driver
from common import *
import numeric

import os
SEED = int(os.environ.get("GEN_SEED", "20260908"))
STEM_OFFSET = int(os.environ.get("STEM_OFFSET", "0"))


def build(scale=1.0, only=None):
    rng = random.Random(SEED)
    cases = []
    q = lambda n: max(1, int(n * scale))
    for ty in INT_TYPES:
        cases += numeric.gen_int_area(rng, ty, q(170))
    for ty in FLOAT_TYPES:
        cases += numeric.gen_float_area(rng, ty, q(130))
    cases += numeric.gen_negation(rng, q(130))
    cases += numeric.gen_bitwise(rng, q(150))
    cases += numeric.gen_boolean(rng, q(130))
    cases += numeric.gen_atoms(rng, q(110))
    try:
        import data
        cases += data.gen_all(rng, q)
    except ImportError:
        pass
    try:
        import control
        cases += control.gen_all(rng, q)
    except ImportError:
        pass
    try:
        import systems
        cases += systems.gen_all(rng, q)
    except ImportError:
        pass
    if only:
        cases = [c for c in cases if c.area in only]
    # dedupe identical success sources
    seen = set()
    uniq = []
    for c in cases:
        key = (c.area, c.src)
        if key in seen:
            continue
        seen.add(key)
        uniq.append(c)
    cases = driver.finalize(uniq, rng, SEED, STEM_OFFSET)
    # cross-run duplicate check: drop any candidate whose success body (header comment
    # stripped) already exists as an installed train_* trial, and any failure candidate
    # whose body already exists in error_enforcement_addition.
    if os.environ.get("EXCLUDE_EXISTING", "1") != "0":
        root = os.environ.get("SILICA_ROOT") or os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
        existing_ok, existing_fail = set(), set()
        for dp, dn, fn in os.walk(os.path.join(root, "trials")):
            for f in fn:
                if f.startswith("train_") and f.endswith(".silica"):
                    body = open(os.path.join(dp, f), encoding="utf-8", errors="replace").read()
                    if "error_enforcement_addition" in dp:
                        existing_fail.add(strip_header(body, 2))
                    else:
                        existing_ok.add(strip_header(body, 1))
        kept = []
        for c in cases:
            if strip_header(c.src, 1) in existing_ok:
                continue
            c.fails = [f for f in c.fails if strip_header(f.src, 2) not in existing_fail]
            if c.fails:
                kept.append(c)
        print(f"cross-run dedupe: {len(cases)} -> {len(kept)} (existing: {len(existing_ok)} ok, {len(existing_fail)} fail)")
        cases = kept
    limit = int(os.environ.get("MAX_CASES", "0"))
    if limit and len(cases) > limit:
        # stratified: drop proportionally from every area, never the tail of the list
        by_area = {}
        for c in cases:
            by_area.setdefault(c.area, []).append(c)
        frac = limit / len(cases)
        picked = []
        for area, cs in by_area.items():
            picked += cs[:max(1, round(len(cs) * frac))]
        cases = picked[:limit]
    return cases


def strip_header(src, n):
    """Drop the first n comment lines so equal programs compare equal."""
    lines = src.split("\n")
    i = 0
    while i < n and i < len(lines) and lines[i].startswith("//"):
        i += 1
    return "\n".join(lines[i:]).strip()


if __name__ == "__main__":
    scale = float(sys.argv[1]) if len(sys.argv) > 1 else 1.0
    out = sys.argv[2] if len(sys.argv) > 2 else "manifest.json"
    only = sys.argv[3].split(",") if len(sys.argv) > 3 else None
    cases = build(scale, only)
    data = driver.dump(cases, out)
    print("cases:", len(cases))
    print(Counter(c.area for c in cases))
    print("first failure codes:", Counter(c.fails[0].code for c in cases if c.fails))
