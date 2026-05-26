#ifndef SILICA_LEGACY_MATH_WRAPPER_H
#define SILICA_LEGACY_MATH_WRAPPER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int64_t silica_legacy_math_add_int64(int64_t left, int64_t right);
void silica_ffi_sigfpe_probe(void);

#ifdef __cplusplus
}
#endif

#endif
