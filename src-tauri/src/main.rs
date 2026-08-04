// AnyWords Desktop - Tauri wrapper

use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

/// Open a directory in the system file manager
fn open_in_file_manager(path: &std::path::Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(path.as_os_str()).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path.as_os_str()).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path.as_os_str()).spawn();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Start the AnyWords server synchronously so the window always
            // points at the real port (no sleep-and-guess race).
            let port = match tauri::async_runtime::block_on(anywords::run_server()) {
                Ok(port) => {
                    eprintln!("[AnyWords] Server started on port {}", port);
                    port
                }
                Err(e) => {
                    eprintln!("[AnyWords] Failed to start server: {}", e);
                    9921
                }
            };

            // ── Native application menu ──────────────────────────
            let file_menu = SubmenuBuilder::new(app, "文件(&F)")
                .item(
                    &MenuItemBuilder::with_id("open_config_dir", "打开配置目录")
                        .build(app)?,
                )
                .item(
                    &MenuItemBuilder::with_id("open_index_dir", "打开索引目录")
                        .build(app)?,
                )
                .separator()
                .quit_with_text("退出 AnyWords")
                .build()?;

            let view_menu = SubmenuBuilder::new(app, "视图(&V)")
                .item(
                    &MenuItemBuilder::with_id("reload", "重新加载")
                        .accelerator("CmdOrControl+R")
                        .build(app)?,
                )
                .fullscreen_with_text("切换全屏")
                .separator()
                .item(
                    &MenuItemBuilder::with_id("devtools", "开发者工具")
                        .accelerator("F12")
                        .build(app)?,
                )
                .build()?;

            let help_menu = SubmenuBuilder::new(app, "帮助(&H)")
                .item(
                    &MenuItemBuilder::with_id("docs", "项目主页 (GitHub)")
                        .build(app)?,
                )
                .item(
                    &MenuItemBuilder::with_id("about", "关于 AnyWords")
                        .build(app)?,
                )
                .build()?;

            let menu = MenuBuilder::new(app)
                .items(&[&file_menu, &view_menu, &help_menu])
                .build()?;
            app.set_menu(menu)?;

            app.on_menu_event(|app_handle, event| {
                match event.id().0.as_str() {
                    "reload" => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.eval("location.reload()");
                        }
                    }
                    "devtools" => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            window.open_devtools();
                        }
                    }
                    "open_config_dir" => {
                        // anywords.yml lives in the working directory
                        if let Ok(cwd) = std::env::current_dir() {
                            open_in_file_manager(&cwd);
                        }
                    }
                    "open_index_dir" => {
                        if let Ok(cfg) = anywords::config::Config::load(None) {
                            open_in_file_manager(&cfg.index.dir);
                        }
                    }
                    "docs" => {
                        let _ = app_handle
                            .shell()
                            .open("https://github.com/lichong0429/AnyWords", None::<&str>);
                    }
                    "about" => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.eval(
                                "alert('AnyWords v0.1.0\\n本地文件全文搜索引擎\\nhttps://github.com/lichong0429/AnyWords')",
                            );
                        }
                    }
                    _ => {}
                }
            });

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
