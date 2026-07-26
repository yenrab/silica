# Embedded Actor Runtime (Bare Metal)

## Purpose

This document describes an Erlang-inspired actor runtime for embedded systems running without an operating system.

It also defines how **independent applications on the same piece of hardware** exchange messages—including **non-ASCII text**—without an operating system or network stack. Communication stays on-device; payloads are opaque bytes with **UTF-8** as the standard encoding whenever applications send Silica `string` values or other human-readable text.

Examples include

- STM32
- RP2040
- AVR
- PIC
- MSP430
- Bare-metal ESP32

The runtime provides asynchronous message passing between actors while performing all scheduling internally. Applications on the same device never share mutable memory; they communicate only through bounded datagram-style messages.

---

# Design Goals

- No operating system
- No shared mutable state
- Message-oriented communication
- Same-device application isolation with local-only delivery
- UTF-8 text interchange (full Unicode scalar values, including non-ASCII)
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

# Same-Device Applications

Multiple applications may run on one piece of hardware without a general-purpose operating system—for example separate firmware images, core-local runtimes, or MPU-isolated regions on a multi-core MCU.

```
Device
│
├── Application: UI
│   └── actors …
│
├── Application: Control
│   └── actors …
│
└── Application: Logger
    └── actors …
```

Each application owns private memory. The runtime (or a thin shared transport layer) delivers **datagram messages** between application mailboxes. Messages never leave the device and never traverse TCP/IP or other off-board links as part of this design.

Applications agree on logical names and message types at compile time or through a shared registry; they do not pass raw pointers across the boundary.

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

Suggested wire record (selfhost dialect: inline record type, bound to a variable)

```silica
// L_ipc: static lifetime for mailbox storage (bare-metal runtime region)
// MAX_PAYLOAD: compile-time byte capacity (e.g. 256)

msg: {
    type: uint16,
    sender: uint16,
    length: uint16,
    payload: buf(L_ipc, normal, uint8, MAX_PAYLOAD)
} <- {
    type: msg_type,
    sender: sender_id,
    length: byte_len,
    payload: payload_buf
}
```

Application-level text uses Silica `string` at the API boundary; the runtime encodes it into `payload` as UTF-8 bytes before send and decodes after receive:

```silica
log_line: Line { text: string } <- Line { text: greeting }
```

The `length` field is the **byte length** of the UTF-8 (or other opaque) bytes copied into `payload`, not a Unicode character count.

---

# Text and Unicode

Silica `string` values are UTF-8 encoded (see `silica-specification.md` §4). When applications exchange human-readable text—including accented letters, CJK, emoji, and other non-ASCII characters—the **payload bytes MUST be valid UTF-8**.

Rules:

- The transport is **encoding-neutral**: it copies and delivers opaque byte sequences and preserves message boundaries. It does not inspect or normalize text.
- Each complete UTF-8 string intended for the receiver MUST fit in a single message payload, unless the applications define an explicit multi-part reassembly protocol.
- Do not rely on NUL (`0x00`) termination; use `length` (and application message types) instead.
- If a receiver decodes a text payload and finds invalid UTF-8, that is an **application-level error**; the transport has already delivered the datagram successfully.
- Binary and text payloads may coexist: distinguish them with the `type` field or application-defined message variants.

Example: sending the greeting `"café 🔥"` from one application to another copies eleven UTF-8 bytes (including multi-byte `é` and four-byte emoji) as one bounded payload; the receiver reconstructs a Silica `string` from those bytes.

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
- Local-only messaging on one device
- Full Unicode text without an OS locale or socket stack
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

Independent applications on the same hardware communicate through bounded local datagrams—never through shared mutable memory or off-device networking.

Text payloads use UTF-8 so applications can exchange non-ASCII and full Unicode strings consistently with Silica `string`.

The runtime scheduler dispatches one message at a time to ready actors.

Every actor belongs to a supervisor.

The result is an Erlang-inspired runtime that provides messaging, supervision, and fault recovery while requiring no operating system.