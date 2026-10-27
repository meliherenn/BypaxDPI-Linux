const { test } = require('node:test');
const assert = require('node:assert/strict');
const build = async (config, platform = 'linux', extra = {}) => (await import('../src/engineArgs.js')).buildEngineArgs({config:{linuxTlsRecordSplit:false,...config},platform,host:'127.0.0.1',port:8080,...extra});
test('Linux uses headless flags and actual engine DNS parser values', async () => {
  const args = await build({selectedDns:'cloudflare',dpiMethod:'2',httpsChunkSize:1});
  assert.ok(args.includes('--no-tui')); assert.ok(args.includes('--tcp-timeout')); assert.ok(args.includes('https'));
  assert.ok(!args.includes('--silent')); assert.ok(!args.includes('--https-fake-count'));
  assert.ok((await build({selectedDns:'system'})).includes('system'));
});
test('strong mode honors the selected chunk size, including 32 and 128', async () => {
  for (const chunk of [1,2,4,8,16,32,64,128]) {
    const args = await build({dpiMethod:'2',httpsChunkSize:chunk});
    assert.equal(args[args.indexOf('--https-chunk-size')+1],String(chunk));
  }
});
test('invalid saved settings cannot introduce engine flags', async () => {
  const args = await build({selectedDns:'--config',httpsChunkSize:'1 --config /tmp/config',dpiMethod:'bad'});
  assert.ok(args.includes('system')); assert.ok(!args.includes('--config'));
  assert.equal(args[args.indexOf('--https-chunk-size')+1],'1');
  await assert.rejects(build({},'linux',{host:'example.com'}));
  await assert.rejects(build({},'linux',{port:80}));
});
test('Windows advanced mode requires a driver; Linux never enables it', async () => {
  const cfg={dpiMethod:'2',selectedDns:'system',advancedBypass:true};
  assert.ok((await build(cfg,'windows',{hasDriver:true})).includes('--https-fake-count'));
  assert.ok(!(await build(cfg,'windows')).includes('--https-fake-count'));
  assert.ok(!(await build(cfg,'linux',{hasDriver:true})).includes('--https-fake-count'));
});

test('TLS record mode is the Linux default and leaves Windows arguments unchanged', async () => {
  const {buildEngineArgs}=await import('../src/engineArgs.js');
  const args=buildEngineArgs({config:{selectedDns:'cloudflare',dpiMethod:'0'},platform:'linux',host:'127.0.0.1',port:8081});
  assert.equal(args[args.indexOf('--https-split-mode')+1],'record');
  assert.equal(args[args.indexOf('--tcp-timeout')+1],'8000');
  assert.ok(args.includes('https')); assert.ok(!args.includes('--https-chunk-size'));
  assert.ok(!(await build({linuxTlsRecordSplit:true},'windows')).includes('record'));
  assert.ok((await build({linuxTlsRecordSplit:false,dpiMethod:'0'})).includes('sni'));
});
