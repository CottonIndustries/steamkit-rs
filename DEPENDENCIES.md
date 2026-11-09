# Dependencies and maintenance

Last audited: **2026-11-09**. Crate **1.0.0**, MSRV **1.89**, edition 2021.
This file is the single maintenance reference: the dependency inventory, the next upgrade chain, and
the checklist for a SteamKit2/protobuf refresh. Architecture is in `SUMMARY.md` and
`DEVELOPER_GUIDE.md`; file locations are in `FILE_INDEX.md`.

## Direct dependencies (36)

| Crate | Manifest | Locked | Latest stable | Action |
|---|---|---|---|---|
| prost | `0.14` | 0.14.4 | 0.14.4 | current |
| prost-types | `0.14` | 0.14.4 | 0.14.4 | current |
| prost-build | `0.14` | 0.14.4 | 0.14.4 | current |
| protoc-bin-vendored | `3` | 3.3.0 | 3.3.0 | current |
| tokio | `1` | 1.54.1 | 1.54.1 | current |
| tokio-util | `0.7` | 0.7.20 | 0.7.20 | current |
| async-trait | `0.1` | 0.1.92 | 0.1.92 | current |
| futures | `0.3` | 0.3.34 | 0.3.34 | current |
| thiserror | `2` | 2.0.21 | 2.0.21 | current |
| anyhow | `1` | 1.0.104 | 1.0.104 | current |
| tracing | `0.1` | 0.1.44 | 0.1.44 | current |
| tracing-subscriber | `0.3` | 0.3.23 | 0.3.23 | current |
| bytes | `1` | 1.12.1 | 1.12.1 | current |
| hex | `0.4` | 0.4.3 | 0.4.3 | current |
| base64 | `0.23` | 0.23.1 | 0.23.1 | current |
| rand | `0.10` | 0.10.3 | 0.10.3 | current |
| serde | `1` | 1.0.229 | 1.0.229 | current |
| serde_json | `1` | 1.0.154 | 1.0.154 | current |
| percent-encoding | `2` | 2.3.2 | 2.3.2 | current |
| chrono | `0.4` | 0.4.45 | 0.4.45 | current |
| aes | `0.9` | 0.9.4 | 0.9.4 | current |
| cbc | `0.2` | 0.2.1 | 0.2.1 | current |
| ecb | `0.2` | 0.2.1 | 0.2.1 | current |
| hmac | `0.13` | 0.13.0 | 0.13.0 | current |
| sha1 | `0.11` | 0.11.0 | 0.11.0 | current |
| sha2 | `0.11` | 0.11.1 | 0.11.1 | current |
| md-5 | `0.11` | 0.11.0 | 0.11.0 | current |
| rsa | `0.10` | 0.10.0 | 0.10.0 | current |
| crc32fast | `1` | 1.5.4 | 1.5.4 | current |
| zstd | `0.14` | 0.14.3 | 0.14.3 | current |
| flate2 | `1` | 1.1.10 | 1.1.11 | `cargo update` |
| dashmap | `6` | 6.2.1 | 6.2.1 (7.0.0-rc2 only) | wait for 7.0.0 stable |
| parking_lot | `0.13` | 0.13.0 | 0.13.0 | current (0.12.5 also via tokio) |
| reqwest | `0.13` | 0.13.5 | 0.13.5 | current |
| tokio-tungstenite | `0.30` | 0.30.0 | 0.30.0 | current |
| windows | `0.62` | 0.62.2 | 0.62.2 | current; next major when 0.63+ |

## Transitive updates available now (compatible)

`cargo update` would move exactly these; no manifest change is needed:

- `cc` 1.11.0 → 1.11.1
- `fixedbitset` 0.5.7 → 0.5.9
- `flate2` 1.1.10 → 1.1.11
- `inout` 0.2.5 → 0.2.6
- `itoa` 1.0.18 → 1.0.19
- `mime` 0.3.17 → 0.3.18
- `smallvec` 1.16.4 → 1.16.5
- `syn` 3.0.9 → 3.0.10

