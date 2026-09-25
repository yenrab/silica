#include <dlfcn.h>

/* The runtime's fault handler calls dladdr to name the faulting code. This definition, linked into the
   program, takes the place of the C library's and faults, so the second fault is raised while the fault
   handler is running. Nothing else in this program calls dladdr. */
int dladdr(const void *addr, Dl_info *info) {
    (void)addr;
    (void)info;
    volatile int *p = 0;
    return *p;
}

/* The first fault: before main, after the runtime's constructor has installed the handlers (the
   Makefile links this object after the Silica runtime object). */
__attribute__((constructor))
static void silica_trial_first_fault(void) {
    volatile int *p = 0;
    (void)*p;
}
