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

Inventory date: 2026-07-27

Audit commands:

```text
cargo metadata --locked --format-version 1
cargo tree --locked --edges normal,build,dev
cargo tree --locked --target all --edges normal,build,dev
```

`Cargo.lock` and `cargo metadata` resolve 30 third-party packages plus the
`replay-host-doctor` workspace package. Every third-party row below has the
exact Cargo source
`registry+https://github.com/rust-lang/crates.io-index`; “crates.io” in the
Source column is that exact registry, not an alternate registry or mirror.
Checksums are the immutable registry checksums recorded in `Cargo.lock`.
Selected features come from `resolve.nodes[].features` in locked Cargo
metadata, including features activated transitively.

| Package | Version | Source | Checksum | Published license | Provenance repository | Selected features | Build script review | Direct edge(s) requiring it |
|---|---:|---|---|---|---|---|---|---|
| `bitflags` | `2.13.1` | crates.io | `b588b76d00fde79687d7646a9b5bdf3cc0f655e0bbd080335a95d7e96f3587da` | `MIT OR Apache-2.0` | `https://github.com/bitflags/bitflags` | `std` | No build script. | `rustix` (normal) |
| `block-buffer` | `0.12.1` | crates.io | `d2f6c7dbe95a6ed67ad9f18e57daf93a2f034c524b99fd2b76d18fdfeb6660aa` | `MIT OR Apache-2.0` | `https://github.com/RustCrypto/utils` | none | No build script. | `digest` (normal) |
| `cfg-if` | `1.0.4` | crates.io | `9330f8b2ff13f34540b44e946ef35111825727b38d33286ef986142615121801` | `MIT OR Apache-2.0` | `https://github.com/rust-lang/cfg-if` | none | No build script. | `libloading`, `sha2` (normal) |
| `cpufeatures` | `0.3.0` | crates.io | `8b2a41393f66f16b0823bb79094d54ac5fbd34ab292ddafb9a0456ac9f87d201` | `MIT OR Apache-2.0` | `https://github.com/RustCrypto/utils` | none | No build script. | `sha2` (normal) |
| `crypto-common` | `0.2.2` | crates.io | `ce6e4c961d6cd6c9a86db418387425e8bdeaf05b3c8bc1411e6dca4c252f1453` | `MIT OR Apache-2.0` | `https://github.com/RustCrypto/traits` | none | No build script. | `digest` (normal) |
| `digest` | `0.11.3` | crates.io | `f1dd6dbb5841937940781866fa1281a1ff7bd3bf827091440879f9994983d5c2` | `MIT OR Apache-2.0` | `https://github.com/RustCrypto/traits` | `block-api`, `default` | No build script. | `sha2` (normal) |
| `errno` | `0.3.14` | crates.io | `39cab71617ae0d63f51a36d69f866391735b51691dbda63cf6f96d042b63efeb` | `MIT OR Apache-2.0` | `https://github.com/lambda-fairy/rust-errno` | `std` | No build script. | `rustix` (normal; non-Linux/target-dependent path) |
| `gethostname` | `1.1.0` | crates.io | `1bd49230192a3797a9a4d6abe9b3eed6f7fa4c8a8a4947977c6f80025f92cbd8` | `Apache-2.0` | `https://codeberg.org/swsnr/gethostname.rs.git` | none | No build script. | `x11rb` (normal) |
| `hybrid-array` | `0.4.13` | crates.io | `818356c5132c1fede50f837ca96afbe78ff42413047f4abb886217845e1b6c8c` | `MIT OR Apache-2.0` | `https://github.com/RustCrypto/hybrid-array` | none | No build script. | `block-buffer`, `crypto-common` (normal) |
| `itoa` | `1.0.18` | crates.io | `8f42a60cbdf9a97f5d2305f08a87dc4e09308d1276d28c869c684d7777685682` | `MIT OR Apache-2.0` | `https://github.com/dtolnay/itoa` | none | No build script. | `serde_json` (normal) |
| `libc` | `0.2.189` | crates.io | `3eaf3ede3fee6db1a4c2ee091bf8a8b4dccdc6d17f656fb07896ee72867612f2` | `MIT OR Apache-2.0` | `https://github.com/rust-lang/libc` | `std` | Yes — reviewed target/rustc/ABI detection; invokes only toolchain or target-local version probes and performs no network or source-tree write. | `cpufeatures`, `errno`, `rustix` (normal; target-dependent) |
| `libloading` | `0.9.0` | crates.io | `754ca22de805bb5744484a5b151a9e1a8e837d5dc232c2d7d8c2e3492edc8b60` | `ISC` | `https://github.com/nagisa/rust_libloading/` | `std` | No build script. | `replay-host-doctor` (normal) |
| `linux-raw-sys` | `0.12.1` | crates.io | `32a66949e030da00e8c7d4434b251670a91556f4144941d37452769c25d58a53` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | `https://github.com/sunfishcode/linux-raw-sys` | `auxvec`, `elf`, `errno`, `general`, `if_ether`, `ioctl`, `net`, `netlink`, `no_std`, `prctl`, `system`, `xdp` | No build script. | `rustix` (normal) |
| `memchr` | `2.8.3` | crates.io | `cf8baf1c55e62ffcace7a9f06f4bd9cd3f0c4beb022d3b367256b91b87513d98` | `Unlicense OR MIT` | `https://github.com/BurntSushi/memchr` | `alloc`, `std` | No build script. | `serde_json` (normal) |
| `proc-macro2` | `1.0.107` | crates.io | `985e7ec9bb745e6ce6535b544d84d6cd6f7ad8bd711c398938ae983b91a766d9` | `MIT OR Apache-2.0` | `https://github.com/dtolnay/proc-macro2` | `proc-macro` | Yes — reviewed rustc capability probes and temporary `OUT_DIR` cleanup; no network or source-tree write. | `quote`, `serde_derive`, `syn` (normal) |
| `quote` | `1.0.47` | crates.io | `1fbf4db142a473a8d80c26bbf18454ed458bf8d26c8219c331daecfdbd079001` | `MIT OR Apache-2.0` | `https://github.com/dtolnay/quote` | `proc-macro` | Yes — reviewed rustc-version cfg selection only; no network or file write. | `serde_derive`, `syn` (normal) |
| `rustix` | `1.1.4` | crates.io | `b6fe4565b9518b83ef4f91bb47ce29620ca828bd32cb7e408f0062e9930ba190` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | `https://github.com/bytecodealliance/rustix` | `alloc`, `event`, `fs`, `net`, `process`, `std`, `system` | Yes — reviewed target/backend selection and rustc metadata probes under `OUT_DIR`; no network or source-tree write. | `gethostname`, `replay-host-doctor`, `x11rb` (normal) |
| `serde` | `1.0.229` | crates.io | `4148590afebada386688f18773da617792bf2ef03ffc1e4cbd2b1d45b023e0ba` | `MIT OR Apache-2.0` | `https://github.com/serde-rs/serde` | `derive`, `serde_derive`, `std` | Yes — reviewed rustc-version cfg selection and generated private module written only to `OUT_DIR`; no network or source-tree write. | `replay-host-doctor`, `serde_json` (normal) |
| `serde_core` | `1.0.229` | crates.io | `67dca2c9c51e58a4791a4b1ed58308b39c64224d349a935ab5039aa360942a48` | `MIT OR Apache-2.0` | `https://github.com/serde-rs/serde` | `result`, `std` | Yes — reviewed rustc/target cfg selection and generated private module written only to `OUT_DIR`; no network or source-tree write. | `serde`, `serde_json` (normal) |
| `serde_derive` | `1.0.229` | crates.io | `e7a5d71263a5a7d47b41f6b3f06ba276f10cc18b0931f1799f710578e2309348` | `MIT OR Apache-2.0` | `https://github.com/serde-rs/serde` | `default` | No build script. | `serde`, `serde_core` (normal) |
| `serde_json` | `1.0.151` | crates.io | `c841b55ecdae098c80dcae9cf767f6f8a0c2cdb3416bbef72181df4d0fe73f14` | `MIT OR Apache-2.0` | `https://github.com/serde-rs/json` | `raw_value`, `std` | Yes — reviewed target arithmetic cfg selection only; no command, network, or file write. | `replay-host-doctor` (normal) |
| `sha2` | `0.11.0` | crates.io | `446ba717509524cb3f22f17ecc096f10f4822d76ab5c0b9822c5f9c284e825f4` | `MIT OR Apache-2.0` | `https://github.com/RustCrypto/hashes` | none | No build script. | `replay-host-doctor` (normal) |
| `syn` | `3.0.3` | crates.io | `53e9bae58849f64dfa4f5d5ae372c8341f7305f82a3868709269343628b659a3` | `MIT OR Apache-2.0` | `https://github.com/dtolnay/syn` | `clone-impls`, `derive`, `parsing`, `printing`, `proc-macro` | No build script. | `serde_derive` (normal) |
| `typenum` | `1.20.1` | crates.io | `b6f5e870be6c3b371b77fe0ee0bafb859fa4964b4404c27de1d380043c4dda20` | `MIT OR Apache-2.0` | `https://github.com/paholg/typenum` | `const-generics` | No build script. | `hybrid-array` (normal) |
| `unicode-ident` | `1.0.24` | crates.io | `e6e4313cd5fcd3dad5cafa179702e2b244f760991f45397d14d4ebf38247da75` | `(MIT OR Apache-2.0) AND Unicode-3.0` | `https://github.com/dtolnay/unicode-ident` | none | No build script. | `proc-macro2`, `syn` (normal) |
| `windows-link` | `0.2.1` | crates.io | `f0805222e57f7521d6a62e36fa9163bc891acd422f971defe97d64e70d0a4fe5` | `MIT OR Apache-2.0` | `https://github.com/microsoft/windows-rs` | none | No build script. | `gethostname`, `libloading`, `windows-sys` (normal; Windows target) |
| `windows-sys` | `0.61.2` | crates.io | `ae137229bcbd6cdf0f7b80a31df61766145077ddf49416a728b02cb3921ff3fc` | `MIT OR Apache-2.0` | `https://github.com/microsoft/windows-rs` | `default`, `Win32`, `Win32_Foundation`, `Win32_Networking`, `Win32_Networking_WinSock`, `Win32_System`, `Win32_System_Diagnostics`, `Win32_System_Diagnostics_Debug` | No build script. | `errno`, `rustix` (normal; Windows target) |
| `x11rb` | `0.14.0` | crates.io | `5a8885a854a8bfdf87a301e53e41b17c5f8f33639903131338b997b1eb614f44` | `MIT OR Apache-2.0` | `https://github.com/psychon/x11rb` | `randr`, `render` | No build script. | `replay-host-doctor` (normal) |
| `x11rb-protocol` | `0.14.0` | crates.io | `acf4d1bc32aa46eec18caa634ec3cf4c05bfa151f12b93b510b15190f69a1ca8` | `MIT OR Apache-2.0` | `https://github.com/psychon/x11rb` | `randr`, `render`, `std` | No build script. | `x11rb` (normal) |
| `zmij` | `1.0.23` | crates.io | `29666d0abbfad1e3dc4dcf6144730dd3a3ab225bbbdac83319345b1b44ccfc1b` | `MIT` | `https://github.com/dtolnay/zmij` | none | Yes — reviewed rustc-version and optimization cfg selection only; no network or file write. | `serde_json` (normal) |

