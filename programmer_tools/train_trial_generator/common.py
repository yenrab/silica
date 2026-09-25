"""Shared infrastructure for Silica training-pair generation.

A Case is one success program plus a list of candidate failure twins.  Each
failure twin is a single, deliberate edit of the success program and names the
diagnostic it is expected to trigger.  The pipeline verifies both halves with
the real compiler before anything is installed.
"""
import random
import re
from dataclasses import dataclass, field

INT_TYPES = ["int8", "int16", "int32", "int64", "uint8", "uint16", "uint32", "uint64"]
INT_RANGE = {
    "int8": (-128, 127), "int16": (-32768, 32767), "int32": (-2**31, 2**31 - 1), "int64": (-2**63, 2**63 - 1),
    "uint8": (0, 255), "uint16": (0, 65535), "uint32": (0, 2**32 - 1), "uint64": (0, 2**64 - 1),
}
FLOAT_TYPES = ["float16", "float32", "float64"]
FLOAT_BITS = {"float16": 11, "float32": 24, "float64": 53}
FLOAT_MAX = {"float16": 60000.0, "float32": 3.0e38, "float64": 1.0e300}


@dataclass
class Fail:
    code: str
    desc: str
    src: str


@dataclass
class Case:
    area: str            # trial directory name, e.g. "int8_addition"
    topic: str           # short slug used in the file name
    src: str             # success program
    out: str             # expected stdout
    exit: int            # expected exit status (0..255)
    fails: list = field(default_factory=list)
    info: dict = field(default_factory=dict)
    stem: str = ""

    def add_fail(self, code, desc, src):
        if src is not None and src != self.src:
            self.fails.append(Fail(code, desc, src))


def exit_of(v, ty):
    """Exit status for a main returning integer-like value v of type ty."""
    if ty in INT_TYPES:
        return int(v) & 0xFF
    if ty in FLOAT_TYPES:
        return int(v) & 0xFF  # truncation toward zero, then low byte
    if ty == "boolean":
        return 1 if v else 0
    raise ValueError(ty)


def fmt_float(v, ty):
    """Reproduce the runtime float printer: truncated integer part, '.', then
    digits from repeated *10 on the residual, stop at exact zero, at most
    15 digits (float64) / 7 digits (float16, float32), at least one digit."""
    from fractions import Fraction
    fr = Fraction(v)
    neg = fr < 0
    fr = abs(fr)
    ip = int(fr)
    rem = fr - ip
    maxd = 15 if ty == "float64" else 7
    digits = ""
    while len(digits) < maxd:
        rem *= 10
        d = int(rem)
        rem -= d
        digits += str(d)
        if rem == 0:
            break
    if not digits:
        digits = "0"
    return ("-" if neg else "") + str(ip) + "." + digits


def float_exact(v, ty):
    """True when v (a Fraction or dyadic float) is exactly representable in ty
    with a short binary expansion (so the printer output is predictable)."""
    from fractions import Fraction
    fr = Fraction(v)
    if fr == 0:
        return True
    if abs(fr) > FLOAT_MAX[ty]:
        return False
    den = fr.denominator
    if den & (den - 1):
        return False  # not a power of two
    num = abs(fr.numerator)
    return num.bit_length() <= FLOAT_BITS[ty]


def flit(fr):
    """Silica float literal text for a dyadic Fraction (digits on both sides)."""
    from fractions import Fraction
    fr = Fraction(fr)
    s = str(float(fr))
    if "e" in s or "E" in s:
        # expand scientific notation produced by Python for big/small values
        from decimal import Decimal
        s = format(Decimal(fr.numerator) / Decimal(fr.denominator), "f")
    if "." not in s:
        s += ".0"
    return s


# ----------------------------------------------------------------------------
# source-text helpers
# ----------------------------------------------------------------------------

def seq_block(effects, stmts, produce, style=0, indent=4):
    """Render a sequence block.  stmts: list of statement strings without ';'."""
    pad = " " * indent
    pad2 = " " * (indent + 4)
    proc = f" proc[{', '.join(effects)}]" if effects else ""
    body = ";\n".join(pad2 + s for s in stmts)
    if style == 0:
        # spread form
        parts = [pad + "sequence" + proc]
        if body:
            parts.append(body)
        parts.append(pad + "produces")
        parts.append(pad2 + "pure " + produce)
        parts.append(pad + "end")
        return "\n".join(parts)
    if style == 1:
        # corpus one-line header form: "sequence proc[...] first: T <- ...;"
        if stmts:
            first = stmts[0]
            rest = ";\n".join(pad2 + s for s in stmts[1:])
            head = pad + "sequence" + proc + " " + first
            if rest:
                head += ";\n" + rest
            return head + "\n" + pad2 + "produces pure " + produce + " end"
        return pad + "sequence" + proc + " produces pure " + produce + " end"
    # style 2: compact
    parts = [pad + "sequence" + proc]
    if body:
        parts.append(body)
    parts.append(pad + "produces pure " + produce + " end")
    return "\n".join(parts)


