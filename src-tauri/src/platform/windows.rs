use super::ProxySetupResult;
use std::sync::{Mutex, OnceLock};
use winreg::enums::*;
use winreg::RegKey;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

#[derive(Debug, Clone, Default)]
struct OriginalProxySettings {
    proxy_enable: Option<u32>,
    proxy_server: Option<String>,
    proxy_override: Option<String>,
}

fn original_proxy_store() -> &'static Mutex<Option<OriginalProxySettings>> {
    static STORE: OnceLock<Mutex<Option<OriginalProxySettings>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(None))
}

fn read_value_string(name: &str) -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(INTERNET_SETTINGS).ok()?;
    key.get_value(name).ok()
}

fn read_value_dword(name: &str) -> Option<u32> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(INTERNET_SETTINGS).ok()?;
    key.get_value(name).ok()
}

fn registry_can_access() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(INTERNET_SETTINGS).is_ok()
}

fn set_registry_proxy(proxy_addr: &str, port: u16) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(INTERNET_SETTINGS)
        .map_err(|e| format!("Registry açılamadı: {}", e))?;

    key.set_value("ProxyServer", &format!("{}:{}", proxy_addr, port))
        .map_err(|e| format!("ProxyServer: {}", e))?;
    key.set_value("ProxyEnable", &1u32)
        .map_err(|e| format!("ProxyEnable: {}", e))?;

    let proxy_override = [
        "<local>",
        "10.*",
        "172.16.*",
        "172.17.*",
        "172.18.*",
        "172.19.*",
        "172.20.*",
        "172.21.*",
        "172.22.*",
        "172.23.*",
        "172.24.*",
        "172.25.*",
        "172.26.*",
        "172.27.*",
        "172.28.*",
        "172.29.*",
        "172.30.*",
        "172.31.*",
        "192.168.*",
        "*.msftconnecttest.com",
        "*.msftncsi.com",
        "dns.msn.com",
        "ipv6.msftconnecttest.com",
        "connectivitycheck.gstatic.com",
        "connectivitycheck.android.com",
        "clients3.google.com",
        "play.googleapis.com",
        "captive.apple.com",
        "gsp1.apple.com",
        "connectivitycheck.samsung.com",
        "*.windowsupdate.com",
        "*.delivery.mp.microsoft.com",
        "*.steamcontent.com",
        "*.steamstatic.com",
        "clientconfig.akamai.steamstatic.com",
        "*.cm.steampowered.com",
        "*.epicgames.com",
        "*.unrealengine.com",
        "download.epicgames.com",
        "launcher-public-service-prod06.ol.epicgames.com",
        "*.riotgames.com",
        "*.leagueoflegends.com",
        "riotgames-update.akamaized.net",
        "*.ea.com",
        "*.origin.com",
        "*.blizzard.com",
        "*.battle.net",
        "blzddist1-a.akamaihd.net",
        "*.ubisoft.com",
        "*.ubi.com",
        "*.xboxlive.com",
        "*.xbox.com",
        "*.microsoft.com",
        "*.cachefly.net",
    ]
    .join(";");
    key.set_value("ProxyOverride", &proxy_override)
        .map_err(|e| format!("ProxyOverride: {}", e))?;
    Ok(())
}

fn clear_registry_proxy() -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(INTERNET_SETTINGS)
        .map_err(|e| format!("Registry açılamadı: {}", e))?;

    key.set_value("ProxyEnable", &0u32)
        .map_err(|e| format!("ProxyEnable: {}", e))?;
    let _ = key.delete_value("ProxyServer");
    let _ = key.delete_value("ProxyOverride");
    let _ = key.delete_value("AutoConfigURL");
    Ok(())
}

fn restore_registry_proxy(
    server: &str,
    enable: u32,
    override_val: Option<&str>,
) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(INTERNET_SETTINGS)
        .map_err(|e| format!("Registry açılamadı: {}", e))?;

    key.set_value("ProxyServer", &server)
        .map_err(|e| format!("ProxyServer: {}", e))?;
    key.set_value("ProxyEnable", &enable)
        .map_err(|e| format!("ProxyEnable: {}", e))?;
    if let Some(ov) = override_val {
        key.set_value("ProxyOverride", &ov)
            .map_err(|e| format!("ProxyOverride: {}", e))?;
    }
    Ok(())
}

