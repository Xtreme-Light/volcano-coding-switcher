//! 暴露给前端调用的 Tauri 命令。

use crate::ark::{ArkClient, QuotaProvider};
use crate::cc_switch_cli::{CcProvider, CcSwitchCli, DetectResult};
use crate::cc_switch_proc;
use crate::config::{AppConfig, ArkAccount, ArkCredentials};
use crate::error::{AppError, AppResult};
use crate::state::{AppState, QuotaSnapshot};
use serde::{Deserialize, Serialize};
use tauri::State;

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> AppResult<AppConfig> {
    let cfg = state.config.read().await;
    Ok(cfg.clone())
}

#[tauri::command]
pub async fn save_config(state: State<'_, AppState>, config: AppConfig) -> AppResult<()> {
    config.save()?;
    let mut guard = state.config.write().await;
    *guard = config;
    Ok(())
}

// ---- 方舟账号管理 ----

#[tauri::command]
pub async fn list_accounts(state: State<'_, AppState>) -> AppResult<Vec<ArkAccount>> {
    let cfg = state.config.read().await;
    Ok(cfg.accounts.clone())
}

/// 新增或更新一个账号；id 为空时新建。
#[tauri::command]
pub async fn upsert_account(
    state: State<'_, AppState>,
    account: ArkAccountInput,
) -> AppResult<ArkAccount> {
    let mut cfg = state.config.write().await;
    let creds = ArkCredentials {
        api_key: account.api_key.unwrap_or_default(),
        access_key_id: account.access_key_id,
        access_key_secret: account.access_key_secret,
        region: if account.region.trim().is_empty() {
            "cn-beijing".to_string()
        } else {
            account.region
        },
    };
    let api_version = if account.api_version.trim().is_empty() {
        "2024-01-01".to_string()
    } else {
        account.api_version
    };
    let stored = if let Some(id) = account.id.filter(|s| !s.is_empty()) {
        match cfg.accounts.iter_mut().find(|a| a.id == id) {
            Some(a) => {
                a.name = account.name;
                a.credentials = creds;
                a.use_coding_plan = account.use_coding_plan;
                a.api_version = api_version;
                a.clone()
            }
            None => {
                let acc = ArkAccount {
                    id,
                    name: account.name,
                    credentials: creds,
                    use_coding_plan: account.use_coding_plan,
                    api_version,
                };
                cfg.accounts.push(acc.clone());
                acc
            }
        }
    } else {
        let acc = ArkAccount {
            id: format!("acc-{}", chrono::Utc::now().timestamp_millis()),
            name: account.name,
            credentials: creds,
            use_coding_plan: account.use_coding_plan,
            api_version,
        };
        cfg.accounts.push(acc.clone());
        acc
    };
    cfg.save()?;
    Ok(stored)
}

#[tauri::command]
pub async fn delete_account(state: State<'_, AppState>, id: String) -> AppResult<()> {
    let mut cfg = state.config.write().await;
    cfg.accounts.retain(|a| a.id != id);
    // 同步清除指向该账号的所有绑定
    cfg.bindings.retain(|_, v| v != &id);
    cfg.save()?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ArkAccountInput {
    pub id: Option<String>,
    pub name: String,
    pub access_key_id: String,
    pub access_key_secret: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_true")]
    pub use_coding_plan: bool,
    #[serde(default)]
    pub api_version: String,
}

fn default_true() -> bool {
    true
}

// ---- 绑定 ----

#[derive(Debug, Serialize)]
pub struct BindingView {
    pub provider_id: String,
    pub provider_name: String,
    pub is_current: bool,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub region: Option<String>,
}

/// 列出 cc-switch 中所有 Claude 套餐 + 它们的当前绑定情况。
#[tauri::command]
pub async fn list_bindings(state: State<'_, AppState>) -> AppResult<Vec<BindingView>> {
    let cfg = state.config.read().await;
    let db = CcSwitchCli::open(&cfg.cc_switch_db_path)?;
    let providers = db.list_claude_providers()?;
    let mut out = Vec::with_capacity(providers.len());
    for p in providers {
        let acc = cfg.account_for_provider(&p.id);
        out.push(BindingView {
            provider_id: p.id.clone(),
            provider_name: p.name.clone(),
            is_current: p.is_current,
            account_id: acc.map(|a| a.id.clone()),
            account_name: acc.map(|a| a.name.clone()),
            region: acc.map(|a| a.credentials.region.clone()),
        });
    }
    Ok(out)
}

#[derive(Debug, Deserialize)]
pub struct BindRequest {
    pub provider_id: String,
    pub account_id: String,
    /// 当 provider 已经绑定别的账号时，是否强制覆盖。
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Serialize)]
pub struct BindResult {
    pub conflict: bool,
    pub previous_account_id: Option<String>,
    pub previous_account_name: Option<String>,
    pub bound: bool,
}

