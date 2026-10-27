const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

function targetInfo(triple) {
  const supported = {
    'x86_64-unknown-linux-gnu': { os: 'linux', arch: 'amd64', machine: 62 },
    'aarch64-unknown-linux-gnu': { os: 'linux', arch: 'arm64', machine: 183 },
    'x86_64-unknown-linux-musl': { os: 'linux', arch: 'amd64', machine: 62 },
    'aarch64-unknown-linux-musl': { os: 'linux', arch: 'arm64', machine: 183 },
    'x86_64-pc-windows-msvc': { os: 'windows', arch: 'amd64', machine: 0x8664 },
    'aarch64-pc-windows-msvc': { os: 'windows', arch: 'arm64', machine: 0xaa64 },
  };
  if (!supported[triple]) throw new Error(`Unsupported sidecar target: ${triple}`);
  return { triple, ...supported[triple], extension: supported[triple].os === 'windows' ? '.exe' : '' };
}
function hostTriple() {
  const result = spawnSync('rustc', ['-vV'], { encoding: 'utf8' });
  const host = result.stdout?.match(/^host: (.+)$/m)?.[1];
  if (result.error || result.status !== 0 || !host) throw new Error('rustc -vV failed; install the Rust toolchain');
  return host;
}
function validateBinary(file, target) {
  const data = fs.readFileSync(file);
  if (target.os === 'linux') {
    if (data.length < 20 || data.subarray(0, 4).toString('hex') !== '7f454c46' || data[5] !== 1 || data.readUInt16LE(18) !== target.machine) {
      throw new Error(`Sidecar is not an ELF binary for ${target.triple}: ${file}`);
    }
  } else {
    if (data.length < 64 || data.subarray(0, 2).toString() !== 'MZ') throw new Error('Sidecar is not a Windows PE binary');
    const offset = data.readUInt32LE(60);
    if (offset + 6 > data.length || data.readUInt32LE(offset) !== 0x4550 || data.readUInt16LE(offset + 4) !== target.machine) throw new Error(`Sidecar architecture does not match ${target.triple}`);
  }
}
function findOnPath(name) {
  return (process.env.PATH || '').split(path.delimiter).map(dir => path.join(dir, name)).find(file => {
    try { fs.accessSync(file, fs.constants.X_OK); return fs.statSync(file).isFile(); } catch { return false; }
  });
}
function checkCompatibility(file, target) {
  validateBinary(file, target);
  if (target.triple !== hostTriple()) return; // Cross-compiled executables cannot be run on this host.
  const help = spawnSync(file, ['--help'], { encoding: 'utf8', timeout: 5000 });
  const flags = ['--clean', '--listen-addr', '--dns-mode', '--dns-https-url', '--https-split-mode', '--https-chunk-size'];
  flags.push(...(target.os === 'linux' ? ['--no-tui', '--tcp-timeout'] : ['--silent', '--timeout']));
  if (help.error || help.status !== 0 || flags.some(flag => !help.stdout.includes(flag))) throw new Error('SpoofDPI CLI is incompatible with this application');
  if (target.os === 'linux' && !help.stdout.includes('"record"')) throw new Error('Linux requires the TLS-record engine patch; run npm run build-proxy');
}
module.exports = { targetInfo, hostTriple, validateBinary, findOnPath, checkCompatibility };