fn backup_proxy_settings() {
    let settings = OriginalProxySettings {
        proxy_enable: read_value_dword("ProxyEnable"),
        proxy_server: read_value_string("ProxyServer"),
        proxy_override: read_value_string("ProxyOverride"),
    };

    if let Ok(mut guard) = original_proxy_store().lock() {
        if guard.is_none() {
            eprintln!("[PROXY-BACKUP] Orijinal ayarlar yedeklendi: {:?}", settings);
            *guard = Some(settings);
        }
    }
}

fn restore_proxy_settings() -> bool {
    let original = match original_proxy_store().lock() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => {
            eprintln!("[WARN] proxy backup lock poisoned, recovering");
            poisoned.into_inner().clone()
        }
    };

    if let Some(orig) = original {
        if let Some(ref server) = orig.proxy_server {
            if !server.is_empty() && !server.starts_with("127.0.0.1:") {
                eprintln!("[PROXY-RESTORE] Kurumsal proxy geri yükleniyor: {}", server);

                let enable_val = orig.proxy_enable.unwrap_or(0);
                let _ = restore_registry_proxy(server, enable_val, orig.proxy_override.as_deref());
                return true;
            }
        }
    }
    false
}

pub fn name() -> &'static str {
    "windows"
}

pub fn should_bind_lan_for_game_mode(enable_game_mode: bool) -> bool {
    enable_game_mode
}

