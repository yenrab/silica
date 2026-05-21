#!/usr/bin/env bash

# Usage:
#   ./find_do_done_end.sh /path/to/search
#
# Finds occurrences in .silica files where "do" or "done" is followed by "end",
# even across multiple lines, and prints start/end line numbers.

set -euo pipefail

DIR="${1:-.}"

find "$DIR" -type f -name '*.silica' -print0 |
while IFS= read -r -d '' file; do
  perl -0777 -ne '
    while (/\bdo(?:ne)?\b.*?\bend\b/sg) {
      my $match_start = $-[0];
      my $match_end   = $+[0];

      my $start_line = 1 + (substr($_, 0, $match_start) =~ tr/\n//);
      my $end_line   = 1 + (substr($_, 0, $match_end)   =~ tr/\n//);

      print "$ARGV:$start_line-$end_line\n";
    }
  ' "$file"
done
