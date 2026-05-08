#!/usr/bin/env bash

file="src/silica-compiler"

bytes=$(stat -f '%z' "$file")
megabytes=$(awk -v b="$bytes" 'BEGIN { printf "%.2f MB", b / 1024 / 1024 }')

printf "%s %s\n" "$file" "$megabytes"
