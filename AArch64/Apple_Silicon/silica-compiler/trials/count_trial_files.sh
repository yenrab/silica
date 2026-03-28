#!/usr/bin/env bash
# Count all .silica files under this directory (trials) and its subdirectories.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
count="$(find "$SCRIPT_DIR" -type f -name '*.silica' | wc -l | tr -d ' ')"

echo "$count"
