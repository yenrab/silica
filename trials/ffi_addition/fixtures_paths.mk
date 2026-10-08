# Where the ffi_addition fixtures live, per platform. Pure variable definitions (no rules), so any
# suite can include it, including ones that must not inherit fixtures.mk's targets.
# Include after silica_compiler.mk (it supplies SILICA_HOST_EMIT_TARGET / SILICA_EMIT_TARGET).
#
# The fixtures are C compiled by this machine's own toolchain, so their objects and static libraries
# are only good on the platform that built them. They used to share one directory; a tree synced
# from the Mac to a Linux host carried the Mac's arm64 .a files with it, make judged them up to
# date, and the x86-64 links failed. Each platform now has its own directory, named the way
# ASCOMP_EXT / GOLDEN_FAIL_EXT name a platform (the emit target: apple_silicon_mac, linux_aarch64,
# linux_x86_64), and a platform that has not built there yet simply builds its own.
#
#   fixtures/build/<platform>/obj/                        *.o
#   fixtures/build/<platform>/dangerous_exposure_source/  the tree an app links against:
#       lib/*.a                                             this platform's archives
#       abi legacy net other text                           symlinks to the shared .meta/.h sources
#
# Apps link <app>/dangerous_exposure_source (a symlink made with ln -sfn) to the platform's view, so
# the silica.link goldens ("dangerous_exposure_source/lib/libsilica_text.a") are unchanged.
# The fixture C is built for this host, so the platform is the host's emit target; it equals
# SILICA_EMIT_TARGET on every hosted run (a board target compiles only, it never links fixtures).
FFI_FIXTURES_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/fixtures)
FFI_PLATFORM := $(or $(SILICA_HOST_EMIT_TARGET),$(SILICA_EMIT_TARGET),unknown)
FFI_PLATFORM_DIR := $(FFI_FIXTURES_DIR)/build/$(FFI_PLATFORM)
FFI_SRC_DIR := $(FFI_FIXTURES_DIR)/src
FFI_SHARED_SOURCE := $(FFI_FIXTURES_DIR)/dangerous_exposure_source
FFI_SOURCE_VIEW := $(FFI_PLATFORM_DIR)/dangerous_exposure_source
FFI_LIB_DIR := $(FFI_SOURCE_VIEW)/lib
FFI_BUILD_DIR := $(FFI_PLATFORM_DIR)/obj
