import { useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

export default function ConnectionDiagnostics({ connected, port, automatic, language, onSetup }) {
  const tr = language !== 'en';
  const labels = tr ? {tls_or_reset:'TLS bağlantısı kesildi',dns_error:'DNS hatası',connection_failed:'Bağlantı kurulamadı',timeout:'Zaman aşımı',certificate_error:'Sertifika hatası',request_failed:'İstek başarısız',tool_unavailable:'curl bulunamadı',http_error:'HTTP hatası'} : {tls_or_reset:'TLS connection reset',dns_error:'DNS error',connection_failed:'Connection failed',timeout:'Timed out',certificate_error:'Certificate error',request_failed:'Request failed',tool_unavailable:'curl unavailable',http_error:'HTTP error'};
  const [results, setResults] = useState(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [copied, setCopied] = useState(false);
  const generation = useRef(0);
  const running = useRef(false);
  useEffect(() => { generation.current++; setResults(null); setError(''); }, [connected, port]);
  useEffect(() => () => { generation.current++; }, []);
  const run = async () => {
    if (running.current) return;
    running.current = true;
    const current = generation.current;
    setBusy(true); setError('');
    try {
      const report = await invoke('diagnose_connection', { port: connected ? port : null });
      if (generation.current === current) setResults(report);
    } catch (err) { if (generation.current === current) setError(String(err)); }
    finally { running.current = false; setBusy(false); }
  };
  const copy = async () => {
    try {
      await writeText(`curl -q -4 --noproxy '' --proxy http://127.0.0.1:${port} --connect-timeout 5 --max-time 12 -v -o /dev/null https://discord.com/app`);
      setCopied(true); setTimeout(() => setCopied(false), 2000);
    } catch (err) { setError(String(err)); }
  };
  return <section className="connection-diagnostics" aria-label={tr ? 'Bağlantı testi' : 'Connection test'}>
    {connected && <p><strong>{automatic ? (tr ? 'Masaüstü proxy etkin' : 'Desktop proxy enabled') : (tr ? 'Uygulama proxy ayarı gerekli' : 'Per-app proxy setup required')}</strong><br />HTTP/HTTPS · 127.0.0.1:{port}</p>}
    <div className="diagnostic-actions">
      <button onClick={run} disabled={busy}>{busy ? (tr ? 'Test ediliyor…' : 'Testing…') : (tr ? 'Discord bağlantısını test et' : 'Test Discord connection')}</button>
      {connected && <button onClick={onSetup}>{tr ? 'Proxy ayarı' : 'Proxy setup'}</button>}
      {connected && <button onClick={copy}>{copied ? (tr ? 'Kopyalandı' : 'Copied') : (tr ? 'Test komutunu kopyala' : 'Copy test command')}</button>}
    </div>
    <div aria-live="polite">
      {results?.map((result) => <div className="diagnostic-result" key={`${result.target}-${result.route}`}>
        <span>{result.target} · {result.route === 'proxy' ? 'Proxy' : (tr ? 'Doğrudan' : 'Direct')}</span>
        <strong className={result.reachable ? 'probe-ok' : 'probe-failed'} title={result.detail}>
          {result.reachable ? `HTTP ${result.httpStatus}` : (result.httpStatus ? `HTTP ${result.httpStatus}` : (labels[result.category] || result.category))}
        </strong>
      </div>)}
      {error && <p className="probe-failed">{error}</p>}
    </div>
    <p className="diagnostic-note">{tr ? 'Bu HTTP proxy yalnızca onu kullanan uygulamaları kapsar. Discord ses/video (UDP) trafiğini taşımaz.' : 'This HTTP proxy covers only apps configured to use it. It does not carry Discord voice/video (UDP).'}</p>
  </section>;
}
