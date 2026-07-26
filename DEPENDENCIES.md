# Dependency Review

ReplayDesktop resolves dependencies only after the direct-package legitimacy
audit below has passed. Every direct dependency is pinned exactly, comes from
the canonical crates.io registry, and is restricted to the reviewed feature
surface in `Cargo.toml`.

## Package Legitimacy Audit

Audit date: 2026-07-27

Evidence sources:

- crates.io immutable version metadata and sparse-index checksums
- the upstream repository published by each crates.io version
- each crate's published SPDX license expression and feature map

| Package | Canonical registry | Upstream repository | Published license | Registry checksum | Purpose | Selected features | Default-feature disposition | Review |
|---|---|---|---|---|---|---|---|---|
| `serde =1.0.229` | `https://crates.io/crates/serde/1.0.229` | `https://github.com/serde-rs/serde` | `MIT OR Apache-2.0` | `4148590afebada386688f18773da617792bf2ef03ffc1e4cbd2b1d45b023e0ba` | Derive the fixed evidence data model and implement narrow custom visitors. | `derive`, `std` | Disabled, then the reviewed `std` capability is enabled explicitly. | PASS — established serde-rs package, non-yanked exact release, expected repository and license. |
| `serde_json =1.0.151` | `https://crates.io/crates/serde_json/1.0.151` | `https://github.com/serde-rs/json` | `MIT OR Apache-2.0` | `c841b55ecdae098c80dcae9cf767f6f8a0c2cdb3416bbef72181df4d0fe73f14` | Parse strict JSON and retain extension payload text through `RawValue`. | `raw_value`, `std` | Disabled, then only `raw_value` and the required `std` support are enabled. | PASS — established serde-rs package, non-yanked exact release, expected repository and license. |
| `sha2 =0.11.0` | `https://crates.io/crates/sha2/0.11.0` | `https://github.com/RustCrypto/hashes` | `MIT OR Apache-2.0` | `446ba717509524cb3f22f17ecc096f10f4822d76ab5c0b9822c5f9c284e825f4` | Provide the sole SHA-256 implementation behind evidence identities. | none | Disabled; the default `alloc` and `oid` features are not required by the digest API. | PASS — RustCrypto package, non-yanked exact release, expected repository and license. |
| `libloading =0.9.0` | `https://crates.io/crates/libloading/0.9.0` | `https://github.com/nagisa/rust_libloading/` | `ISC` | `754ca22de805bb5744484a5b151a9e1a8e837d5dc232c2d7d8c2e3492edc8b60` | Load explicitly authenticated NVIDIA runtime libraries in later host probes. | `std` | Disabled, then the reviewed `std` API is enabled explicitly. | PASS — established package, non-yanked exact release, expected repository and license. |
| `rustix =1.1.4` | `https://crates.io/crates/rustix/1.1.4` | `https://github.com/bytecodealliance/rustix` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | `b6fe4565b9518b83ef4f91bb47ce29620ca828bd32cb7e408f0062e9930ba190` | Supply bounded filesystem, process, and Unix-socket primitives without shell mediation. | `std`, `fs`, `process`, `net` | Disabled; only the reviewed API groups and their required `std` support are enabled. | PASS — Bytecode Alliance package, non-yanked exact release, expected repository and license. |
| `x11rb =0.14.0` | `https://crates.io/crates/x11rb/0.14.0` | `https://github.com/psychon/x11rb` | `MIT OR Apache-2.0` | `5a8885a854a8bfdf87a301e53e41b17c5f8f33639903131338b997b1eb614f44` | Query the real X11 setup and RandR output/provider state in later host probes. | `randr` (which necessarily enables protocol `render`) | Disabled; no all-extensions, unsafe/libxcb, image, cursor, or unrelated protocol feature is admitted. | PASS — established x11rb package, non-yanked exact release, expected repository and license. |

All six direct packages passed review. No path, git, alternate-registry,
wildcard, caret-range, or unreviewed direct dependency is authorized.

## Resolved Dependency Inventory

The complete normal/build/dev inventory is appended after `Cargo.lock` is
resolved and reviewed in Task 3. Until then, the direct audit above is the
authorization boundary for lockfile creation.

