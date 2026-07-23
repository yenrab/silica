# Embedded Actor Runtime (Bare Metal)

## Purpose

This document describes an Erlang-inspired actor runtime for embedded systems running without an operating system.

Examples include

- STM32
- RP2040
- AVR
- PIC
- MSP430
- Bare-metal ESP32

The runtime provides asynchronous message passing between actors while performing all scheduling internally.

---

# Design Goals

- No operating system
- No shared mutable state
- Message-oriented communication
- Deterministic execution
- Automatic supervision
- Automatic restart
- Interrupt-safe
- Zero dynamic allocation
- Portable across microcontrollers
- Small memory footprint

---

# System Model

Each actor is a runtime-managed execution unit.

Each actor owns

- private state
- mailbox
- supervisor

```
Runtime
│
├── Scheduler
├── Supervisor
├── Actor A
├── Actor B
├── Actor C
└── Actor D
```

Actors communicate only through messages.

Actors never directly invoke one another.

---

# Mailboxes

Each actor owns one circular mailbox.

```
Mailbox

+-----+
| Msg |
+-----+
| Msg |
+-----+
| Msg |
+-----+
```

Only the owning actor removes messages.

Many actors may enqueue messages.

---

# Message Format

```
+---------+---------+----------+----------+
| Type    | Sender  | Length   | Payload  |
+---------+---------+----------+----------+
```

Suggested structure

```c
typedef struct
{
    uint16_t type;
    uint16_t sender;
    uint16_t length;
    uint8_t payload[MAX_PAYLOAD];
} Message;
```

---

# Sending

```
send(destination, message)
```

Runtime steps

1. Locate destination mailbox.
2. Copy message.
3. Mark destination actor ready.
4. Return immediately.

---

# Receiving

```
receive()
```

Runtime steps

1. Remove oldest message.
2. Process message.
3. Return to scheduler.

Actors never block.

If no message exists, the scheduler selects another ready actor.

---

# Scheduling

Scheduling is performed by the runtime.

```
while (true)
{
    actor = next_ready_actor();

    dispatch_one_message(actor);
}
```

Each scheduling step processes one message.

Actors voluntarily return control after completing the message.

---

# Supervision

Every actor belongs to exactly one supervisor.

```
Root Supervisor
│
├── Communications
│   ├── UART
│   ├── SPI
│   └── CAN
│
├── Sensors
│   ├── GPS
│   ├── IMU
│   └── Temperature
│
└── Application
    ├── Navigation
    └── Control
```

Supervisors

- create actors
- monitor actors
- restart actors
- apply restart policies

Actors never supervise one another.

---

# Restart

Restarting an actor consists of

1. Clearing mailbox.
2. Reinitializing actor state.
3. Incrementing generation.
4. Returning actor to the scheduler.

Other actors continue executing normally.

---

# Registration

Actors communicate using logical identities.

```
navigation

control

gps

logger
```

The runtime maps

```
Logical Name
        ↓
Mailbox
        ↓
Actor Descriptor
```

Applications never use memory addresses directly.

---

# Interrupt Handling

Interrupt handlers never execute application logic.

```
Interrupt

↓

Create Message

↓

Queue Message

↓

Return
```

The scheduler later delivers the message to the appropriate actor.

---

# Memory

Each actor owns

- mailbox
- private state

All memory is statically allocated.

No heap allocation is required.

---

# Advantages

- No operating system required
- Deterministic execution
- Very small footprint
- No shared mutable state
- Automatic supervision
- Automatic restart
- Excellent modularity
- Highly portable

---

# Future Extensions

- Priority mailboxes
- Timers
- Delayed messages
- Publish/Subscribe
- DMA message producers
- Multi-core support

---

# Summary

Each actor owns private state and one mailbox.

The runtime scheduler dispatches one message at a time to ready actors.

Every actor belongs to a supervisor.

The result is an Erlang-inspired runtime that provides messaging, supervision, and fault recovery while requiring no operating system.