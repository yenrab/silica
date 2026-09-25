"""Copy verified generated files into the real trials tree.
usage: install.py <out_root> [--dry]
Refuses to overwrite any existing file that is not itself a train_* file."""
import os
import shutil
import sys

ROOT = (os.environ.get("SILICA_ROOT") or os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))) + "/trials"


def main():
    out = sys.argv[1]
    dry = "--dry" in sys.argv
    copied, per_area = 0, {}
    for area in sorted(os.listdir(out)):
        src_dir = os.path.join(out, area)
        if not os.path.isdir(src_dir):
            continue
        dst_dir = os.path.join(ROOT, area)
        if not os.path.isdir(dst_dir):
            raise SystemExit(f"target area missing: {dst_dir}")
        n = 0
        for dp, dn, fn in os.walk(src_dir):
            rel = os.path.relpath(dp, src_dir)
            for f in fn:
                s = os.path.join(dp, f)
                d = os.path.join(dst_dir, rel, f) if rel != "." else os.path.join(dst_dir, f)
                base = os.path.basename(d)
                if os.path.exists(d) and not base.startswith("train_"):
                    raise SystemExit(f"refusing to overwrite non-train file: {d}")
                if not dry:
                    os.makedirs(os.path.dirname(d), exist_ok=True)
                    shutil.copy2(s, d)
                n += 1
        per_area[area] = n
        copied += n
    for a, n in per_area.items():
        print(f"{a}: {n} files")
    print("total files:", copied, "(dry run)" if dry else "")


if __name__ == "__main__":
    main()
