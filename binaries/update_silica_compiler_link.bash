#!/usr/bin/env bash

set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

latest_file="$(
    find . \
        -maxdepth 1 \
        -type f \
        -name 'silica-[0-9][0-9][0-9][0-9][0-9][0-9]-*' \
        -print |
    sed 's|^\./||' |
    LC_ALL=C sort |
    head -n 1
)"

if [[ -z "$latest_file" ]]; then
    echo "No versioned silica compiler file was found." >&2
    exit 1
fi

chmod +x "$latest_file"
ln -sfn "$latest_file" silica-compiler

echo "silica-compiler -> $latest_file"
