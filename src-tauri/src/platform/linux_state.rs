//! User-owned recovery files. Never use a shared /tmp PID or proxy backup.
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static OWNS_INSTANCE: AtomicBool = AtomicBool::new(false);

pub fn state_dir() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state")))
        .ok_or("No user state directory is available")?;
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let dir = base.join("bypaxdpi");
    match DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e.to_string()),
    }
    let meta = fs::symlink_metadata(&dir).map_err(|e| e.to_string())?;
    if !meta.is_dir() || meta.uid() != unsafe { libc::getuid() } || meta.mode() & 0o077 != 0 {
        return Err(format!(
            "Recovery directory must be owned by this user with mode 0700: {}",
            dir.display()
        ));
    }
    Ok(dir)
}

pub fn write_json<T: serde::Serialize>(name: &str, value: &T) -> Result<(), String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = state_dir()?;
    let temp = dir.join(format!(
        ".{name}.{}.{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        let data = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        file.write_all(&data)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        fs::rename(&temp, dir.join(name)).map_err(|e| e.to_string())?;
        File::open(&dir)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())
    })();
    let _ = fs::remove_file(temp);
    result
}

pub fn lock_instance() -> Result<File, String> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(state_dir()?.join("instance.lock"))
        .map_err(|e| e.to_string())?;
    file.try_lock().map_err(|e| {
        format!("BypaxDPI is already running or its instance lock is unavailable: {e}")
    })?;
    OWNS_INSTANCE.store(true, Ordering::SeqCst);
    Ok(file)
}

pub fn owns_instance() -> bool {
    OWNS_INSTANCE.load(Ordering::SeqCst)
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct ProcessIdentity {
    pid: u32,
    start_time: String,
    executable: PathBuf,
    boot_id: String,
}

fn proc_fields(stat: &str) -> Result<(u32, String), String> {
    // comm can contain spaces and parentheses. Fields after the final ')' start at state (3).
    let fields: Vec<_> = stat
        .rsplit_once(')')
        .ok_or("Invalid process stat")?
        .1
        .split_whitespace()
        .collect();
    let parent = fields
        .get(1)
        .ok_or("Missing parent PID")?
        .parse::<u32>()
        .map_err(|e| e.to_string())?;
    let start = fields
        .get(19)
        .ok_or("Missing process start time")?
        .to_string();
    Ok((parent, start))
}

fn identity(pid: u32) -> Result<(u32, ProcessIdentity), String> {
    if pid <= 1 {
        return Err("Invalid sidecar PID".into());
    }
    let (parent, start_time) =
        proc_fields(&fs::read_to_string(format!("/proc/{pid}/stat")).map_err(|e| e.to_string())?)?;
    Ok((
        parent,
        ProcessIdentity {
            pid,
            start_time,
            executable: fs::read_link(format!("/proc/{pid}/exe")).map_err(|e| e.to_string())?,
            boot_id: fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .map_err(|e| e.to_string())?,
        },
    ))
}

pub fn save_pid(pid: u32) -> Result<(), String> {
    let (parent, record) = identity(pid)?;
    let name = record
        .executable
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if parent != std::process::id() || !(name == "bypax-proxy" || name.starts_with("bypax-proxy-"))
    {
        return Err("Refusing to record a process that is not this app's sidecar".into());
    }
    write_json("sidecar.json", &record)
}

pub fn stop_orphan() -> Result<String, String> {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    let path = state_dir()?.join("sidecar.json");
    let data = match fs::read(&path) {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok("No orphaned sidecar".into())
        }
        Err(e) => return Err(e.to_string()),
    };
    let saved: ProcessIdentity = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
    // A pidfd pins the process, avoiding PID reuse between verification and signal delivery.
    let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, saved.pid, 0) } as i32;
    if raw < 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() != Some(libc::ESRCH) {
            return Err(err.to_string());
        }
    } else {
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        if identity(saved.pid)
            .map(|(_, actual)| actual == saved)
            .unwrap_or(false)
        {
            let result = unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    fd.as_raw_fd(),
                    libc::SIGTERM,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                )
            };
            if result < 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
    }
    fs::remove_file(path).map_err(|e| e.to_string())?;
    Ok("Sidecar recovery completed (process identity checked)".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_process_name_with_spaces_and_parentheses() {
        let stat = format!(
            "42 (proxy (test)) S 123 {} 98765 0",
            vec!["0"; 17].join(" ")
        );
        assert_eq!(proc_fields(&stat).unwrap(), (123, "98765".into()));
        assert!(proc_fields("invalid").is_err());
    }
    #[test]
    fn will_not_record_the_app_as_its_own_sidecar() {
        assert!(save_pid(std::process::id()).is_err());
        assert!(identity(0).is_err());
    }
}
