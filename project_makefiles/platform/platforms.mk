# Host platform ids and emit-target names.
# GNU make 3.81. Safe to include: goals exist only when this file is the
# makefile make was invoked on (an included copy must not become the default goal).
#
#   make -s --no-print-directory -f project_makefiles/platform/platforms.mk host-platform
#   make -s --no-print-directory -f project_makefiles/platform/platforms.mk host-emit-target
#   make -s --no-print-directory -f project_makefiles/platform/platforms.mk platforms
#   make -s --no-print-directory -f project_makefiles/platform/platforms.mk emit-targets
#   make -s --no-print-directory -f project_makefiles/platform/platforms.mk platform-for-target EMIT_TARGET=linux_x86_64
#
# Host rows are uname-s:uname-m:emit-target:platform-id. "_" is an empty field
# (Darwin x86_64 has a platform id and no emitter). Linux rows match the
# Debian-family rule in binaries/install_compiler.bash.
# From another makefile: $(call silica_platform_for_target,linux_x86_64)

_SILICA_HOST_ROWS := \
	Darwin:arm64:apple_silicon_mac:macos-applesilicon \
	Darwin:x86_64:_:macos-x86_64 \
	Linux:aarch64:linux_aarch64:linux-aarch64 \
	Linux:x86_64:linux_x86_64:linux-x86_64

# emit-target:platform-id. "_" means this emit target is not a host OS.
_SILICA_TARGET_ROWS := \
	apple_silicon_mac:macos-applesilicon \
	linux_aarch64:linux-aarch64 \
	linux_x86_64:linux-x86_64 \
	ESP32-S3_raw:_

_silica_blank = $(if $(filter _,$(1)),,$(1))

_silica_os := $(shell uname -s 2>/dev/null)
_silica_arch := $(shell uname -m 2>/dev/null)

# Debian-family only, same rule as binaries/install_compiler.bash.
# ID=ubuntu has ID_LIKE=debian; Raspberry Pi OS is often ID=raspbian, ID_LIKE=debian.
# Read in the shell without parentheses: a raw ")" inside $(shell) would end the function.
_silica_id := $(shell if [ -r /etc/os-release ]; then . /etc/os-release; printf '%s' "$${ID:-}"; fi)
_silica_id_like := $(shell if [ -r /etc/os-release ]; then . /etc/os-release; printf '%s' "$${ID_LIKE:-}"; fi)
_silica_debian := $(filter debian ubuntu,$(_silica_id) $(_silica_id_like))

_silica_host_row := $(firstword $(foreach row,$(_SILICA_HOST_ROWS),$(if $(filter $(_silica_os):$(_silica_arch):%,$(row)),$(row))))
ifeq ($(_silica_os),Linux)
  ifeq ($(and $(_silica_debian),$(filter aarch64 x86_64,$(_silica_arch))),)
    _silica_host_row :=
  endif
endif

_silica_host_fields := $(subst :, ,$(_silica_host_row))

SILICA_HOST_UNAME_S := $(_silica_os)
SILICA_HOST_UNAME_M := $(_silica_arch)
SILICA_HOST_EMIT_TARGET := $(call _silica_blank,$(word 3,$(_silica_host_fields)))
SILICA_HOST_PLATFORM := $(call _silica_blank,$(word 4,$(_silica_host_fields)))

# $(call silica_platform_for_target,<emit target>) -> host platform id, or empty.
silica_platform_for_target = $(call _silica_blank,$(word 2,$(subst :, ,$(firstword $(filter $(1):%,$(_SILICA_TARGET_ROWS))))))

ifeq ($(notdir $(firstword $(MAKEFILE_LIST))),platforms.mk)

.PHONY: host-platform host-emit-target platforms emit-targets platform-for-target

host-platform:
	@printf '%s\n' '$(SILICA_HOST_PLATFORM)'

host-emit-target:
	@printf '%s\n' '$(SILICA_HOST_EMIT_TARGET)'

platforms:
	@printf '%s\n' $(foreach row,$(_SILICA_HOST_ROWS),$(word 4,$(subst :, ,$(row))))

emit-targets:
	@printf '%s\n' $(foreach row,$(_SILICA_TARGET_ROWS),$(word 1,$(subst :, ,$(row))))

platform-for-target:
	@known=; \
	for name in $(foreach row,$(_SILICA_TARGET_ROWS),$(word 1,$(subst :, ,$(row)))); do \
		if [ "$$name" = '$(EMIT_TARGET)' ]; then known=yes; fi; \
	done; \
	if [ -z "$$known" ]; then \
		echo "platforms.mk: unknown emit target '$(EMIT_TARGET)'" >&2; \
		exit 1; \
	fi; \
	printf '%s\n' '$(call silica_platform_for_target,$(EMIT_TARGET))'

endif
