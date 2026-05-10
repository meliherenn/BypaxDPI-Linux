#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProxySetupResult {
    pub platform: &'static str,
    pub host: String,
    pub port: u16,
    pub automatic: bool,
    pub message: String,
}

#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn name() -> &'static str {
    std::env::consts::OS
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn should_bind_lan_for_game_mode(_enable_game_mode: bool) -> bool {
    false
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn set_system_proxy(port: u16, _enable_winhttp: bool) -> Result<ProxySetupResult, String> {
    Ok(ProxySetupResult {
        platform: name(),
        host: "127.0.0.1".to_string(),
        port,
        automatic: false,
        message: format!("Manual proxy fallback: 127.0.0.1:{}", port),
    })
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn clear_system_proxy() -> Result<(), String> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn manage_firewall_rules(_enable: bool, _proxy_port: u16, _pac_port: u16) {}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn check_admin() -> bool {
    true
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn check_driver() -> bool {
    false
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn install_driver(_app: tauri::AppHandle) -> Result<(), String> {
    Err("Driver installation is only supported on Windows.".to_string())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn kill_zombie_sidecar() -> Result<String, String> {
    Ok("No platform-specific zombie sidecar cleanup is available.".to_string())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn cleanup_on_panic() {}
