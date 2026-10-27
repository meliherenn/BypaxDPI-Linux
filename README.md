# BypaxDPI Linux

Unofficial Linux port of [BypaxDPI-Windows](https://github.com/BypaxDPI/BypaxDPI-Windows), maintained by Melih Eren and powered by [SpoofDPI](https://github.com/xvzc/spoofdpi).

A React/Tauri desktop application that manages a local **HTTP/HTTPS proxy**. It is not a VPN or a system-wide packet filter. Applications must actually use its proxy. HTTPS remains encrypted by the destination's TLS connection; BypaxDPI does not add a separate encryption layer.

## What works on Linux

- Start/stop a bundled SpoofDPI sidecar, with an available local port selected automatically.
- TLS record splitting inside SNI by default on Linux, plus selectable legacy TCP fragmentation profiles. DNS provider settings apply to either method. The chunk-size setting applies to TCP modes.
- Direct versus proxied HTTPS diagnostics for Discord and a control site. An open local port or `CONNECT 200` is **not** reported as proof that Discord is reachable.
- GNOME-family desktop proxy configuration with a durable backup and verified writes. KDE and other desktops receive explicit per-application setup instructions.
- User-owned recovery state, Linux single-instance locking, and orphan cleanup that checks PID, process start time, executable and boot ID before signaling through a pidfd.
- LAN/PAC sharing when enabled, logs, tray and autostart settings.

Npcap, registry/WinHTTP changes, UWP exemptions and Windows firewall management remain Windows-only. Linux operation does not require root and does not change nftables or disable OpenSnitch.

## Build and run

Requirements:

- Node.js/npm and Rust **1.89 or newer** (the locked dependencies may require a newer toolchain; this build used Rust 1.95).
- [Tauri's Linux prerequisites](https://v2.tauri.app/start/prerequisites/#linux).
- `curl` for end-to-end connection diagnostics (also declared in the deb/rpm dependencies).
- Git and Go **1.26.2 or newer** for the Linux engine build. The first build downloads the pinned official source and Go dependencies.

```sh
npm ci
npm run build-proxy
npm run tauri -- dev
```

`build-proxy` fetches official SpoofDPI **v1.5.4**, verifies commit `eb07db9ef0069c68a0d37f3f6368178b7a30d025`, and applies the reviewed [TLS-record patch](engine-patches/spoofdpi-tls-record.patch) in a temporary build directory. The original checkout stays untouched. Engine tests run before compilation. Architecture, CLI compatibility, engine SHA-256 and patch SHA-256 are recorded in `src-tauri/binaries/engine-manifest.json`. The resulting version is `1.5.4-bypax-record1`. It is a Bypax-patched build, not an unmodified upstream release. See [patch notes](engine-patches/README.md).

To use an existing **clean checkout at that exact commit**, provide its path:

```sh
SPOOFDPI_SOURCE_DIR=/absolute/path/to/spoofdpi npm run build-proxy
```

The host Rust target is detected instead of assuming x86_64. `TAURI_TARGET_TRIPLE` can select a supported target; cross builds require source. An installed host binary cannot be relabeled for a different target. ARM and Windows packages have not been validated on devices in this change.

```sh
npm run check
npm run tauri -- build
```

The release executable is `src-tauri/target/release/bypaxDPI`; keep the adjacent `bypax-proxy` with it when running from the build directory. Deb/rpm packages are under `src-tauri/target/release/bundle/`. AppImage packaging remains outside the validated targets.

`npm run dev` alone previews the frontend; native operations require Tauri.

## Discord and KDE

After connecting, use **Discord bağlantısını test et**. The report separately shows direct and proxied requests, including TLS failures, timeouts and HTTP errors. These are IPv4 HTTPS tests with certificate validation enabled, at most 12 seconds per request. They do not test login, WebSocket sessions, voice or video.

On KDE, opening a local proxy does not automatically route every application through it. Use **Proxy ayarı** and the displayed address in the application's HTTP and HTTPS proxy settings. The port is dynamic; use the displayed port, not a hardcoded 8080.

For example, if the app displays port 8081:

```sh
curl -q -4 --noproxy '' --proxy http://127.0.0.1:8081 \
  --connect-timeout 5 --max-time 12 -v -o /dev/null https://discord.com/app
```

`curl --noproxy '*'` explicitly bypasses BypaxDPI. Environment proxy variables are also not proof of whether desktop proxy settings are active.

For the usual Electron Discord desktop client, fully quit existing Discord processes and start it with `discord --proxy-server=http://127.0.0.1:8081` (substitute the actual port). Sandboxed/packaged clients may behave differently. **This HTTP proxy does not carry Discord voice/video UDP traffic.** Restore manually configured application proxy settings when stopping BypaxDPI.

Initial checks failed Discord TLS with ordinary SpoofDPI TCP fragmentation. On 16 October 2026 the new TLS-record method reached Discord `/app` and `/api/v10/gateway` with HTTP 200 through the proxy. Access depends on the network; use the application test rather than treating “Proxy hazır” as proof of site access. See [validation notes](docs/VALIDATION.md).

## Recovery and existing settings

Recovery data is stored in `$XDG_STATE_HOME/bypaxdpi`, or `~/.local/state/bypaxdpi`. The directory is user-owned with mode 0700; newly written recovery files use 0600 permissions. Proxy settings are saved atomically before modification. Failed restores keep their journal for retry; settings changed externally are not overwritten during restoration. Clearing without an owned backup leaves existing proxy settings alone.

Shared legacy `/tmp/bypaxdpi_*` files are deliberately not trusted for signaling processes or restoring settings. If an older version left a manual proxy configured, restore your known previous value using your desktop/application settings. Do not delete the new recovery journal while its proxy settings still need restoration.

The application does not flush firewall rules. The NFQUEUE rules in the initial report match [OpenSnitch's documented setup](https://github.com/evilsocket/opensnitch/discussions/1465); they alone do not establish that an old DPI service is still active.

## Validation

`npm run check` runs Node regression tests, the production frontend build, Rust formatting verification and Rust tests. Rust tests cover backup/rollback/retry, externally changed settings, process identity parsing, and an actual local CONNECT-only fixture that must fail TLS validation. They require `curl` but no public network.

Additional checks performed and their limits are recorded in [docs/VALIDATION.md](docs/VALIDATION.md). Browser mocks, native virtual-display tests and live network tests are distinguished there. Real GNOME-session behavior, distribution package installation, Windows/ARM devices and Discord voice remain unverified.

## Credits and license

- Linux port: Melih Eren
- Original application: BypaxDPI-Windows / ConsolAktif, MIT; see [LICENSE](LICENSE).
- Proxy engine: SpoofDPI, Apache-2.0.

This fork preserves upstream attribution and is not a from-scratch original project.
