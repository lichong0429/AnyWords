// AnyWords Desktop - Tauri wrapper

use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;
use tauri_plugin_global_shortcut::{
    Builder as GlobalShortcutBuilder, Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
};
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

/// Show and focus the main window
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Toggle the main window visibility (global hotkey / tray click)
fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false) {
            let _ = window.hide();
        } else {
            show_main_window(app);
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(GlobalShortcutBuilder::new().build())
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

            // ── Global hotkey: Ctrl+Shift+Space toggles quick search ──
            let quick_search =
                Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);
            app.global_shortcut().on_shortcut(
                quick_search,
                |app_handle, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_main_window(app_handle);
                    }
                },
            )?;

            // ── System tray ──────────────────────────────────────
            let tray_menu = MenuBuilder::new(app)
                .item(
                    &MenuItemBuilder::with_id("tray_show", "显示主窗口  Ctrl+Shift+Space")
                        .build(app)?,
                )
                .separator()
                .item(&MenuItemBuilder::with_id("tray_quit", "退出 AnyWords").build(app)?)
                .build()?;

            let mut tray_builder = TrayIconBuilder::with_id("main-tray")
                .menu(&tray_menu)
                .tooltip("AnyWords - 本地全文搜索")
                .on_menu_event(|app_handle, event| match event.id().0.as_str() {
                    "tray_show" => show_main_window(app_handle),
                    "tray_quit" => app_handle.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_main_window(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            tray_builder.build(app)?;

            // ── Native application menu ──────────────────────────
            let file_menu = SubmenuBuilder::new(app, "文件(&F)")
                .item(
                    &MenuItemBuilder::with_id("quick_search", "快速搜索")
                        .build(app)?,
                )
                .separator()
                .item(
                    &MenuItemBuilder::with_id("open_config_dir", "打开配置目录")
                        .build(app)?,
                )
                .item(
                    &MenuItemBuilder::with_id("open_index_dir", "打开索引目录")
                        .build(app)?,
                )
                .separator()
                .item(
                    &MenuItemBuilder::with_id("hide_to_tray", "最小化到托盘")
                        .build(app)?,
                )
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
                    "quick_search" => show_main_window(app_handle),
                    "hide_to_tray" => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
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
                                "alert('AnyWords v0.1.0\\n本地文件全文搜索引擎\\n全局热键: Ctrl+Shift+Space\\nhttps://github.com/lichong0429/AnyWords')",
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
        // Close button hides the window to the tray instead of quitting,
        // so the global hotkey keeps working. Use 文件 > 退出 (or the tray
        // menu) to really exit.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
