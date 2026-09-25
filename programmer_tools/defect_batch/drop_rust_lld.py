#!/usr/bin/env python3
"""Remove the retired rust-lld linker path from a trial makefile. clang is the only
supported linker; on Linux the clang driver still calls GNU ld, which is unaffected.
Usage: drop_rust_lld.py <makefile> [...]   (writes in place, prints what it changed)"""
import re, sys

def transform(s):
    n = 0
    # the two-branch link recipe: keep the clang branch, drop the rust-lld branch
    pat = re.compile(
        r'[ \t]*if test -x "\$\(RUST_LLD\)"; then \\\n'
        r'.*?\n'                                  # the rust-lld command line
        r'[ \t]*else \\\n'
        r'(?P<clang>[ \t]*.*?)\n'                 # the clang command line
        r'[ \t]*fi; \\\n', re.DOTALL)
    def keep_clang(m):
        nonlocal n; n += 1
        line = m.group('clang')
        return line.rstrip() + "\n" if line.rstrip().endswith('\\') else line + "\n"
    s = pat.sub(keep_clang, s)
    # variable definitions and the linker choice
    drops = [
        r'^.*Rust toolchain linker \(rust-lld\).*\n',
        r'^RUST_SYSROOT :=.*\n', r'^RUST_TARGET :=.*\n', r'^RUST_LLD :=.*\n',
        r'^.*Use rust-lld if it exists.*\n',
        r'^LDFLAGS_rust-lld :=.*\n',
        r'^.*when using rust-lld.*\n',
    ]
    # stale prose that survives the structural edits
    subs = [
        (r'Assembler: clang -c; linker: rust-lld if available, else clang\.', 'Assembler and linker: clang.'),
        (r'Link \.o -> executable \(rust-lld or clang\)', 'Link .o -> executable (clang)'),
        (r'\(ASSEMBLER, ASFLAGS_macos, LINKER, RUST_LLD, LDFLAGS_rust-lld, LDFLAGS_clang\)',
         '(ASSEMBLER, ASFLAGS_macos, LINKER, LDFLAGS_clang)'),
    ]
    for d in drops:
        s, k = re.subn(d, '', s, flags=re.M); n += k
    s, k = re.subn(r'^LINKER := \$\(shell test -x "\$\(RUST_LLD\)".*\n', 'LINKER := clang\n', s, flags=re.M); n += k
    for a, b in subs:
        s, k = re.subn(a, b, s); n += k
    s, k = re.subn(r'Linker: rust-lld if available, else clang', 'Linker: clang', s); n += k
    s, k = re.subn(r'Assembler: clang -c \(Rust toolchain does not ship a standalone assembler\)', 'Assembler: clang -c', s); n += k
    return s, n

total = 0
for p in sys.argv[1:]:
    src = open(p).read()
    out, n = transform(src)
    if n:
        open(p, 'w').write(out); total += 1
        print(f"{p}: {n} edits")
    left = out.count('RUST_LLD') + out.count('rust-lld')
    if left:
        print(f"  !! {p} still mentions the rust linker {left} time(s); check by hand")
print(f"{total} file(s) changed")
