# steamkit-rs

A Rust port of [SteamKit2](https://github.com/SteamRE/SteamKit), the .NET library for talking to the Steam network.

## Status

**Complete.** This is a full port of SteamKit2 **4.0.0-Beta.1** (commit `94812dc3`), including the post-Beta.1 fixes through `131fb033` (`4.0.0-Beta.2` + 8): every public type, all 14 samples, all 5 tools and all 33 C# test files are ported, with no known behavioural gaps. The crate is **1.0.1**.

- 491 tests pass (`cargo test --workspace --no-fail-fast`); the live tests are `#[ignore]`d.
- Protocol pins: SteamKit2 `94812dc3` (base) + fixes through `131fb033`, Protobufs `bdb3957d`.

It covers the CM protocol (TCP, WebSocket and UDP transports, channel encryption), the handlers (`SteamUser`, `SteamFriends`, `SteamApps`, unified messages, game coordinator, ...), the authentication flows (credentials and QR code), the Web API, the CDN client and depot manifests. It also ports SteamKit's tooling: the `.steamd` code generator, the protobuf dumper, and the NetHook2 DLL plus its dump analyzer.

## Using it

```toml
[dependencies]
steamkit-rs = "1.0.1"
```

The package is `steamkit-rs` on crates.io (the `steamkit` name was taken), but the library is still called `steamkit`:

```rust
use steamkit::steam::steam_client::steam_client::SteamClient;
```

## Building

```sh
cargo build --workspace
cargo test --workspace --no-fail-fast
```

The test suite needs no network access; the few live tests are `#[ignore]`d.

## Documentation

- [`SUMMARY.md`](crates/SUMMARY.md) - the entry point: status, layout, architecture, key flows, deviations.
- [`DEVELOPER_GUIDE.md`](crates/DEVELOPER_GUIDE.md) - the per-file guide to every module.
- [`FILE_INDEX.md`](crates/FILE_INDEX.md) - module index with guide line numbers.
- [`DEPENDENCIES.md`](crates/DEPENDENCIES.md) - dependency inventory and release checklist.

## Platform support

Windows, macOS and Linux. The library is pure Rust and forbids `unsafe`.

- **Linux** needs OpenSSL headers, because the TLS backend is `native-tls`:
  `sudo apt install libssl-dev pkg-config` (or the equivalent for your distribution).
- **`steamkit-nethook`** is a Windows DLL injected into `Steam.exe`. Its source is `#![cfg(windows)]`
  and its `windows` dependency is Windows-only, so the workspace builds on the other platforms too.
- **MSRV** is 1.89, required by `aes` 0.9. The workspace uses edition 2021.

## Samples

`crates/steamkit-samples` has ports of the SteamKit2 samples. The QR login one needs no password:

```sh
cargo run -p steamkit-samples --bin 001_authentication_with_qr_code
```

Scan the code with the Steam mobile app. The code is also written to `%TEMP%\steamkit-qr-login.svg`, which is handy if your console font has no block-drawing glyphs.

Three more samples run PASS/FAIL checks against live Steam:

- `900_live_check` logs on with a QR code and checks a logged-on session. It doesn't change anything on the account.
- `901_live_cdn` logs on anonymously (no QR code, no account) and downloads a small free depot through the CDN, checking every file against the manifest.
- `902_live_account` logs on with a QR code and checks persona state (only `Invisible`, then back to `Offline`), the Dota 2 game coordinator and a CDN download with the account's depot key, then keeps the session up for `STEAMKIT_SOAK_MINUTES` minutes (default 30). It sends no chat message and doesn't change the account name.

## Crates

| Crate | What it is |
|---|---|
| `steamkit` | the library, published as `steamkit-rs` |
| `steamkit-protos` | generated protobuf messages, published as `steamkit-rs-protos` (built from `proto/` at compile time) |
| `steamkit-tools` | `.steamd` code generator, protobuf generator / dumper, NetHook dump analyzer |
| `steamkit-nethook` | NetHook2, a Windows DLL that logs the Steam client's network traffic |
| `steamkit-samples` | the sample programs |
| `steamkit-tests` | the test suite |

## License

LGPL-2.1, the same as SteamKit2. See [LICENSE](LICENSE).
