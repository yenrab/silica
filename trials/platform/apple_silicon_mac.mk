# trials/platform/apple_silicon_mac.mk -- toolchain values for trials whose emitted code runs on
# an Apple Silicon Mac. These are the values inlined today in 43 trial makefiles, unchanged.
#
# Nothing includes this file yet (see linux_x86_64.mk for the intended include). When it is
# included from trials/silica_compiler.mk, the following lines are deleted from each makefile
# below (counts over the 43 files; `?=` and `:=` variants both appear):
#
#   RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)                              40
#   RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")                  40
#   RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld                     40
#   MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)                    40
#   LDFLAGS_clang := -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)                39 (+1 aligned)
#   ASSEMBLER := clang                                                                      37 (+1 aligned)
#   ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)                              37 (+1 aligned)
#   LINKER := $(shell test -x "$(RUST_LLD)" && echo "$(RUST_LLD)" || echo "clang")          36
#   LDFLAGS_rust-lld := -arch arm64 -platform_version macos ... -lSystem -e main            35
#   MACOS_MIN_VERSION ?= 26.0  /  := 26.0                                                   34 / 8
#   ARCH := arm64, ASFLAGS := -mmacosx-version-min=..., LDFLAGS_rust-lld := -arch $(ARCH) ...  4 (ffi apps)
#   deep_frame_spill_addition only: both LDFLAGS carry -stack_size $(STACK_SIZE)             1
#
# Files (trials/): actor_registration_addition actor_stacks_addition actors_addition
#   atoms_addition base(makefile) bitwise_addition boolean_addition case_addition
#   codegen_defects_addition compiler_addition cpu_discovery_and_spawn_pinning
#   deep_frame_spill_addition effect_check_addition error_enforcement_addition/ffi_addition
#   ffi_addition/app_guarded_fault_trials ffi_addition/app_supervisor_dangerous_guarded_fault_trials
#   ffi_addition/common_app.mk float16_addition float32_addition float64_addition
#   functions_addition int16_addition int32_addition int64_addition int8_addition list_addition
#   memory_region_addition modules_addition negation_addition ordered_data_structures(+leaf.mk,
#   wbt_core) records_addition recursive_function_addition sequence_block_addition string_addition
#   supervisors_addition traits_addition tuples_addition uint16_addition uint32_addition
#   uint64_addition uint8_addition
#
# The rust-lld path: 40 makefiles still prefer rust-lld from the Rust sysroot when rustc is
# installed and fall back to clang. The Rust bootstrap is retired, so once this fragment is the
# only definition the rust-lld branch can go too (LINKER := clang); it is kept here verbatim so
# that including this file changes no behavior.

PLATFORM_ID := macos-applesilicon
EMIT_TARGET := apple_silicon_mac

ASSEMBLER := clang
MACOS_SDK := $(shell xcrun --sdk macosx --show-sdk-path 2>/dev/null)
MACOS_MIN_VERSION ?= 26.0
ASFLAGS_macos := -mmacosx-version-min=$(MACOS_MIN_VERSION)
ASFLAGS := $(ASFLAGS_macos)

RUST_SYSROOT := $(shell rustc --print sysroot 2>/dev/null)
RUST_TARGET := $(shell rustc -vV 2>/dev/null | sed -n "s/^host: *//p")
RUST_LLD := $(RUST_SYSROOT)/lib/rustlib/$(RUST_TARGET)/bin/rust-lld
LINKER := $(shell test -x "$(RUST_LLD)" && echo "$(RUST_LLD)" || echo "clang")

ARCH := arm64
LDFLAGS_rust-lld := -arch $(ARCH) -platform_version macos $(MACOS_MIN_VERSION) $(MACOS_MIN_VERSION) -syslibroot $(MACOS_SDK) -lSystem -e main
LDFLAGS_clang := -Wl,-e,main -Wl,-macos_version_min,$(MACOS_MIN_VERSION)
LDFLAGS := $(LDFLAGS_clang)
# deep_frame_spill_addition appends -Wl,-stack_size,$(STACK_SIZE) / -stack_size $(STACK_SIZE) itself.
LDFLAGS_STACK :=

FFI_CC := clang
FFI_ARCH := arm64
FFI_HOST_CFLAGS := -arch $(FFI_ARCH) -mmacosx-version-min=$(MACOS_MIN_VERSION)
FFI_FIXTURE_CFLAGS := -std=c11 -O2 $(FFI_HOST_CFLAGS)

# The bare .ascomp is the original Darwin golden.
GOLDEN_SUFFIX := .ascomp
SKIP_TRIALS :=

STAT_SIZE := stat -f %z
STAT_MTIME := stat -f %m