def fn_decl(name, params, ret, lines, tail, comment=None):
    """Render a function.  params: list of (name, type) or raw strings."""
    ps = ", ".join(p if isinstance(p, str) else f"{p[0]}: {p[1]}" for p in params)
    out = []
    if comment:
        out.append(comment)
    out.append(f"fn {name}({ps}) -> {ret} {{")
    for ln in lines:
        out.append("    " + ln + ";" if not ln.endswith(";") and not ln.startswith("//") else "    " + ln)
    if tail is not None:
        out.append("    " + tail)
    out.append("}")
    return "\n".join(out)


def program(*parts, header=None):
    body = "\n\n".join(p for p in parts if p)
    if header:
        return header + "\n" + body + "\n"
    return body + "\n"


# ----------------------------------------------------------------------------
# generic single-edit mutators.  Each returns a new source or None.
# The pipeline verifies the resulting diagnostic; a wrong guess is dropped.
# ----------------------------------------------------------------------------

BIND_RE = re.compile(r"^(\s*)([A-Za-z_][A-Za-z0-9_]*): ([^<\n]+?) <- (.*)$", re.M)


def rep(src, old, new, count=1):
    """str.replace that skips occurrences inside // comments."""
    out, i, done = [], 0, 0
    while True:
        j = src.find(old, i)
        if j < 0 or (count > 0 and done >= count):
            out.append(src[i:])
            break
        if in_comment(src, j):
            out.append(src[i:j + len(old)])
            i = j + len(old)
            continue
        out.append(src[i:j])
        out.append(new)
        i = j + len(old)
        done += 1
    return "".join(out)


def in_comment(src, pos):
    """True when pos lies inside a // line comment."""
    ls = src.rfind("\n", 0, pos) + 1
    return "//" in src[ls:pos]


def used_bindings(src):
    """Names bound with 'name: T <- ...' that are referenced later."""
    res = []
    for m in BIND_RE.finditer(src):
        name = m.group(2)
        if name == "_" or name.startswith("_"):
            continue
        rest = src[m.end():]
        if re.search(r"\b" + re.escape(name) + r"\b", rest):
            res.append((m, name, m.group(3)))
    return res


def m_strip_binding_type(src, rng):
    cands = used_bindings(src)
    if not cands:
        return None
    m, name, ty = rng.choice(cands)
    return src[:m.start()] + f"{m.group(1)}{name} <- {m.group(4)}" + src[m.end():], name


def m_unused_binding(src, rng, ty="int64", lit="41"):
    """Insert an unread binding as the first statement of the first function body."""
    m = re.search(r"^fn [^\n]*\{\n", src, re.M)
    if not m:
        return None
    name = rng.choice(["unused_value", "scratch", "leftover", "temp_val", "spare"])
    return src[:m.end()] + f"    {name}: {ty} <- {lit};\n" + src[m.end():], name


FN_HEAD_RE = re.compile(r"^fn ([A-Za-z_][A-Za-z0-9_]*)\(([^)]*)\) -> ([^{\n]+?) \{", re.M)


def m_drop_return_type(src, rng):
    heads = list(FN_HEAD_RE.finditer(src))
    if not heads:
        return None
    m = rng.choice(heads)
    style = rng.choice(["bare", "arrow"])
    if style == "bare":
        new = f"fn {m.group(1)}({m.group(2)}) {{"
    else:
        new = f"fn {m.group(1)}({m.group(2)}) -> {{"
    return src[:m.start()] + new + src[m.end():], m.group(1)


def m_drop_fn_keyword(src, rng):
    heads = list(FN_HEAD_RE.finditer(src))
    if not heads:
        return None
    m = rng.choice(heads)
    return src[:m.start()] + src[m.start() + 3:], m.group(1)


def m_unclosed_params(src, rng):
    heads = [m for m in FN_HEAD_RE.finditer(src) if m.group(2).strip()]
    if not heads:
        return None
    m = rng.choice(heads)
    new = f"fn {m.group(1)}({m.group(2)} -> {m.group(3)} {{"
    return src[:m.start()] + new + src[m.end():], m.group(1)


