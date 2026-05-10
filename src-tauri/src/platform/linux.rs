use super::ProxySetupResult;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

const PROXY_HOST: &str = "127.0.0.1";

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct GSettingsProxyBackup {
    mode: Option<String>,
    http_host: Option<String>,
    http_port: Option<String>,
    https_host: Option<String>,
    https_port: Option<String>,
}

fn backup_store() -> &'static Mutex<Option<GSettingsProxyBackup>> {
    static STORE: OnceLock<Mutex<Option<GSettingsProxyBackup>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(None))
}

fn backup_path() -> std::path::PathBuf {
    std::env::temp_dir().join("bypaxdpi_linux_proxy_backup.json")
}

fn gsettings_get(schema: &str, key: &str) -> Option<String> {
    let output = Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn gsettings_writable(schema: &str, key: &str) -> bool {
    let output = Command::new("gsettings")
        .args(["writable", schema, key])
        .output();
    let Ok(output) = output else {
        return false;
    };
    output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true"
}

fn gsettings_set(schema: &str, key: &str, value: &str) -> bool {
    if !gsettings_writable(schema, key) {
        return false;
    }
    Command::new("gsettings")
        .args(["set", schema, key, value])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn gsettings_set_optional(schema: &str, key: &str, value: &str) -> bool {
    if !gsettings_writable(schema, key) {
        return true;
    }
    gsettings_set(schema, key, value)
}

fn gsettings_available() -> bool {
    gsettings_writable("org.gnome.system.proxy", "mode")
}

fn read_backup_from_disk() -> Option<GSettingsProxyBackup> {
    let contents = std::fs::read_to_string(backup_path()).ok()?;
    serde_json::from_str(&contents).ok()
}

fn write_backup_to_disk(settings: &GSettingsProxyBackup) {
    if let Ok(contents) = serde_json::to_string(settings) {
        let _ = std::fs::write(backup_path(), contents);
    }
}

fn backup_proxy_settings() {
    let settings = GSettingsProxyBackup {
        mode: gsettings_get("org.gnome.system.proxy", "mode"),
        http_host: gsettings_get("org.gnome.system.proxy.http", "host"),
        http_port: gsettings_get("org.gnome.system.proxy.http", "port"),
        https_host: gsettings_get("org.gnome.system.proxy.https", "host"),
        https_port: gsettings_get("org.gnome.system.proxy.https", "port"),
    };

    if let Ok(mut guard) = backup_store().lock() {
        if guard.is_none() {
            *guard = Some(settings.clone());
            write_backup_to_disk(&settings);
        }
    }
}

fn restore_raw(schema: &str, key: &str, value: &Option<String>) {
    if let Some(value) = value {
        let _ = gsettings_set(schema, key, value);
    }
}

fn restore_proxy_settings() -> bool {
    let settings = match backup_store().lock() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
    .or_else(read_backup_from_disk);

    let Some(settings) = settings else {
        return false;
    };

    restore_raw("org.gnome.system.proxy.http", "host", &settings.http_host);
    restore_raw("org.gnome.system.proxy.http", "port", &settings.http_port);
    restore_raw("org.gnome.system.proxy.https", "host", &settings.https_host);
    restore_raw("org.gnome.system.proxy.https", "port", &settings.https_port);
    restore_raw("org.gnome.system.proxy", "mode", &settings.mode);

    if let Ok(mut guard) = backup_store().lock() {
        *guard = None;
    }
    let _ = std::fs::remove_file(backup_path());
    true
}

pub fn name() -> &'static str {
    "linux"
}

pub fn should_bind_lan_for_game_mode(_enable_game_mode: bool) -> bool {
    false
}

pub fn set_system_proxy(port: u16, _enable_winhttp: bool) -> Result<ProxySetupResult, String> {
    if !gsettings_available() {
        return Ok(manual_fallback(port, "gsettings is unavailable"));
    }

    backup_proxy_settings();

    let port_value = port.to_string();
    let required_ok = gsettings_set("org.gnome.system.proxy.http", "host", "'127.0.0.1'")
        && gsettings_set("org.gnome.system.proxy.http", "port", &port_value)
        && gsettings_set("org.gnome.system.proxy", "mode", "'manual'");

    let optional_ok = gsettings_set_optional("org.gnome.system.proxy.https", "host", "'127.0.0.1'")
        && gsettings_set_optional("org.gnome.system.proxy.https", "port", &port_value);

    if required_ok && optional_ok {
        Ok(ProxySetupResult {
            platform: name(),
            host: PROXY_HOST.to_string(),
            port,
            automatic: true,
            message: format!("Linux GNOME proxy applied at {}:{}", PROXY_HOST, port),
        })
    } else {
        let _ = restore_proxy_settings();
        Ok(manual_fallback(port, "gsettings proxy setup failed"))
    }
}

pub fn clear_system_proxy() -> Result<(), String> {
    if !gsettings_available() {
        return Ok(());
    }

    if !restore_proxy_settings() {
        let _ = gsettings_set("org.gnome.system.proxy", "mode", "'none'");
    }

    Ok(())
}

fn manual_fallback(port: u16, reason: &str) -> ProxySetupResult {
    ProxySetupResult {
        platform: name(),
        host: PROXY_HOST.to_string(),
        port,
        automatic: false,
        message: format!(
            "Manual proxy fallback ({}): set HTTP/HTTPS proxy to {}:{}",
            reason, PROXY_HOST, port
        ),
    }
}

pub fn manage_firewall_rules(_enable: bool, _proxy_port: u16, _pac_port: u16) {}

pub fn check_admin() -> bool {
    true
}

pub fn check_driver() -> bool {
    false
}

pub fn install_driver(_app: tauri::AppHandle) -> Result<(), String> {
    Err("Npcap driver installation is only supported on Windows.".to_string())
}

pub fn kill_zombie_sidecar() -> Result<String, String> {
    let pid_file = std::env::temp_dir().join("bypaxdpi_sidecar.pid");
    let Ok(pid_str) = std::fs::read_to_string(&pid_file) else {
        return Ok("Zombie PID file was not found.".to_string());
    };

    let pid = pid_str
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("Invalid zombie PID file: {}", e))?;
    if pid == 0 {
        let _ = std::fs::remove_file(&pid_file);
        return Ok("Zombie PID file was empty.".to_string());
    }

    let status = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let _ = std::fs::remove_file(&pid_file);

    match status {
        Ok(status) if status.success() => {
            Ok(format!("Zombie sidecar process (PID {}) stopped.", pid))
        }
        Ok(_) => Ok(format!(
            "Zombie sidecar process (PID {}) was not running.",
            pid
        )),
        Err(e) => Err(format!("Failed to stop zombie sidecar PID {}: {}", pid, e)),
    }
}

pub fn cleanup_on_panic() {
    let _ = clear_system_proxy();
    let _ = kill_zombie_sidecar();
}