#[tauri::command]
pub async fn bind_provider(
    state: State<'_, AppState>,
    request: BindRequest,
) -> AppResult<BindResult> {
    let mut cfg = state.config.write().await;
    if !cfg.accounts.iter().any(|a| a.id == request.account_id) {
        return Err(AppError::Config(format!(
            "找不到账号 id={}",
            request.account_id
        )));
    }
    let existing = cfg.bindings.get(&request.provider_id).cloned();
    if let Some(prev) = existing.clone() {
        if prev != request.account_id && !request.overwrite {
            let prev_name = cfg
                .accounts
                .iter()
                .find(|a| a.id == prev)
                .map(|a| a.name.clone());
            return Ok(BindResult {
                conflict: true,
                previous_account_id: Some(prev),
                previous_account_name: prev_name,
                bound: false,
            });
        }
    }
    cfg.bindings
        .insert(request.provider_id, request.account_id);
    cfg.save()?;
    Ok(BindResult {
        conflict: false,
        previous_account_id: existing,
        previous_account_name: None,
        bound: true,
    })
}

#[tauri::command]
pub async fn unbind_provider(
    state: State<'_, AppState>,
    provider_id: String,
) -> AppResult<()> {
    let mut cfg = state.config.write().await;
    cfg.bindings.remove(&provider_id);
    cfg.save()?;
    Ok(())
}

// ---- 用量 ----

/// 仅查询"当前激活的 cc-switch 套餐"绑定账号的用量。
#[tauri::command]
pub async fn fetch_quota(state: State<'_, AppState>) -> AppResult<QuotaSnapshot> {
    let (db_path, account) = {
        let cfg = state.config.read().await;
        let db_path = cfg.cc_switch_db_path.clone();
        let account = match CcSwitchCli::open(&db_path)
            .ok()
            .and_then(|db| db.get_active_claude_provider().ok().flatten())
        {
            Some(active) => cfg
                .account_for_provider(&active.id)
                .cloned()
                .or_else(|| cfg.accounts.first().cloned()),
            None => cfg.accounts.first().cloned(),
        };
        (db_path, account)
    };
    let _ = db_path;
    let account = account.unwrap_or_else(|| ArkAccount {
        id: String::new(),
        name: "未配置".into(),
        credentials: Default::default(),
        use_coding_plan: true,
        api_version: "2024-01-01".into(),
    });
    let client = ArkClient::new();
    let snapshot = client
        .fetch_quota(&account.credentials, account.effective_action(), &account.api_version)
        .await?;
    let mut last = state.last_quota.write().await;
    *last = Some(snapshot.clone());
    Ok(snapshot)
}

/// 按指定账号 ID 查询用量（用于用量区域切换账号显示）。
#[tauri::command]
pub async fn fetch_quota_by_account(
    state: State<'_, AppState>,
    account_id: String,
) -> AppResult<QuotaSnapshot> {
    let account = {
        let cfg = state.config.read().await;
        cfg.accounts
            .iter()
            .find(|a| a.id == account_id)
            .cloned()
            .ok_or_else(|| AppError::Config(format!("找不到账号 id={}", account_id)))?
    };
    let client = ArkClient::new();
    let snapshot = client
        .fetch_quota(&account.credentials, account.effective_action(), &account.api_version)
        .await?;
    Ok(snapshot)
}

