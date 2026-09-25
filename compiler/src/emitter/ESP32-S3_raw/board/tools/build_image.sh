#!/bin/sh
# Copyright 2026 Lee Scott Barney
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Build a bare-metal ESP32-S3 image from program sources plus the board runtime.
#
#   build_image.sh [-q] [-r <runtime-cache-dir>] -o <out_dir>/<name> <source>...
#   build_image.sh -r <runtime-cache-dir> --prepare
#
# -r keeps the assembled runtime objects in <runtime-cache-dir> and reuses them (reassembling a
#    file only when its source is newer), instead of assembling the runtime for every image; the
#    trial driver prepares the cache once per run with --prepare, before any parallel use.
# -q prints nothing on success (the section sizes and the image path are otherwise printed).
#
# Sources are .S/.s hand-written assembly or .sams emitted by an ESP32-S3_raw silica-compiler.
# Produces <name>.elf, <name>.map and <name>.bin (flash at offset 0x0 with flash_image.sh).
#
# Toolchain: Espressif's xtensa-esp-elf GCC (assembler, linker, libgcc) and esptool from the
# ESP-IDF Python environment. No ESP-IDF component, header or library is linked.
# Override XTENSA_BIN / ESPTOOL if they live elsewhere.

set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
RT="$HERE/../runtime"
XTENSA_BIN=${XTENSA_BIN:-$HOME/.espressif/tools/xtensa-esp-elf/esp-16.1.0_20260609/xtensa-esp-elf/bin}
ESPTOOL=${ESPTOOL:-$HOME/.espressif/python_env/idf6.2_py3.14_env/bin/esptool}
CC="$XTENSA_BIN/xtensa-esp32s3-elf-gcc"

out=""
rtcache=""
quiet=0
prepare=0
while [ $# -gt 0 ]; do
    case "$1" in
        -o) out=$2; shift 2 ;;
        -r) rtcache=$2; shift 2 ;;
        -q) quiet=1; shift ;;
        --prepare) prepare=1; shift ;;
        *) break ;;
    esac
done
if [ "$prepare" = 1 ] && [ -z "$rtcache" ]; then
    echo "usage: $0 -r <runtime-cache-dir> --prepare" >&2
    exit 2
fi
if [ "$prepare" = 0 ] && { [ -z "$out" ] || [ $# -eq 0 ]; }; then
    echo "usage: $0 [-q] [-r <runtime-cache-dir>] -o <out_dir>/<name> <source>..." >&2
    exit 2
fi

# --longcalls: CALL8 reaches +-512 KB, relaxed to L32R+CALLX8 when a target is further away.
# --auto-litpools is NOT used: literals go to .literal sections, which the link script puts in
# front of .text (L32R only reaches backwards, 256 KB).
ASFLAGS="-c -mlongcalls"

RUNTIME_SOURCES="rt_vectors.S rt_start.S rt_console.S rt_heap.S rt_board.S rt_string.S rt_list.S rt_float.S rt_ordering.S rt_actors_stub.S"

# Runtime objects: from the cache (assembled only when missing or older than the source; written
# to a temporary name and renamed, so a concurrent reader never sees a partial object), or fresh.
objs=""
if [ -n "$rtcache" ]; then
    mkdir -p "$rtcache"
    for f in $RUNTIME_SOURCES; do
        obj="$rtcache/${f%.S}.o"
        if [ ! -f "$obj" ] || [ "$RT/$f" -nt "$obj" ]; then
            "$CC" $ASFLAGS "$RT/$f" -o "$obj.tmp.$$" && mv "$obj.tmp.$$" "$obj"
        fi
        objs="$objs $obj"
    done
    [ "$prepare" = 1 ] && exit 0
fi

mkdir -p "$(dirname "$out")"
objdir="$out.objs"
rm -rf "$objdir"
mkdir -p "$objdir"

n=0
if [ -z "$rtcache" ]; then
    for f in $RUNTIME_SOURCES; do
        n=$((n + 1))
        obj="$objdir/$n.${f%.S}.o"
        "$CC" $ASFLAGS "$RT/$f" -o "$obj"
        objs="$objs $obj"
    done
fi
for src in "$@"; do
    n=$((n + 1))
    base=$(basename "$src")
    obj="$objdir/$n.${base%.*}.o"
    case "$src" in
        *.sams) "$CC" $ASFLAGS -x assembler "$src" -o "$obj" ;;
        *)      "$CC" $ASFLAGS "$src" -o "$obj" ;;
    esac
    objs="$objs $obj"
done

"$CC" -nostdlib -nostartfiles -Wl,--gc-sections -Wl,-T,"$RT/silica_esp32s3.ld" \
    -Wl,-Map,"$out.map" -o "$out.elf" $objs -lgcc

"$ESPTOOL" --chip esp32s3 elf2image --flash-mode dio --flash-freq 80m --flash-size 2MB \
    -o "$out.bin" "$out.elf" >/dev/null

[ "$quiet" = 1 ] && exit 0
"$XTENSA_BIN/xtensa-esp32s3-elf-size" -A "$out.elf" | awk '/^\.(iram0|dram0)/ {printf "  %-16s %7d bytes @ %s\n", $1, $2, sprintf("0x%x", $3)}'
echo "  image: $out.bin"
