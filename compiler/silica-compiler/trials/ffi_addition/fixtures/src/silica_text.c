#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define ECHO_PREFIX "Echo: "

typedef struct silica_text_echo_result {
    int64_t tag;
    const uint8_t *text_ptr;
    uint64_t text_len;
    int64_t error_code;
} silica_text_echo_result;

silica_text_echo_result silica_text_echo(const uint8_t *text_ptr, uint64_t text_len) {
    silica_text_echo_result result = {0, 0, 0, 0};

    if (text_ptr == 0) {
        result.tag = 1;
        result.error_code = 1;
        return result;
    }

    const size_t prefix_len = strlen(ECHO_PREFIX);
    const size_t total_len = prefix_len + (size_t)text_len;
    uint8_t *buf = (uint8_t *)malloc(total_len);
    if (buf == 0) {
        result.tag = 1;
        result.error_code = 2;
        return result;
    }

    memcpy(buf, ECHO_PREFIX, prefix_len);
    if (text_len > 0) {
        memcpy(buf + prefix_len, text_ptr, (size_t)text_len);
    }

    result.tag = 0;
    result.text_ptr = buf;
    result.text_len = (uint64_t)total_len;
    result.error_code = 0;
    return result;
}
