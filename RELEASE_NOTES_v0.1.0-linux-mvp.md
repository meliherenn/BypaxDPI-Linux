# BypaxDPI Linux v0.1.0 MVP

Initial Linux MVP release of BypaxDPI Linux.

## Status

This is an unofficial Linux MVP port and should be treated as a pre-release.

## Highlights

- Added Linux platform support
- Split Windows/Linux Rust platform logic
- Added Linux-safe proxy behavior
- Added Linux SpoofDPI sidecar support
- Fixed Linux connect flow
- Updated branding to BypaxDPI Linux
- Added credits for BypaxDPI-Windows and SpoofDPI
- Disabled/separated Windows-only behavior on Linux, including Npcap, WinHTTP, UWP CheckNetIsolation, Windows firewall, and registry logic

## Validation

Passed locally:

- `npm install`
- `npm run build`
- `cd src-tauri && cargo check`
- `cd src-tauri && cargo fmt`
- `npm run build-proxy`
- `file src-tauri/binaries/bypax-proxy-x86_64-unknown-linux-gnu`
- `./src-tauri/binaries/bypax-proxy-x86_64-unknown-linux-gnu --help`
- `npm run tauri build`

Local proxy sidecar validation confirmed the Linux sidecar is executable and supports:

- `--listen-addr`
- `--no-tui`
- `--dns-mode`
- `--dns-https-url`
- `--https-split-mode`
- `--https-chunk-size`

Generated bundles:

- Debian package: `BypaxDPI Linux_0.1.0_amd64.deb`
- RPM package: `BypaxDPI Linux-0.1.0-1.x86_64.rpm`

AppImage note: AppImage packaging was attempted but failed in this environment during the Tauri `linuxdeploy` step. The MVP artifacts for this draft are `.deb` and `.rpm`.

## Known limitations

- Initial Linux MVP
- Tested primarily on my Linux environment
- GNOME/gsettings proxy support may vary by desktop environment
- KDE and other desktop environment support may need further testing
- Manual proxy fallback may still be needed on some systems
- AppImage packaging needs follow-up because `linuxdeploy` failed in this environment

## Credits

- Original project: BypaxDPI-Windows
- Linux port maintainer: Melih Eren
- Proxy engine: SpoofDPI

## License

- BypaxDPI-Windows: MIT License
- SpoofDPI: Apache License 2.0
