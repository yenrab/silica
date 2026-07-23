# Local Inter-Process Messaging Architecture

## Purpose

This document describes a message-passing architecture for applications running on modern operating systems such as:

- macOS
- Linux
- iOS
- Android

The goal is to provide Erlang-like asynchronous messaging between independent operating system processes without shared memory.

---

# Design Goals

- Message-oriented communication
- Private process memory
- Asynchronous send
- Blocking or asynchronous receive
- Reliable delivery
- Ordered delivery from each sender
- Automatic process supervision
- Automatic process restart
- Stable process identities
- Simple binary protocol
- High throughput
- Low latency
- Portable implementation

---

# Transport

The transport uses Unix Domain Datagram sockets.

```
AF_UNIX
SOCK_DGRAM
```

Reasons:

- Supported on all target operating systems.
- Preserves message boundaries.
- Bypasses the TCP/IP stack.
- Kernel-managed communication.
- Allows many senders to communicate with one receiver.
- Low overhead.

---

# Mailbox Model

Every process owns exactly one mailbox.

```
             +----------------+
             |   Process C    |
             |                |
             |   Mailbox      |
             +----------------+
                ▲     ▲     ▲
                │     │     │
          +-----+     │     +------+
          │           │            │
      Process A   Process B    Process D
```

Every mailbox is represented by a Unix domain socket.

Each process binds exactly one socket.

Other processes send directly to that mailbox.

No central router participates in normal message delivery.

---

# Process Identity

Applications never communicate using operating system process IDs.

Instead every process owns a stable logical identity.

Example

```
storage

logger

worker_a

worker_b
```

The runtime registry maps

```
Logical Name
        ↓
Mailbox
        ↓
OS Process ID
        ↓
Generation
```

Example

```
worker_a

↓

Mailbox:
/tmp/runtime/worker_a.sock

OS PID:
48192

Generation:
14
```

If the process crashes and is restarted

```
worker_a

↓

Mailbox:
/tmp/runtime/worker_a.sock

OS PID:
49071

Generation:
15
```

Applications continue sending to the logical process name.

---

# Message Format

Messages consist of a fixed-size header followed by an opaque payload.

```
+---------+---------+----------+----------------+
| Version | Type    | Sender   | Payload        |
+---------+---------+----------+----------------+
```

Suggested header

```c
struct Header
{
    uint8_t  version;
    uint8_t  flags;
    uint16_t type;
    uint32_t sender;
    uint64_t correlation;
};
```

The payload is application defined.

---

# Sending

```
send(destination, message)
```

Runtime steps

1. Lookup destination.
2. Encode message.
3. sendto().
4. Return immediately.

The sender never blocks waiting for the receiver to process the message.

---

# Receiving

```
receive()
```

Runtime steps

1. recvfrom()
2. Decode header.
3. Deliver message.

The receiver processes one message at a time.

---

# Supervision

Every process belongs to exactly one supervisor.

Example

```
Root Supervisor
│
├── Storage Supervisor
│   ├── Storage
│   └── Cache
│
├── Network Supervisor
│   ├── Receiver
│   └── Sender
│
└── Application Supervisor
    ├── Worker A
    └── Worker B
```

Supervisors

- start child processes
- monitor child processes
- restart failed child processes
- apply restart policies
- report failures

Workers never monitor one another directly.

This eliminates the complete-graph monitoring problem.

---

# Restart Policies

The runtime supports supervisor restart strategies.

Examples include

- Permanent
- Transient
- Temporary

Possible supervision strategies include

- One-for-one
- One-for-all
- Rest-for-one

---

# Registry

The registry maintains runtime information.

```
Logical Name
        ↓
Mailbox
        ↓
Generation
        ↓
Capabilities
```

The registry performs only address resolution.

It is **not** responsible for monitoring process health.

---

# Failure Detection

Supervisors detect failures using operating system notifications.

Examples include

- child process exit
- abnormal termination
- signal delivery

Workers do not periodically exchange keep-alive messages.

The monitoring graph forms a tree rather than a complete graph.

---

# Advantages

- Independent processes
- No shared memory
- No mutexes
- Natural actor programming model
- Automatic restart
- Stable identities
- Sparse supervision graph
- Excellent portability
- Kernel-managed buffering
- Direct process-to-process messaging

---

# Future Extensions

Possible additions include

- Process groups
- Publish/Subscribe
- Broadcast
- Priority mailboxes
- Distributed supervision
- Transparent remote messaging
- Zero-copy large binary transport
- Capability-based security

---

# Summary

Each operating system process owns a mailbox implemented using a Unix Domain Datagram socket.

Processes communicate only through asynchronous messages.

Every process belongs to a supervisor.

Supervisors maintain process lifecycles while remaining outside the normal message path.

The result is an Erlang-inspired runtime supporting supervision, automatic restart, stable identities, and efficient local messaging across macOS, Linux, iOS, and Android.