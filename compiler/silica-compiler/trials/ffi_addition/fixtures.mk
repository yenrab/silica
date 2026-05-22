# Build prebuilt C wrapper static libraries for ffi_addition trials.
# Included by ffi_addition/Makefile and common_app.mk (app integrate targets).

FFI_FIXTURES_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/fixtures)
FFI_SRC_DIR := $(FFI_FIXTURES_DIR)/src
FFI_LIB_DIR := $(FFI_FIXTURES_DIR)/dangerous_exposure_source/lib
FFI_BUILD_DIR := $(FFI_FIXTURES_DIR)/build

FFI_CC := clang
FFI_ARCH := arm64
FFI_CFLAGS := -std=c11 -Wall -Wextra -O2 -arch $(FFI_ARCH) -mmacosx-version-min=26.0 \
	-I$(FFI_FIXTURES_DIR)/dangerous_exposure_source/legacy \
	-I$(FFI_FIXTURES_DIR)/dangerous_exposure_source/text \
	-I$(FFI_FIXTURES_DIR)/dangerous_exposure_source/net

FFI_LEGACY_OBJ := $(FFI_BUILD_DIR)/silica_legacy_math.o
FFI_TEXT_OBJ := $(FFI_BUILD_DIR)/silica_text.o
FFI_NET_OBJ := $(FFI_BUILD_DIR)/silica_net.o

FFI_LEGACY_ARCHIVE := $(FFI_LIB_DIR)/libsilica_legacy_math.a
FFI_TEXT_ARCHIVE := $(FFI_LIB_DIR)/libsilica_text.a
FFI_NET_ARCHIVE := $(FFI_LIB_DIR)/libsilica_net.a

FFI_WRAPPER_ARCHIVES := $(FFI_LEGACY_ARCHIVE) $(FFI_TEXT_ARCHIVE) $(FFI_NET_ARCHIVE)

.PHONY: ffi-wrapper-archives

ffi-wrapper-archives: $(FFI_WRAPPER_ARCHIVES)
	@test -f "$(FFI_LEGACY_ARCHIVE)" && test -f "$(FFI_TEXT_ARCHIVE)" && test -f "$(FFI_NET_ARCHIVE)"

$(FFI_BUILD_DIR):
	@mkdir -p $(FFI_BUILD_DIR)

$(FFI_LIB_DIR):
	@mkdir -p $(FFI_LIB_DIR)

$(FFI_LEGACY_OBJ): $(FFI_SRC_DIR)/silica_legacy_math.c | $(FFI_BUILD_DIR)
	$(FFI_CC) $(FFI_CFLAGS) -c $< -o $@

$(FFI_TEXT_OBJ): $(FFI_SRC_DIR)/silica_text.c | $(FFI_BUILD_DIR)
	$(FFI_CC) $(FFI_CFLAGS) -c $< -o $@

$(FFI_NET_OBJ): $(FFI_SRC_DIR)/silica_net.c | $(FFI_BUILD_DIR)
	$(FFI_CC) $(FFI_CFLAGS) -c $< -o $@

$(FFI_LEGACY_ARCHIVE): $(FFI_LEGACY_OBJ) | $(FFI_LIB_DIR)
	ar rcs $@ $^

$(FFI_TEXT_ARCHIVE): $(FFI_TEXT_OBJ) | $(FFI_LIB_DIR)
	ar rcs $@ $^

$(FFI_NET_ARCHIVE): $(FFI_NET_OBJ) | $(FFI_LIB_DIR)
	ar rcs $@ $^
