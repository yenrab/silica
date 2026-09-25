"""Generators for case_addition, functions_addition, recursive_function_addition,
sequence_block_addition, base, deep_frame_spill_addition, compiler_addition,
effect_check_addition."""
import random
from common import *
from numeric import int_expr, E
from data import s_lit, WORDS, L


def gen_all(rng, q):
    cases = []
    cases += gen_case(rng, q(320))
    cases += gen_functions(rng, q(300))
    cases += gen_recursive(rng, q(200))
    cases += gen_sequence(rng, q(140))
    cases += gen_base(rng, q(40))
    cases += gen_deep_frame(rng, q(20))
    cases += gen_compiler(rng, q(200))
    cases += gen_effect(rng, q(150))
    return cases


def ints(rng, n, lo=0, hi=40):
    return [rng.randint(lo, hi) for _ in range(n)]


# ---------------------------------------------------------------------------
# case_addition
# ---------------------------------------------------------------------------

def gen_case(rng, count):
    area = "case_addition"
    makers = [t_case_ranges, t_case_bool_nested, t_case_block_arms, t_case_binding_rhs, t_case_string_mode,
              t_case_atom_result, t_case_guard_fn, t_case_tuple_scrut, t_case_seq_arms, t_case_literal_mix]
    cases = []
    for i in range(count):
        c = makers[i % len(makers)](rng, area)
        if c:
            cases.append(c)
    return cases


def t_case_ranges(rng, area):
    n = rng.randint(0, 150)
    lo, hi = sorted(rng.sample(range(1, 120), 2))
    r1, r2, r3 = [rng.choice(["n * 2", "n + 100", "n - 1", "n", "0", "n / 2"]) for _ in range(3)]
    def ev(e, n):
        return eval(e.replace("/", "//"))
    val = ev(r1, n) if n < lo else ev(r2, n) if n < hi else ev(r3, n)
    if val < 0 or val > 255:
        return None
    lines = [f"n: int64 <- {n};"]
    tail = f"case n of {{\n        n: int64 if n < {lo} -> {r1};\n        n: int64 if n >= {lo} and n < {hi} -> {r2};\n        n: int64 if n >= {hi} -> {r3}\n    }}"
    hdr = f"// guards partition int64 into three ranges; n = {n} gives {val}"
    src = program(fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "ranges", src, "", val)
    c.info["names"] = ["n"]
    c.add_fail("E2003", "guard expression is an integer, not a boolean", rep(src, f"if n < {lo} ->", "if n ->", 1))
    c.add_fail("E2001", "string result in one arm of an integer case", rep(src, f"-> {r1};", f'-> "{r1}";', 1))
    return c


def t_case_bool_nested(rng, area):
    a, b = rng.choice([True, False]), rng.choice([True, False])
    tt, tf, ft, ff = ints(rng, 4, 0, 99)
    lit = lambda x: "true" if x else "false"
    lines = [f"a: boolean <- {lit(a)};", f"b: boolean <- {lit(b)};"]
    tail = f"case a of {{\n        true -> case b of {{\n            true -> {tt};\n            false -> {tf}\n        }};\n        false -> case b of {{\n            true -> {ft};\n            false -> {ff}\n        }}\n    }}"
    val = tt if a and b else tf if a else ft if b else ff
    hdr = f"// nested boolean cases; ({lit(a)}, {lit(b)}) selects {val}"
    src = program(fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "boolnest", src, "", val)
    c.info["names"] = ["a", "b"]
    c.add_fail("E2005", "inner boolean case missing its false arm", rep(src, f"            true -> {tt};\n            false -> {tf}\n", f"            true -> {tt}\n", 1))
    c.add_fail("E2001", "integer literal bound to a boolean", rep(src, f"a: boolean <- {lit(a)}", "a: boolean <- 1", 1))
    return c


def t_case_block_arms(rng, area):
    x = rng.randint(0, 5)
    k1, k2 = ints(rng, 2, 1, 20)
    lines = [f"x: int64 <- {x};"]
    tail = f"case x of {{\n        0 -> {{\n            base: int64 <- {k1};\n            base * 2\n        }}\n        _: int64 -> {{\n            step: int64 <- {k2};\n            x * step + 1\n        }}\n    }}"
    val = k1 * 2 if x == 0 else x * k2 + 1
    hdr = f"// case arms with multi-statement block bodies; expect {val}"
    src = program(fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "blockarms", src, "", val & 0xFF)
    c.info["names"] = ["x", "base", "step"]
    c.add_fail("E1041", "trailing ';' after a block arm's final expression", rep(src, "base * 2\n", "base * 2;\n", 1))
    c.add_fail("E1055", "block-local binding never used", rep(src, "base * 2", f"{k1} * 2", 1))
    return c


def t_case_binding_rhs(rng, area):
    x = rng.randint(0, 30)
    t1, t2 = ints(rng, 2, 0, 20)
    lines = [f"x: int64 <- {x};",
             f"label: int64 <- case x % 3 of {{\n        0 -> {t1};\n        1 -> {t2};\n        _: int64 -> x\n    }};"]
    val = t1 if x % 3 == 0 else t2 if x % 3 == 1 else x
    tail = "label + 1"
    hdr = f"// case expression as a binding's right-hand side; expect {val + 1}"
    src = program(fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "bindrhs", src, "", (val + 1) & 0xFF)
    c.info["names"] = ["x", "label"]
    c.add_fail("E2001", "string arm in a case bound to an int64", rep(src, f"1 -> {t2};", f'1 -> "{t2}";', 1))
    c.add_fail("E2005", "boolean literal pattern against an integer scrutinee", rep(src, f"0 -> {t1};", f"true -> {t1};", 1))
    return c


def t_case_string_mode(rng, area):
    modes = rng.sample(["verbose", "quiet", "debug", "trace", "silent"], 2)
    pick = rng.choice(modes + ["other"])
    m1, m2, d = ints(rng, 3, 0, 9)
    lines = [f"mode: string <- {s_lit(pick)};"]
    tail = (f"case mode of {{\n        {s_lit(modes[0])} -> {{\n            sequence proc[device_io]\n                print_string({s_lit(modes[0])});\n                println(\"\");\n                print_int64({m1});\n                println(\"\")\n            produces\n                pure :ok\n            end\n        }}\n"
            f"        {s_lit(modes[1])} -> {{\n            sequence proc[device_io]\n                print_int64({m2});\n                println(\"\")\n            produces\n                pure :ok\n            end\n        }}\n"
            f"        _: string -> {{\n            sequence proc[device_io]\n                print_string(\"default\");\n                println(\"\")\n            produces\n                pure :ok\n            end\n        }}\n    }}")
    out = f"{modes[0]}\n{m1}\n" if pick == modes[0] else f"{m2}\n" if pick == modes[1] else "default\n"
    hdr = "// string case with an effectful sequence in each arm"
    src = program(fn_decl("main", [], "atom", lines, tail), header=hdr)
    c = Case(area, "strmode", src, out, 2)
    c.info["names"] = ["mode"]
    c.info["fnames"] = ["print_string", "print_int64"]
    c.info["has_seq"] = True
    c.add_fail("E3002", "device_io used in an arm whose sequence declares mem(normal)",
               rep(src, "sequence proc[device_io]\n                print_int64", "sequence proc[mem(normal)]\n                print_int64", 1))
    c.add_fail("E2005", "integer pattern against a string scrutinee", rep(src, f"{s_lit(modes[1])} -> {{", "2 -> {", 1))
    return c


