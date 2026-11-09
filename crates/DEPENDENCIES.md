# Dependencies and maintenance

Last audited: **2026-11-09**. Crate **1.0.1**, MSRV **1.89**, edition 2021.
This file is the dependency inventory. Architecture is in `SUMMARY.md` and `DEVELOPER_GUIDE.md`;
file locations are in `FILE_INDEX.md`.

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
| rsa | `0.10` | 0.10.0 | 0.10.0 | current (as of 2026-11-09 19:31 UTC) |
| crc32fast | `1` | 1.5.4 | 1.5.4 | current |
| zstd | `0.14` | 0.14.3 | 0.14.3 | current |
| flate2 | `1` | 1.1.10 | 1.1.11 | `cargo update` |
| dashmap | `6` | 6.2.1 | 6.2.1 (7.0.0-rc2 only) | wait for 7.0.0 stable |
| parking_lot | `0.13` | 0.13.0 | 0.13.0 | current (0.12.5 also via tokio) |
| reqwest | `0.13` | 0.13.5 | 0.13.5 | current |
| tokio-tungstenite | `0.30` | 0.30.0 | 0.30.0 | current |
| windows | `0.62` | 0.62.2 | 0.62.2 | current; next major when 0.63+ |
