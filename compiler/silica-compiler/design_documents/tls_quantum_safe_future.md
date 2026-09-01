# First-Class Quantum-Safe TLS

**Status:** Future development. Not implemented. Not normative for the current compiler. When this work starts, [silica-specification.md](silica-specification.md) §20.4, [silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md), and the supervisor plan gain a TLS section; this document stays the design authority until then.

## Related Documents

| Document | Purpose |
| --- | --- |
| [silica-specification.md](silica-specification.md) | Effects (§9), actors and supervisors (§15), networking sketch (§20.4) |
| [silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md) | Fifi: `dangerous_*`, `spawn_dangerous`, `external_danger`, structural taint |
| [silica_actor_capabilities_specification.md](silica_actor_capabilities_specification.md) | Cast-only / dangerous actor capabilities |
| [crypto-proposal-introduction.md](crypto-proposal-introduction.md) | Secret labels, constant-time rules, zeroization |
| [brokered_ipc_isolation_architecture.md](brokered_ipc_isolation_architecture.md) | Process isolation for unsafe networking libraries; broker authenticity |
| [IPC_OS.md](IPC_OS.md) | OS-hosted local datagrams (not TLS) |
| [IPC_Bare.md](IPC_Bare.md) | Bare-metal local datagrams (not TLS) |

---

## 1. Purpose

Silica TLS is a **first-class language and runtime feature**, in the same class as `List`, `uint64`, `actor_ref`, and `supervisor_ref`. Every TLS connection is a **supervised `spawn_dangerous` actor**. Handshake, record protection, and peer authentication are **quantum-safe by default** (TLS 1.3 + hybrid ML-KEM). Application code never sees a file descriptor, an `SSL *`, or a foreign engine type.

This document does not implement TLS. It locks the types, actor tree, taint rules, and algorithm profile so later work does not invent “encrypt some bytes and `net.tcp.write`.”

---

## 2. What first-class means

- The compiler knows the types. User code cannot construct a TLS session from a buffer or a foreign pointer.
- Session create, use, and close are **intrinsics** (same category as `spawn`, `cast`, `spawn_registered_supervisor`), not an optional library an application `use`s by accident.
- Effects, taint, and supervision are checked. A plaintext `net.tcp` handle cannot be passed where a TLS session is required.
- Diagnostics cite this document and the spec sections it extends.
- Backends (s2n, rustls, Mbed TLS + a FIPS 203 KEM, hardware offload) are invisible to application source.

First-class does **not** mean “looks pure.” Each connection still follows Fifi’s dangerous-worker and taint rules.

---

## 3. Non-goals

- Implementing TLS in the current compiler or runtime
- Transpiling or lifting OpenSSL / BoringSSL
- HTTP, QUIC, or DTLS (later documents may sit on `tls_session_ref`)
- Validator-based de-taint (Fifi does not define it)
- Using TLS to authenticate a local broker or supervisor
- Encrypting `IPC_Bare` / `IPC_OS` datagrams with this stack

---

## 4. Quantum-safe profile (locked)

Industry practice is **TLS 1.3 + hybrid KEM**. Pure classical key exchange is not Silica TLS. Pure post-quantum key exchange without a classical hybrid is allowed only as an explicit policy, not the default (if ML-KEM were broken, hybrid still has X25519).

| Item | Required default | Forbidden |
| --- | --- | --- |
| Protocol | TLS 1.3 only (RFC 8446) | TLS 1.0–1.2, SSL |
| Key exchange | Hybrid **X25519MLKEM768** (X25519 + ML-KEM-768, FIPS 203) | RSA kex, static DH, X25519-only, P-256-only |
| Optional KEM | `ML-KEM-1024` or hybrid with ML-KEM-1024 when `kem_mode = kem_pq_strict` | Pre-FIPS Kyber drafts |
| Record AEAD | AES-256-GCM or ChaCha20-Poly1305 | AES-128-GCM, 3DES, CBC+HMAC suites |
| Transcript hash | SHA-256 or SHA-384 as TLS 1.3 dictates for the suite | MD5, SHA-1 |
| Certificates (`signature_mode = sig_transition`) | Ed25519 / ECDSA-P256 **or** ML-DSA-65 (FIPS 204) | RSA-1024, SHA-1 signatures |
| Certificates (`signature_mode = sig_pq_only`) | ML-DSA-65 or SLH-DSA (FIPS 205) only | Any classical signature |
| 0-RTT | Off. No early data | 0-RTT unless a later revision names a policy that allows it |
| Compression / renegotiation | None | Both |