def t_case_atom_result(rng, area):
    n = rng.randint(0, 20)
    thr = rng.randint(1, 15)
    f = fn_decl("grade", [("n", "int64")], "atom", [],
                f"case n of {{\n        0 -> :zero;\n        k: int64 if k < {thr} -> :low;\n        _: int64 -> :high\n    }}")
    lines = [f"g: atom <- grade({n});"]
    tail = "case g of {\n        :zero -> 0;\n        :low -> 1;\n        :high -> 2;\n        _: atom -> 9\n    }"
    val = 0 if n == 0 else 1 if n < thr else 2
    hdr = f"// case producing atoms then case dispatching on them; grade({n}) -> {val}"
    src = program(f, fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "atomres", src, "", val)
    c.info["names"] = ["g", "n"]
    c.info["fnames"] = ["grade"]
    c.info["calls"] = [("grade", 1)]
    c.info["case_scrut"] = "atom"
    c.add_fail("E2001", "integer arm inside an atom-returning case", rep(src, "0 -> :zero;", "0 -> 0;", 1))
    c.add_fail("E2005", "string pattern against an atom scrutinee", rep(src, ":low -> 1;", '"low" -> 1;', 1))
    return c


def t_case_guard_fn(rng, area):
    z = rng.randint(-20, 40)
    f = fn_decl("is_positive", [("v", "int64")], "boolean", [], "v > 0")
    lines = [f"z: int64 <- {z};"]
    tail = "case z of {\n        z: int64 if is_positive(z) -> z * 2;\n        _: int64 -> 0\n    }"
    val = z * 2 if z > 0 else 0
    hdr = f"// guard calls a boolean helper; z = {z} gives {val}"
    src = program(f, fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "guardfn", src, "", val & 0xFF)
    c.info["names"] = ["z", "v"]
    c.info["fnames"] = ["is_positive"]
    c.info["calls"] = [("is_positive", 1)]
    c.add_fail("E2001", "helper declared boolean but returns an integer literal", rep(src, "v > 0\n}", "1\n}", 1))
    c.add_fail("E2003", "guard helper returns int64 rather than boolean", rep(src, ") -> boolean {", ") -> int64 {", 1).replace("v > 0\n}", "v\n}", 1))
    return c


def t_case_tuple_scrut(rng, area):
    a, b = ints(rng, 2, 0, 20)
    f = fn_decl("pair", [("n", "int64")], "(int64, int64)", [], "(n, n + 1)")
    tail = f"case pair({a}) of {{\n        t: (int64, int64) -> {{\n            (x: int64, y: int64) <- t;\n            x + y + {b}\n        }}\n    }}"
    val = 2 * a + 1 + b
    hdr = f"// case scrutinee is a tuple-returning call; expect {val}"
    src = program(f, fn_decl("main", [], "int64", [], tail), header=hdr)
    c = Case(area, "tuplescrut", src, "", val & 0xFF)
    c.info["names"] = ["t", "x", "y", "n"]
    c.info["fnames"] = ["pair"]
    c.info["calls"] = [("pair", 1)]
    c.add_fail("E2005", "tuple pattern annotated with the wrong arity", rep(src, "t: (int64, int64) ->", "t: (int64, int64, int64) ->", 1))
    return c


def t_case_seq_arms(rng, area):
    flag = rng.choice([True, False])
    n1, n2 = ints(rng, 2, 1, 9)
    lines = [f"flag: boolean <- {str(flag).lower()};"]
    body = (f"sequence proc[device_io]\n        v: int64 <- case flag of {{\n            true -> {n1};\n            false -> {n2}\n        }};\n        result: atom <- print_int64(v)\n    produces\n        pure result\n    end")
    hdr = f"// case inside a sequence chooses what to print"
    src = program(fn_decl("main", [], "atom", lines, body), header=hdr)
    c = Case(area, "seqarm", src, str(n1 if flag else n2), 0)
    c.info["names"] = ["flag", "v"]
    c.info["fnames"] = ["print_int64"]
    c.info["has_seq"] = True
    c.add_fail("E2001", "case arms disagree with the binding's int64 type", rep(src, f"false -> {n2}", f'false -> "{n2}"', 1))
    return c


def t_case_literal_mix(rng, area):
    ty = rng.choice(["int64", "uint8", "int32", "uint64"])
    v = rng.randint(0, 15)
    lits = rng.sample(range(0, 16), 3)
    outs = ints(rng, 3, 0, 50)
    guard = rng.randint(1, 15)
    gout = rng.randint(0, 50)
    d = rng.randint(0, 50)
    clauses = [f"{l} -> {o}" for l, o in zip(lits, outs)] + [f"m: {ty} if m > {guard} -> {gout}", f"_: {ty} -> {d}"]
    lines = [f"v: {ty} <- {v};"]
    tail = "case v of {\n        " + ";\n        ".join(clauses) + "\n    }"
    val = outs[lits.index(v)] if v in lits else gout if v > guard else d
    hdr = f"// literal patterns, a guard and a typed wildcard on {ty}; v = {v} gives {val}"
    src = program(fn_decl("main", [], ty, lines, tail), header=hdr)
    c = Case(area, "litmix", src, "", val & 0xFF)
    c.info["names"] = ["v", "m"]
    c.info["case_scrut"] = ty
    c.add_fail("E2005", "atom pattern against an integer scrutinee", rep(src, f"{lits[0]} -> {outs[0]}", f":{lits[0]} -> {outs[0]}", 1))
    return c


# ---------------------------------------------------------------------------
# functions_addition
# ---------------------------------------------------------------------------

def gen_functions(rng, count):
    area = "functions_addition"
    makers = [t_fn_params_n, t_fn_four, t_fn_lambda_param, t_fn_lambda_bound, t_fn_value_param,
              t_fn_bool_helpers, t_fn_string_helper, t_fn_mixed_types, t_fn_nullary]
    cases = []
    for i in range(count):
        c = makers[i % len(makers)](rng, area)
        if c:
            cases.append(c)
    return cases


def t_fn_params_n(rng, area):
    n = rng.randint(1, 8)
    names = ["a", "b", "c", "d", "e", "f", "g", "h"][:n]
    vals = ints(rng, n, 0, 25)
    ty = rng.choice(["int64", "int64", "int32", "uint16"])
    fname = f"sum{n}"
    f = fn_decl(fname, [(p, ty) for p in names], ty, [], " + ".join(names))
    val = sum(vals)
    if val > 255:
        return None
    call = f"{fname}({', '.join(map(str, vals))})"
    hdr = f"// function with {n} parameter(s); {call} = {val}"
    src = program(f, fn_decl("main", [], ty, [], call), header=hdr)
    c = Case(area, f"params{n}", src, "", val)
    c.info["names"] = names
    c.info["fnames"] = [fname]
    c.info["calls"] = [(fname, n)]
    if n == 8:
        c.add_fail("E3010", f"{fname} extended to nine parameters",
                   rep(src, f"h: {ty}) -> {ty} {{", f"h: {ty}, i: {ty}) -> {ty} {{", 1).replace(" + h\n", " + h + i\n", 1).replace(f", {vals[-1]})", f", {vals[-1]}, 1)", 1))
    c.add_fail("E2005", "call with one argument too few", rep(src, call, fname + "(" + ", ".join(map(str, vals[:-1])) + ")", 1) if n > 1 else None)
    return c


