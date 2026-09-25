"""Build all cases, attach generic failure candidates, assign stems, and dump
a manifest for the pipeline."""
import json
import random
import sys
from collections import Counter
from common import *

# generic mutators: (code, weight, callable(src, rng, info) -> (src, desc) | src | None)


def _wrap(fn, desc, needs=None):
    def run(src, rng, info):
        if needs and not info.get(needs):
            return None
        r = fn(src, rng)
        if r is None:
            return None
        if isinstance(r, tuple):
            return r[0], desc.format(r[1])
        return r, desc
    return run


def _needs_names(fn, desc, key):
    def run(src, rng, info):
        v = info.get(key)
        if not v:
            return None
        r = fn(src, rng, v)
        if r is None:
            return None
        if isinstance(r, tuple):
            return r[0], desc.format(r[1])
        return r, desc
    return run


def _seq(kind, desc):
    return _wrap(lambda s, r: m_seq_keyword(s, r, kind), desc)


GENERIC = [
    ("E2018", 10, _wrap(m_strip_binding_type, "binding '{}' has no type annotation")),
    ("E1055", 6, _wrap(m_unused_binding, "binding '{}' is never used")),
    ("E1042", 6, _wrap(m_drop_return_type, "function '{}' has no return type")),
    ("E1045", 4, _wrap(m_drop_fn_keyword, "function '{}' declared without the fn keyword")),
    ("E1065", 4, _wrap(m_unclosed_params, "parameter list of '{}' is not closed")),
    ("E1040", 6, _wrap(m_drop_semicolon, "missing ';' between two statements")),
    ("E1041", 5, _wrap(m_tail_semicolon, "trailing ';' after the block's final expression")),
    ("E1041", 2, _wrap(m_pure_semicolon, "trailing ';' after the pure expression")),
    ("E1043", 3, _wrap(m_spaced_colon, "whitespace between binding name '{}' and ':'")),
    ("E1044", 2, _wrap(m_spaced_atom, "whitespace between ':' and the atom name")),
    ("E1048", 2, _seq("remove_sequence", "produces without a sequence keyword")),
    ("E1049", 2, _seq("remove_produces", "sequence block without produces")),
    ("E1050", 2, _seq("remove_pure", "produces clause without pure")),
    ("E1051", 2, _seq("remove_end", "sequence block without a closing end")),
    ("E1052", 1, _seq("dup_sequence", "duplicate sequence keyword")),
    ("E1053", 1, _seq("dup_produces", "duplicate produces keyword")),
    ("E1054", 1, _seq("extra_end", "extra end after the sequence block")),
    ("E1056", 2, _seq("empty_proc", "empty proc[] on a pure sequence")),
    ("E3009", 3, _wrap(m_proc_on_return, "proc[...] written on the function return type instead of the sequence")),
    ("E3002", 5, _wrap(m_wrong_effect, "sequence declares the wrong effect; {} is used but not declared")),
    ("E3010", 4, _wrap(m_unused_effect, "sequence declares {} but nothing in it needs that effect")),
    ("E2002", 9, _needs_names(m_undefined_identifier, "undefined identifier '{}'", "names")),
    ("E2005", 6, _needs_names(m_unknown_function, "call to unknown function '{}'", "fnames")),
    ("E2005", 5, _needs_names(m_arg_count, "wrong argument count in call to '{}'", "calls")),
    ("E4015", 2, _wrap(m_dup_function, "function '{}' defined twice")),
    ("E1060", 3, _wrap(m_brace, "unbalanced braces")),
    ("E4016", 2, _wrap(m_rename_main, "no main/0: entry function renamed to '{}'")),
    ("E1047", 2, _wrap(m_struct_decl, "named struct declaration '{}' is not allowed")),
    ("E1046", 3, _wrap(m_record_trailing_punct, "stray '{}' after the last record field")),
    ("E2015", 4, _wrap(m_list_drop_space, "List type without its mem(Space)")),
]


def attach_generic(case, rng, max_candidates=4):
    """Add up to max_candidates generic failure candidates, weighted random order."""
    pool = list(GENERIC)
    added = 0
    tried = set()
    while pool and added < max_candidates:
        total = sum(w for _, w, _ in pool)
        r = rng.uniform(0, total)
        acc = 0
        for i, (code, w, fn) in enumerate(pool):
            acc += w
            if r <= acc:
                break
        code, w, fn = pool.pop(i)
        try:
            res = fn(case.src, rng, case.info)
        except Exception:
            res = None
        if res is None:
            continue
        new_src, desc = res
        if new_src == case.src or (code, desc) in tried:
            continue
        tried.add((code, desc))
        case.fails.append(Fail(code, desc, new_src))
        added += 1


def finalize(cases, rng, seed, offset=0):
    """Interleave template-specific and generic failure candidates, assign stems."""
    counters = Counter()
    out = []
    for c in cases:
        specific = list(c.fails)
        c.fails = []
        attach_generic(c, rng)
        generic = c.fails
        # prefer template-specific candidates (they carry type semantics) 60% of the time
        cands = []
        if specific and (not generic or rng.random() < 0.6):
            rng.shuffle(specific)
            cands = specific + generic
        else:
            cands = generic + specific
        if c.info.get("multi"):
            # failure twins compile alone: only parse-stage and module-resolution codes are reachable
            ok = {"E1009", "E1010", "E1040", "E1041", "E1042", "E1043", "E1044", "E1045", "E1046", "E1047", "E1048",
                  "E1049", "E1050", "E1051", "E1052", "E1053", "E1054", "E1055", "E1056", "E1060", "E1063", "E1065", "E3009", "E3010", "E2012"}
            cands = [f for f in cands if f.code in ok]
        c.fails = [Fail(f.code, f.desc, f"// expected failure {f.code}: {f.desc}\n" + f.src) for f in cands[:5]]
        if not c.fails:
            continue
        counters[(c.area, c.topic)] += 1
        n = counters[(c.area, c.topic)] + offset
        short = c.area.replace("_addition", "")
        c.stem = f"train_{short}_{c.topic}_{n:04d}"
        out.append(c)
    return out


def dump(cases, path):
    data = []
    for c in cases:
        data.append({
            "area": c.area, "topic": c.topic, "stem": c.stem, "src": c.src, "out": c.out, "exit": c.exit,
            "fails": [{"code": f.code, "desc": f.desc, "src": f.src} for f in c.fails],
            "info": {k: v for k, v in c.info.items() if k in ("multi", "extra_files", "runs")},
        })
    with open(path, "w") as fh:
        json.dump(data, fh)
    return data
