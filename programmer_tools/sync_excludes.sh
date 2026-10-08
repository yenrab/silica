# Shared rsync exclusions for the platform scripts (build_all_platforms.sh,
# run_trials_all_platforms.sh, rebuild_refresh_verify.sh). Source this file; it defines
#
#   SYNC_BUILD_EXCLUDES        rsync arguments: everything a platform builds, so nothing built on one
#                              machine crosses to another. A tree synced from the Mac once carried
#                              the Mac's arm64 .a fixtures to nix, make judged them up to date, and
#                              the x86-64 links failed.
#   sync_exe_excludes DIR PREFIX
#                              prints one anchored rsync exclude pattern per built trial executable
#                              under DIR (see below); PREFIX is DIR's path inside the transfer root.
#
# Sources, goldens (.ascomp, .scout, .golden_fail*), .meta/.h fixture sources, Makefiles, shell
# scripts and the stdlib's own tracked files all still travel: nothing here matches them.
#
# What the exclusions mean:
#   *.o *.a *.so *.dylib       machine code and libraries (the stdlib has no such tracked file)
#   compiler/build/            (and /build/, for a transfer rooted at compiler/) the per-target compiler build directories (compiler/src/Makefile):
#                              .sams, .iface, .o, silica.config and the linked compiler of each
#                              target, plus the unit symlinks. Each machine builds its own. The one
#                              hand-off that needs a build directory (bootstrap-assembly's .sams)
#                              sends it explicitly, by its own rsync, in build_all_platforms.sh.
#   fixtures/build/            ffi_addition's per-platform fixture objects and archives
#   app_*/dangerous_exposure_source   the per-app link into the platform's fixture view, recreated
#                              (ln -sfn) by every app integrate; a synced one would dangle
#   trial outputs              .sams .iface .sout .cur_fail .integrate* silica.config/.link/.atoms/
#                              .shapes/.needs_runtime/.compile.order, and each trial's linked
#                              executable (sync_exe_excludes)
#   /compiler/src/silica-*     the old in-tree compiler binaries, if an older checkout left any
#
# silica.link.scout is a golden and is kept: only the exact name silica.link is excluded.
# Patterns are for rsync (-a, no --delete-excluded, so nothing already on the remote is removed).

SYNC_BUILD_EXCLUDES=(
    --exclude '*.o' --exclude '*.a' --exclude '*.so' --exclude '*.dylib'
    --exclude '*.sams' --exclude '*.iface' --exclude '*.sout' --exclude '*.cur_fail'
    --exclude '.integrate*' --exclude '.stdlib_cache' --exclude '__pycache__'
    --exclude 'silica.config' --exclude 'silica.target' --exclude '.silica.config.units'
    --exclude 'silica.link' --exclude 'silica.atoms' --exclude 'silica.shapes'
    --exclude 'silica.needs_runtime' --exclude 'silica.compile.order'
    --exclude 'compiler/build/' --exclude '/build/' --exclude 'fixtures/build/'
    --exclude 'app_*/dangerous_exposure_source'
    --exclude '/compiler/src/silica-*'
)

# Built trial executables. They have no extension (the trial's name), the execute bit, and sit
# beside their .o; every script in the tree (.sh, .py) has an extension, so "executable file whose
# name has no dot" is exactly a built binary (11,903 of them, some committed by accident, none
# needed: each trial is relinked on the machine that runs it). Written as anchored literal patterns
# because rsync has no way to say "no dot in the name".
sync_exe_excludes() {  # $1 = directory to scan, $2 = its path within the transfer ("" or "/trials")
    local dir="$1" prefix="${2:-}"
    [ -d "$dir" ] || return 0
    ( cd "$dir" && find . -type f -perm -u+x ! -name '*.*' -print ) \
        | sed -e 's|^\./||' -e 's/[][*?\\]/\\&/g' -e "s|^|$prefix/|"
}
