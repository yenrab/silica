/*
   Copyright 2026 Lee Scott Barney

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

// Bring-up app 12: the actor runtime (rt_actors.S) driven from hand-written assembly, in the
// emitted-code ABI (arguments in a10:a11, a12:a13, a14:a15; X3/X4 in the core's register block at
// THREADPTR; results in
// a10:a11), before the emitter itself uses it. A behaviour written the way the emitter writes one
// (a2..a7 in, tuple pointer out through the X2 buffer).
//
// Expected console:
//   call 1: 5        (spawn(2, adder), call(a, 3) -> 5)
//   call 2: 12       (call(a, 7): state 5 + 7)
//   cast: 1          (cast(a, 100) -> 1; the actor's state becomes 112 once main waits)
//   status 0         (wait_for_exit ends the program once every actor is idle, as the host's exit
//                    on the harness's "exit" line does; the registry and kill_abnormal code after
//                    it below never runs)
// With two cores the adder is runtime-assigned (round robin), so it may run on either core.

    .section .text.main, "ax"
    .literal_position

// adder(msg: int64, state: int64) -> (:reply, msg + state, msg + state)
    .align  4
adder:
    entry   a1, 32
    add     a8, a2, a4                  // low words only in this test
    movi    a9, 0
    s32i    a9, a6, 0
    s32i    a9, a6, 4
    s32i    a8, a6, 8
    s32i    a9, a6, 12
    s32i    a8, a6, 16
    s32i    a9, a6, 20
    mov     a2, a6
    movi    a3, 0
    retw

// echo(msg, state) -> (:reply, msg, state)
    .align  4
echo:
    entry   a1, 32
    movi    a9, 0
    s32i    a2, a6, 8
    s32i    a3, a6, 12
    s32i    a4, a6, 16
    s32i    a5, a6, 20
    mov     a2, a6
    movi    a3, 0
    retw

    .align  4
    .global main
    .type   main, @function
main:
    entry   a1, 64
    call8   silica_pid_registry_init
    call8   silica_dangerous_pid_registry_init
    // a = spawn(2, adder, core 0, no trampoline, policy default)
    rur     a8, threadptr
    movi    a9, 0
    s32i    a9, a8, 0
    s32i    a9, a8, 8
    movi    a10, 2
    movi    a11, 0
    movi    a12, adder
    movi    a13, 0
    movi    a14, 0
    movi    a15, 0
    call8   silica_rt_actor_spawn
    mov     a2, a10                     // a2 = a
    movi    a10, msg_call1
    call8   silica_rt_puts
    mov     a10, a2
    movi    a11, 0
    movi    a12, 3
    movi    a13, 0
    call8   silica_rt_actor_call
    call8   silica_rt_print_u32
    movi    a10, 10
    call8   silica_rt_putc
    movi    a10, msg_call2
    call8   silica_rt_puts
    mov     a10, a2
    movi    a11, 0
    movi    a12, 7
    movi    a13, 0
    call8   silica_rt_actor_call
    call8   silica_rt_print_u32
    movi    a10, 10
    call8   silica_rt_putc
    movi    a10, msg_cast
    call8   silica_rt_puts
    mov     a10, a2
    movi    a11, 0
    movi    a12, 100
    movi    a13, 0
    call8   silica_rt_actor_cast
    call8   silica_rt_print_u32
    movi    a10, 10
    call8   silica_rt_putc
    call8   silica_rt_wait_for_exit
    movi    a10, msg_after
    call8   silica_rt_puts
    mov     a10, a2
    movi    a11, 0
    movi    a12, 100
    movi    a13, 0
    call8   silica_rt_actor_call
    call8   silica_rt_print_u32
    movi    a10, 10
    call8   silica_rt_putc
    // registry: register a under name 5, look up 5 and 6 through the registry actor
    movi    a10, msg_registry
    call8   silica_rt_puts
    mov     a10, a2
    movi    a11, 0
    movi    a12, 5
    movi    a13, 0
    call8   silica_rt_register_actor_named
    call8   silica_rt_get_registry_actor
    mov     a3, a10                     // a3 = registry
    movi    a4, 16                      // op :lookup
    s32i    a4, a1, 0                   // message record {op, name, pid} at [a1, a1+24)
    movi    a4, 0
    s32i    a4, a1, 4
    movi    a4, 5
    s32i    a4, a1, 8
    s32i    a2, a1, 16
    mov     a10, a3
    movi    a11, 0
    mov     a12, a1
    movi    a13, 0
    call8   silica_rt_actor_call
    l32i    a10, a10, 0                 // the (status, ref) pair: status
    call8   silica_rt_print_u32
    movi    a10, 32
    call8   silica_rt_putc
    movi    a4, 6
    s32i    a4, a1, 8
    mov     a10, a3
    movi    a11, 0
    mov     a12, a1
    movi    a13, 0
    call8   silica_rt_actor_call
    l32i    a10, a10, 0
    call8   silica_rt_print_u32
    movi    a10, 10
    call8   silica_rt_putc
    // e = spawn(0, echo); kill_abnormal(e); call(e, 9) -> 0
    rur     a8, threadptr
    movi    a9, 0
    s32i    a9, a8, 0
    s32i    a9, a8, 8
    movi    a10, 0
    movi    a11, 0
    movi    a12, echo
    movi    a13, 0
    movi    a14, 0
    movi    a15, 0
    call8   silica_rt_actor_spawn
    mov     a5, a10
    call8   silica_rt_actor_kill_abnormal
    movi    a10, msg_killed
    call8   silica_rt_puts
    mov     a10, a5
    movi    a11, 0
    movi    a12, 9
    movi    a13, 0
    call8   silica_rt_actor_call
    call8   silica_rt_print_u32
    movi    a10, 10
    call8   silica_rt_putc
    movi    a2, 42
    movi    a3, 0
    retw
    .size   main, . - main

    .section .rodata.main, "a"
msg_call1:    .asciz "call 1: "
msg_call2:    .asciz "call 2: "
msg_cast:     .asciz "cast: "
msg_after:    .asciz "after wait: "
msg_registry: .asciz "registry: "
msg_killed:   .asciz "killed call: "
