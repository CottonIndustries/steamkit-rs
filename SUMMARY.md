# steamkit-rs — Summary

A faithful Rust port of [SteamKit2](https://github.com/SteamRE/SteamKit), the .NET library for the Steam network. This file is the entry point: status, layout, architecture, key flows, conventions, and where the detailed documents live.

## 1. What this is / status

SteamKit2 is pinned at commit `94812dc3` (**4.0.0-Beta.1**, still upstream `master`); the protobufs are pinned at `bdb3957d`. Every public type, all 14 samples, all 5 tools and all 33 C# test files are ported, and the behaviour matches the pinned commit.

| Item | Value |
|---|---|
| C# spec | SteamKit2 4.0.0-Beta.1, commit `94812dc3` |
| Protobufs | SteamDatabase/Protobufs `bdb3957d` — 155 staged `.proto` |
| Crate | `steamkit-rs` **1.0.0** (library name `steamkit`) |
| MSRV / edition | 1.89 / 2021 |
| Release | **v1.0.0** tagged and released 2026-11-08 (`CottonIndustries/steamkit-rs`) |
| crates.io | 0.1.4 — 0.1.5 and 1.0.0 are not published |
| Tests | `cargo test --workspace --no-fail-fast` → **476 passed, 0 failed, 4 ignored** (75 binaries) |
| Crypto stack | `rsa` 0.10.0, `hmac` 0.13.0, `sha1` 0.11.0 since 2026-11-09, one `digest` 0.11.3 |
| `CURRENT_PROTOCOL` | 65581 |

- Coverage: 0 missing public types, 0 missing public members, 14/14 samples, 5/5 tools, 33/33 C# test files.
- The Rust surface is a superset: 43 extra public symbols (mostly C# `internal`/`protected` made public) and 10 signature differences, 7 of them deliberate.
- Every behavioural difference found by the final parity pass is closed; what remains is language- or platform-shaped and is listed in §5.
- Four tests are `#[ignore]`d; each states its reason in the source.
- The crypto stack moved to `rsa` 0.10.0 with `hmac` 0.13.0 and `sha1` 0.11.0 on 2026-11-09, leaving a single `digest` 0.11.3, no `num-bigint-dig` and no second `sha1`/`digest` subtree. SHA-1 output and the wire format are unchanged.

Scope: the CM protocol (TCP, WebSocket and UDP transports, channel encryption), the 15 handlers (`SteamUser`, `SteamFriends`, `SteamApps`, unified messages, game coordinator, ...), the authentication flows (credentials and QR code), the Web API, the CDN client and depot manifests, and SteamKit's tooling — the `.steamd` code generator, the protobuf dumper, and NetHook2 with its dump analyzer.

## 2. Workspace and crate layout

| Path | Crate | What it is |
|---|---|---|
| `crates/steamkit` | `steamkit` | the library, published as `steamkit-rs` |
| `crates/steamkit-protos` | `steamkit-protos` | prost messages generated from `proto/` at compile time, published as `steamkit-rs-protos`; 7 sets, 6,657 structs and 852 enums |
| `crates/steamkit-tools` | `steamkit-tools` | `.steamd` generator (`steamkit-lang-gen`), protobuf generator and dumper, depot dump, NetHook dump analyzer |
| `crates/steamkit-samples` | `steamkit-samples` | ports of the 14 C# samples, plus the Rust-only live checks |
| `crates/steamkit-tests` | `steamkit-tests` | port of SteamKit2/Tests — 33 mapped files, about 41 test files with `mock/mod.rs` |
| `crates/steamkit-nethook` | `steamkit-nethook` | NetHook2, a Windows DLL injected into `Steam.exe`; `#![cfg(windows)]`, the only crate that uses `unsafe` |

Dependency graph: `steamkit` → `steamkit-protos`; `steamkit-tools` and `steamkit-samples` → `steamkit` + protos; `steamkit-tests` → `steamkit` + `steamkit-tools` + protos; `steamkit-nethook` → protos only, so the workspace builds on Windows, macOS and Linux.

- The tree is 308 `.rs` files, about 87k lines, with 155 protos staged into the protos crate.
- `cargo build --workspace` and `cargo test --workspace --no-fail-fast` are the gates; the suite needs no network access.
- Linux needs OpenSSL headers (`libssl-dev pkg-config`) because the TLS backend is `native-tls`.
- MSRV 1.89 is forced by `aes` 0.9; the workspace is edition 2021.

## 3. Architecture map

`crates/steamkit/src/lib.rs` re-exports the primary public API at the crate root; module paths mirror the C# namespaces, one Rust file per C# file.

| Area | Modules | Responsibility |
|---|---|---|
| Base types and framing | `src/base/`: `msg_base.rs` (479), `client_msg.rs` (560), `packet_base.rs` (217), `gc/`, `generated/` | `MsgBase` reader/writer over `Vec<u8>`, protobuf and non-protobuf client messages, packet views and GC headers; `steam_language.rs` (about 18.8k lines) holds the 91 Valve enums — 3,054 variants plus 39 aliases for the C# 3,093 members; `steam_language_internal.rs` (1,168) holds the hand-written wire structs |
| Value types and utilities | `src/types/` (9 files), `src/util/` | `SteamID` (820), `JobID`/`AsyncJob` (515), `KeyValue` (1,137), depot and legacy manifests (644/251), `GlobalID`, `GameID`, `MessageObject`, `PubFile`; crypto, LZMA, zip/vzip/vzstd, machine info, logging, timers, Adler32 |
| Networking and envelope encryption | `src/networking/steam3/` | `IConnection` (200) plus TCP (323), UDP (727) and WebSocket (229 + 564) transports, the proxy `CONNECT` tunnel (479), `EnvelopeEncryptedConnection` (489) and `NetFilterEncryptionWithHMAC` (271) |
| Client core | `src/steam/cm_client.rs` (1,116), `src/steam/steam_client/` | `CMClient` owns the connection, heartbeat and dispatch loop; `SteamClient` (725) adds the handler registry, `AsyncJobManager` (513), `CallbackManager` (325) and `SteamConfiguration` (172); `src/error.rs` (105) holds the single `SteamError` |
| Handlers | `src/steam/handlers/` | 15 `ClientMsgHandler`s: apps, auth_ticket, cloud, content, friends, game_coordinator, game_server, master_server, matchmaking, networking, screenshots, unified_messages, user, user_stats, workshop |
| Unified messages | `src/steam/handlers/steam_unified_messages/` | the RPC transport that modern Steam services are built on (478, plus 152 for the callbacks) |
| Authentication | `src/steam/authentication/` | `SteamAuthentication` (509), `AuthSession` (413), credential (90) and QR (124) sessions, authenticator traits |
| CDN | `src/steam/cdn/` | `DepotChunk` (174), `Server` (180), `Client` (464), Lancache (121) |
| Web API and discovery | `src/steam/webapi/`, `src/steam/discovery/` | `WebApi` (661), `SteamDirectory` (118), `ContentServerDirectoryService` (141); `SmartCMServerList` (899), `ServerRecord` (290), the four server-list providers and the basic-list envelope |

- The message path is transport → `EnvelopeEncryptedConnection` → `CMClient` (header parse, message construction) → `SteamClient` handler registry → handler → `CallbackManager` → subscriber.
- The error model is one crate-level `thiserror` enum, `steamkit::error::SteamError`; every fallible public API returns `Result<T, SteamError>`.
- Protobuf code is generated by `prost` from 155 staged `.proto` files; the stage script strips `additional_includes`, `boxed_type` and `synthetic_default`, none of which affect the wire format.
- There is no in-session CM switch: a dead connection ends the session, and the caller reconnects through the server list.

## 4. Key flows

### 4.1 Connect and the channel-encryption handshake

1. `SteamClient::connect` is synchronous and fire-and-forget; a failure arrives later as `DisconnectedCallback`.
2. `CMClient` takes a candidate from `SmartCMServerList`, resolves its host, and builds a transport through the `ConnectionFactory` — TCP first, then WebSocket, then UDP.
3. `EnvelopeEncryptedConnection` starts in `Disconnected`. The transport raises `Connected`, and the server immediately sends `ChannelEncryptRequest`.
4. Each state accepts only its own message: `Connected` only the request, `Challenged` only the result, `Encrypted` anything. A challenge shorter than 16 bytes disconnects.
5. On the request the port generates a random 32-byte session key, builds `session_key || challenge`, encrypts it with **RSA-OAEP-SHA1** under the universe public key from `KeyDictionary`, computes a CRC32, and answers with `ChannelEncryptResponse` (protocol 1, key size 128, the 128-byte blob, the CRC).
6. `ChannelEncryptResult` with `EResult::OK` moves the state to `Encrypted` and raises `Connected` — the client is reported connected only once the channel is encrypted. Any other result disconnects.
7. Every later frame uses the channel cipher: **AES-256-CBC with PKCS#7** for the payload and an **HMAC-SHA1** folded into the IV. The IV's last 3 bytes are random; the first 13 are the truncated HMAC over those 3 bytes plus the plaintext; the 16-byte IV is stored AES-256-ECB-encrypted, followed by the CBC ciphertext. The HMAC secret is the session key's first 16 bytes. A bad HMAC disconnects.

### 4.2 Callback and async-job dispatch

1. A request allocates a `JobID` from an `AtomicU64`; `AsyncJobManager` inserts a `JobEntry` into a `DashMap<JobID, Arc<JobEntry>>` with the expected `TypeId`, the timeout (default 10 s) and a oneshot completer (`Single` or `Multiple`).
2. An inbound packet is parsed by `CMClient` (protobuf, extended or plain header) and dispatched to the handler registry; the job manager then completes the matching job, cloning the callback into the waiting oneshot. Unknown ids are ignored; a `TypeId` mismatch fails with `SteamError::Malformed` and disconnects.
3. Failure paths map to `JobFailed`, `Cancelled` and `Timeout`; a 1 s sweep enforces timeouts, and a heartbeat extends a live job by 10 s.
4. `AsyncJobMultiple` accumulates results and completes when its finish predicate holds; single jobs complete on the first match.
5. Subscriptions live in `CallbackManager` (`RwLock<Vec<(SubscriptionId, CallbackSubscription)>>`). Dispatch walks a snapshot in registration order and matches via `CallbackMsg::is_a` / `as_a`, so a base-type subscription receives a derived callback.
6. `subscribe` returns a `SubscriptionId`; dropping it does not unsubscribe, matching the explicit C# `Dispose`. `subscribe_guarded` returns a `SubscriptionGuard` that unsubscribes on `Drop`.
7. `get_callback` / `wait_for_callback` take one callback at a time from the queue; `run_callbacks` dispatches the queued ones.

### 4.3 Unified-message services

1. `create_service(name)` registers one `Arc<UnifiedService>` atomically; the response and notification decoders are keyed `(service, method)` on the handler and survive remove/recreate. A second registration with a different body type is rejected — the first wins.
2. `subscribe_service_response` / `subscribe_service_notification` also create the service and register the decoder, so a subscription works without a prior `create_service`.
3. `send_message` allocates a job, writes `target_job_name = Service.Method#1`, sends `EMsg::ServiceMethodCallFromClient` (`.NonAuthed` when the client has no SteamID yet) and returns the `AsyncJob` without awaiting it.
4. `EMsg::ServiceMethodResponse` is decoded with the registered decoder and completes the job; an unregistered service drops the message and the job times out.
5. `send_notification` carries no source job id, and `ServiceMethodNotification` carries `JobID::INVALID`, so it can never complete a job.

### 4.4 Credential and QR authentication

1. `SteamAuthentication` talks to the `Authentication` service (`Authentication.Method#1`) over the same wire, with a hand-rolled `call_service::<Req, Resp>` and a pending-request map.
2. `begin_auth_session_via_credentials(username, password)` fetches the key with `get_password_rsa_public_key` and sends the password **RSA PKCS#1 v1.5**-encrypted. `begin_auth_session_via_qr` needs a connected client and returns the challenge URL; the sample also writes the code to an SVG file in the temp directory.
3. `AuthSession::poll` runs the poll loop at the server's interval under a `CancellationToken`. Allowed confirmations are ordered: none, device confirmation, device code, email code, email confirmation, machine token.
4. `send_steam_guard_code` submits a guard code (a duplicate request is tolerated); `generate_access_token_for_app` exchanges the refresh token for an app token.
5. Both session kinds share one `AuthSession`; only the start call differs. Credentials also take the persistence flag that decides whether Steam remembers the session.

### 4.5 CDN manifest and chunk fetch

1. `cdn::Client::from_steam_client` calls `HttpClientFactory(CDN)` eagerly. Request timeout 10 s, body timeout 60 s, `MANIFEST_VERSION = 5`.
2. The `Server` value carries the protocol, virtual host and port. A proxy server marked `use_as_proxy` with a request-path template rewrites host and path; the CDN auth token is appended as the raw query string.
3. Manifest: `depot/{depot_id}/manifest/{manifest_id}/5[/{request_code}]`, a single-entry ZIP (stored or deflate) inflated by a hand-written reader, parsed by `DepotManifest::deserialize`, then filename decryption when a depot key is supplied (AES-256 with an ECB-derived IV, CBC for the names).
4. Chunk: `depot/{depot_id}/chunk/{hex(chunk_id)}`. Content-Length is checked against `CompressedLength`; with a depot key, `DepotChunk::process` decrypts the first 16 bytes with AES-256-ECB to recover the IV, AES-256-CBC-decrypts the rest, dispatches on the magic (Valve zstd, Valve LZMA, or a plain PKzip archive) and verifies the Adler32 against the manifest's `ChunkData::checksum`.
5. Without a depot key the bytes are copied verbatim into the destination, and the compressed length must fit.

### 4.6 Server-list resolution

1. `SmartCMServerList::resolve_server_list(force)` reads the provider's timestamp; a list older than 7 days is force-refreshed, and a bad server is remembered for 5 minutes.
2. A forced refresh fetches `ISteamDirectory/GetCMListForConnect` v1 in `vdf` format through `HttpClientFactory(WebAPI)` — the factory is invoked lazily, on the first fetch, never while a `SteamConfiguration` is built.
3. Fallback order: Steam Directory, then the provider's list, then the built-in fallback servers. A non-empty directory result is written back to the provider; a provider result is not.
4. Resolve is single-flight (an in-flight flag plus a `watch` channel), mirroring the C# `listTask`; blocking accessors never use the caller's runtime and panic with a clear message when a fetch is already in flight on a current-thread runtime.
5. `best_candidate` picks the smallest `last_bad_connection_time_utc` (never-failed first), ties keeping insertion order, and narrows the record to a single protocol flag; the DNS host is kept for the TLS server name. `mark_server_as_bad` poisons every endpoint of that host.

## 5. Conventions and deliberate deviations

- **No `unsafe`.** The library is `#![forbid(unsafe_code)]`; only `steamkit-nethook`, the injected Windows DLL, uses it. Platform-specific code is either `#[cfg(target_os = ...)]`-gated or shells out to the OS instead of calling FFI.
- **Naming.** One Rust file per C# file, snake_case basename; nested C# namespaces become nested modules; types keep their C# names; properties become `name()` / `set_name()` pairs, with no `get_`/`set_` prefixes.
- **Lock discipline.** `parking_lot` for synchronously-scoped state, `tokio::sync::Mutex` where a guard must be held across an `.await`; a guard is never held across an await.

The async model is the largest deliberate redesign:

| C# | Rust |
|---|---|
| `Task` / `Task<T>` | `async fn -> Result<T, SteamError>` |
| `TaskCompletionSource<T>` | `tokio::sync::oneshot` |
| `event EventHandler<T>` | `tokio::sync::broadcast` |
| `BufferBlock<CallbackMsg>` | unbounded `tokio::sync::mpsc` |
| `ConcurrentDictionary` | `dashmap::DashMap` |
| `ImmutableList` + `ImmutableInterlocked` | clone-on-write `RwLock<Vec<_>>` |
| `lock` | `parking_lot::Mutex` / `tokio::sync::Mutex` |
| `IDisposable` | `Drop`, or `SubscriptionGuard` for callbacks |

- **Deviations are documented, not incidental.** Anything not called out in this file or in the per-file entries of `DEVELOPER_GUIDE.md` is intended to be behaviour-identical.
- **Hardening beyond the C#.** A 16 MiB TCP frame cap, hard errors for a non-32-byte AES key and a short channel challenge, and a failed envelope decrypt disconnects instead of silently dropping the frame.
- **Remaining known differences**, all language- or platform-shaped: WebSocket proxy discovery reads environment variables only; unknown protobuf fields are dropped (`prost` has no `IExtensible`); nested `WebApiValue::List` values render empty; `log_debug` has no variadic formatting; the process start time is read once through PowerShell; transports are tried TCP, WebSocket, UDP.

## 6. Where to look

| File | What it holds |
|---|---|
| `README.md` | status, usage, build, platform support, samples, crate table |
| `DEVELOPER_GUIDE.md` (repo: `crates/DEVELOPER_GUIDE.md`) | the ~10.5k-line per-file guide (969 KB): §3 architecture, §5 module reference, §10 known deviations, §12 bug history |
| `FILE_INDEX.md` (repo: `crates/FILE_INDEX.md`) | 274 entries: module path → guide line number + LOC |
| `DEPENDENCIES.md` | dependency inventory, upgrade chain, release checklist |
| `crates/steamkit/README.md` | the library crate's own README |
| `crates/steamkit-protos/README.md` | the protos crate's own README |
| `crates/steamkit-samples/src/bin/` | `000`-`013`, `020`-`022`, the `900`-`902` live checks, `999_test_bed` |
| `crates/steamkit-tests/tests/` | the ported suite plus `mock/mod.rs`; live tests are `#[ignore]`d |

Typical path: read §3 of the guide for the shape, find the module in `FILE_INDEX.md`, then jump to that guide line for the per-file contract, gotchas and deviations. The three live samples (`900_live_check`, `901_live_cdn`, `902_live_account`) are the end-to-end checks against real Steam.

## 7. Maintenance and release

Pins and the refresh procedure live in **`DEPENDENCIES.md`**; that file is the single maintenance reference. In brief:

- Pins: SteamKit2 `94812dc3`, protobufs `bdb3957d`, `CURRENT_PROTOCOL` 65581, bundled protoc 3.3.0.
- 36 direct dependencies; only `flate2` 1.1.11 (patch), `dashmap` 7 (release candidate), `windows` 0.63+ and `rsa` 0.10.1 are pending.
- A version bump touches the workspace `Cargo.toml`, the protos pin in `crates/steamkit/Cargo.toml`, `Cargo.lock`, the root `README.md` and the two crate READMEs, then tags and releases.
- Gates: `cargo check --workspace --all-targets`, then `cargo test --workspace --no-fail-fast` (476 tests), then the live checks — QR logon, TCP and WebSocket, a soak, the proxy tunnel and the CDN sample.
- After any SteamKit2 or protobuf re-pin: regenerate, re-audit the changed handlers, update `CURRENT_PROTOCOL` if it moved, and refresh `SUMMARY.md`, `DEVELOPER_GUIDE.md` and `FILE_INDEX.md`.
- Docs are local-only, committed in the repository; they are not published with the crates.

## 8. Porting rules and frozen signatures

The old `PORTING_CONTRACT.md` was consolidated here on 2026-11-09. The rules that must not drift:

- `JobID` is a newtype over `u64`; `JobID::INVALID` is `u64::MAX`.
- `SteamID` keeps the C# bit layout and constructors; do not replace it with a raw `u64`.
- `IClientMsg` / `IPacketMsg` are the only message abstractions crossing the networking boundary.
- `IConnection::connect(&self, end_point: SocketAddr, timeout: Duration)` is frozen; the TLS host
  name lives in `ServerRecord::get_host()`.
- `SteamResult<T> = Result<T, SteamError>` is the public alias; `SteamError` is the only error type.
- Handlers: `get_required_handler<T>() -> Result<Arc<T>, SteamError>`; no panics in library code.
- Naming: one Rust file per C# file, snake_case basenames, C# type names preserved, `name()` /
  `set_name()` accessors.
- No `unsafe` in the library; `steamkit-nethook` is the only exception.
