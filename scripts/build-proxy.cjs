#!/usr/bin/env node
/**
 * Build SpoofDPI 1.2.1 (bypax-proxy) and copy to spoofdpi/ and src-tauri/binaries/.
 * Requires Go in PATH.
 */
const { spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const root = path.resolve(__dirname, '..');
const spoofDpiDir = path.join(root, 'SpoofDPI-1.2.1', 'SpoofDPI-1.2.1');
const isWindows = process.platform === 'win32';
const targetTriple = process.env.TAURI_TARGET_TRIPLE
  || process.env.TARGET_TRIPLE
  || (isWindows ? 'x86_64-pc-windows-msvc' : 'x86_64-unknown-linux-gnu');
const outName = isWindows ? 'bypax-proxy.exe' : 'bypax-proxy';
const outFile = path.join(root, 'spoofdpi', outName);

const spoofdpiDir = path.join(root, 'spoofdpi');
if (!fs.existsSync(spoofdpiDir)) {
  fs.mkdirSync(spoofdpiDir, { recursive: true });
}

const spoofDpiSourceExists = fs.existsSync(path.join(spoofDpiDir, 'go.mod'));
if (spoofDpiSourceExists) {
  console.log(`Building SpoofDPI (bypax-proxy) for ${targetTriple} with release flags...`);
  const go = spawnSync('go', ['build', '-trimpath', '-ldflags=-s -w', '-o', outFile, './cmd/spoofdpi'], {
    cwd: spoofDpiDir,
    stdio: 'inherit',
    shell: true,
  });

  if (go.status !== 0) {
    console.error('go build failed');
    process.exit(go.status || 1);
  }
} else if (!isWindows) {
  const which = spawnSync('command', ['-v', 'spoofdpi'], {
    shell: true,
    encoding: 'utf8',
  });
  const systemSpoofDpi = which.stdout.trim();

  if (!systemSpoofDpi) {
    console.error('SpoofDPI source not found at', spoofDpiDir);
    console.error('System spoofdpi binary not found in PATH. Install spoofdpi or provide SpoofDPI source before packaging.');
    process.exit(1);
  }

  console.log(`SpoofDPI source not found at ${spoofDpiDir}`);
  console.log(`Using system spoofdpi binary for Linux sidecar: ${systemSpoofDpi}`);
  fs.copyFileSync(systemSpoofDpi, outFile);
} else {
  console.error('SpoofDPI source not found at', spoofDpiDir);
  process.exit(1);
}

if (!isWindows) {
  fs.chmodSync(outFile, 0o755);
}

console.log('Build OK:', outFile);
console.log('Copying to src-tauri/binaries/...');
const copy = spawnSync('node', [path.join(__dirname, 'copy-proxy.cjs')], {
  cwd: root,
  stdio: 'inherit',
  env: { ...process.env, TAURI_TARGET_TRIPLE: targetTriple },
});
process.exit(copy.status || 0);