The workspace package itself is `replay-host-doctor 0.1.0`, has no external
source or checksum, is `publish = false`, and declares
`AGPL-3.0-only`. It has no build script.

## Review Conclusions

- **Graph agreement:** Locked metadata and `cargo tree --target all` contain
  exactly the 30 registry packages above. The host-target tree is the expected
  subset because Cargo omits inactive Windows/alternate-backend edges for the
  current Linux target.
- **Source and checksum:** All external packages use the canonical crates.io
  index. There are no path, git, or alternate-registry packages. Cargo
  verified each downloaded archive against the corresponding locked checksum.
- **Licenses and provenance:** Every package has a published SPDX expression
  and repository in locked metadata. All expressions are covered by the
  project policy in `deny.toml`; no clarification or exception is required.
- **Features:** The six direct packages retain the reviewed feature surface.
  Additional feature names shown above are only their declared transitive
  feature implications and are pinned exactly in `deny.toml`.
- **Build scripts:** The eight scripts marked “Yes” were reviewed from the
  checksum-verified registry source. They perform compiler/target detection or
  `OUT_DIR` generation only. No script downloads data, invokes a package
  manager, or modifies the source tree.
- **Duplicates:** No package name resolves to more than one version; there is
  no duplicate major or exception to explain.
- **Conditional denial check:** `cargo-deny 0.20.2` is not installed on this
  host, so `cargo deny check` was not executed and is not represented as a
  pass. The checked-in policy targets that exact version and is ready for the
  conditional verification command in the plan.