def m_drop_semicolon(src, rng):
    """Remove a ';' that separates two statements (line ends with ';' and the
    next line is another statement, not 'produces')."""
    lines = src.split("\n")
    cands = []
    for i, ln in enumerate(lines[:-1]):
        s = ln.rstrip()
        nxt = lines[i + 1].strip()
        if re.match(r"^\s*[a-z_][A-Za-z0-9_]*\(.*\);$", s) and nxt and not nxt.startswith("produces") and not nxt.startswith("}") \
                and not nxt.startswith("end") and not nxt.startswith("//") and "produces pure" not in s:
            cands.append(i)
    if not cands:
        return None
    i = rng.choice(cands)
    lines[i] = lines[i].rstrip()[:-1]
    return "\n".join(lines)


def m_tail_semicolon(src, rng):
    """Append ';' to a block's final expression (line before a closing '}')."""
    lines = src.split("\n")
    cands = []
    for i, ln in enumerate(lines[:-1]):
        s = ln.rstrip()
        nxt = lines[i + 1].strip()
        if nxt == "}" and s and not s.endswith(";") and not s.endswith("{") and not s.endswith("end") \
                and "//" not in s and "->" not in s:
            cands.append(i)
    if not cands:
        return None
    i = rng.choice(cands)
    lines[i] = lines[i].rstrip() + ";"
    return "\n".join(lines)


def m_pure_semicolon(src, rng):
    m = list(re.finditer(r"pure ([^\n]+?)( end|\n)", src))
    if not m:
        return None
    mm = rng.choice(m)
    return src[:mm.start()] + f"pure {mm.group(1)};{mm.group(2)}" + src[mm.end():]


def m_spaced_colon(src, rng):
    cands = used_bindings(src)
    if not cands:
        return None
    m, name, ty = rng.choice(cands)
    return src[:m.start()] + f"{m.group(1)}{name} :{ty} <- {m.group(4)}" + src[m.end():], name


ATOM_RE = re.compile(r"(?<![A-Za-z0-9_\]]):([a-z_][A-Za-z0-9_]*)")


def m_spaced_atom(src, rng):
    ms = [m for m in ATOM_RE.finditer(src) if not src[max(0, m.start() - 1):m.start()].isalnum() and not in_comment(src, m.start())]
    # avoid the ':' in 'name: type' - ATOM_RE requires ':' followed directly by a name
    if not ms:
        return None
    m = rng.choice(ms)
    return src[:m.start()] + ": " + m.group(1) + src[m.end():]


SEQ_STYLE_RE = re.compile(r"sequence( proc\[[^\]]*\])?")


def m_seq_keyword(src, rng, kind):
    """Sequence keyword surgery: kind in remove_sequence, remove_produces,
    remove_pure, remove_end, dup_sequence, dup_produces, extra_end, empty_proc."""
    if "sequence" not in src:
        return None
    if kind == "remove_sequence":
        # only fires when produces directly follows the block opener
        if not re.search(r"sequence( proc\[[^\]]*\])? produces", src):
            return None
        return SEQ_STYLE_RE.sub("", src, count=1).replace("\n\n    \n", "\n")
    if kind == "remove_produces":
        return re.sub(r"produces\s+pure", "pure", src, count=1)
    if kind == "remove_pure":
        return re.sub(r"produces(\s+)pure ", r"produces\1", src, count=1)
    if kind == "remove_end":
        m = re.search(r"pure ([^\n]+?) end\b", src)
        if m:
            return src[:m.start()] + f"pure {m.group(1)}" + src[m.end():]
        m = re.search(r"\n(\s*)end\n", src)
        if m:
            return src[:m.start()] + "\n" + src[m.end():]
        return None
    if kind == "dup_sequence":
        return re.sub(r"\bsequence\b", "sequence sequence", src, count=1)
    if kind == "dup_produces":
        return re.sub(r"\bproduces\b", "produces produces", src, count=1)
    if kind == "extra_end":
        m = re.search(r"pure ([^\n]+?) end\b", src)
        if m:
            return src[:m.end()] + " end" + src[m.end():]
        m = re.search(r"\n(\s*)end\n", src)
        if m:
            return src[:m.end()] + m.group(1) + "end\n" + src[m.end():]
        return None
    if kind == "empty_proc":
        # only meaningful on a pure sequence
        if re.search(r"sequence proc\[", src):
            return None
        return re.sub(r"\bsequence\b", "sequence proc[]", src, count=1)
    return None


