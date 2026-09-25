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

// Actor-runtime entry points every program's main calls, before the ESP32-S3 actor runtime exists.
//
// The emitter injects calls to the two PID-registry initialisers at the top of every program entry
// point (emitter_core emit_sir_function). On AArch64 they spawn the registry actors; here there are no
// actors yet, so they do nothing. They are replaced when the cooperative scheduler is ported.

    .section .text.silica_rt_actors_stub, "ax"

    .align  4
    .global silica_pid_registry_init
    .type   silica_pid_registry_init, @function
silica_pid_registry_init:
    entry   a1, 32
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_pid_registry_init, . - silica_pid_registry_init

    .align  4
    .global silica_dangerous_pid_registry_init
    .type   silica_dangerous_pid_registry_init, @function
silica_dangerous_pid_registry_init:
    entry   a1, 32
    movi    a2, 0
    movi    a3, 0
    retw
    .size   silica_dangerous_pid_registry_init, . - silica_dangerous_pid_registry_init
