// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    // ✅ Sorun 1: Panic handler — uygulama çökerse proxy'yi temizle
    // Bu sayede kullanıcı internet erişimini kaybetmez
    std::panic::set_hook(Box::new(|panic_info| {
        // Proxy'yi temizlemeye çalış (best-effort)
        bypax_tauri_lib::cleanup_on_panic();

        eprintln!("BypaxDPI PANIC: {}", panic_info);
    }));

    bypax_tauri_lib::run()
}
