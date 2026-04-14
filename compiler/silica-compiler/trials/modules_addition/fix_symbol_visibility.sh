#!/bin/bash
# Workaround for compiler bug: adds .global directives for simple function names
# The compiler generates ".global lib_module.function" but defines "function:"
# This script adds ".global function" so the linker can find them

for f in lib/*.sams; do
  [ -f "$f" ] || continue
  # Extract simple names from qualified .global directives and add them
  temp=$(mktemp)
  cat "$f" > "$temp"
  grep "^\.global [a-zA-Z_][a-zA-Z0-9_]*\." "$f" | sed 's/^\.global [a-zA-Z_][a-zA-Z0-9_]*\.\([a-zA-Z_][a-zA-Z0-9_]*\)$/.global \1/' >> "$temp"
  mv "$temp" "$f"
done
