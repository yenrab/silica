#ifndef SILICA_TEXT_WRAPPER_H
#define SILICA_TEXT_WRAPPER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct silica_text_echo_result {
    int64_t tag;
    const uint8_t *text_ptr;
    uint64_t text_len;
    int64_t error_code;
} silica_text_echo_result;

silica_text_echo_result silica_text_echo(const uint8_t *text_ptr, uint64_t text_len);

#ifdef __cplusplus
}
#endif

#endif
