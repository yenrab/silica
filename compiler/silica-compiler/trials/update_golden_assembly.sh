#!/usr/bin/env bash

set -euo pipefail

find . -type f -name '*.sams' -print0 | while IFS= read -r -d '' file; do
  mv -- "$file" "${file%.sams}.ascomp"
done
