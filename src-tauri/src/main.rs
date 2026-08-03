// AnyWords Desktop - Tauri wrapper

use std::sync::OnceLock;

static SERVER_PORT: OnceLock<u16> = OnceLock::new();

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Start the AnyWords server in background
            tauri::async_runtime::spawn(async {
                match anywords::run_server().await {
                    Ok(port) => {
                        eprintln!("[AnyWords] Server started on port {}", port);
                        SERVER_PORT.set(port).ok();
                    }
                    Err(e) => {
                        eprintln!("[AnyWords] Failed to start server: {}", e);
                    }
                }
            });

            // Give the server a moment to start
            std::thread::sleep(std::time::Duration::from_millis(800));

            let port = SERVER_PORT.get().copied().unwrap_or(9921);
            let url = format!("http://127.0.0.1:{}", port);

            let _window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External(url.parse().unwrap()),
            )
            .title("AnyWords")
            .inner_size(1200.0, 800.0)
            .min_inner_size(800.0, 600.0)
            .center()
            .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
