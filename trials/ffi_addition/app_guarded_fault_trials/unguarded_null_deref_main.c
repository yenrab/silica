#include <stdint.h>

void silica_ffi_null_deref(void);

int64_t silica_rt_ffi_trial_arena_base(void) {
    return 0;
}

int main(void) {
    silica_ffi_null_deref();
    return 0;
}
