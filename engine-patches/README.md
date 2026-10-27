# SpoofDPI TLS-record mode

Linux uses official SpoofDPI v1.5.4 at commit `eb07db9ef0069c68a0d37f3f6368178b7a30d025`, plus `spoofdpi-tls-record.patch`. The build identifies itself as `1.5.4-bypax-record1`. Upstream is Apache-2.0; its license is included here and in the Linux bundle. Changes made 16 October 2026 add the `record` mode and its regression tests.

`--https-split-mode record` splits one complete ClientHello TLS record immediately after the first SNI hostname byte. It writes two valid record headers with adjusted lengths; concatenating their payloads reproduces the original handshake exactly. TLS encryption, certificate verification and the destination are unchanged. This follows the handshake fragmentation allowed by [RFC 8446 section 5.1](https://www.rfc-editor.org/rfc/rfc8446#section-5.1). It is independently implemented using SpoofDPI's existing SNI parser. ByeDPI was used to test whether this technique worked on the network; its code and executable are not bundled.

Missing/malformed SNI or incomplete records retain the existing pass-through fallback. Already-fragmented ClientHello records without a complete SNI cannot be split further by this implementation. The mode does not use the TCP chunk-size/disorder settings. Existing modes retain their upstream behavior. This is not a general DPI bypass guarantee and does not add UDP support.

`npm run build-proxy` downloads the pinned checkout into ignored `spoofdpi/upstream` if needed. Alternatively `SPOOFDPI_SOURCE_DIR` may point to a clean checkout at the same revision. A revision mismatch or local source changes cause an error without modifying that checkout. The build copies source to a temporary directory, verifies/applies the patch, runs `go test ./internal/desync ./internal/config`, compiles with Go and validates the resulting executable. Unpatched Linux binaries are rejected.

Tests cover CLI parsing, reconstruction of the original handshake, malformed input fallback, and real local TLS 1.2/1.3 handshakes with trusted test-server certificates. Live validation is documented separately in `docs/VALIDATION.md`.
