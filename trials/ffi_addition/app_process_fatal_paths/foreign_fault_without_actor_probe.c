/* A guarded foreign call made with no current actor, as the emitted code makes one: enter the guarded
   region, arm the recovery point, call the foreign code, and on the recovery return hand the fault to
   silica_rt_ffi_guarded_fault_finish. Silica source cannot do this (a guarded call has to be the root
   body of a spawn_dangerous behavior, so an actor is always current), so it runs here, in a
   constructor, before main. The recovery point returns twice; which return this is is read from a
   flag, not from the entry's result, because the x86-64 runtime answers in rdi, not in the C result
   register. The Makefile links this object after the Silica runtime object, so the runtime's
   constructor has installed the fault handlers by now. */
extern void silica_rt_ffi_guarded_enter(void);
extern void silica_rt_ffi_guarded_setjmp(void);
extern void silica_rt_ffi_guarded_fault_finish(void);

static volatile int silica_trial_armed = 0;

static void silica_trial_foreign_code(void) {
    volatile int *p = 0;
    (void)*p;
}

__attribute__((constructor))
static void silica_trial_guarded_call_without_actor(void) {
    silica_rt_ffi_guarded_enter();
    silica_rt_ffi_guarded_setjmp();
    if (silica_trial_armed) {
        silica_rt_ffi_guarded_fault_finish(); /* never returns */
        return;
    }
    silica_trial_armed = 1;
    silica_trial_foreign_code();
}
