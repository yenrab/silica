/* Linked after the Silica runtime object (see Makefile), so this constructor runs after the runtime's
   own constructor has installed the fault handlers, and before main. */
__attribute__((constructor))
static void silica_trial_fault_before_main(void) {
    volatile int *p = 0;
    (void)*p;
}
