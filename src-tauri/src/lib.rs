//! 库入口：构建 Tauri 应用、注册插件、托盘与命令。

mod ark;
mod cc_switch_db;
mod cc_switch_proc;
mod commands;
mod config;
mod error;
mod monitor;
mod sign;
mod state;
mod tray;
mod tray_icon;

/// 仅供 examples / 集成测试使用的辅助 re-export。
#[doc(hidden)]
pub mod __test_support {
    pub use crate::ark::{ArkClient, QuotaProvider};
    pub use crate::config::ArkCredentials;
}

use state::AppState;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = config::AppConfig::load().unwrap_or_default();
    let state = AppState {
        config: Arc::new(RwLock::new(config)),
        last_quota: Arc::new(RwLock::new(None)),
    };

    let monitor_state = state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .on_window_event(|window, event| {
            // 关闭窗口时隐藏到托盘，而不是退出。
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(move |app| {
            // 构建系统托盘（v2 运行时 API）。
            tray::setup(app.handle())?;

            // 启动后台轮询，临近阈值时触发通知 / cc-switch 切换。
            monitor::spawn(app.handle().clone(), monitor_state.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::fetch_quota,
            commands::fetch_quota_by_account,
            commands::fetch_all_quotas,
            commands::switch_plan,
            commands::list_plans,
            commands::detect_cc_switch,
            commands::list_cc_providers,
            commands::get_active_cc_provider,
            commands::list_accounts,
            commands::upsert_account,
            commands::delete_account,
            commands::list_bindings,
            commands::bind_provider,
            commands::unbind_provider,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
