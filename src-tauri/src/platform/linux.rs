use super::{linux_state, ProxySetupResult};
use std::process::Command;

const PROXY_HOST: &str = "127.0.0.1";
const BACKUP: &str = "proxy-backup.json";
// Mode is written last: every endpoint must be ready before enabling the proxy.
const KEYS: &[(&str, &str)] = &[
    ("org.gnome.system.proxy.http", "host"),
    ("org.gnome.system.proxy.http", "port"),
    ("org.gnome.system.proxy.https", "host"),
    ("org.gnome.system.proxy.https", "port"),
    ("org.gnome.system.proxy.http", "use-authentication"),
    ("org.gnome.system.proxy", "use-same-proxy"),
    ("org.gnome.system.proxy", "mode"),
];

trait Settings {
    fn get(&self, schema: &str, key: &str) -> Result<String, String>;
    fn set(&self, schema: &str, key: &str, value: &str) -> Result<(), String>;
}
struct GSettings;
impl Settings for GSettings {
    fn get(&self, schema: &str, key: &str) -> Result<String, String> {
        let output = Command::new("gsettings")
            .args(["get", schema, key])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().into())
    }
    fn set(&self, schema: &str, key: &str, value: &str) -> Result<(), String> {
        let output = Command::new("gsettings")
            .args(["set", schema, key, value])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().into());
        }
        if self.get(schema, key)? != value {
            return Err(format!("Proxy setting did not persist: {schema} {key}"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Backup {
    original: Vec<String>,
    applied: Vec<String>,
}
trait Recovery {
    fn read(&self) -> Result<Option<Backup>, String>;
    fn save(&self, backup: &Backup) -> Result<(), String>;
    fn remove(&self) -> Result<(), String>;
}
struct Disk;
impl Recovery for Disk {
    fn read(&self) -> Result<Option<Backup>, String> {
        match std::fs::read(linux_state::state_dir()?.join(BACKUP)) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| format!("Cannot read proxy recovery data: {e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn save(&self, backup: &Backup) -> Result<(), String> {
        linux_state::write_json(BACKUP, backup)
    }
    fn remove(&self) -> Result<(), String> {
        std::fs::remove_file(linux_state::state_dir()?.join(BACKUP)).map_err(|e| e.to_string())
    }
}

fn restore(settings: &impl Settings, recovery: &impl Recovery) -> Result<bool, String> {
    let Some(backup) = recovery.read()? else {
        return Ok(false);
    };
    if backup.original.len() != KEYS.len() || backup.applied.len() != KEYS.len() {
        return Err("Invalid proxy recovery data; backup retained".into());
    }
    let mut errors = Vec::new();
    for (i, (schema, key)) in KEYS.iter().enumerate() {
        match settings.get(schema, key) {
            // Preserve settings changed by the user or another application during this session.
            Ok(current) if current == backup.applied[i] && current != backup.original[i] => {
                if let Err(err) = settings.set(schema, key, &backup.original[i]) {
                    errors.push(err);
                }
            }
            Ok(_) => (),
            Err(err) => errors.push(err),
        }
    }
    if !errors.is_empty() {
        return Err(format!(
            "Proxy restore incomplete; recovery backup retained: {}",
            errors.join("; ")
        ));
    }
    recovery.remove()?;
    Ok(true)
}

fn apply(settings: &impl Settings, recovery: &impl Recovery, port: u16) -> Result<(), String> {
    restore(settings, recovery)?;
    let original = KEYS
        .iter()
        .map(|(s, k)| settings.get(s, k))
        .collect::<Result<Vec<_>, _>>()?;
    let applied = vec![
        "'127.0.0.1'".into(),
        port.to_string(),
        "'127.0.0.1'".into(),
        port.to_string(),
        "false".into(),
        "false".into(),
        "'manual'".into(),
    ];
    let backup = Backup { original, applied };
    // Persist the complete recovery journal BEFORE touching desktop settings.
    recovery.save(&backup)?;
    for (i, (schema, key)) in KEYS.iter().enumerate() {
        if let Err(err) = settings.set(schema, key, &backup.applied[i]) {
            return match restore(settings, recovery) {
                Ok(_) => Err(format!("Proxy setup failed and was rolled back: {err}")),
                Err(restore_err) => Err(format!("{err}; {restore_err}")),
            };
        }
    }
    Ok(())
}

fn supported_desktop(desktop: &str) -> bool {
    desktop.split(':').any(|part| {
        matches!(
            part.to_ascii_lowercase().as_str(),
            "gnome" | "unity" | "cinnamon" | "x-cinnamon"
        )
    })
}

pub fn name() -> &'static str {
    "linux"
}
pub fn should_bind_lan_for_game_mode(_enable: bool) -> bool {
    false
}

pub fn set_system_proxy(port: u16, _enable_winhttp: bool) -> Result<ProxySetupResult, String> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    if !supported_desktop(&desktop) {
        return Ok(manual_fallback(
            port,
            &format!(
                "desktop '{}' requires per-application proxy configuration",
                desktop
            ),
        ));
    }
    match apply(&GSettings, &Disk, port) {
        Ok(()) => Ok(ProxySetupResult {
            platform: name(),
            host: PROXY_HOST.into(),
            port,
            automatic: true,
            message: format!("Desktop HTTP/HTTPS proxy applied at {PROXY_HOST}:{port}"),
        }),
        Err(err) => {
            // An unsuccessful rollback is actionable: do not hide it behind manual fallback.
            if Disk.read()?.is_some() {
                return Err(err);
            }
            Ok(manual_fallback(port, &err))
        }
    }
}

pub fn clear_system_proxy() -> Result<(), String> {
    restore(&GSettings, &Disk).map(|_| ())
}
pub fn startup_cleanup() -> Result<bool, String> {
    restore(&GSettings, &Disk)
}

fn manual_fallback(port: u16, reason: &str) -> ProxySetupResult {
    ProxySetupResult {
        platform: name(),
        host: PROXY_HOST.into(),
        port,
        automatic: false,
        message: format!("Manual proxy ({reason}): set HTTP/HTTPS proxy to {PROXY_HOST}:{port}"),
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
    Err("Npcap driver installation is only supported on Windows.".into())
}
pub fn kill_zombie_sidecar() -> Result<String, String> {
    linux_state::stop_orphan()
}
pub fn cleanup_on_panic() {
    if linux_state::owns_instance() {
        let _ = clear_system_proxy();
        let _ = kill_zombie_sidecar();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    struct FakeSettings {
        values: RefCell<Vec<String>>,
        fail: RefCell<Option<usize>>,
    }
    impl FakeSettings {
        fn new() -> Self {
            Self {
                values: RefCell::new(vec![
                    "'original'".into(),
                    "3128".into(),
                    "'secure'".into(),
                    "3129".into(),
                    "true".into(),
                    "true".into(),
                    "'auto'".into(),
                ]),
                fail: RefCell::new(None),
            }
        }
        fn index(s: &str, k: &str) -> usize {
            KEYS.iter()
                .position(|&(schema, key)| schema == s && key == k)
                .unwrap()
        }
    }
    impl Settings for FakeSettings {
        fn get(&self, s: &str, k: &str) -> Result<String, String> {
            Ok(self.values.borrow()[Self::index(s, k)].clone())
        }
        fn set(&self, s: &str, k: &str, v: &str) -> Result<(), String> {
            let i = Self::index(s, k);
            if *self.fail.borrow() == Some(i) {
                return Err("locked setting".into());
            }
            self.values.borrow_mut()[i] = v.into();
            Ok(())
        }
    }
    #[derive(Default)]
    struct Memory(RefCell<Option<Backup>>);
    impl Recovery for Memory {
        fn read(&self) -> Result<Option<Backup>, String> {
            Ok(self.0.borrow().clone())
        }
        fn save(&self, b: &Backup) -> Result<(), String> {
            *self.0.borrow_mut() = Some(b.clone());
            Ok(())
        }
        fn remove(&self) -> Result<(), String> {
            *self.0.borrow_mut() = None;
            Ok(())
        }
    }
    #[test]
    fn no_backup_does_not_change_an_existing_proxy() {
        let settings = FakeSettings::new();
        let before = settings.values.borrow().clone();
        assert!(!restore(&settings, &Memory::default()).unwrap());
        assert_eq!(*settings.values.borrow(), before);
    }
    #[test]
    fn complete_round_trip_and_repeated_cleanup() {
        let settings = FakeSettings::new();
        let before = settings.values.borrow().clone();
        let recovery = Memory::default();
        apply(&settings, &recovery, 8080).unwrap();
        assert_eq!(settings.values.borrow()[6], "'manual'");
        assert!(restore(&settings, &recovery).unwrap());
        assert!(!restore(&settings, &recovery).unwrap());
        assert_eq!(*settings.values.borrow(), before);
    }
    #[test]
    fn partial_setup_rolls_back() {
        let settings = FakeSettings::new();
        let before = settings.values.borrow().clone();
        let recovery = Memory::default();
        *settings.fail.borrow_mut() = Some(3);
        assert!(apply(&settings, &recovery, 8080).is_err());
        assert_eq!(*settings.values.borrow(), before);
        assert!(recovery.read().unwrap().is_none());
    }
    #[test]
    fn failed_restore_retains_backup_for_retry() {
        let settings = FakeSettings::new();
        let before = settings.values.borrow().clone();
        let recovery = Memory::default();
        apply(&settings, &recovery, 8080).unwrap();
        *settings.fail.borrow_mut() = Some(0);
        assert!(restore(&settings, &recovery).is_err());
        assert!(recovery.read().unwrap().is_some());
        *settings.fail.borrow_mut() = None;
        restore(&settings, &recovery).unwrap();
        assert_eq!(*settings.values.borrow(), before);
    }
    #[test]
    fn external_changes_are_preserved() {
        let settings = FakeSettings::new();
        let recovery = Memory::default();
        apply(&settings, &recovery, 8080).unwrap();
        settings.values.borrow_mut()[0] = "'user-new-proxy'".into();
        settings.values.borrow_mut()[6] = "'none'".into();
        restore(&settings, &recovery).unwrap();
        assert_eq!(settings.values.borrow()[0], "'user-new-proxy'");
        assert_eq!(settings.values.borrow()[6], "'none'");
    }
    #[test]
    fn kde_is_not_reported_as_gnome() {
        assert!(!supported_desktop("KDE"));
        assert!(!supported_desktop(""));
        assert!(supported_desktop("ubuntu:GNOME"));
        assert!(supported_desktop("X-Cinnamon"));
    }
}
