#include <stdint.h>

int64_t silica_legacy_math_add_int64(int64_t left, int64_t right) {
    return left + right;
}

/* Recurses n deep on the caller's stack (the actor's runtime-managed stack when called from a
 * spawn_dangerous worker), returning the depth reached. volatile keeps the frames real. */
int64_t silica_legacy_math_recurse(int64_t n) {
    volatile int64_t pad[8];
    pad[0] = n;
    if (n <= 0) {
        return 0;
    }
    return silica_legacy_math_recurse(n - 1) + 1 + (pad[0] - n);
}