Nothing is held back in the crypto stack: `rsa` 0.10.0, `hmac` 0.13.0 and `sha1` 0.11.0 landed on
2026-11-09, and the tree has a single `digest` 0.11.3. `dashmap` 7 is RC-only; `parking_lot` 0.12.5
remains only because tokio 1.54.1 depends on it; `flate2` 1.1.11 is a pending patch.

## Completed: RSA 0.10 upgrade (2026-11-09)

`rsa` 0.10.0 went stable on 2026-11-08 and replaced 0.9.10:

- `hmac` 0.12.1 → 0.13.0, `sha1` 0.10.7 → 0.11.0; one `digest` 0.11.3.
- Removed: `num-bigint-dig`, `rand` 0.8.8, `generic-array`, the duplicate `digest` 0.10 / `sha1` 0.10.
- `envelope_encrypted_connection.rs`: `PublicKey::oaep_encrypt::<Sha1>` replaces `Oaep` + `OsRng`.
- `steam_authentication.rs`: `BoxedUint::from_be_slice_vartime` + `PublicKey::new` +
  `pkcs1v15_encrypt` replace `BigUint` + `Pkcs1v15Encrypt` + `OsRng`; the `pkcs1v15` feature is on.
- `net_filter_encryption_with_hmac.rs`: `<HmacSha1 as KeyInit>::new_from_slice` (digest 0.11 moved
  key setup off `Mac`).
- Gates: `cargo check --workspace --all-targets` clean; `cargo test --workspace --no-fail-fast`
  **476 passed, 0 failed, 4 ignored**. No live QR logon after the upgrade.
- `rsa` 0.10.0 is one day old; prefer 0.10.1 before publishing a release.

## Next upgrades (1.1.0)

- `dashmap` 7.0.0 when stable (currently `7.0.0-rc2`); it also unifies `parking_lot` with tokio.
- `windows` 0.63+ when released; re-check the features in `crates/steamkit-nethook/Cargo.toml`.
- `flate2` 1.1.11 and the eight patch updates above.
- Edition 2024 / MSRV review: workspace is 1.89 / edition 2021; `rsa` 0.10 needs 1.87,
  `hmac`/`sha1` 1.85.
- `rsa` 0.10.1 once available.

## Non-crate pins — SteamKit2 release checklist

1. **SteamKit2** — pinned at commit `94812dc3` (4.0.0-Beta.1). On each upstream release: bump the
   pin, re-audit the changed handlers, update `CURRENT_PROTOCOL` (currently `65581`) if it changed,
   run the full suite (476 tests), refresh `SUMMARY.md`, `DEVELOPER_GUIDE.md` and `FILE_INDEX.md`.
2. **Protobufs** — pinned at `bdb3957d`. Regenerate `steamkit-protos` with `steamkit-proto-gen`;
   re-check the `csgo`/`dota2`/`tf2`/`deadlock`/`webui` feature gates and the bundled protoc (3.3.0).
3. **SteamLanguage** — `crates/steamkit/src/base/generated/steam_language.rs` (18,877 lines);
   regenerate with `steamkit-lang-gen` when `SteamLanguage.cs`/`.steamd` changes.
4. **Rust** — `rust-version = "1.89"`, edition 2021; installed toolchain 1.97.1. Consider edition 2024
   and an MSRV bump in the next minor.
5. **Version bump locations** — workspace `Cargo.toml`, the `steamkit-protos` pin in
   `crates/steamkit/Cargo.toml`, `Cargo.lock`, root `README.md`, `crates/steamkit/README.md` and
   `crates/steamkit-protos/README.md`; then tag and release.
6. **Docs** — `SUMMARY.md` is the index, `DEVELOPER_GUIDE.md` the per-file guide, `FILE_INDEX.md` the
   module map. The line-count refreshers (`guide_counts.py`, `file_index.py`) live in
   `%TEMP%\opencode`; keep copies in the source tree if reproducible releases matter.
7. **Security** — neither `cargo-audit` nor `cargo-deny` is installed. Add `cargo audit` to the
   release checklist (and ideally CI).
8. **Live verification after any crypto/TLS/protocol bump** — QR logon, TCP + WebSocket,
   5-minute soak, proxy tunnel, CDN sample.
