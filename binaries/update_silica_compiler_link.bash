#!/usr/bin/env bash

set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

# Versioned compilers look like:
#   silica-<NNNNNN>-<kind>-<platform>
# e.g. silica-999998-seed-macos-applesilicon
# Lower NNNNNN sorts first and is treated as the latest build.

versioned_files="$(
    find . \
        -maxdepth 1 \
        -type f \
        -name 'silica-[0-9][0-9][0-9][0-9][0-9][0-9]-*' \
        -print |
        sed 's|^\./||' |
        LC_ALL=C sort
)"

if [[ -z "$versioned_files" ]]; then
    echo "No versioned silica compiler file was found in $(pwd)." >&2
    echo "Expected names like: silica-999998-seed-macos-applesilicon" >&2
    exit 1
fi

# Extract unique platforms (suffix after silica-NNNNNN-<kind>-).
platforms="$(
    printf '%s\n' "$versioned_files" |
        sed -nE 's/^silica-[0-9]{6}-[^-]+-(.+)$/\1/p' |
        LC_ALL=C sort -u
)"

if [[ -z "$platforms" ]]; then
    echo "Found versioned silica files, but none include a platform suffix." >&2
    echo "Expected names like: silica-999998-seed-macos-applesilicon" >&2
    echo >&2
    echo "Files found:" >&2
    printf '%s\n' "$versioned_files" | sed 's/^/  /' >&2
    exit 1
fi

platform_count="$(printf '%s\n' "$platforms" | grep -c .)"

latest_for_platform() {
    local platform="$1"
    printf '%s\n' "$versioned_files" |
        grep -E "^silica-[0-9]{6}-[^-]+-${platform}\$" |
        head -n 1
}

count_for_platform() {
    local platform="$1"
    printf '%s\n' "$versioned_files" |
        grep -E "^silica-[0-9]{6}-[^-]+-${platform}\$" |
        grep -c .
}

platform_at_index() {
    local index="$1"
    printf '%s\n' "$platforms" | sed -n "${index}p"
}

platform_known() {
    local needle="$1"
    printf '%s\n' "$platforms" | grep -Fxq -- "$needle"
}

# Map this host to a silica platform id (e.g. macos-applesilicon).
# Returns 0 and prints the id when known; returns 1 when unsupported/unknown.
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
                debian:aarch64|ubuntu:aarch64)
                    echo "debian-aarch64"
                    ;;
                debian:x86_64|ubuntu:x86_64)
                    echo "debian-x86_64"
                    ;;
                *)
                    # Debian-family IDs (e.g. raspbian) via ID_LIKE.
                    case " ${id_like} " in
                        *" debian "*)
                            case "$arch" in
                                aarch64) echo "debian-aarch64" ;;
                                x86_64)  echo "debian-x86_64" ;;
                                *)       return 1 ;;
                            esac
                            ;;
                        *)
                            return 1
                            ;;
                    esac
                    ;;
            esac
            ;;
        *)
            return 1
            ;;
    esac
}

detected_platform=""
if detected_platform="$(detect_local_platform)"; then
    :
else
    detected_platform=""
fi

default_platform=""
if [[ -n "$detected_platform" ]] && platform_known "$detected_platform"; then
    default_platform="$detected_platform"
fi

print_usage() {
    cat <<EOF
Update the local silica-compiler symlink to the latest compiler for your
local host platform (the machine where the compiler runs).

This is NOT a cross-compile target. Choose the platform of this computer
so silica-compiler is a native binary you can execute locally.

When run with no argument, the script detects this host (macOS / Linux
distro + arch) and, if a matching binary platform exists, offers it as the
default — press Enter to accept.

Usage:
  $(basename -- "$0") [local-platform]

Local platforms found in $(pwd):
$(printf '%s\n' "$platforms" | sed 's/^/  - /')
$(
    if [[ -n "$detected_platform" ]]; then
        echo
        echo "Detected host platform: $detected_platform"
        if [[ -n "$default_platform" ]]; then
            echo "Default selection:     $default_platform"
        else
            echo "No matching binary found for the detected host platform."
        fi
    else
        echo
        echo "Host platform could not be auto-detected."
    fi
)

Examples:
  $(basename -- "$0")
  $(basename -- "$0") macos-applesilicon

The selected local platform's newest binary (lowest silica-NNNNNN number) becomes:
  silica-compiler -> <chosen binary>
EOF
}

selected_platform="${1:-}"

if [[ "$selected_platform" == "-h" || "$selected_platform" == "--help" ]]; then
    print_usage
    exit 0
fi

if [[ -z "$selected_platform" ]]; then
    echo "Update local silica-compiler link"
    echo
    echo "Select the LOCAL host platform — the machine where this compiler will run."
    echo "Do not choose a cross-compile target; this only picks which native binary"
    echo "becomes ./silica-compiler on this computer."
    echo
    if [[ -n "$detected_platform" ]]; then
        echo "Detected host platform: $detected_platform"
        if [[ -n "$default_platform" ]]; then
            echo "Press Enter to accept the default: $default_platform"
        else
            echo "No binary matching the detected host was found; choose from the list."
        fi
        echo
    fi
    echo "Local platforms found:"
    i=1
    while IFS= read -r platform; do
        latest="$(latest_for_platform "$platform")"
        count="$(count_for_platform "$platform")"
        marker=""
        if [[ -n "$default_platform" && "$platform" == "$default_platform" ]]; then
            marker="  [default]"
        fi
        printf '  %2d) %-24s  (latest: %s, %d build(s))%s\n' \
            "$i" "$platform" "$latest" "$count" "$marker"
        i=$((i + 1))
    done <<< "$platforms"
    echo
    echo "Enter a local platform name (e.g. macos-applesilicon) or a number from the list."
    echo "Press Ctrl-C to cancel."
    echo
    if [[ -n "$default_platform" ]]; then
        read -r -p "Local platform [${default_platform}]: " selected_platform
        selected_platform="${selected_platform:-$default_platform}"
    else
        read -r -p "Local platform: " selected_platform
    fi
fi

if [[ -z "$selected_platform" ]]; then
    echo "No platform selected." >&2
    exit 1
fi

# Allow selecting by list number.
if [[ "$selected_platform" =~ ^[0-9]+$ ]]; then
    if (( selected_platform < 1 || selected_platform > platform_count )); then
        echo "Invalid selection: $selected_platform" >&2
        echo "Choose a number between 1 and ${platform_count}." >&2
        exit 1
    fi
    selected_platform="$(platform_at_index "$selected_platform")"
fi

if ! platform_known "$selected_platform"; then
    echo "Unknown local platform: $selected_platform" >&2
    echo >&2
    echo "Local platforms found:" >&2
    printf '%s\n' "$platforms" | sed 's/^/  - /' >&2
    echo >&2
    echo "Run: $(basename -- "$0") --help" >&2
    exit 1
fi

latest_file="$(latest_for_platform "$selected_platform")"

if [[ -z "$latest_file" ]]; then
    echo "No compiler binary found for local platform: $selected_platform" >&2
    exit 1
fi

chmod +x "$latest_file"
ln -sfn "$latest_file" silica-compiler

echo
echo "Selected local platform: $selected_platform"
echo "silica-compiler -> $latest_file"
echo
echo "silica-compiler is now the native compiler binary for this local host."
echo "This selection is not a cross-compile target."
echo "Rebuild or re-run this script after adding a newer silica-NNNNNN-*-$selected_platform binary."
