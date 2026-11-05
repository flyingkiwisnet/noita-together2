# Local app/game WebSocket

This MIT-licensed source crate shares Tokio/tungstenite framing and standard
HMAC-SHA256 between the desktop listener and the x86 broker static library.
The broker remains the only native networking owner. Lua exports are unchanged.
PollNet's inspected string queue API was not adopted for these binary messages.

Production binds literal `127.0.0.1:25570`, exact `/nt2/local/v1`, without
compression or pipe fallback. Private component fixtures can bind another
loopback port to represent separate PCs. Origin, extensions, subprotocols,
nonliteral Host, alternate paths and query strings are refused.

The user-only `.nt2-local/bridge.json` contains version 1, endpoint, 16-byte app
incarnation and 32-byte random secret, encoded as lowercase hex. The listener
binds before publication. Publication uses a private temporary file and atomic
rename; cleanup compares the app incarnation before removing anything. Never
distribute or capture this descriptor. Installation code remains hash verified.
The native locator resolves its own loaded mod mount to its physical root;
private directory/file redirection remains refused.
Possession of this capability is explicitly **not executable attestation**.

Authentication uses binary messages with an 8-byte header: `NLW1`, version byte
1, kind byte, two zero bytes. Kind 1 contains app[16], native incarnation[16]
and fresh client nonce[32]. Kind 2 contains fresh server nonce[32] and server
proof[32]. Kind 3 contains client proof[32]. Kind 4 contains the assigned
nonzero connection generation (little-endian u64).

Proofs use HMAC-SHA256 over the role label (`nt2/local/v1 server\0` or
`nt2/local/v1 client\0`), app, native incarnation, client nonce, server nonce,
in that order. Proof verification uses the HMAC crate's constant-time verifier.
The native client verifies the server before sending its own proof. The secret
is never sent. Handshakes have a two-second deadline and four socket slots.

Application messages have a 32-byte header: `NLM1`, kind (request 1/reply 2),
three zero bytes, generation u64, request ID u64, payload length u32, four zero
bytes. Integers are little endian. Payloads are the existing binary protocols,
32 through 131072 bytes. Frame and reassembled-message limits are 131104 bytes;
fragmentation is supported. Eight jobs are permitted across all connections,
including work left over from a disconnected generation. Replies must match
both ID and generation and meet the original monotonic request deadline.

Authenticated ownership combines app/native incarnations with the existing
world context. A new socket generation retires old socket work without granting
world authority. The adapter rechecks the current world immediately before
delivery; original uncertain mutations remain in the existing journal/status
flow. Reconnect uses bounded 250 ms to two-second backoff with jitter.

The transport crate runs independently of Win32 IPC. Its private-file adapter
uses Windows ACLs or Linux ownership and 0700/0600 permissions. Windows loopback
tests exercise real framing, authentication and files. Linux/Proton execution,
native Linux UI, keyring integration and packaging require separate evidence.

Listener close categories are numeric and contain no peer text: 1 transport/EOF,
2 close frame, 3 malformed envelope, 4 stale/replayed generation or ID, 5 work
bound, 6 shutdown, 7 private fixture outage. Native deadline misses and retry
delay are measured by the native client; listener latency is dispatch/delivery
latency, not a claim about game-thread or internet latency.
