#!/usr/bin/env node
/**
 * Copy bypax-proxy from spoofdpi/ to src-tauri/binaries/
 * so Tauri sidecar finds it (dev + bundle).
 * Run after building the proxy: npm run build-proxy && npm run copy-proxy
 */
const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const isWindows = process.platform === 'win32';
const targetTriple = process.env.TAURI_TARGET_TRIPLE
  || process.env.TARGET_TRIPLE
  || (isWindows ? 'x86_64-pc-windows-msvc' : 'x86_64-unknown-linux-gnu');
const baseName = 'bypax-proxy';
const extension = isWindows ? '.exe' : '';
const src = path.join(root, 'spoofdpi', `${baseName}${extension}`);
const destDir = path.join(root, 'src-tauri', 'binaries');
const destBase = path.join(destDir, `${baseName}${extension}`);
const destTriple = path.join(destDir, `${baseName}-${targetTriple}${extension}`);

if (!fs.existsSync(src)) {
  console.error(`spoofdpi/${baseName}${extension} not found. Build it first: npm run build-proxy`);
  process.exit(1);
}

if (!fs.existsSync(destDir)) {
  fs.mkdirSync(destDir, { recursive: true });
}

fs.copyFileSync(src, destBase);
fs.copyFileSync(src, destTriple);
if (!isWindows) {
  fs.chmodSync(destBase, 0o755);
  fs.chmodSync(destTriple, 0o755);
}
console.log(`Copied ${baseName}${extension} to src-tauri/binaries/ (and ${path.basename(destTriple)})`);
