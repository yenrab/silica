# trials/platform/linux_x86_64.mk -- toolchain values for trials whose emitted code runs on a
# Linux x86-64 host (the values the plan's Step 2.1 asks for).
#
# Nothing includes this file yet. It is meant to be included from trials/silica_compiler.mk as
#   include $(_SILICA_COMPILER_MK_DIR)platform/$(SILICA_HOST_EMIT_TARGET).mk
# and to replace both trials/linux_host.mk and the macOS lines still inlined in 43 trial makefiles
# (the list is in apple_silicon_mac.mk beside this file). Until those lines are deleted, every
# variable they assign after the include is set here with `override`, exactly as linux_host.mk
# does, so including this file today changes nothing on macOS and works on Linux.
#
# Verified on nix (Ubuntu 24.04.3, clang 18.1.3, GNU as/ld 2.42, GNU make 4.3) 2026-09-18 with
# trials/base/linux_x86_64_smoke.

PLATFORM_ID := linux-x86_64
EMIT_TARGET := linux_x86_64

# Assembler: clang's integrated assembler or GNU as through cc; both accept the emitted text and
# both are checked by the backend lint. clang when present, cc otherwise.
override ASSEMBLER := $(shell command -v clang >/dev/null 2>&1 && echo clang || echo cc)
override ASFLAGS := -c -x assembler
override ASFLAGS_macos :=
override MACOS_SDK :=
override MACOS_MIN_VERSION :=

# Linker driver. GNU ld (bfd) is the default because lld is not installed on nix; when it is
# (sudo apt install lld), LINKER_USE_LLD=1 adds -fuse-ld=lld.
override LINKER := $(ASSEMBLER)
override RUST_LLD := /nonexistent
# crt1.o calls main, so no -e main. -no-pie: emitted data tables hold absolute .quad addresses
# and the distribution's compilers default to PIE. -rdynamic: the actor failure banner resolves
# the running behavior's name with dladdr, which on glibc sees only .dynsym.
override LDFLAGS := -no-pie -rdynamic -lpthread $(if $(LINKER_USE_LLD),-fuse-ld=lld,)
override LDFLAGS_clang := $(LDFLAGS)
override LDFLAGS_rust-lld :=
override LDFLAGS_STACK :=

# FFI fixtures (ffi_addition/fixtures.mk already has this host split; these are the same values).
override FFI_CC := $(ASSEMBLER)
override FFI_ARCH :=
override FFI_HOST_CFLAGS := -D_GNU_SOURCE -fPIC
FFI_FIXTURE_CFLAGS := -std=c11 -O2 -fPIC -D_GNU_SOURCE

# Assembly golden a trial is compared against: <unit>.linux_x86_64.ascomp (silica_compiler.mk
# already derives this as ASCOMP_EXT from SILICA_EMIT_TARGET; kept here for makefiles that do not
# go through it).
GOLDEN_SUFFIX := .linux_x86_64.ascomp

# Suites that cannot pass on this target yet (plan Step 3.10 clears the list).
SKIP_TRIALS := cpu_discovery_and_spawn_pinning

# GNU stat (linux_host.mk's reason for existing was also BSD stat vs GNU stat; the shared wrapper
# in silica_compiler.mk already tries both forms).
STAT_SIZE := stat -c %s
STAT_MTIME := stat -c %Y