#[derive(Debug, Serialize)]
pub struct ProviderQuota {
    pub provider_id: String,
    pub provider_name: String,
    pub is_current: bool,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    /// 0~1 范围。"近5小时"周期（session/FiveHour）的使用率，
    /// 用于套餐列表展示和"选最低用量套餐"切换策略。
    pub short_term_ratio: f64,
    /// 出错时填错误信息（snapshot 仍可能为 None）。
    pub error: Option<String>,
    pub snapshot: Option<QuotaSnapshot>,
}

/// 查询所有"已绑定到方舟账号"的 cc-switch 套餐用量；
/// 用于：1) 主页一览；2) 自动 / 手动选择"用量最低"的下一个套餐切换。
#[tauri::command]
pub async fn fetch_all_quotas(state: State<'_, AppState>) -> AppResult<Vec<ProviderQuota>> {
    let (providers, accounts_index, bindings) = {
        let cfg = state.config.read().await;
        let db = CcSwitchCli::open(&cfg.cc_switch_db_path)?;
        let providers = db.list_claude_providers()?;
        let accounts: std::collections::HashMap<String, ArkAccount> = cfg
            .accounts
            .iter()
            .cloned()
            .map(|a| (a.id.clone(), a))
            .collect();
        (providers, accounts, cfg.bindings.clone())
    };

    let client = ArkClient::new();
    let mut out = Vec::with_capacity(providers.len());
    for p in providers {
        let acc_id = bindings.get(&p.id).cloned();
        let account = acc_id.as_ref().and_then(|id| accounts_index.get(id));
        let mut row = ProviderQuota {
            provider_id: p.id.clone(),
            provider_name: p.name.clone(),
            is_current: p.is_current,
            account_id: acc_id.clone(),
            account_name: account.map(|a| a.name.clone()),
            short_term_ratio: 1.0, // 未绑定/出错的视为已满，避免被自动选中
            error: None,
            snapshot: None,
        };
        let Some(account) = account else {
            row.error = Some("未绑定方舟账号".into());
            out.push(row);
            continue;
        };
        match client
            .fetch_quota(
                &account.credentials,
                account.effective_action(),
                &account.api_version,
            )
            .await
        {
            Ok(snap) => {
                row.short_term_ratio = snap.short_term_ratio();
                row.snapshot = Some(snap);
            }
            Err(e) => {
                row.error = Some(e.to_string());
            }
        }
        out.push(row);
    }
    Ok(out)
}

// ---- cc-switch 状态/列表/切换 ----

#[tauri::command]
pub async fn detect_cc_switch(state: State<'_, AppState>) -> AppResult<DetectResult> {
    let path = {
        let cfg = state.config.read().await;
        cfg.cc_switch_db_path.clone()
    };
    CcSwitchCli::detect(&path)
}

#[tauri::command]
pub async fn list_cc_providers(state: State<'_, AppState>) -> AppResult<Vec<CcProvider>> {
    let path = {
        let cfg = state.config.read().await;
        cfg.cc_switch_db_path.clone()
    };
    let db = CcSwitchCli::open(&path)?;
    db.list_claude_providers()
}

#[tauri::command]
pub async fn get_active_cc_provider(state: State<'_, AppState>) -> AppResult<Option<CcProvider>> {
    let path = {
        let cfg = state.config.read().await;
        cfg.cc_switch_db_path.clone()
    };
    let db = CcSwitchCli::open(&path)?;
    db.get_active_claude_provider()
}

