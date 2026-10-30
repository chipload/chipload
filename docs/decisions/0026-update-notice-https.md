# 0026: The update notice, Chipload's first network access

Status: accepted (2026-10-30). Spec: docs/specs/update-notice.md (agreed 2026-10-30).

## Context
The user asked to be told when a newer release is on GitHub. Until now Chipload made no
network request at all, and CLAUDE.md keeps the library crates off the network.

## Decision
- **Only the window asks,** once a day at start and only after the user said yes; *Help >
  Check for new versions now* on request. One HTTPS `GET` of GitHub's latest published
  release; nothing about the user is sent; nothing is downloaded or installed.
- **`ureq` 3.5** (MIT OR Apache-2.0), without default features, over **`rustls` 0.23**
  (Apache-2.0 OR ISC OR MIT) with **`ring` 0.17** (Apache-2.0 AND ISC) as its crypto, in
  `chipload-gui` only. Blocking, on a thread of its own: no async runtime.
- **Certificates from the system's own store,** loaded by **`rustls-native-certs` 0.8**
  (Apache-2.0 OR ISC OR MIT). Not `webpki-roots` (ureq's default) nor
  `rustls-platform-verifier`, which brings `webpki-root-certs` for wasm32: both lists are
  CDLA-Permissive-2.0, which isn't on the allowed list, and `cargo deny` checks every
  target. The system's store also follows a company's own certificates.
- New crates in the lock file, all allowed: base64, core-foundation, getrandom 0.2, http,
  httparse, openssl-probe, ring, rustls, rustls-native-certs, rustls-pki-types,
  rustls-webpki (ISC), schannel (MIT), security-framework(-sys), subtle (BSD-3-Clause),
  untrusted (ISC), ureq, ureq-proto, utf8-zero, wasi, zeroize.

## Consequences
- `ring` compiles C and assembly: ARM64 Windows builds need Clang (ring's build uses it
  there).
- A proxy in `HTTPS_PROXY` is used (ureq reads it).
- The library crates stay off the network; the architecture test is unchanged.
