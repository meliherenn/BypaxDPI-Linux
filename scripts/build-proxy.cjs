#!/usr/bin/env node
// Build the pinned, patched Linux engine; retain compatible Windows packaging.
const { spawnSync } = require('node:child_process');
const path = require('node:path');
const fs = require('node:fs');
const { targetInfo, hostTriple, findOnPath, checkCompatibility } = require('./proxy-utils.cjs');
const { buildPatchedLinux } = require('./patched-engine.cjs');
const root = path.resolve(__dirname, '..');
try {
  const target = targetInfo(process.env.TAURI_TARGET_TRIPLE || process.env.TARGET_TRIPLE || hostTriple());
  const source = process.env.SPOOFDPI_SOURCE_DIR || path.join(root, 'SpoofDPI-1.2.1', 'SpoofDPI-1.2.1');
  const output = path.join(root, 'spoofdpi', `bypax-proxy${target.extension}`);
  fs.mkdirSync(path.dirname(output), { recursive: true });
  const temporary = `${output}.building${target.extension}`;
  try {
    if (target.os === 'linux') {
      buildPatchedLinux(root, temporary, target);
    } else if (fs.existsSync(path.join(source, 'go.mod'))) {
      const result = spawnSync('go', ['build', '-trimpath', '-ldflags=-s -w', '-o', temporary, './cmd/spoofdpi'], {
        cwd: source, stdio: 'inherit', env: { ...process.env, GOOS: target.os, GOARCH: target.arch, CGO_ENABLED: '0' },
      });
      if (result.error || result.status !== 0) throw new Error(`go build failed: ${result.error || result.status}`);
    } else {
      if (target.triple !== hostTriple()) throw new Error('Cross-compilation requires SPOOFDPI_SOURCE_DIR; an installed binary cannot be relabeled for another target');
      const installed = findOnPath(target.os === 'windows' ? 'spoofdpi.exe' : 'spoofdpi');
      if (!installed) throw new Error('Install SpoofDPI or set SPOOFDPI_SOURCE_DIR to an upstream checkout');
      fs.copyFileSync(installed, temporary);
    }
    if (target.os === 'linux') fs.chmodSync(temporary, 0o755);
    checkCompatibility(temporary, target);
    fs.renameSync(temporary, output);
  } finally { if (fs.existsSync(temporary)) fs.unlinkSync(temporary); }
  const result = spawnSync(process.execPath, [path.join(__dirname, 'copy-proxy.cjs')], {
    cwd: root, stdio: 'inherit', env: { ...process.env, TAURI_TARGET_TRIPLE: target.triple },
  });
  if (result.error || result.status !== 0) throw new Error(`Sidecar copy failed: ${result.error || result.status}`);
} catch (error) { console.error(error.message); process.exitCode = 1; }
