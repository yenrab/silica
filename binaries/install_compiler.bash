#!/usr/bin/env bash
# Install a freshly built compiler into binaries/ and repoint its stable link at it.
#
#   install_compiler.bash seed     <path-to-built-binary>   ->  binaries/seed-compiler
#   install_compiler.bash selfhost <path-to-built-binary>   ->  binaries/silica-compiler
#
# Naming follows the convention documented in update_silica_compiler_link.bash:
#
#     seed      silica-<NNNNNN>-seed-<platform>    reached through binaries/seed-compiler
#     selfhost  silica-<NNNNNN>-<platform>         reached through binaries/silica-compiler
#
# A LOWER NNNNNN is the newer build, so installing takes (lowest existing for this kind)
# minus one. The two kinds are numbered independently and their links never cross: the
# selfhost tree is compiled by seed-compiler and base's .silica files are compiled by
# silica-compiler, so letting one link resolve to the other kind would silently compile a
# tree with the wrong compiler -- which is precisely the mistake that differential testing
# between the two is meant to catch.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

die() {
    echo "install_compiler: $*" >&2
    exit 1
}

# Must agree with detect_local_platform in update_silica_compiler_link.bash.
detect_local_platform() {
    local os arch distro id_like
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Darwin)
            case "$arch" in
                arm64)  echo "macos-applesilicon" ;;
                x86_64) echo "macos-x86_64" ;;
                *)      return 1 ;;
            esac
            ;;
        Linux)
            distro="unknown"
            id_like=""
            if [[ -r /etc/os-release ]]; then
                local ID="" ID_LIKE=""
                # shellcheck disable=SC1091
                . /etc/os-release
                distro="${ID:-unknown}"
                id_like="${ID_LIKE:-}"
            fi
            case "${distro}:${arch}" in
                debian:aarch64|ubuntu:aarch64) echo "debian-aarch64" ;;
                debian:x86_64|ubuntu:x86_64)   echo "debian-x86_64" ;;
                *)
                    case " ${id_like} " in
                        *" debian "*)
                            case "$arch" in
                                aarch64) echo "debian-aarch64" ;;
                                x86_64)  echo "debian-x86_64" ;;
                                *)       return 1 ;;
                            esac
                            ;;
                        *) return 1 ;;
                    esac
                    ;;
            esac
            ;;
        *) return 1 ;;
    esac
}

[[ $# -eq 2 ]] || die "usage: install_compiler.bash <seed|selfhost> <built-binary>"

kind="$1"
built="$2"

case "$kind" in
    seed|selfhost) ;;
    *) die "kind must be 'seed' or 'selfhost', got '$kind'" ;;
esac

[[ -f "$built" ]] || die "built binary not found: $built"
[[ -x "$built" ]] || die "built binary is not executable: $built"

platform="$(detect_local_platform)" \
    || die "unrecognized host platform ($(uname -s)/$(uname -m)); see CANONICAL_PLATFORMS in update_silica_compiler_link.bash"

if [[ "$kind" == "seed" ]]; then
    # silica-NNNNNN-seed-<platform>
    pattern="^silica-([0-9]{6})-seed-${platform}\$"
    link="seed-compiler"
else
    # silica-NNNNNN-<platform>, with no kind token, so this cannot match a seed name.
    pattern="^silica-([0-9]{6})-${platform}\$"
    link="silica-compiler"
fi

# Lowest existing number for this kind. Anchoring the pattern at both ends skips the
# archived variants (silica-999990-macos-applesilicon.smoke-aug6 and similar), which must
# not influence numbering or be mistaken for the current build.
lowest=""
while IFS= read -r name; do
    [[ -n "$name" ]] || continue
    if [[ "$name" =~ $pattern ]]; then
        n="${BASH_REMATCH[1]}"
        if [[ -z "$lowest" || "$n" -lt "$lowest" ]]; then
            lowest="$n"
        fi
    fi
done < <(cd "$SCRIPT_DIR" && ls -1)

if [[ -z "$lowest" ]]; then
    next="999999"
    echo "install_compiler: no existing $kind binary for $platform; starting at $next"
else
    next="$(printf '%06d' $((10#$lowest - 1)))"
    [[ "$next" != "999999" && $((10#$next)) -ge 0 ]] || die "cannot decrement below $lowest"
fi

if [[ "$kind" == "seed" ]]; then
    target="silica-${next}-seed-${platform}"
else
    target="silica-${next}-${platform}"
fi

[[ ! -e "$SCRIPT_DIR/$target" ]] \
    || die "$target already exists; refusing to overwrite an installed build"

install -m 755 "$built" "$SCRIPT_DIR/$target"
ln -sfn "$target" "$SCRIPT_DIR/$link"

echo "✅ Installed $kind compiler: binaries/$target"
echo "   binaries/$link -> $target"
