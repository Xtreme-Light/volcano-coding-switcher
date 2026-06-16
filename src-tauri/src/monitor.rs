//! 后台轮询：定期获取限额，临近阈值时发送系统通知并按需自动切换 cc-switch 套餐。

use crate::ark::{ArkClient, QuotaProvider};
use crate::cc_switch_db::CcSwitchDb;
use crate::cc_switch_proc;
use crate::config::ArkAccount;
use crate::error::{AppError, AppResult};
use crate::state::{AppState, QuotaSnapshot};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_notification::NotificationExt;
use tokio::time::sleep;

pub fn spawn<R: Runtime>(app: AppHandle<R>, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let client = ArkClient::new();
        // 是否已就当前“逼近阈值”的状态发过通知，避免每轮重复弹。
        let mut warned = false;

        loop {
            let (account, threshold, interval, auto_switch, cc_db_path, restart_cc) = {
                let cfg = state.config.read().await;
                let account = match CcSwitchDb::open(&cfg.cc_switch_db_path)
                    .ok()
                    .and_then(|db| db.get_active_claude_provider().ok().flatten())
                {
                    Some(active) => cfg
                        .account_for_provider(&active.id)
                        .cloned()
                        .or_else(|| cfg.accounts.first().cloned()),
                    None => cfg.accounts.first().cloned(),
                };
                (
                    account,
                    cfg.threshold,
                    cfg.poll_interval_secs.max(30),
                    cfg.auto_switch,
                    cfg.cc_switch_db_path.clone(),
                    cfg.restart_cc_switch_after_switch,
                )
            };

            if let Some(account) = account {
                let action = account.effective_action();
                match client
                    .fetch_quota(&account.credentials, action, &account.api_version)
                    .await
                {
                    Ok(snapshot) => {
                        let ratio = snapshot.peak_ratio();
                        {
                            let mut last = state.last_quota.write().await;
                            *last = Some(snapshot.clone());
                        }
                        let _ = app.emit("quota-updated", &snapshot);
                        update_tray_tooltip(&app, &snapshot);
                        tracing::info!(
                            plan_type = snapshot.plan_type.as_str(),
                            peak_ratio = ratio,
                            "quota refreshed"
                        );

                        if ratio >= threshold {
                            if !warned {
                                warned = true;
                                notify_quota_alert(&app, &snapshot, threshold);
                            }

                            if auto_switch {
                                match auto_switch_to_lowest(&state, &cc_db_path).await {
                                    Ok(name) => {
                                        tracing::info!(plan = name.as_str(), "套餐切换成功");
                                        notify_plan_switched(&app, &name);
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
                                        tracing::error!("自动切换失败: {}", err);
                                        notify_switch_failed(&app, "下一个套餐", &err.to_string());
                                        let _ = app.emit("plan-switch-failed", err.to_string());
                                    }
                                }
                            }
                        } else {
                            // 用量回落到阈值以下后，允许下次重新提醒。
                            warned = false;
                        }
                    }
                    Err(err) => {
                        tracing::warn!("拉取额度失败: {}", err);
                        let _ = app.emit("quota-error", err.to_string());
                    }
                }
            }

            sleep(Duration::from_secs(interval)).await;
        }
    });
}

/// 在 cc-switch 中查询所有"已绑定方舟账号"的套餐用量，挑出
/// 不是当前激活、且"近5小时"使用率最低的那个，切换到它。
async fn auto_switch_to_lowest(state: &AppState, cc_db_path: &str) -> AppResult<String> {
    let cfg = state.config.read().await.clone();
    let db = CcSwitchDb::open(cc_db_path)?;
    let providers = db.list_claude_providers()?;
    let accounts: std::collections::HashMap<String, ArkAccount> = cfg
        .accounts
        .iter()
        .cloned()
        .map(|a| (a.id.clone(), a))
        .collect();
    let client = ArkClient::new();
    let mut candidates: Vec<(crate::cc_switch_db::CcProvider, f64)> = Vec::new();
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

fn update_tray_tooltip<R: Runtime>(app: &AppHandle<R>, snapshot: &QuotaSnapshot) {
    let ratio = snapshot.peak_ratio();
    let plan = if snapshot.plan_type.is_empty() {
        "未订阅".to_string()
    } else {
        snapshot.plan_type.clone()
    };
    let label = snapshot.peak_label().unwrap_or_else(|| "无".to_string());
    let tip = format!(
        "火山方舟 code_plan 切换器\n套餐: {}\n峰值周期: {} - {:.1}%",
        plan,
        label,
        ratio * 100.0
    );
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(tip));
        let icon = crate::tray_icon::make_tray_image(ratio as f32);
        let _ = tray.set_icon(Some(icon));
    }
}

fn notify_quota_alert<R: Runtime>(app: &AppHandle<R>, snapshot: &QuotaSnapshot, threshold: f64) {
    let label = snapshot.peak_label().unwrap_or_else(|| "无".to_string());
    let body = format!(
        "{} 周期已使用 {:.1}%（阈值 {:.0}%）",
        label,
        snapshot.peak_ratio() * 100.0,
        threshold * 100.0
    );
    if let Err(err) = app
        .notification()
        .builder()
        .title("方舟用量临近上限")
        .body(body)
        .show()
    {
        tracing::warn!("发送通知失败: {}", err);
    }
}

fn notify_plan_switched<R: Runtime>(app: &AppHandle<R>, plan: &str) {
    let _ = app
        .notification()
        .builder()
        .title("已自动切换套餐")
        .body(format!("Claude settings 已切换到 {}", plan))
        .show();
}

fn notify_switch_failed<R: Runtime>(app: &AppHandle<R>, plan: &str, err: &str) {
    let _ = app
        .notification()
        .builder()
        .title("自动切换失败")
        .body(format!("目标套餐 {} 切换失败：{}", plan, err))
        .show();
}
