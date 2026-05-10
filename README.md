# BypaxDPI Linux

Unofficial Linux port of BypaxDPI-Windows, maintained by Melih Eren.

BypaxDPI Linux is a work-in-progress Linux compatibility fork of the original BypaxDPI-Windows desktop app. It keeps the React/Tauri app experience, separates platform-specific Rust logic, and uses SpoofDPI as the proxy engine for the Linux MVP.

## About

This project is not a from-scratch rewrite and is not the original Windows project. It is an unofficial Linux port maintained by Melih Eren, based on BypaxDPI-Windows and powered by SpoofDPI.

Current positioning:

- BypaxDPI Linux
- Unofficial Linux port maintained by Melih Eren.
- Based on BypaxDPI-Windows.
- Powered by SpoofDPI.

The Linux port focuses on making the app usable on Linux while preserving attribution and license information from the upstream work.

## Features

- React + Tauri desktop UI adapted under the visible name BypaxDPI Linux.
- SpoofDPI sidecar support for the Linux proxy engine.
- Linux-safe proxy behavior for the current MVP path.
- Platform-specific Rust logic separated so Linux and Windows-only behavior can be handled independently.
- Existing DNS, DPI mode, LAN/PAC sharing, logs, tray, and settings surfaces are kept where they apply.
- Original BypaxDPI-Windows concepts and user experience are preserved where they are still relevant to Linux.

## Linux Status

Status: Initial Linux MVP, work in progress.

This fork focuses on Linux compatibility. The Linux implementation keeps proxy startup behavior separate from Windows-only system integrations and avoids enabling Windows-specific code paths on Linux.

Separated or disabled on Linux:

- Npcap / WinPcap driver logic
- WinHTTP proxy tunnel logic
- UWP CheckNetIsolation handling
- Windows firewall rule management
- Windows registry proxy management
- Windows installer-specific behavior

Linux-safe proxy behavior and Linux SpoofDPI sidecar support have been added, but the port should still be treated as an MVP until more real-world Linux distributions and desktop environments are validated.

## Installation / Development

Prerequisites:

- Node.js and npm
- Rust toolchain
- Tauri Linux build dependencies for your distribution

Install dependencies:

```bash
npm install
```

Run the frontend build:

```bash
npm run build
```

Run Tauri development mode:

```bash
npm run tauri -- dev
```

Check the Rust side:

```bash
cd src-tauri
cargo check
```

Format Rust code:

```bash
cd src-tauri
cargo fmt
```

## Validation / Testing

Recommended validation for this fork:

```bash
npm run build
cd src-tauri
cargo fmt
cargo check
```

Manual validation should include:

- Launching the app on Linux.
- Confirming the UI shows BypaxDPI Linux.
- Confirming About / Credits identifies Melih Eren as Linux port maintainer.
- Confirming BypaxDPI-Windows and SpoofDPI attribution is visible.
- Starting and stopping the proxy sidecar.
- Confirming proxy cleanup after disconnect and app exit.

## Credits

BypaxDPI Linux<br>
Unofficial Linux port maintained by Melih Eren.<br>
Based on BypaxDPI-Windows.<br>
Powered by SpoofDPI.

- Linux port maintainer: Melih Eren
- Original project: [BypaxDPI-Windows](https://github.com/BypaxDPI/BypaxDPI-Windows)
- Proxy engine: [SpoofDPI](https://github.com/xvzc/SpoofDPI)

Original project credit and license information are intentionally preserved. This fork should not be presented as a fully original project.

## License

This repository keeps the existing MIT license for the BypaxDPI-Windows-derived application code. See [LICENSE](LICENSE).

Licenses:

- BypaxDPI-Windows: MIT License
- SpoofDPI: Apache License 2.0

SpoofDPI is used as the proxy engine and remains under its own upstream license.