def m_proc_on_return(src, rng):
    """Move proc[...] onto main's return type: the E3009 shape.  Only applied
    when the program has exactly one sequence with effects."""
    ms = list(re.finditer(r"sequence proc\[([^\]]*)\]", src))
    if len(ms) != 1:
        return None
    eff = ms[0].group(1)
    heads = list(FN_HEAD_RE.finditer(src))
    if not heads:
        return None
    # attach to the function that contains the sequence
    target = None
    for h in heads:
        if h.start() < ms[0].start():
            target = h
    if target is None:
        return None
    new_head = f"fn {target.group(1)}({target.group(2)}) -> {target.group(3)} proc[{eff}] {{"
    return src[:target.start()] + new_head + src[target.end():]


def m_wrong_effect(src, rng):
    """Swap the declared effect for a different one (E3002)."""
    m = re.search(r"sequence proc\[([^\]]*)\]", src)
    if not m:
        return None
    eff = m.group(1)
    if eff == "device_io":
        return src[:m.start()] + "sequence proc[mem(normal)]" + src[m.end():], "device_io"
    if eff == "mem(normal)":
        return src[:m.start()] + "sequence proc[device_io]" + src[m.end():], "mem(normal)"
    if eff == "concurrency":
        return src[:m.start()] + "sequence proc[device_io]" + src[m.end():], "concurrency"
    return None


def m_unused_effect(src, rng):
    """Declare an effect nothing in the block needs (E3010 effect variant)."""
    m = re.search(r"sequence proc\[([^\]]*)\]", src)
    if m:
        eff = m.group(1)
        if "mem(" in eff:
            return None
        return src[:m.start()] + f"sequence proc[{eff}, mem(normal)]" + src[m.end():], "mem(normal)"
    return None


def m_undefined_identifier(src, rng, names):
    """Replace one use of a bound name with a misspelling."""
    cands = []
    for name in names:
        for m in re.finditer(r"(?<![A-Za-z0-9_:.])" + re.escape(name) + r"(?![A-Za-z0-9_(])", src):
            # skip the binding site itself ("name: T <-") and comment lines
            after = src[m.end():m.end() + 2]
            if after.startswith(":"):
                continue
            line_start = src.rfind("\n", 0, m.start()) + 1
            if src[line_start:m.start()].lstrip().startswith("//"):
                continue
            cands.append((m, name))
    # a name must keep at least one other use, or the unused-binding check fires first
    from collections import Counter
    cnt = Counter(n for _, n in cands)
    cands = [(m, n) for m, n in cands if cnt[n] >= 2]
    if not cands:
        return None
    m, name = rng.choice(cands)
    bad = rng.choice([name + "s", name + "_", name[:-1] if len(name) > 2 else name + "x", name + "2", "missing_" + name])
    if bad == name:
        bad = name + "x"
    return src[:m.start()] + bad + src[m.end():], bad


def m_unknown_function(src, rng, fnames):
    cands = []
    for f in fnames:
        for m in re.finditer(r"(?<![A-Za-z0-9_@])" + re.escape(f) + r"\(", src):
            # skip the declaration 'fn f('
            if src[max(0, m.start() - 3):m.start()] == "fn " or in_comment(src, m.start()):
                continue
            cands.append((m, f))
    if not cands:
        return None
    m, f = rng.choice(cands)
    bad = rng.choice([f + "x", f + "_all", f[:-1] + "z" if len(f) > 3 else f + "2", "my_" + f])
    return src[:m.start()] + bad + "(" + src[m.end():], bad


def m_arg_count(src, rng, calls):
    """calls: list of (fname, nargs).  Drop or add one argument at a call site."""
    cands = []
    for f, n in calls:
        for m in re.finditer(r"(?<![A-Za-z0-9_@])" + re.escape(f) + r"\(([^()\n]*)\)", src):
            if src[max(0, m.start() - 3):m.start()] == "fn ":
                continue
            cands.append((m, f, n))
    if not cands:
        return None
    m, f, n = rng.choice(cands)
    args = [a.strip() for a in m.group(1).split(",")] if m.group(1).strip() else []
    if len(args) != n:
        return None
    if n == 0:
        return None
    args = args[:-1]
    return src[:m.start()] + f + "(" + ", ".join(args) + ")" + src[m.end():], f


def m_replace_once(src, old, new):
    i = src.find(old)
    if i < 0:
        return None
    return src[:i] + new + src[i + len(old):]


