#!/bin/sh
# Read archive paths from silica.link (paths relative to the manifest directory).
# Prints a whitespace-separated list suitable for linker command lines.
set -eu

dir=${1:-.}
file="$dir/silica.link"

if [ ! -f "$file" ]; then
	exit 0
fi

awk -F'"' '/^archive:/ { printf "%s ", $2 } END { print "" }' "$file"