def t_fn_four(rng, area):
    a, b = rng.randint(1, 30), rng.randint(1, 9)
    fs = "\n\n".join([
        fn_decl("add", [("x", "int64"), ("y", "int64")], "int64", [], "x + y"),
        fn_decl("sub", [("x", "int64"), ("y", "int64")], "int64", [], "x - y"),
        fn_decl("mul", [("x", "int64"), ("y", "int64")], "int64", [], "x * y"),
        fn_decl("div", [("x", "int64"), ("y", "int64")], "int64", [], "x / y")])
    picks = rng.sample(["add", "sub", "mul", "div"], rng.randint(2, 4))
    terms, val = [], 0
    for p in picks:
        x, y = rng.randint(b, 30), rng.randint(1, 9)
        terms.append(f"{p}({x}, {y})")
        val += {"add": x + y, "sub": x - y, "mul": x * y, "div": x // y}[p]
    if val > 255:
        return None
    tail = " + ".join(terms)
    hdr = f"// four arithmetic helpers; {tail} = {val}"
    src = program(fs, fn_decl("main", [], "int64", [], tail + f"//{val}"), header=hdr)
    c = Case(area, "fourfns", src, "", val)
    c.info["names"] = ["x", "y"]
    c.info["fnames"] = picks
    c.info["calls"] = [(p, 2) for p in picks]
    c.add_fail("E4015", "the mul helper is defined twice", rep(src, "fn mul(x: int64, y: int64) -> int64 {\n    x * y\n}", "fn mul(x: int64, y: int64) -> int64 {\n    x * y\n}\n\nfn mul(x: int64, y: int64) -> int64 {\n    x * y\n}", 1))
    return c


def t_fn_lambda_param(rng, area):
    z = rng.randint(0, 30)
    n = rng.randint(1, 20)
    op = rng.choice(["a + b + z", "a * b + z", "a - b + z"])
    val = eval(op.replace("a", str(n)).replace("b", str(n)).replace("z", str(z)))
    if not (0 <= val <= 255):
        return None
    f = fn_decl("apply_pair", [("num", "int64"), ("op", "fn(a: int64, b: int64) -> int64")], "int64", [], "op(num, num)")
    lines = [f"z: int64 <- {z};"]
    tail = f"apply_pair({n}, fn(a: int64, b: int64) -> int64 {{\n        {op}\n    }})"
    hdr = f"// function literal passed as a parameter, capturing z; expect {val}"
    src = program(f, fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "lambdaparam", src, "", val)
    c.info["names"] = ["z", "num"]
    c.info["fnames"] = ["apply_pair"]
    c.info["calls"] = [("apply_pair", 2)]
    c.add_fail("E2005", "function literal called with one argument instead of two", rep(src, "op(num, num)", "op(num)", 1))
    c.add_fail("E2001", "function literal returns a string where int64 is declared", rep(src, f"        {op}\n    }})", '        "sum"\n    })', 1))
    return c


def t_fn_lambda_bound(rng, area):
    k = rng.randint(1, 5)
    n = rng.randint(0, 40)
    kind = rng.choice(["scale", "offset"])
    body = f"x * {k}" if kind == "scale" else f"x + {k}"
    val = n * k if kind == "scale" else n + k
    if val > 255:
        return None
    lines = [f"f: fn(int64) -> int64 <- fn(x: int64) -> int64 {{ {body} }};"]
    tail = f"f({n})"
    hdr = f"// function literal bound to a typed variable; f({n}) = {val}"
    src = program(fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "lambdabound", src, "", val)
    c.info["names"] = ["f", "x"]
    c.add_fail("E2005", "bound function called with no arguments", rep(src, f"f({n})", "f()", 1))
    c.add_fail("E2005", "calling an integer binding as a function", rep(src, "f: fn(int64) -> int64 <- fn(x: int64) -> int64 { " + body + " };", f"f: int64 <- {k};", 1))
    return c


def t_fn_value_param(rng, area):
    n = rng.randint(0, 40)
    helpers = "\n\n".join([fn_decl("twice", [("x", "int64")], "int64", [], "x * 2"),
                            fn_decl("succ", [("x", "int64")], "int64", [], "x + 1"),
                            fn_decl("apply", [("f", "fn(int64) -> int64"), ("v", "int64")], "int64", [], "f(v)")])
    pick = rng.choice(["twice", "succ"])
    val = n * 2 if pick == "twice" else n + 1
    if val > 255:
        return None
    tail = f"apply({pick}, {n})"
    hdr = f"// named function passed as a value; apply({pick}, {n}) = {val}"
    src = program(helpers, fn_decl("main", [], "int64", [], tail), header=hdr)
    c = Case(area, "fnvalue", src, "", val)
    c.info["names"] = ["x", "f", "v"]
    c.info["fnames"] = ["apply", "twice", "succ"]
    c.info["calls"] = [("apply", 2)]
    c.add_fail("E2005", "apply called with the function only", rep(src, f"apply({pick}, {n})", f"apply({pick})", 1))
    return c


def t_fn_make_adder(rng, area):
    k, n = rng.randint(1, 20), rng.randint(0, 40)
    f = fn_decl("make_adder", [("k", "int64")], "fn(int64) -> int64", [], "fn(x: int64) -> int64 { x + k }")
    lines = [f"add_k: fn(int64) -> int64 <- make_adder({k});"]
    tail = f"add_k({n})"
    val = k + n
    if val > 255:
        return None
    hdr = f"// closure returned from make_adder({k}); add_k({n}) = {val}"
    src = program(f, fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "closure", src, "", val)
    c.info["names"] = ["add_k", "k", "x"]
    c.info["fnames"] = ["make_adder"]
    c.info["calls"] = [("make_adder", 1)]
    c.add_fail("E2003", "closure bound with a mismatched function type", rep(src, "add_k: fn(int64) -> int64 <-", "add_k: fn(string) -> int64 <-", 1))
    return c


def t_fn_bool_helpers(rng, area):
    f = "\n\n".join([fn_decl("bit", [("b", "boolean"), ("w", "int64")], "int64", [], "case b of { true -> w; false -> 0 }"),
                     fn_decl("tt", [], "boolean", [], "true"), fn_decl("ff", [], "boolean", [], "false")])
    combos = [("tt() and tt()", True), ("tt() and ff()", False), ("ff() or tt()", True), ("ff() or ff()", False), ("tt() == ff()", False), ("ff() == ff()", True)]
    picks = rng.sample(combos, 3)
    terms = [f"bit({e}, {1 << i})" for i, (e, _) in enumerate(picks)]
    val = sum((1 << i) for i, (_, v) in enumerate(picks) if v)
    hdr = f"// boolean helpers combined into bits; expect {val}"
    src = program(f, fn_decl("main", [], "int64", [], " + ".join(terms)), header=hdr)
    c = Case(area, "boolfns", src, "", val)
    c.info["names"] = ["b", "w"]
    c.info["fnames"] = ["bit", "tt", "ff"]
    c.info["calls"] = [("bit", 2), ("tt", 0), ("ff", 0)]
    c.add_fail("E2001", "integer literal passed where bit expects a boolean", rep(src, terms[0], terms[0].replace(picks[0][0], "1", 1), 1))
    return c


def t_fn_string_helper(rng, area):
    w = rng.choice(WORDS)
    kind = rng.choice(["greet", "len2"])
    if kind == "greet":
        f = fn_decl("greet", [("name", "string")], "string", [], 'concatenate("hi ", name)')
        stmts = [f"print_string(greet({s_lit(w)}))", 'println("")']
        out = "hi " + w + "\n"
        body = seq_block(["device_io"], stmts, ":ok", rng.randint(0, 2))
        src = program(f, f"fn main() -> atom {{\n{body}\n}}", header=f"// string helper greet prints {out}")
        c = Case(area, "strfn", src, out, 2)
        c.info["has_seq"] = True
        c.add_fail("E2001", "greet returns an integer literal", rep(src, 'concatenate("hi ", name)', "42", 1))
    else:
        f = fn_decl("double_len", [("s", "string")], "int64", [], "length_chars(s) * 2")
        src = program(f, fn_decl("main", [], "int64", [], f"double_len({s_lit(w)})"), header=f"// double_len({s_lit(w)}) = {2 * len(w)}")
        c = Case(area, "strfn", src, "", 2 * len(w))
        c.add_fail("E2003", "string parameter used in integer arithmetic", rep(src, "length_chars(s) * 2", "s * 2", 1))
    c.info["names"] = ["name", "s"]
    c.info["fnames"] = ["greet", "double_len"]
    c.info["calls"] = [("greet", 1), ("double_len", 1)]
    return c


def t_fn_mixed_types(rng, area):
    n = rng.randint(0, 60)
    s = rng.choice(WORDS)
    b = rng.choice([True, False])
    f = fn_decl("describe", [("n", "int64"), ("s", "string"), ("flag", "boolean")], "int64", [],
                "case flag of {\n        true -> n + length_chars(s);\n        false -> n\n    }")
    val = n + len(s) if b else n
    call = f"describe({n}, {s_lit(s)}, {str(b).lower()})"
    hdr = f"// parameters of three different types; {call} = {val}"
    src = program(f, fn_decl("main", [], "int64", [], call), header=hdr)
    c = Case(area, "mixedparams", src, "", val & 0xFF)
    c.info["names"] = ["n", "s", "flag"]
    c.info["fnames"] = ["describe"]
    c.info["calls"] = [("describe", 3)]
    c.add_fail("E2001", "string literal in the int64 argument slot", rep(src, call, f"describe({s_lit(s)}, {s_lit(s)}, {str(b).lower()})", 1))
    return c


def t_fn_nullary(rng, area):
    k = rng.randint(0, 100)
    kind = rng.choice(["const", "chain"])
    if kind == "const":
        f = fn_decl("answer", [], "int64", [], str(k))
        src = program(f, fn_decl("main", [], "int64", [], "answer()"), header=f"// nullary function returning {k}")
        c = Case(area, "nullary", src, "", k)
        c.info["fnames"] = ["answer"]
        c.info["calls"] = [("answer", 0)]
        c.add_fail("E2005", "nullary function called with an argument", rep(src, "answer()", "answer(1)", 1))
    else:
        f = "\n\n".join([fn_decl("base", [], "int64", [], str(k)), fn_decl("next", [], "int64", [], "base() + 1")])
        src = program(f, fn_decl("main", [], "int64", [], "next() + base()"), header=f"// nullary functions calling each other; expect {2 * k + 1}")
        c = Case(area, "nullary", src, "", (2 * k + 1) & 0xFF)
        c.info["fnames"] = ["base", "next"]
        c.info["calls"] = [("base", 0), ("next", 0)]
        c.add_fail("E2003", "function name used as a value without calling it", rep(src, "next() + base()", "next + base()", 1))
    return c


# ---------------------------------------------------------------------------
# recursive_function_addition
# ---------------------------------------------------------------------------

def gen_recursive(rng, count):
    area = "recursive_function_addition"
    makers = [t_rec_factorial, t_rec_fib, t_rec_gcd, t_rec_power, t_rec_mutual, t_rec_mcc, t_rec_ackermann, t_rec_string, t_rec_collatz, t_rec_sum_digits]
    cases = []
    for i in range(count):
        c = makers[i % len(makers)](rng, area)
        if c:
            cases.append(c)
    return cases


def rec_case(cond, base, step):
    return f"case {cond} of {{\n        true -> {base};\n        false -> {step}\n    }}"


def t_rec_factorial(rng, area):
    n = rng.randint(0, 5)
    acc = rng.random() < 0.5
    import math
    val = math.factorial(n)
    if acc:
        f = fn_decl("factorial_acc", [("n", "int64"), ("acc", "int64")], "int64", [], rec_case("n <= 1", "acc", "factorial_acc(n - 1, acc * n)"))
        call = f"factorial_acc({n}, 1)"
        fname, ar = "factorial_acc", 2
    else:
        f = fn_decl("factorial", [("n", "int64")], "int64", [], rec_case("n <= 1", "1", "n * factorial(n - 1)"))
        call = f"factorial({n})"
        fname, ar = "factorial", 1
    hdr = f"// {'tail-recursive' if acc else 'direct'} factorial; {call} = {val}"
    src = program(f, fn_decl("main", [], "int64", [], call), header=hdr)
    c = Case(area, "factorial", src, "", val & 0xFF)
    c.info["names"] = ["n", "acc"]
    c.info["fnames"] = [fname]
    c.info["calls"] = [(fname, ar)]
    c.add_fail("E2005", "recursive call with the wrong argument count", rep(src, "factorial_acc(n - 1, acc * n)", "factorial_acc(n - 1)", 1) if acc else rep(src, "n * factorial(n - 1)", "n * factorial(n - 1, 1)", 1))
    c.add_fail("E2005", "boolean case without its false arm", m_case_drop_false(src, rng))
    return c


def t_rec_fib(rng, area):
    n = rng.randint(0, 12)
    fibs = [0, 1]
    for _ in range(12):
        fibs.append(fibs[-1] + fibs[-2])
    val = fibs[n]
    acc = rng.random() < 0.6
    if acc:
        f = fn_decl("fib_iter", [("n", "int64"), ("a", "int64"), ("b", "int64")], "int64", [], rec_case("n <= 0", "a", "fib_iter(n - 1, b, a + b)"))
        call, fname, ar = f"fib_iter({n}, 0, 1)", "fib_iter", 3
    else:
        n = min(n, 9)
        val = fibs[n]
        f = fn_decl("fib", [("n", "int64")], "int64", [], "case n of {\n        0 -> 0;\n        1 -> 1;\n        _: int64 -> fib(n - 1) + fib(n - 2)\n    }")
        call, fname, ar = f"fib({n})", "fib", 1
    hdr = f"// fibonacci; {call} = {val}"
    src = program(f, fn_decl("main", [], "int64", [], call), header=hdr)
    c = Case(area, "fib", src, "", val & 0xFF)
    c.info["names"] = ["n", "a", "b"]
    c.info["fnames"] = [fname]
    c.info["calls"] = [(fname, ar)]
    c.add_fail("E2002", "undefined accumulator name in the recursive call", rep(src, "fib_iter(n - 1, b, a + b)", "fib_iter(n - 1, b, a + c)", 1) if acc else rep(src, "fib(n - 1) + fib(n - 2)", "fib(m - 1) + fib(n - 2)", 1))
    return c


def t_rec_gcd(rng, area):
    import math
    a, b = rng.randint(1, 120), rng.randint(1, 120)
    f = fn_decl("gcd", [("a", "int64"), ("b", "int64")], "int64", [], rec_case("b == 0", "a", "gcd(b, a % b)"))
    call = f"gcd({a}, {b})"
    val = math.gcd(a, b)
    hdr = f"// Euclid's gcd; {call} = {val}"
    src = program(f, fn_decl("main", [], "int64", [], call), header=hdr)
    c = Case(area, "gcd", src, "", val)
    c.info["names"] = ["a", "b"]
    c.info["fnames"] = ["gcd"]
    c.info["calls"] = [("gcd", 2)]
    c.add_fail("E2001", "atom returned from the base case of gcd", rep(src, "true -> a;", "true -> :done;", 1))
    return c


def t_rec_power(rng, area):
    base, e = rng.randint(1, 6), rng.randint(0, 5)
    val = base ** e
    if val > 255:
        return None
    f = fn_decl("power", [("b", "int64"), ("e", "int64")], "int64", [], rec_case("e == 0", "1", "b * power(b, e - 1)"))
    call = f"power({base}, {e})"
    hdr = f"// {call} = {val}"
    src = program(f, fn_decl("main", [], "int64", [], call), header=hdr)
    c = Case(area, "power", src, "", val)
    c.info["names"] = ["b", "e"]
    c.info["fnames"] = ["power"]
    c.info["calls"] = [("power", 2)]
    c.add_fail("E2003", "exponent parameter declared as a string", rep(src, '("b", "int64"), ("e", "int64")', "", 1).replace("e: int64) -> int64 {", "e: string) -> int64 {", 1))
    return c


def t_rec_mutual(rng, area):
    n, m = rng.randint(0, 15), rng.randint(0, 15)
    f = "\n\n".join([fn_decl("even", [("n", "int64")], "int64", [], rec_case("n == 0", "1", "odd(n - 1)")),
                     fn_decl("odd", [("n", "int64")], "int64", [], rec_case("n == 0", "0", "even(n - 1)"))])
    val = (1 if n % 2 == 0 else 0) + (1 if m % 2 == 1 else 0)
    call = f"even({n}) + odd({m})"
    hdr = f"// mutual recursion even/odd; {call} = {val}"
    src = program(f, fn_decl("main", [], "int64", [], call), header=hdr)
    c = Case(area, "mutual", src, "", val)
    c.info["names"] = ["n"]
    c.info["fnames"] = ["even", "odd"]
    c.info["calls"] = [("even", 1), ("odd", 1)]
    c.add_fail("E2005", "call to an undefined third helper", rep(src, "odd(n - 1)", "oddly(n - 1)", 1))
    return c


def t_rec_mcc(rng, area):
    n = rng.randint(1, 120)
    f = fn_decl("mcc91", [("n", "int64")], "int64", [], rec_case("n > 100", "n - 10", "mcc91(mcc91(n + 11))"))
    val = 91 if n <= 100 else n - 10
    hdr = f"// McCarthy 91; mcc91({n}) = {val}"
    src = program(f, fn_decl("main", [], "int64", [], f"mcc91({n})"), header=hdr)
    c = Case(area, "mcc91", src, "", val)
    c.info["names"] = ["n"]
    c.info["fnames"] = ["mcc91"]
    c.info["calls"] = [("mcc91", 1)]
    return c


def t_rec_ackermann(rng, area):
    m, n = rng.choice([(0, 3), (1, 2), (1, 4), (2, 1), (2, 2), (2, 3), (3, 1), (3, 2), (3, 3)])
    def ack(m, n):
        return n + 1 if m == 0 else ack(m - 1, 1) if n == 0 else ack(m - 1, ack(m, n - 1))
    val = ack(m, n)
    f = "\n\n".join([fn_decl("ack_nonzero", [("m", "int64"), ("n", "int64")], "int64", [], rec_case("n == 0", "ack(m - 1, 1)", "ack(m - 1, ack(m, n - 1))")),
                     fn_decl("ack", [("m", "int64"), ("n", "int64")], "int64", [], rec_case("m == 0", "n + 1", "ack_nonzero(m, n)"))])
    hdr = f"// Ackermann A({m}, {n}) = {val}"
    src = program(f, fn_decl("main", [], "int64", [], f"ack({m}, {n})"), header=hdr)
    c = Case(area, "ackermann", src, "", val & 0xFF)
    c.info["names"] = ["m", "n"]
    c.info["fnames"] = ["ack", "ack_nonzero"]
    c.info["calls"] = [("ack", 2), ("ack_nonzero", 2)]
    return c


def t_rec_string(rng, area):
    w = rng.choice(WORDS + ["", "a"])
    kind = rng.choice(["len", "count_char"])
    if kind == "len":
        f = fn_decl("str_len_acc", [("s", "string"), ("acc", "int64")], "int64", [],
                    rec_case("length_chars(s) == 0", "acc", "str_len_acc(substring(s, 1, length_chars(s)), acc + 1)"))
        call, val, fname, ar = f"str_len_acc({s_lit(w)}, 0)", len(w), "str_len_acc", 2
    else:
        ch = rng.choice(["a", "e", "l", "o"])
        f = "\n\n".join([fn_decl("count_rec", [("s", "string")], "int64", [], rec_case("length_chars(s) == 0", "0", "count_step(s)")),
                         fn_decl("count_step", [("s", "string")], "int64", ["head: string <- substring(s, 0, 1);"],
                                 rec_case(f'head == "{ch}"', "1 + count_rec(substring(s, 1, length_chars(s)))", "count_rec(substring(s, 1, length_chars(s)))"))])
        call, val, fname, ar = f"count_rec({s_lit(w)})", w.count(ch), "count_rec", 1
    hdr = f"// string recursion via substring; {call} = {val}"
    src = program(f, fn_decl("main", [], "int64", [], call), header=hdr)
    c = Case(area, "strrec", src, "", val)
    c.info["names"] = ["s", "acc", "head"]
    c.info["fnames"] = [fname]
    c.info["calls"] = [(fname, ar)]
    c.add_fail("E2003", "string parameter compared against an integer", rep(src, "length_chars(s) == 0", "s == 0", 1))
    return c


def t_rec_collatz(rng, area):
    n = rng.randint(1, 30)
    def steps(n):
        s = 0
        while n != 1:
            n = n // 2 if n % 2 == 0 else 3 * n + 1
            s += 1
        return s
    val = steps(n)
    f = fn_decl("collatz", [("n", "int64"), ("acc", "int64")], "int64", [],
                "case n == 1 of {\n        true -> acc;\n        false -> case n % 2 == 0 of {\n            true -> collatz(n / 2, acc + 1);\n            false -> collatz(3 * n + 1, acc + 1)\n        }\n    }")
    hdr = f"// collatz steps for {n} = {val}"
    src = program(f, fn_decl("main", [], "int64", [], f"collatz({n}, 0)"), header=hdr)
    c = Case(area, "collatz", src, "", val & 0xFF)
    c.info["names"] = ["n", "acc"]
    c.info["fnames"] = ["collatz"]
    c.info["calls"] = [("collatz", 2)]
    c.add_fail("E2005", "inner boolean case lacks its false arm", rep(src, "            true -> collatz(n / 2, acc + 1);\n            false -> collatz(3 * n + 1, acc + 1)\n", "            true -> collatz(n / 2, acc + 1)\n", 1))
    return c


def t_rec_sum_digits(rng, area):
    n = rng.randint(0, 9999)
    val = sum(int(d) for d in str(n))
    f = fn_decl("digit_sum", [("n", "int64")], "int64", [], rec_case("n < 10", "n", "n % 10 + digit_sum(n / 10)"))
    hdr = f"// digit_sum({n}) = {val}"
    src = program(f, fn_decl("main", [], "int64", [], f"digit_sum({n})"), header=hdr)
    c = Case(area, "digitsum", src, "", val)
    c.info["names"] = ["n"]
    c.info["fnames"] = ["digit_sum"]
    c.info["calls"] = [("digit_sum", 1)]
    return c


# ---------------------------------------------------------------------------
# sequence_block_addition
# ---------------------------------------------------------------------------

def gen_sequence(rng, count):
    area = "sequence_block_addition"
    makers = [t_seq_pure, t_seq_bound, t_seq_statement, t_seq_io_then_value, t_seq_in_helper]
    cases = []
    for i in range(count):
        c = makers[i % len(makers)](rng, area)
        if c:
            cases.append(c)
    return cases


def t_seq_pure(rng, area):
    k = rng.randint(0, 200)
    kind = rng.choice(["oneline", "binding", "two"])
    if kind == "oneline":
        body = f"    sequence produces pure {k} end"
        val = k
    elif kind == "binding":
        body = seq_block([], [f"result: int64 <- {k}"], "result", 0)
        val = k
    else:
        a, b = rng.randint(0, 50), rng.randint(0, 50)
        body = seq_block([], [f"a: int64 <- {a}", f"b: int64 <- {b}", "total: int64 <- a + b"], "total", rng.choice([0, 2]))
        val = a + b
    hdr = f"// pure sequence block ({kind}); expect {val}"
    src = program(f"fn main() -> int64 {{\n{body}\n}}", header=hdr)
    c = Case(area, "pure", src, "", val & 0xFF)
    c.info["names"] = ["result", "a", "b", "total"]
    c.info["has_seq"] = True
    c.add_fail("E1056", "empty proc[] on a pure sequence", rep(src, "sequence", "sequence proc[]", 1) if kind != "oneline" else None)
    return c


def t_seq_bound(rng, area):
    k, d = rng.randint(0, 100), rng.randint(0, 50)
    lines = [f"value: int64 <- sequence produces pure {k} end;"]
    tail = f"value + {d}"
    hdr = f"// sequence result bound to a variable; expect {k + d}"
    src = program(fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "bound", src, "", (k + d) & 0xFF)
    c.info["names"] = ["value"]
    c.info["has_seq"] = True
    c.add_fail("E2001", "sequence produces a string but is bound to int64", rep(src, f"pure {k} end", f'pure "{k}" end', 1))
    return c


def t_seq_nested(rng, area):
    a, b = rng.randint(0, 60), rng.randint(0, 60)
    body = f"    sequence\n        inner: int64 <- sequence\n            x: int64 <- {a}\n        produces\n            pure x\n        end;\n        y: int64 <- {b}\n    produces\n        pure inner + y\n    end"
    hdr = f"// nested pure sequences; expect {a + b}"
    src = program(f"fn main() -> int64 {{\n{body}\n}}", header=hdr)
    c = Case(area, "nested", src, "", (a + b) & 0xFF)
    c.info["names"] = ["inner", "x", "y"]
    c.info["has_seq"] = True
    c.add_fail("E1040", "missing ';' after the inner sequence's end", rep(src, "        end;\n", "        end\n", 1))
    return c


def t_seq_statement(rng, area):
    k = rng.randint(0, 100)
    w = rng.choice(WORDS)
    lines = [f"sequence proc[device_io]\n        print_string({s_lit(w)});\n        println(\"\")\n    produces pure :ok end;"]
    hdr = f"// effectful sequence used as a statement before the return value {k}"
    src = program(fn_decl("main", [], "int64", lines, str(k)), header=hdr)
    c = Case(area, "stmt", src, w + "\n", k)
    c.info["fnames"] = ["print_string"]
    c.info["has_seq"] = True
    c.add_fail("E1040", "missing ';' after the sequence statement", rep(src, "produces pure :ok end;", "produces pure :ok end", 1))
    return c


def t_seq_io_then_value(rng, area):
    a, b = rng.randint(0, 40), rng.randint(0, 40)
    stmts = [f"a: int64 <- {a}", f"b: int64 <- {b}", "print_int64(a + b)", 'println("")', "total: int64 <- a + b"]
    body = seq_block(["device_io"], stmts, "total", rng.randint(0, 2))
    hdr = f"// sequence prints and returns the same sum {a + b}"
    src = program(f"fn main() -> int64 {{\n{body}\n}}", header=hdr)
    c = Case(area, "iovalue", src, str(a + b) + "\n", (a + b) & 0xFF)
    c.info["names"] = ["a", "b", "total"]
    c.info["fnames"] = ["print_int64"]
    c.info["has_seq"] = True
    c.add_fail("E2003", "sum bound as a boolean", rep(src, "total: int64 <- a + b", "total: boolean <- a + b", 1))
    return c


def t_seq_in_helper(rng, area):
    n = rng.randint(1, 9)
    f = fn_decl("churn", [("n", "int64")], "int64", [],
                "sequence\n        a: string <- concatenate(\"xx\", \"yy\");\n        b: string <- concatenate(a, a)\n    produces\n        pure length_bytes(b) + n\n    end")
    hdr = f"// pure sequence inside a helper; churn({n}) = {8 + n}"
    src = program(f, fn_decl("main", [], "int64", [], f"churn({n})"), header=hdr)
    c = Case(area, "helperseq", src, "", 8 + n)
    c.info["names"] = ["a", "b", "n"]
    c.info["fnames"] = ["churn", "concatenate", "length_bytes"]
    c.info["calls"] = [("churn", 1)]
    c.info["has_seq"] = True
    c.add_fail("E1049", "sequence without produces", rep(src, "    produces\n        pure length_bytes(b) + n", "        pure length_bytes(b) + n", 1))
    return c


# ---------------------------------------------------------------------------
# base + deep frame
# ---------------------------------------------------------------------------

def gen_base(rng, count):
    area = "base"
    cases = []
    for i in range(count):
        k = rng.randint(0, 255)
        style = i % 4
        if style == 0:
            src = program(f"fn main() -> int64 {{ {k} }}", header=f"// minimal program returning {k}")
        elif style == 1:
            src = program(fn_decl("main", [], "int64", [], str(k)), header=f"// minimal program returning {k}")
        elif style == 2:
            a = rng.randint(0, k)
            src = program(fn_decl("main", [], "int64", [], f"{a} + {k - a}"), header=f"// {a} + {k - a} = {k}")
        else:
            src = program(fn_decl("main", [], "int64", [f"v: int64 <- {k};"], "v"), header=f"// bind then return {k}")
        c = Case(area, "min", src, "", k)
        c.info["names"] = ["v"]
        c.add_fail("E2001", "string literal returned from main", rep(src, f"{k} }}", f'"{k}" }}', 1) if style == 0 else None)
        cases.append(c)
    return cases


def gen_deep_frame(rng, count):
    area = "deep_frame_spill_addition"
    cases = []
    for i in range(count):
        n = rng.choice([20, 24, 28, 32, 36, 40])
        f = fn_decl("id", [("x", "int64")], "int64", [], "x")
        inner = "\n".join(f"            v{j:02d}: int64 <- id({j});" for j in range(1, n + 1))
        total = n * (n + 1) // 2
        body = (f"    sequence proc[device_io]\n        sum: int64 <- {{\n{inner}\n            " + " + ".join(f"v{j:02d}" for j in range(1, n + 1)) +
                f"\n        }};\n        result: atom <- print_int64(sum)\n    produces\n        pure result\n    end")
        hdr = f"// {n} live values across calls force frame spills; prints {total}"
        src = program(f, f"fn main() -> atom {{\n{body}\n}}", header=hdr)
        c = Case(area, "spill", src, str(total), 0)
        c.info["names"] = ["sum"]
        c.info["fnames"] = ["id", "print_int64"]
        c.info["has_seq"] = True
        c.add_fail("E2002", "one live value referenced under a misspelled name", rep(src, "v01 + v02", "v01 + v0_2", 1))
        cases.append(c)
    return cases


# ---------------------------------------------------------------------------
# compiler_addition: mixed regression-style programs
# ---------------------------------------------------------------------------

def gen_compiler(rng, count):
    area = "compiler_addition"
    makers = [t_cmp_comments, t_cmp_logic_bits, t_cmp_typed_string_block, t_cmp_list_fold_concat, t_cmp_or_continued,
              t_cmp_seq_literal_rhs, t_cmp_case_string_escape, t_cmp_list_fields, t_cmp_region_case_arm, t_cmp_bool_alias]
    cases = []
    for i in range(count):
        c = makers[i % len(makers)](rng, area)
        if c:
            cases.append(c)
    return cases


def t_cmp_comments(rng, area):
    a, b, k = rng.randint(1, 40), rng.randint(1, 9), rng.randint(0, 20)
    val = a // b + k
    src = program(f"// whole-line comment before the function\nfn main() -> int64 {{\n    a: int64 <- {a}; // end-of-line after a binding\n    // whole-line comment between statements\n    b: int64 <- {b}; // another\n    a / b + {k} // end-of-line after the return expression\n}}",
                  header=f"// comments must stop at the newline; want {val}")
    c = Case(area, "comments", src, "", val & 0xFF)
    c.info["names"] = ["a", "b"]
    return c


def t_cmp_logic_bits(rng, area):
    f = "\n\n".join([fn_decl("tt", [], "bool", [], "true"), fn_decl("ff", [], "bool", [], "false"),
                     fn_decl("bit", [("b", "bool"), ("w", "int64")], "int64", [], "case b of { true -> w; false -> 0 }")])
    combos = [("tt() and tt()", True), ("tt() and ff()", False), ("ff() and tt()", False), ("ff() and ff()", False),
              ("tt() or tt()", True), ("tt() or ff()", True), ("ff() or tt()", True), ("ff() or ff()", False)]
    picks = rng.sample(combos, rng.randint(3, 6))
    lines = [f"r{i}: int64 <- bit({e}, {1 << i});" for i, (e, _) in enumerate(picks)]
    val = sum(1 << i for i, (_, v) in enumerate(picks) if v)
    tail = " + ".join(f"r{i}" for i in range(len(picks)))
    hdr = f"// and/or truth table bits (bool alias accepted); want {val}"
    src = program(f, fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "logic", src, "", val)
    c.info["names"] = [f"r{i}" for i in range(len(picks))] + ["b", "w"]
    c.info["fnames"] = ["bit", "tt", "ff"]
    c.info["calls"] = [("bit", 2), ("tt", 0), ("ff", 0)]
    c.add_fail("E2001", "integer literal operand to and", rep(src, picks[0][0], "1 and tt()", 1))
    return c


def t_cmp_typed_string_block(rng, area):
    w = rng.choice(WORDS)
    ch = w[0]
    f = fn_decl("first_char", [("s", "string")], "string", [],
                'case s == "" of {\n        false -> string {\n            c: string <- substring(s, 0, 1);\n            c\n        };\n        true -> ""\n    }')
    tail = f"case first_char({s_lit(w)}) == {s_lit(ch)} of {{\n        true -> 1;\n        false -> 0\n    }}"
    hdr = "// typed `string { ... }` case arm body; want 1"
    src = program(f, fn_decl("main", [], "int64", [], tail), header=hdr)
    c = Case(area, "typedblock", src, "", 1)
    c.info["names"] = ["s", "c"]
    c.info["fnames"] = ["first_char"]
    c.info["calls"] = [("first_char", 1)]
    c.add_fail("E2001", "integer literal in the string-typed arm", rep(src, 'true -> ""', "true -> 0", 1))
    return c


def t_cmp_list_fold_concat(rng, area):
    words = [rng.choice(WORDS) for _ in range(rng.randint(1, 4))]
    f = fn_decl("build", [("xs", L("string"))], "string", [],
                f"sequence proc[mem(normal)]\n        result: string <- case xs of {{\n            []: {L('string')} -> \"\";\n            [h: string, t: {L('string')}] -> concat(h, build(t));\n            _: {L('string')} -> \"\"\n        }}\n    produces\n        pure result\n    end")
    body = f"    sequence proc[mem(normal)]\n        xs: {L('string')} <- [{', '.join(s_lit(w) for w in words)}];\n        s: string <- build(xs)\n    produces\n        pure length_bytes(s)\n    end"
    val = sum(len(w.encode()) for w in words)
    hdr = f"// fold a string list with concat; byte length {val}"
    src = program(f, f"fn main() -> int64 {{\n{body}\n}}", header=hdr)
    c = Case(area, "foldconcat", src, "", val & 0xFF)
    c.info["names"] = ["xs", "s", "result", "h", "t"]
    c.info["fnames"] = ["build", "concat"]
    c.info["calls"] = [("build", 1)]
    c.info["has_seq"] = True
    c.info["has_list"] = True
    c.add_fail("E2015", "list parameter without mem(Space)", rep(src, f"xs: {L('string')}) -> string", "xs: List[string]) -> string", 1))
    return c


def t_cmp_or_continued(rng, area):
    a, b = rng.randint(0, 9), rng.randint(0, 9)
    t = rng.randint(0, 9)
    lines = [f"a: int64 <- {a};", f"b: int64 <- {b};",
             f"hit: boolean <- a > {t} or\n\n        b > {t};"]
    tail = "case hit of {\n        true -> 1;\n        false -> 0\n    }"
    val = 1 if (a > t or b > t) else 0
    hdr = "// line-ending `or` followed by a blank line stays one expression"
    src = program(fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "orcont", src, "", val)
    c.info["names"] = ["a", "b", "hit"]
    c.add_fail("E2003", "or applied to an integer operand", rep(src, f"hit: boolean <- a > {t} or", "hit: boolean <- a or", 1))
    return c


def t_cmp_seq_literal_rhs(rng, area):
    k = rng.randint(0, 200)
    body = f"    sequence\n        result: int64 <- {k}\n    produces\n        pure result\n    end"
    hdr = f"// sequence binding RHS literal then produces with no semicolon; want {k}"
    src = program(f"fn main() -> int64 {{\n{body}\n}}", header=hdr)
    c = Case(area, "seqlit", src, "", k)
    c.info["names"] = ["result"]
    c.info["has_seq"] = True
    c.add_fail("E1041", "semicolon after the last binding before produces", rep(src, f"result: int64 <- {k}\n", f"result: int64 <- {k};\n", 1))
    return c


def t_cmp_case_string_escape(rng, area):
    kind = rng.choice(["newline", "quote", "tab"])
    esc = {"newline": "\\n", "quote": '\\"', "tab": "\\t"}[kind]
    lines = [f's: string <- "{esc}";']
    tail = f'case s of {{\n        "{esc}" -> 1;\n        _: string -> 0\n    }}'
    hdr = f"// case patterns unescape {kind} like string expressions; want 1"
    src = program(fn_decl("main", [], "int64", lines, tail), header=hdr)
    c = Case(area, "strescape", src, "", 1)
    c.info["names"] = ["s"]
    c.info["case_scrut"] = "string"
    c.add_fail("E2005", "integer pattern against a string scrutinee", rep(src, f'"{esc}" -> 1;', "1 -> 1;", 1))
    return c


def t_cmp_list_fields(rng, area):
    xs = ints(rng, rng.randint(1, 5), 0, 50)
    body = f"    sequence proc[mem(normal)]\n        xs: {L('int64')} <- [{', '.join(map(str, xs))}];\n        r: int64 <- case xs.is_nil of {{\n            true -> 0;\n            false -> xs.head\n        }}\n    produces\n        pure r\n    end"
    hdr = f"// list field accessors is_nil/head; want {xs[0]}"
    src = program(f"fn main() -> int64 {{\n{body}\n}}", header=hdr)
    c = Case(area, "listfields", src, "", xs[0])
    c.info["names"] = ["xs", "r"]
    c.info["has_seq"] = True
    c.info["has_list"] = True
    c.add_fail("E2005", "unknown list field", rep(src, "xs.head", "xs.first", 1))
    return c


def t_cmp_region_case_arm(rng, area):
    n = rng.randint(1, 99)
    f = fn_decl("pick", [("r", "region(L1, normal)"), ("n", "int64")], "int64", [],
                "sequence proc[mem(normal)]\n        result: int64 <- case n > 0 of {\n            true -> {\n                cell: ref(L1, normal, int64) <- alloc_ref(r, n);\n                read_ref(cell)\n            };\n            false -> 0\n        }\n    produces\n        pure result\n    end")
    body = f"    sequence proc[mem(normal)]\n        L1: lifetime <- fresh_lifetime();\n        r: region(L1, normal) <- alloc_region(normal);\n        v: int64 <- pick(r, {n})\n    produces\n        pure v\n    end"
    hdr = f"// region handle passed to a helper that allocates inside a case arm; want {n}"
    src = program(f, f"fn main() -> int64 {{\n{body}\n}}", header=hdr)
    c = Case(area, "regionarm", src, "", n)
    c.info["names"] = ["r", "n", "v", "cell", "result"]
    c.info["fnames"] = ["pick"]
    c.info["calls"] = [("pick", 2)]
    c.info["has_seq"] = True
    c.add_fail("E3002", "region allocation under a device_io-only sequence", rep(src, "sequence proc[mem(normal)]\n        L1", "sequence proc[device_io]\n        L1", 1))
    return c


def t_cmp_bool_alias(rng, area):
    a, b = rng.choice([True, False]), rng.choice([True, False])
    f = fn_decl("both", [("x", "bool"), ("y", "boolean")], "bool", [], "x and y")
    tail = f"case both({str(a).lower()}, {str(b).lower()}) of {{\n        true -> 1;\n        false -> 0\n    }}"
    hdr = "// bool and boolean name the same type"
    src = program(f, fn_decl("main", [], "int64", [], tail), header=hdr)
    c = Case(area, "boolalias", src, "", 1 if (a and b) else 0)
    c.info["names"] = ["x", "y"]
    c.info["fnames"] = ["both"]
    c.info["calls"] = [("both", 2)]
    c.add_fail("E2001", "integer literal passed to a bool parameter", rep(src, f"both({str(a).lower()},", "both(1,", 1))
    return c


# ---------------------------------------------------------------------------
# effect_check_addition
# ---------------------------------------------------------------------------

def gen_effect(rng, count):
    area = "effect_check_addition"
    makers = [t_eff_io, t_eff_mem, t_eff_both, t_eff_nested_case, t_eff_helper_pick, t_eff_mem_in_arms, t_eff_two_seqs]
    cases = []
    for i in range(count):
        c = makers[i % len(makers)](rng, area)
        if c:
            cases.append(c)
    return cases


def t_eff_io(rng, area):
    k = rng.randint(0, 99)
    body = seq_block(["device_io"], [f"result: atom <- print_int64({k})"], "result", rng.randint(0, 2))
    hdr = "// Positive: device_io use belongs in sequence proc[device_io] (spec §9.3.1)"
    src = program(f"fn main() -> atom {{\n{body}\n}}", header=hdr)
    c = Case(area, "io", src, str(k), 0)
    c.info["fnames"] = ["print_int64"]
    c.info["has_seq"] = True
    c.add_fail("E3001", "print_int64 outside any sequence", program(f"fn main() -> atom {{\n    print_int64({k})\n}}", header=hdr))
    c.add_fail("E3002", "device_io used under proc[mem(normal)]", rep(src, "proc[device_io]", "proc[mem(normal)]", 1))
    c.add_fail("E3010", "concurrency declared but unused", rep(src, "proc[device_io]", "proc[device_io, concurrency]", 1))
    return c


def t_eff_mem(rng, area):
    xs = ints(rng, rng.randint(1, 5))
    sp = rng.choice(["normal", "normal_writethrough", "normal_writeback"])
    body = seq_block([f"mem({sp})"], [f"xs: {L('int64', sp)} <- [{', '.join(map(str, xs))}]", f"n: int64 <- length[int64, mem({sp})](xs)"], "n", rng.randint(0, 2))
    hdr = f"// Positive: list allocation requires sequence proc[mem({sp})] (spec §9.3.1)"
    src = program(f"fn main() -> int64 {{\n{body}\n}}", header=hdr)
    c = Case(area, "mem", src, "", len(xs))
    c.info["names"] = ["xs", "n"]
    c.info["fnames"] = ["length"]
    c.info["has_seq"] = True
    c.info["has_list"] = True
    if sp == "normal":
        c.add_fail("E3002", f"mem({sp}) used under proc[device_io]", rep(src, f"proc[mem({sp})]", "proc[device_io]", 1))
    c.add_fail("E3001", "list built outside any sequence", program(f"fn main() -> int64 {{\n    xs: {L('int64', sp)} <- [{', '.join(map(str, xs))}];\n    length[int64, mem({sp})](xs)\n}}", header=hdr))
    return c


def t_eff_both(rng, area):
    xs = ints(rng, rng.randint(1, 5))
    order = rng.choice(["device_io, mem(normal)", "mem(normal), device_io"])
    body = seq_block([order], [f"xs: {L('int64')} <- [{', '.join(map(str, xs))}]", "n: int64 <- length[int64, mem(normal)](xs)", "result: atom <- print_int64(n)"], "result", rng.randint(0, 2))
    hdr = "// Positive: both device_io and mem(normal) declared and used in one sequence"
    src = program(f"fn main() -> atom {{\n{body}\n}}", header=hdr)
    c = Case(area, "both", src, str(len(xs)), 0)
    c.info["names"] = ["xs", "n"]
    c.info["fnames"] = ["length", "print_int64"]
    c.info["has_seq"] = True
    c.info["has_list"] = True
    c.add_fail("E3002", "device_io dropped from the declaration", rep(src, f"proc[{order}]", "proc[mem(normal)]", 1))
    c.add_fail("E3002", "mem(normal) dropped from the declaration", rep(src, f"proc[{order}]", "proc[device_io]", 1))
    return c


def t_eff_nested_case(rng, area):
    outer, inner = rng.randint(0, 2), rng.randint(0, 2)
    a, b, d = ints(rng, 3, 0, 9)
    lines = [f"outer: int64 <- {outer};", f"inner: int64 <- {inner};"]
    body = (f"sequence proc[device_io]\n        result: atom <- case outer of {{\n            0 -> print_int64({a});\n            _: int64 -> case inner of {{\n                2 -> print_int64({b});\n                _: int64 -> print_int64({d})\n            }}\n        }}\n    produces\n        pure result\n    end")
    val = a if outer == 0 else b if inner == 2 else d
    hdr = "// Positive: device_io only in a nested case arm inside sequence proc[device_io] still counts as used"
    src = program(fn_decl("main", [], "atom", lines, body), header=hdr)
    c = Case(area, "nestedcase", src, str(val), 0)
    c.info["names"] = ["outer", "inner"]
    c.info["fnames"] = ["print_int64"]
    c.info["has_seq"] = True
    c.add_fail("E2001", "string literal passed to print_int64 inside a nested arm", rep(src, f"print_int64({b})", f'print_int64("{b}")', 1))
    return c


def t_eff_helper_pick(rng, area):
    flag = rng.choice([True, False])
    a, b = ints(rng, 2, 0, 9)
    f = fn_decl("pick_value", [("flag", "boolean")], "int64", [], f"case flag of {{\n        true -> {a};\n        false -> {b}\n    }}")
    lines = [f"flag: boolean <- {str(flag).lower()};"]
    body = "sequence proc[device_io]\n        v: int64 <- pick_value(flag);\n        result: atom <- print_int64(v)\n    produces\n        pure result\n    end"
    hdr = "// Positive: pure helper chosen from a case arm; the effect runs in the sequence body"
    src = program(f, fn_decl("main", [], "atom", lines, body), header=hdr)
    c = Case(area, "helperpick", src, str(a if flag else b), 0)
    c.info["names"] = ["flag", "v"]
    c.info["fnames"] = ["pick_value", "print_int64"]
    c.info["calls"] = [("pick_value", 1)]
    c.info["has_seq"] = True
    c.add_fail("E3001", "print moved into the pure helper without a sequence", rep(src, f"true -> {a};", f"true -> {{\n            print_int64({a});\n            {a}\n        }};", 1))
    return c


def t_eff_mem_in_arms(rng, area):
    flag = rng.choice([True, False])
    xs = ints(rng, rng.randint(1, 4))
    lines = [f"flag: boolean <- {str(flag).lower()};"]
    body = (f"sequence proc[mem(normal)]\n        n: int64 <- case flag of {{\n            true -> {{\n                xs: {L('int64')} <- [{', '.join(map(str, xs))}];\n                length[int64, mem(normal)](xs)\n            }};\n            false -> {{\n                ys: {L('int64')} <- empty[int64, mem(normal)]();\n                length[int64, mem(normal)](ys)\n            }}\n        }}\n    produces\n        pure n\n    end")
    hdr = "// Positive: mem(normal) list ops only inside case arms still justify sequence proc[mem(normal)]"
    src = program(fn_decl("main", [], "int64", lines, body), header=hdr)
    c = Case(area, "memarms", src, "", len(xs) if flag else 0)
    c.info["names"] = ["flag", "n", "xs", "ys"]
    c.info["fnames"] = ["length", "empty"]
    c.info["has_seq"] = True
    c.info["has_list"] = True
    c.add_fail("E2015", "list binding in an arm without its mem(Space)", rep(src, f"xs: {L('int64')} <-", "xs: List[int64] <-", 1))
    return c


def t_eff_two_seqs(rng, area):
    a, b = ints(rng, 2, 0, 9)
    lines = [f"sequence proc[device_io]\n        print_int64({a});\n        println(\"\")\n    produces pure :ok end;",
             f"n: int64 <- sequence proc[mem(normal)]\n        xs: {L('int64')} <- [{a}, {b}]\n    produces\n        pure length[int64, mem(normal)](xs)\n    end;"]
    hdr = "// Positive: two sequences with separate effect declarations in one function"
    src = program(fn_decl("main", [], "int64", lines, "n"), header=hdr)
    c = Case(area, "twoseqs", src, str(a) + "\n", 2)
    c.info["names"] = ["n", "xs"]
    c.info["fnames"] = ["print_int64", "length"]
    c.info["has_seq"] = True
    c.info["has_list"] = True
    c.add_fail("E3010", "second sequence declares device_io it never uses", rep(src, "sequence proc[mem(normal)]", "sequence proc[mem(normal), device_io]", 1))
    return c
