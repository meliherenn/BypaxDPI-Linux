const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { targetInfo, validateBinary } = require('../scripts/proxy-utils.cjs');
test('targets use the correct Go architecture and extension', () => {
  assert.equal(targetInfo('aarch64-unknown-linux-gnu').arch, 'arm64');
  assert.equal(targetInfo('x86_64-pc-windows-msvc').extension, '.exe');
  assert.throws(() => targetInfo('unknown-target'));
});
test('an x86 ELF cannot be mislabeled as ARM or Windows', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'bypax-target-test-'));
  try {
    const file = path.join(dir, 'fixture');
    const header = Buffer.alloc(64); header.write('7f454c46', 0, 'hex'); header[5] = 1; header.writeUInt16LE(62, 18);
    fs.writeFileSync(file, header);
    validateBinary(file, targetInfo('x86_64-unknown-linux-gnu'));
    assert.throws(() => validateBinary(file, targetInfo('aarch64-unknown-linux-gnu')));
    assert.throws(() => validateBinary(file, targetInfo('x86_64-pc-windows-msvc')));
    fs.writeFileSync(file, '#!/bin/sh');
    assert.throws(() => validateBinary(file, targetInfo('x86_64-unknown-linux-gnu')));
  } finally { fs.rmSync(dir, { recursive: true, force: true }); }
});
