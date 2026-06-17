//! 系统托盘（Tauri v2 TrayIcon API）。

use crate::ark::{ArkClient, QuotaProvider};
use crate::cc_switch_cli::CcSwitchCli;
use crate::cc_switch_proc;
use crate::config::ArkAccount;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

const ID_SHOW: &str = "show";
const ID_REFRESH: &str = "refresh";
const ID_SWITCH_NEXT: &str = "switch_next";
const ID_QUIT: &str = "quit";

pub fn setup<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, ID_SHOW, "显示主界面", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, ID_REFRESH, "立即刷新用量", true, None::<&str>)?;
    let switch_next = MenuItem::with_id(
        app,
        ID_SWITCH_NEXT,
        "切换到最低用量套餐",
        true,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, ID_QUIT, "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show, &refresh, &switch_next, &sep, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?;

    TrayIconBuilder::with_id("main-tray")
        .tooltip("火山方舟 code_plan 切换器")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            ID_SHOW => show_main_window(app),
            ID_REFRESH => trigger_refresh(app.clone()),
            ID_SWITCH_NEXT => trigger_switch_next(app.clone()),
            ID_QUIT => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

fn trigger_refresh<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        // 用量接口跟着 cc-switch 当前激活套餐绑定的方舟账号走；
        // 没有绑定则回退到 accounts[0]，再没有就直接报错。
        let account = {
            let cfg = state.config.read().await;
            match CcSwitchCli::open(&cfg.cc_switch_db_path)
                .ok()
                .and_then(|db| db.get_active_claude_provider().ok().flatten())
            {
                Some(active) => cfg
                    .account_for_provider(&active.id)
                    .cloned()
                    .or_else(|| cfg.accounts.first().cloned()),
                None => cfg.accounts.first().cloned(),
            }
        };
        let Some(account) = account else {
            let _ = app.emit(
                "quota-error",
                "未配置方舟账号，无法刷新用量".to_string(),
            );
            return;
        };
        let action = account.effective_action();
        let client = ArkClient::new();
        match client
            .fetch_quota(&account.credentials, action, &account.api_version)
            .await
        {
            Ok(snapshot) => {
                {
                    let mut last = state.last_quota.write().await;
                    *last = Some(snapshot.clone());
                }
                let _ = app.emit("quota-updated", &snapshot);
            }
            Err(err) => {
                let _ = app.emit("quota-error", err.to_string());
            }
        }
    });
}

fn trigger_switch_next<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let (cc_db_path, restart_cc) = {
            let cfg = state.config.read().await;
            (
                cfg.cc_switch_db_path.clone(),
                cfg.restart_cc_switch_after_switch,
            )
        };
        // 与 monitor.rs::auto_switch_to_lowest 同一套逻辑：
        // 遍历所有"已绑定方舟账号"的 cc-switch 套餐，挑出不是当前激活、
        // 且 peak_ratio 最低的那个切换过去。
        let result: AppResult<String> = async {
            let cfg = state.config.read().await.clone();
            let db = CcSwitchCli::open(&cc_db_path)?;
            let providers = db.list_claude_providers()?;
            let accounts: std::collections::HashMap<String, ArkAccount> = cfg
                .accounts
                .iter()
                .cloned()
                .map(|a| (a.id.clone(), a))
                .collect();
            let client = ArkClient::new();
            let mut candidates: Vec<(crate::cc_switch_cli::CcProvider, f64)> = Vec::new();
            for p in providers {
                if p.is_current {
                    continue;
                }
                let acc_id = match cfg.bindings.get(&p.id) {
                    Some(v) => v,
                    None => continue,
                };
                let acc = match accounts.get(acc_id) {
                    Some(v) => v,
                    None => continue,
                };
                match client
                    .fetch_quota(&acc.credentials, acc.effective_action(), &acc.api_version)
                    .await
                {
                    Ok(snap) => {
                        candidates.push((p, snap.short_term_ratio()));
                    }
                    Err(e) => {
                        tracing::warn!("查询 {} 用量失败: {}", p.name, e);
                    }
                }
            }
            let target = candidates
                .into_iter()
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(p, _)| p)
                .ok_or_else(|| {
                    AppError::CcSwitch(
                        "没有可切换的候选套餐（其它套餐都未绑定方舟账号或查询全部失败）".into(),
                    )
                })?;
            let activated = db.activate_claude(&target.id)?;
            Ok(activated.name)
        }
        .await;
        match result {
            Ok(name) => {
                {
                    let mut cfg = state.config.write().await;
                    cfg.current_plan = name.clone();
                    let _ = cfg.save();
                }
                if restart_cc {
                    let _ = tauri::async_runtime::spawn_blocking(
                        cc_switch_proc::restart_cc_switch_if_running,
                    )
                    .await;
                }
                let _ = app.emit(
                    "plan-switched",
                    serde_json::json!({ "plan": name }),
                );
            }
            Err(err) => {
                let _ = app.emit("plan-switch-failed", err.to_string());
            }
        }
    });
}
