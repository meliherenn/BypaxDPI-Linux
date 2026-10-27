// Reproducible Linux engine build. The source checkout is never patched in place.
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const REVISION = 'eb07db9ef0069c68a0d37f3f6368178b7a30d025';
function run(command, args, cwd, env = process.env) {
  const result = spawnSync(command, args, { cwd, env, stdio: 'inherit' });
  if (result.error || result.status !== 0) throw new Error(`${command} failed: ${result.error || result.status}`);
}
function readGit(source, args) {
  const result = spawnSync('git', ['-C', source, ...args], { encoding: 'utf8' });
  if (result.error || result.status !== 0) throw new Error('Cannot inspect SpoofDPI source checkout');
  return result.stdout.trim();
}
function buildPatchedLinux(root, output, target) {
  const source = process.env.SPOOFDPI_SOURCE_DIR || path.join(root, 'spoofdpi', 'upstream');
  if (!fs.existsSync(source)) {
    if (process.env.SPOOFDPI_SOURCE_DIR) throw new Error('SPOOFDPI_SOURCE_DIR does not exist');
    console.log(`Fetching official SpoofDPI v1.5.4 (${REVISION}) into the local build cache`);
    run('git', ['clone', '--depth', '1', '--branch', 'v1.5.4', 'https://github.com/xvzc/spoofdpi.git', source], root);
  }
  if (readGit(source, ['rev-parse', 'HEAD']) !== REVISION || readGit(source, ['status', '--porcelain'])) {
    throw new Error(`TLS-record patch requires a clean SpoofDPI checkout at ${REVISION}; existing source was left untouched`);
  }
  const staging = fs.mkdtempSync(path.join(os.tmpdir(), 'bypax-engine-build-'));
  try {
    fs.cpSync(source, staging, { recursive: true, filter: file => path.basename(file) !== '.git' });
    const patch = path.join(root, 'engine-patches', 'spoofdpi-tls-record.patch');
    run('git', ['apply', '--check', patch], staging);
    run('git', ['apply', patch], staging);
    run('go', ['test', './internal/desync', './internal/config'], staging);
    run('go', ['build', '-trimpath', '-ldflags=-s -w -X main.version=1.5.4-bypax-record1 -X main.commit=eb07db9', '-o', output, './cmd/spoofdpi'], staging,
      { ...process.env, GOOS: target.os, GOARCH: target.arch, CGO_ENABLED: '0' });
  } finally { fs.rmSync(staging, { recursive: true, force: true }); }
}
module.exports = { buildPatchedLinux, REVISION };