def m_replace_nth(src, old, new, rng):
    idx = [m.start() for m in re.finditer(re.escape(old), src)]
    if not idx:
        return None
    i = rng.choice(idx)
    return src[:i] + new + src[i + len(old):]


def m_dup_function(src, rng):
    fns = list(re.finditer(r"^fn ([A-Za-z_][A-Za-z0-9_]*)\([^\n]*\{\n(?:.*\n)*?\}\n", src, re.M))
    fns = [m for m in fns if m.group(1) != "main"]
    if not fns:
        return None
    m = rng.choice(fns)
    return src[:m.end()] + "\n" + m.group(0) + src[m.end():], m.group(1)


def m_brace(src, rng):
    kind = rng.choice(["missing", "extra"])
    s = src.rstrip("\n")
    if kind == "missing":
        if not s.endswith("}"):
            return None
        return s[:-1].rstrip() + "\n"
    return s + "}\n"


def m_rename_main(src, rng):
    new = rng.choice(["mainx", "start", "entry", "main_program"])
    return src.replace("fn main()", f"fn {new}()", 1), new


def m_struct_decl(src, rng):
    name = rng.choice(["Point", "Pair", "Config", "Node"])
    decl = f"struct {name} {{ x: int64, y: int64 }}\n\n"
    m = re.search(r"^fn ", src, re.M)
    if not m:
        return None
    return src[:m.start()] + decl + src[m.start():], name


def m_record_trailing_punct(src, rng):
    """Append a punctuation char after the last value of a one-line record literal."""
    ms = list(re.finditer(r"\{ ([a-z_]+: [^{}\n]+?) \}", src))
    ms = [m for m in ms if "<-" not in m.group(1) and ":" in m.group(1) and not in_comment(src, m.start())]
    if not ms:
        return None
    m = rng.choice(ms)
    ch = rng.choice(list("@!:.=>[(<-%+])/*"))
    return src[:m.end() - 2] + ch + " }" + src[m.end():], ch


def m_nine_params(src, rng, fname):
    """Rewrite function fname to take 9 int64 params."""
    m = re.search(r"^fn " + re.escape(fname) + r"\(([^)]*)\) -> ([^{\n]+?) \{\n((?:.*\n)*?)\}\n", src, re.M)
    if not m:
        return None
    names = [f"p{i}" for i in range(1, 10)]
    head = f"fn {fname}9(" + ", ".join(f"{n}: int64" for n in names) + ") -> int64 {\n    " + " + ".join(names) + "\n}\n\n"
    return src[:m.start()] + head + src[m.start():]


def m_case_drop_false(src, rng):
    ms = list(re.finditer(r"\n(\s*)false -> [^\n]*\n", src))
    ms = [m for m in ms if src[m.start() - 1] == ";"]
    if not ms:
        return None
    m = rng.choice(ms)
    # remove the false clause and the ';' that ended the previous clause
    return src[:m.start() - 1] + "\n" + src[m.end():]


def m_case_bad_pattern(src, rng, scrut_type):
    """Replace the first literal pattern with one of another type."""
    if scrut_type in INT_TYPES:
        m = re.search(r"\n(\s*)(-?\d+) -> ", src)
        if not m:
            return None
        bad = rng.choice(['"one"', ":one", "true"])
        return src[:m.start()] + f"\n{m.group(1)}{bad} -> " + src[m.end():]
    if scrut_type == "string":
        m = re.search(r'\n(\s*)"[^"\n]*" -> ', src)
        if not m:
            return None
        return src[:m.start()] + f"\n{m.group(1)}{rng.randint(0, 9)} -> " + src[m.end():]
    if scrut_type == "atom":
        m = re.search(r"\n(\s*):[a-z_]+ -> ", src)
        if not m:
            return None
        return src[:m.start()] + f"\n{m.group(1)}{rng.randint(0, 9)} -> " + src[m.end():]
    return None


def m_list_drop_space(src, rng):
    ms = list(re.finditer(r"List\[([^\[\],]+), mem\([a-z_]+\)\]", src))
    if not ms:
        return None
    m = rng.choice(ms)
    return src[:m.start()] + f"List[{m.group(1)}]" + src[m.end():]


def m_bind_print_result_int(src, rng):
    m = re.search(r"(\s*)_: atom <- (print_[a-z0-9]+\([^\n]*\))", src)
    if not m:
        m = re.search(r"(\s*)([a-z_]+): atom <- (print_[a-z0-9]+\([^\n]*\))", src)
        if not m:
            return None
        return None
    return src[:m.start()] + f"{m.group(1)}n_out: int64 <- {m.group(2)}" + src[m.end():]
