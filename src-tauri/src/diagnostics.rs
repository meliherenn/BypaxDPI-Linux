//! End-to-end HTTPS probes. CONNECT 200 alone does not prove TLS or site access.
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    target: String,
    route: String,
    pub reachable: bool,
    http_status: Option<u16>,
    elapsed_ms: u64,
    category: String,
    detail: String,
}

fn interpret(target: &str, route: &str, exit: i32, stdout: &str, stderr: &str) -> Probe {
    let mut lines = stdout.lines();
    let status = lines
        .next()
        .and_then(|s| s.parse::<u16>().ok())
        .filter(|s| *s >= 100 && *s <= 599);
    let elapsed_ms = (lines
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0)
        * 1000.0) as u64;
    let reachable = exit == 0 && status.is_some_and(|s| (200..400).contains(&s));
    let category = match exit {
        0 if reachable => "ok",
        0 => "http_error",
        5 | 6 => "dns_error",
        7 => "connection_failed",
        28 => "timeout",
        35 | 56 => "tls_or_reset",
        51 | 60 => "certificate_error",
        _ => "request_failed",
    };
    Probe {
        target: target.into(),
        route: route.into(),
        reachable,
        http_status: status,
        elapsed_ms,
        category: category.into(),
        detail: stderr.trim().chars().take(600).collect(),
    }
}

fn probe(target: &str, url: &str, port: Option<u16>) -> Probe {
    let route = if port.is_some() { "proxy" } else { "direct" };
    let mut command = Command::new("curl");
    // -q must be first: ignore ~/.curlrc. Explicit route overrides all proxy environment variables.
    command.args([
        "-q",
        "--ipv4",
        "--silent",
        "--show-error",
        "--connect-timeout",
        "5",
        "--max-time",
        "12",
        "--output",
        if cfg!(windows) { "NUL" } else { "/dev/null" },
        "--write-out",
        "%{http_code}\n%{time_total}\n",
        "--proto",
        "=https",
    ]);
    if let Some(port) = port {
        command.args([
            "--noproxy",
            "",
            "--proxy",
            &format!("http://127.0.0.1:{port}"),
        ]);
    } else {
        command.args(["--noproxy", "*", "--proxy", ""]);
    }
    command.arg(url);
    match command.output() {
        Ok(output) => interpret(
            target,
            route,
            output.status.code().unwrap_or(-1),
            &String::from_utf8_lossy(&output.stdout),
            &String::from_utf8_lossy(&output.stderr),
        ),
        Err(err) => Probe {
            target: target.into(),
            route: route.into(),
            reachable: false,
            http_status: None,
            elapsed_ms: 0,
            category: "tool_unavailable".into(),
            detail: format!("HTTPS diagnostics require curl: {err}"),
        },
    }
}

pub fn run(port: Option<u16>) -> Result<Vec<Probe>, String> {
    if port.is_some_and(|p| p < 1024) {
        return Err("Invalid proxy port".into());
    }
    let mut jobs = Vec::new();
    // Fixed public URLs; callers cannot turn this into an arbitrary URL/command executor.
    for (name, url) in [
        ("Discord", "https://discord.com/app"),
        ("Example", "https://example.com/"),
    ] {
        jobs.push(std::thread::spawn(move || probe(name, url, None)));
        if port.is_some() {
            jobs.push(std::thread::spawn(move || probe(name, url, port)));
        }
    }
    jobs.into_iter()
        .map(|job| job.join().map_err(|_| "HTTPS probe failed".into()))
        .collect()
}

static RUNNING: AtomicBool = AtomicBool::new(false);
struct RunningGuard;
impl Drop for RunningGuard {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::SeqCst);
    }
}

#[tauri::command]
pub async fn diagnose_connection(port: Option<u16>) -> Result<Vec<Probe>, String> {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Err("A connection test is already running".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = RunningGuard;
        run(port)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connect_success_without_tls_is_not_site_success() {
        assert!(!interpret("Discord", "proxy", 35, "000\n0.1\n", "TLS reset").reachable);
        assert!(!interpret("Discord", "proxy", 60, "200\n0.1\n", "certificate failed").reachable);
    }
    #[test]
    fn http_errors_are_distinct_from_tls_errors() {
        let result = interpret("Discord", "proxy", 0, "403\n0.123\n", "");
        assert!(!result.reachable);
        assert_eq!(result.category, "http_error");
        assert_eq!(result.http_status, Some(403));
        assert_eq!(result.elapsed_ms, 123);
        assert!(interpret("Discord", "proxy", 0, "200\n0.1\n", "").reachable);
    }
    #[test]
    fn malformed_output_and_privileged_ports_are_rejected() {
        assert!(!interpret("Discord", "proxy", 0, "nonsense", "").reachable);
        assert!(run(Some(22)).is_err());
    }
    #[test]
    fn connect_only_fixture_cannot_pass_tls_probe() {
        use std::io::{Read, Write};
        assert!(
            Command::new("curl").arg("--version").output().is_ok(),
            "Install curl before running the HTTPS fixture test"
        );
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut buf = [0; 2048];
            let _ = stream.read(&mut buf);
            stream
                .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                .unwrap();
            let _ = stream.read(&mut buf); // ClientHello then close; never send a valid TLS response.
        });
        let result = probe("fixture", "https://example.com/", Some(port));
        server.join().unwrap();
        assert!(!result.reachable);
        assert_eq!(result.category, "tls_or_reset");
    }
}
