import { DNS_MAP, DOH_MAP, DPI_TIMEOUTS } from './constants.js';

// The engine's help text says doh/sys in some releases, but its parser accepts
// https/system. These values have been exercised against the bundled executable.
export function buildEngineArgs({ config, platform, host, port, hasDriver = false }) {
  if (!['linux', 'windows'].includes(platform)) throw new Error('Unsupported platform');
  if (!['127.0.0.1', '0.0.0.0'].includes(host) || !Number.isInteger(port) || port < 1024 || port > 65535) {
    throw new Error('Invalid proxy listener');
  }
  const linux = platform === 'linux';
  const record = linux && config.linuxTlsRecordSplit !== false;
  const mode = ['0', '1', '2'].includes(String(config.dpiMethod)) ? String(config.dpiMethod) : '2';
  const chunk = [1, 2, 4, 8, 16, 32, 64, 128].includes(Number(config.httpsChunkSize)) ? String(config.httpsChunkSize) : '1';
  const args = ['--clean', '--listen-addr', `${host}:${port}`, linux ? '--tcp-timeout' : '--timeout',
    String(DPI_TIMEOUTS[record ? '2' : mode]), linux ? '--no-tui' : '--silent', '--log-level', 'info',
    '--dns-qtype', config.ipv4Only !== false ? 'ipv4' : 'all'];
  const dns = Object.hasOwn(DNS_MAP, config.selectedDns) ? config.selectedDns : 'system';
  if (!DNS_MAP[dns]) args.push('--dns-mode', 'system');
  else if (DOH_MAP[dns]) args.push('--dns-mode', 'https', '--dns-https-url', DOH_MAP[dns]);
  else args.push('--dns-mode', 'udp', '--dns-addr', `${DNS_MAP[dns]}:53`);
  if (record) args.push('--https-split-mode', 'record');
  else if (mode === '0') args.push('--https-split-mode', 'sni');
  else {
    args.push('--https-split-mode', 'chunk', '--https-chunk-size', chunk);
    if (!linux && mode === '2' && hasDriver && config.advancedBypass !== false) args.push('--https-fake-count', '3');
  }
  return args;
}
