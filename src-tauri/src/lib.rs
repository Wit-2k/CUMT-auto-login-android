mod commands;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri::menu::{MenuBuilder, MenuItemBuilder};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri::tray::TrayIconBuilder;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::auth::perform_login,
            commands::network::check_online,
            commands::config::save_config,
            commands::config::load_config,
        ]);

    // 仅在桌面端启用系统托盘
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let builder = builder.setup(|app| {
        let show_item = MenuItemBuilder::with_id("show", "显示窗口").build(app)?;
        let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;
        let menu = MenuBuilder::new(app)
            .item(&show_item)
            .separator()
            .item(&quit_item)
            .build()?;

        let mut tray_builder = TrayIconBuilder::new()
            .menu(&menu)
            .on_menu_event(|app, event| match event.id().as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            });

        if let Some(icon) = app.default_window_icon() {
            tray_builder = tray_builder.icon(icon.clone());
        }

        tray_builder.build(app)?;

        Ok(())
    });

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
