#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

void silica_ffi_null_deref(void) {
    volatile int *p = 0;
    (void)*p;
}

void silica_ffi_past_end_of_ptr(uint8_t *base, int64_t offset) {
    if (base == 0) {
        volatile int *p = 0;
        (void)*p;
        return;
    }
    base[offset] = 1;
}

extern int64_t silica_rt_ffi_trial_arena_base(void);

void silica_ffi_arena_past_end(void) {
    uint8_t *base = (uint8_t *)(uintptr_t)silica_rt_ffi_trial_arena_base();
    if (base == 0) {
        silica_ffi_null_deref();
        return;
    }
    base[262144] = 1;
}

void silica_ffi_sigbus_probe(void) {
    char path[] = "/tmp/silica_sigbus_XXXXXX";
    int fd = mkstemp(path);
    if (fd < 0) {
        return;
    }
    (void)unlink(path);
    if (write(fd, "x", 1) != 1) {
        close(fd);
        return;
    }
    void *p = mmap(0, 4096, PROT_READ, MAP_PRIVATE, fd, 0);
    close(fd);
    if (p == MAP_FAILED) {
        return;
    }
    volatile char c = ((char *)p)[4096];
    (void)c;
}

int main_unguarded_null_deref_trial(void) {
    silica_ffi_null_deref();
    return 0;
}
