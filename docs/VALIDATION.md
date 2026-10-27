# Linux hardening validation — 16 October 2026

Working tree began clean at `66c2e98`. Changes are local and uncommitted.

## Scope and findings

- The running desktop is KDE. Previously, installed/writable GNOME schemas could incorrectly cause automatic proxy setup to be reported as successful on KDE.
- Clearing without a backup previously changed GNOME proxy mode to `none`, including settings the app did not own.
- Recovery used shared `/tmp` filenames and a bare PID; cleanup could target a reused or unrelated PID. It now uses a private directory, a single-instance file lock, process identity and Linux pidfds.
- Partial GSettings failures previously lost the backup and were reported as successful restores. Recovery now keeps the journal until reads and applicable restores succeed.
- StrictMode effect replay, delayed health checks and old sidecar exit callbacks could interfere with a newer connection. Session checks and serialized cleanup protect those transitions.
- Strong mode ignored the selected chunk size. Engine arguments are now centralized and regression-tested.
- A successful CONNECT reply did not prove TLS completed. The diagnostic report now performs certificate-validated HTTPS requests over both explicit routes.
- Native setup instructions previously described phone Wi-Fi settings on KDE; local application setup is now separate from LAN/PAC instructions.

## Automated checks

- `npm test`: 7 tests passed (Linux/Windows arguments, DNS parser values, chunk settings, invalid saved values, binary target validation, Linux TLS-record default and Windows isolation).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked`: 12 tests passed (settings transaction and process identity cases, diagnostic classification, real loopback CONNECT-without-TLS fixture).
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`: passed.
- `npm run build`: passed.
- `npm run tauri -- build`: release executable and both deb/rpm bundles built successfully. Debian package metadata includes curl, GTK, WebKitGTK and AppIndicator dependencies.
- `npm audit`: zero reported vulnerabilities after compatible dependency updates. This is an advisory-database snapshot, not a security guarantee.
- `npm run build-proxy` and `npm run copy-proxy`: passed. Final engine: Bypax-patched SpoofDPI `1.5.4-bypax-record1`, base `eb07db9ef0069c68a0d37f3f6368178b7a30d025`, SHA-256 `070c13e41ccccbb92592dffc97e881ed6155af90b1a08a3177f13f3196231395`. The source is built from the pinned revision plus the checked-in patch; unpatched Linux engines are rejected.
- Engine `go test ./internal/desync ./internal/config`: passed, including CLI parsing, payload preservation, malformed fallback, and certificate-validated local TLS 1.2 and TLS 1.3 handshakes.

## Browser fixture checks

Chromium at 380 × 700, real React frontend and mocked Tauri IPC, driven with Playwright CLI:

- Normal connect/disconnect and KDE manual setup.
- StrictMode recovery invoked exactly once.
- Forced disconnect while health validation is pending does not later apply the proxy.
- An old process exit callback cannot disconnect the next process.
- TLS failures and HTTP 200 control results are displayed separately.
- Settings remain reachable when the report grows.

These prove frontend transitions, not native sidecar execution. Local screenshots and fixture scripts are under ignored `output/playwright/`.

## Native and live checks

The actual Tauri/WebKitGTK application was launched in Xvfb with isolated XDG configuration/state/data and a separate D-Bus session. It ran a real sidecar and sent public HTTPS probes over the current network. This is native virtual-display evidence, not a physical GNOME desktop test.

- Application opened and the first-run flow launched the engine.
- Port 8080 was already owned by Steam; the app selected 8081 without disturbing it.
- The sidecar listened on loopback with LAN sharing disabled.
- A second application instance using the same state directory exited without cleaning or stopping the first instance.
- KDE was reported as requiring per-app proxy configuration.
- Release crash recovery: the isolated test app was killed with SIGKILL while its real sidecar was running. Relaunching the same release with the same private state stopped the recorded orphan and removed its journal.
- Native disconnect stopped the real sidecar; normal UI exit returned code 0 and removed the sidecar recovery record.
- Recovery directory/file permissions were verified as 0700/0600. The actual GNOME proxy mode remained `none` before and after this KDE session.
- Initial diagnostic report with unmodified SpoofDPI: Example HTTP 200 directly and via proxy; Discord TLS/reset failure directly and via proxy.
- Multiple DNS and SNI/chunk/disorder combinations were tested with installed SpoofDPI 1.5.3. Discord did not pass; one strategy timed out. Successful example.com access is not presented as Discord success.

## TLS-record resolution

- Unmodified SpoofDPI 1.5.3 still failed after eight additional first-byte, custom-SNI, disorder and TLS-1.2 combinations.
- An isolated ByeDPI 17.3 build (`ba532298de7b28cfe854aea83d061369d13ca290`) was used only for comparison. TLS-record splitting and OOB strategies reached Discord HTTP 200; plain split/disorder and tested fake TTL values failed. The successful TLS-record technique informed the independent SpoofDPI patch; ByeDPI is not bundled.
- The patched HTTP proxy reached `https://discord.com/app` and `/api/v10/gateway` with HTTP 200 and certificate validation enabled. `gateway.discord.gg` returned HTTP 404 and `cdn.discordapp.com` HTTP 403: these prove completed TLS, not successful authenticated sessions or a valid CDN asset.
- Both HTTP and HTTPS example.com control requests returned 200 through the patched proxy.
- The packaged native application showed Discord direct TLS failure and proxied HTTP 200 in its own diagnostic panel (`output/playwright/record-diagnostics.png`).
- Five consecutive Discord `/app` requests through the patched proxy returned HTTP 200. A separate certificate-validated WebSocket connection to `gateway.discord.gg/?v=10&encoding=json` received HTTP 101 and Discord Hello opcode 10. No token, Identify or login was sent.
- Native settings switched the running engine from record mode to TCP SNI mode and back, replacing the process each time. Normal UI exit returned 0 and removed the sidecar record.
- Linux defaults to the new TLS-record method. Selecting an old profile explicitly switches back to its original TCP method. DNS provider settings remain active.

## Remaining limits

- Authenticated Discord sessions and voice/video have not been tested. This HTTP proxy does not carry UDP voice/video traffic. Other networks may need different strategies.
- GNOME automatic proxy writes/restores are tested with deterministic fixtures; a real GNOME login session has not been used.
- Linux packet-filter/system-wide interception, KDE-wide proxy modification and root-required strategies are not implemented by this change.
- No firewall rule was removed, no existing service was disabled, and no sudo password was requested.
- Package installation on Debian/Fedora, AppImage, ARM and Windows execution remain unvalidated.
