#!/usr/bin/env python3
from pathlib import Path
import sys


def main() -> int:
    directory = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(".")

    if not directory.is_dir():
        print(f"Error: not a directory: {directory}", file=sys.stderr)
        return 1

    for path in directory.glob("*.silica"):
        if not path.is_file():
            continue

        text = path.read_text()
        updated = text.replace("float32", "float16")

        if updated != text:
            path.write_text(updated)
            print(f"Updated {path}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