Symmetric 256-bit is the Grover floor. Hybrid KEM is the harvest-now-decrypt-later floor. Post-quantum **signatures** lag public PKI; that is why they are a policy knob, not a silent default.

Trust material is a Silica `TrustBundleId` (named CA set or SPKI pins). The worker must not use “the library default store” unless the policy names a platform bundle id.

**Clock:** certificate expiry needs trusted time. OS-hosted: kernel clock. Bare metal: board clock, or pins plus a documented “no expiry check.” No silent skip.

---

## 5. Types

New primitives, not user structs. There is no coercion among `tls_session_ref`, `tls_listener_ref`, `actor_ref`, `dangerous_actor_ref`, and `supervisor_ref`.

`tls_session_ref` is in the **dangerous-actor family**: it is obtained only from TLS install intrinsics, the same way `dangerous_actor_ref` is obtained only from `spawn_dangerous`.

Examples below use **selfhost dialect** (inline records, `{` / `}` case arms).

```silica
tls_policy: {
    hostname: string,
    alpn: List[string, mem(normal)],
    trust: TrustBundleId,
    kem_mode: kem_hybrid | kem_pq_strict,
    signature_mode: sig_transition | sig_pq_only,
    role: tls_client | tls_server,
    reconnect: reconnect_never | reconnect_transient
}

tls_error:
    PolicyRejected
  | HandshakeFailed
  | CertUntrusted
  | Closed
  | Timeout
  | MailboxFull
  | SessionDead
```

`hostname` is SNI and certificate name-check input. The runtime validates encoding and length **before** any engine call (same posture as brokered `NetOpen`).

### 5.1 Intrinsics

Application sequences declare **`concurrency` only** (Fifi install vs execute: the install site must not declare `external_danger`).

```text
tls_connect(addr, policy) -> tls_session_ref     proc[concurrency]
tls_listen(addr, policy)  -> tls_listener_ref    proc[concurrency]
tls_close(session)        -> atom                proc[concurrency]
```

### 5.2 Data plane (cast-only)

Clients that talk to a dangerous worker are cast-only. There is no `call` on a TLS session.

```text
cast(session, TlsWrite { payload, reply_to })
cast(session, TlsRecv  { max_bytes, reply_to })
cast(session, TlsClose {})
```

There is no `tls_write` that takes a `tcp_connection`. There is no “AEAD this buffer and `net.tcp.write`.”

---

## 6. One connection, one supervised actor

```text
TlsSupervisor          (spawn_registered_supervisor, one-for-one)
│
├── session A          spawn_dangerous  →  tls_session_ref
├── session B          spawn_dangerous
└── listener L         spawn_dangerous  →  tls_listener_ref
        └── accept → spawn_dangerous child session
```

Rules:

1. **Exactly one TLS session actor per connection.** Sharing one worker across sockets is a compile-time or runtime error.
2. The session actor is installed with `spawn_dangerous` (or `spawn_dangerous_registered`). Ordinary `spawn` must not run handshake, record I/O, or `dangerous_*` engine calls.
3. Engine calls occur only in `sequence proc[external_danger] ... produces pure ... end` inside that actor.
4. `produces pure` is only session state (generation, idle, counters). No foreign handles, no tainted plaintext.
5. Inbound plaintext leaves only on the **FFI result cast** to the `reply_to` named in that request ([silica_ffi_wrapper_specification.md](silica_ffi_wrapper_specification.md) §4.2 / §7.6).
6. The supervisor is one-for-one. Default child restart is **transient**: a dead session does **not** auto-handshake again. Outstanding client casts complete with `SessionDead`. `reconnect_transient` may retry **client** sessions only, with a new generation; in-flight writes are not replayed.
7. Secrets live on the session actor’s stack. Restart and `tls_close` zeroize ([crypto-proposal-introduction.md](crypto-proposal-introduction.md)). A restarted actor does not inherit the old record state or pending calls (spec §15).
8. Intensity limits (max restarts / window) apply to the TLS supervisor like any other supervisor. Handshake storms escalate; they do not grow an unbounded child table.