#[tauri::command]
pub async fn switch_plan(state: State<'_, AppState>, plan: String) -> AppResult<String> {
    let (path, restart) = {
        let cfg = state.config.read().await;
        (
            cfg.cc_switch_db_path.clone(),
            cfg.restart_cc_switch_after_switch,
        )
    };
    let db = CcSwitchCli::open(&path)?;
    let provider = db.activate_claude(&plan)?;

    {
        let mut cfg = state.config.write().await;
        cfg.current_plan = provider.name.clone();
        cfg.save()?;
    }

    let mut msg = format!(
        "已切换到 {}（{}）；cc-switch.db 与 ~/.claude/settings.json 已同步",
        provider.name, provider.base_url
    );
    if restart {
        let outcome =
            tauri::async_runtime::spawn_blocking(cc_switch_proc::restart_cc_switch_if_running)
                .await
                .map_err(|e| AppError::CcSwitch(format!("重启任务失败: {}", e)))??;
        msg.push_str("；");
        msg.push_str(&outcome.message);
    }
    Ok(msg)
}

/// 兼容旧前端：返回 provider 名称列表。
#[tauri::command]
pub async fn list_plans(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    let path = {
        let cfg = state.config.read().await;
        cfg.cc_switch_db_path.clone()
    };
    match CcSwitchCli::open(&path) {
        Ok(db) => Ok(db
            .list_claude_providers()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.name)
            .collect()),
        Err(_) => Ok(Vec::new()),
    }
}

// ---- 快速配置：一键创建 cc-switch provider + 方舟账号 + 绑定 ----

#[derive(Debug, Deserialize)]
pub struct SetupArkProviderInput {
    /// cc-switch provider 显示名。
    pub provider_name: String,
    /// 火山方舟 Coding Plan 专属 API Key（用于 Claude Code 鉴权）。
    pub api_key: String,
    /// 模型名，例如 glm-5.2。
    #[serde(default = "default_model")]
    pub model: String,
    /// 方舟账号名（用于用量查询）。
    pub account_name: String,
    /// AccessKey ID（用于签名查询用量）。
    pub access_key_id: String,
    /// AccessKey Secret。
    pub access_key_secret: String,
    /// 区域，默认 cn-beijing。
    #[serde(default = "default_region_str")]
    pub region: String,
}

fn default_model() -> String {
    "glm-5.2".to_string()
}

fn default_region_str() -> String {
    "cn-beijing".to_string()
}

#[derive(Debug, Serialize)]
pub struct SetupResult {
    pub provider: CcProvider,
    pub account_id: String,
    pub bound: bool,
}

/// 一键配置：在 cc-switch 中创建火山方舟 Coding Plan provider，
/// 同时创建方舟账号（AK/SK 用于用量查询）并自动绑定。
#[tauri::command]
pub async fn setup_ark_provider(
    state: State<'_, AppState>,
    input: SetupArkProviderInput,
) -> AppResult<SetupResult> {
    if input.api_key.trim().is_empty() {
        return Err(AppError::Config("API Key 不能为空".into()));
    }
    if input.access_key_id.trim().is_empty() || input.access_key_secret.trim().is_empty() {
        return Err(AppError::Config("AK / SK 不能为空".into()));
    }

    let db_path = {
        let cfg = state.config.read().await;
        cfg.cc_switch_db_path.clone()
    };
    let db = CcSwitchCli::open(&db_path)?;
    let provider = db.add_claude_provider(
        &input.provider_name,
        &input.api_key,
        &input.model,
    )?;

    let mut cfg = state.config.write().await;
    let account_id = format!("acc-{}", chrono::Utc::now().timestamp_millis());
    let account = ArkAccount {
        id: account_id.clone(),
        name: input.account_name,
        credentials: ArkCredentials {
            api_key: input.api_key,
            access_key_id: input.access_key_id,
            access_key_secret: input.access_key_secret,
            region: if input.region.trim().is_empty() {
                "cn-beijing".to_string()
            } else {
                input.region
            },
        },
        use_coding_plan: true,
        api_version: "2024-01-01".to_string(),
    };
    cfg.accounts.push(account);
    cfg.bindings.insert(provider.id.clone(), account_id.clone());
    cfg.save()?;

    Ok(SetupResult {
        provider,
        account_id,
        bound: true,
    })
}
