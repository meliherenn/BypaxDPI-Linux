#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { spawnSync } = require('node:child_process');
const { targetInfo, hostTriple, checkCompatibility } = require('./proxy-utils.cjs');
try {
  const root = path.resolve(__dirname, '..');
  const target = targetInfo(process.env.TAURI_TARGET_TRIPLE || process.env.TARGET_TRIPLE || hostTriple());
  const source = path.join(root, 'spoofdpi', `bypax-proxy${target.extension}`);
  checkCompatibility(source, target);
  const dir = path.join(root, 'src-tauri', 'binaries');
  fs.mkdirSync(dir, { recursive: true });
  for (const name of [`bypax-proxy${target.extension}`, `bypax-proxy-${target.triple}${target.extension}`]) {
    const dest = path.join(dir, name);
    fs.copyFileSync(source, dest);
    if (target.os === 'linux') fs.chmodSync(dest, 0o755);
  }
  const version = target.triple === hostTriple() ? spawnSync(source, ['--version'], { encoding: 'utf8', timeout: 5000 }).stdout.trim() : 'cross-compiled';
  const patchSha256 = target.os === 'linux' ? crypto.createHash('sha256').update(fs.readFileSync(path.join(root, 'engine-patches/spoofdpi-tls-record.patch'))).digest('hex') : undefined;
  const manifest = { patchSha256, target: target.triple, version, sha256: crypto.createHash('sha256').update(fs.readFileSync(source)).digest('hex') };
  fs.writeFileSync(path.join(dir, 'engine-manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
  console.log(`Sidecar ready: ${target.triple}\n${version}\nSHA256 ${manifest.sha256}`);
} catch (error) { console.error(error.message); process.exitCode = 1; }