Application actors hold `tls_session_ref` and only `cast`. They never hold a file descriptor, `SSL *`, or engine context.

---

## 7. Taint and dangerous

Existing Fifi rules apply unchanged.

| Rule | TLS meaning |
| --- | --- |
| `dangerous_*` only inside the worker’s `external_danger` sequence | Engine (C or hardware wrapper) is not called from application actors |
| Install site is `concurrency` only | `tls_connect` / `tls_listen` must not sit in `proc[external_danger]` |
| Structural taint | Every value that comes from the engine — including decrypted plaintext — is external-danger-touched |
| No tainted data in `produces pure` | Session state stays pure; payload goes out on the result cast |
| No tainted data in `device_io` / `network_io` / `hot_swap` / `register_rwr` | `TlsRecv` bytes cannot be used with `print`, file prims, or another socket in those sequences |
| No ordinary `call` / `cast` of tainted data except the designated result cast | App-to-app forwarding of raw recv bytes is rejected until a later de-taint spec exists |
| No coercion | `tls_session_ref` is not an `actor_ref` |

**Module naming:** Application modules that only use TLS **intrinsics** are **not** required to take the `dangerous_` prefix. That matches `spawn_dangerous` today (install is not a `use dangerous_*`). The engine module, if it is a real `dangerous_*` archive, is used only by the compiler/runtime, not by application `use`. If application code `use`s that engine module directly, the existing cascade applies all the way to the application root.

**Trust:** Worker crypto authenticates the **remote peer** (certificates + hybrid/PQ handshake). It does **not** authenticate a local broker. Broker and TLS-supervisor identity stay OS identity or MPU/linker identity ([brokered_ipc_isolation_architecture.md](brokered_ipc_isolation_architecture.md), Broker Authenticity).

Decrypted bytes are useful and still tainted. Writing them to disk or another network hop requires another dangerous worker or a future validator-based de-taint. Do not weaken taint to make HTTPS easier.

---

## 8. Effects

| Site | Effects |
| --- | --- |
| Application `cast` to a session | `concurrency` |
| `tls_connect` / `tls_listen` / `tls_close` | `concurrency` |
| Session actor engine I/O | `external_danger` (and `network_io` only **inside** that worker if the backend uses kernel sockets — never on the application sequence) |
| Application using recv payload in `print` / file / `network_io` | Compile error (taint × restricted effect) |

`device_io` stays print, file, and console. TLS is not `device_io`.

---

## 9. OS-hosted vs bare metal

Same types and actor tree. Only the backend changes.

- **OS-hosted:** kernel TCP; the TLS engine runs in the session actor (or a brokered worker reached over `IPC_OS`). Unix-domain channels between a safe app and a broker are still not TLS.
- **Bare metal:** a byte-stream prim (not the actor mailbox). The session actor is still `spawn_dangerous`. A post-quantum KEM may be a hardware prim under `register_rwr` **inside** the worker only.

`IPC_Bare` and `IPC_OS` datagrams stay local. They are not `tls_session_ref`.

---

## 10. Compiler obligations (when this leaves “future”)

1. Prelude types: `tls_session_ref`, `tls_listener_ref`, `tls_policy`, `tls_error`.
2. Reject `net.tcp.write` of application payloads where policy requires TLS.
3. Reject `call` on a TLS session (cast-only).
4. Reject one session actor bound to two connections.
5. Reject `external_danger` at `tls_connect` / `tls_listen`.
6. Taint recv payloads; enforce Fifi §7.3.
7. Intrinsic lowering: spawn a supervisor child, install the worker, never expose engine types in `.iface`.
8. Trials: handshake policy reject, taint-to-`print` error, supervisor `SessionDead`, no TLS 1.2 suite.

---

## 11. Implementation order (still future)

1. Types, effects, supervisor tree, and goldens that only check **rejects**.
2. OS-hosted backend: one PQ-capable engine (s2n or rustls + ML-KEM), hybrid default.
3. Bare-metal backend behind the same refs.
4. `sig_pq_only` and ML-DSA certificates when the PKI story exists.

Do not start from a C-to-Silica or assembly-to-Silica conversion of a TLS library.
