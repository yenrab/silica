// Trial-local C entry used by the Silica foreign wrapper (linked via libsilica_legacy_math.a).
// Delegates to the shared guarded-fault probe in fixtures/src/silica_ffi_fault.c.

#include <stdint.h>

void silica_ffi_null_deref(void);

int64_t silica_supervisor_trial_guarded_fault(void) {
    silica_ffi_null_deref();
    return 0;
}