pub fn set_system_proxy(port: u16, enable_winhttp: bool) -> Result<ProxySetupResult, String> {
    use std::os::windows::process::CommandExt;

    if !registry_can_access() {
        return Err("Registry yazma izni yok. Uygulamayı yönetici olarak çalıştırın.".to_string());
    }

    backup_proxy_settings();

    let proxy_addr = "127.0.0.1".to_string();
    set_registry_proxy(&proxy_addr, port).map_err(|e| {
        let _ = clear_registry_proxy();
        format!("Registry güncelleme başarısız, geri alındı: {}", e)
    })?;

    notify_proxy_change();
    exempt_all_uwp_apps();

    if enable_winhttp {
        let winhttp_bypass = format!(
            "bypass-list=\"<local>;{};*.steamcontent.com;*.steamstatic.com;*.cm.steampowered.com;*.epicgames.com;*.unrealengine.com;*.riotgames.com;*.leagueoflegends.com;*.ea.com;*.origin.com;*.blizzard.com;*.battle.net;*.ubisoft.com;*.ubi.com;*.xboxlive.com;*.xbox.com;*.microsoft.com;*.cachefly.net;*.msftconnecttest.com;*.windowsupdate.com\"",
            proxy_addr
        );
        let _ = std::process::Command::new("netsh")
            .args([
                "winhttp",
                "set",
                "proxy",
                &format!("{}:{}", proxy_addr, port),
                &winhttp_bypass,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    Ok(ProxySetupResult {
        platform: name(),
        host: proxy_addr,
        port,
        automatic: true,
        message: format!("Windows system proxy applied at 127.0.0.1:{}", port),
    })
}

pub fn clear_system_proxy() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let has_original = restore_proxy_settings();
    if !has_original {
        let _ = clear_registry_proxy();
    }

    let _ = Command::new("ipconfig")
        .arg("/flushdns")
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    notify_proxy_change();

    let _ = std::process::Command::new("netsh")
        .args(["winhttp", "reset", "proxy"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    manage_firewall_rules(false, 0, 0);

    if let Ok(mut guard) = original_proxy_store().lock() {
        *guard = None;
    }

    Ok(())
}

pub fn manage_firewall_rules(enable: bool, proxy_port: u16, pac_port: u16) {
    std::thread::spawn(move || {
        use std::os::windows::process::CommandExt;

        let _ = std::process::Command::new("netsh")
            .args([
                "advfirewall",
                "firewall",
                "delete",
                "rule",
                "name=BypaxDPI_Proxy",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        let _ = std::process::Command::new("netsh")
            .args([
                "advfirewall",
                "firewall",
                "delete",
                "rule",
                "name=BypaxDPI_PAC",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if enable {
            let _ = std::process::Command::new("netsh")
                .args([
                    "advfirewall",
                    "firewall",
                    "add",
                    "rule",
                    "name=BypaxDPI_Proxy",
                    "dir=in",
                    "action=allow",
                    "protocol=TCP",
                    &format!("localport={}", proxy_port),
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();

            let _ = std::process::Command::new("netsh")
                .args([
                    "advfirewall",
                    "firewall",
                    "add",
                    "rule",
                    "name=BypaxDPI_PAC",
                    "dir=in",
                    "action=allow",
                    "protocol=TCP",
                    &format!("localport={}", pac_port),
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
    });
}

fn notify_proxy_change() {
    use std::ptr::null_mut;
    use winapi::um::wininet::{
        InternetSetOptionW, INTERNET_OPTION_REFRESH, INTERNET_OPTION_SETTINGS_CHANGED,
    };

    unsafe {
        InternetSetOptionW(null_mut(), INTERNET_OPTION_SETTINGS_CHANGED, null_mut(), 0);
        InternetSetOptionW(null_mut(), INTERNET_OPTION_REFRESH, null_mut(), 0);
    }
}

fn exempt_all_uwp_apps() {
    std::thread::spawn(|| {
        use std::os::windows::process::CommandExt;

        let script = r#"
            try {
                $packages = Get-AppxPackage -ErrorAction SilentlyContinue
                foreach ($pkg in $packages) {
                    if ($pkg.PackageFamilyName) {
                        CheckNetIsolation.exe LoopbackExempt -a "-n=$($pkg.PackageFamilyName)"
                    }
                }
            } catch {}
        "#;

        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    });
}

pub fn check_admin() -> bool {
    use std::mem;
    use std::ptr;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, HANDLE, TOKEN_ELEVATION, TOKEN_QUERY};

    unsafe {
        let mut token: HANDLE = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let mut elevation: TOKEN_ELEVATION = mem::zeroed();
        let mut size: u32 = 0;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        );

        CloseHandle(token);
        result != 0 && elevation.TokenIsElevated != 0
    }
}

pub fn check_driver() -> bool {
    std::path::Path::new("C:\\Windows\\System32\\wpcap.dll").exists()
        || std::path::Path::new("C:\\Windows\\SysWOW64\\wpcap.dll").exists()
}

pub fn install_driver(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    let resource_path = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("binaries/npcap-installer.exe");

    if !resource_path.exists() {
        return Err("Sürücü dosyası bulunamadı. Lütfen uygulamayı yeniden yükleyin.".into());
    }

    let status = std::process::Command::new(resource_path)
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("Kurulum kullanıcı tarafından iptal edildi veya başarısız oldu.".into())
    }
}

pub fn kill_zombie_sidecar() -> Result<String, String> {
    use std::os::windows::process::CommandExt;

    let pid_file = std::env::temp_dir().join("bypaxdpi_sidecar.pid");
    if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            if pid > 0 {
                let output = std::process::Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();

                let _ = std::fs::remove_file(&pid_file);

                if let Ok(out) = output {
                    if out.status.success() {
                        return Ok(format!("Zombi süreç (PID {}) durduruldu.", pid));
                    }
                }
            }
        }
    }
    Ok("Zombi PID dosyası bulunamadı.".to_string())
}

pub fn cleanup_on_panic() {
    use std::os::windows::process::CommandExt;

    let _ = std::process::Command::new("reg")
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
            "/v",
            "ProxyEnable",
            "/t",
            "REG_DWORD",
            "/d",
            "0",
            "/f",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status();

    let _ = std::process::Command::new("reg")
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
            "/v",
            "ProxyServer",
            "/t",
            "REG_SZ",
            "/d",
            "",
            "/f",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status();

    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "bypax-proxy.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}
