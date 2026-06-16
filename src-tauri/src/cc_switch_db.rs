//! 直接读写 cc-switch (farion1231/cc-switch) v3.2+ 的 SQLite 数据库
//! `~/.cc-switch/cc-switch.db`，用于：
//! 1. 列出所有 Claude provider（套餐）。
//! 2. 读取当前激活的 provider。
//! 3. 切换激活 provider：原子地把对应 settings_config.env 写入
//!    `~/.claude/settings.json`，并把 providers.is_current 翻到目标行。
//!
//! 设计决策：
//! - 我们绕开 cc-switch 的 GUI / IPC，因为它没有 CLI；但只要按它自己的格式
//!   写 db + settings.json，下次启动 cc-switch 时它能识别。
//! - 写 db 时只 UPDATE `is_current`，不动其他列；用事务保证 (清零 + 置一)
//!   不会留下半状态。

use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

const APP_TYPE_CLAUDE: &str = "claude";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcProvider {
    pub id: String,
    pub name: String,
    pub is_current: bool,
    pub base_url: String,
    pub auth_token: String,
    /// 原始 settings_config JSON 字符串，切换时整体写到 ~/.claude/settings.json。
    pub settings_config: String,
}

pub struct CcSwitchDb {
    pub path: PathBuf,
}

impl CcSwitchDb {
    /// 解析 cc-switch.db 路径；空字符串走默认 `~/.cc-switch/cc-switch.db`。
    pub fn resolve(custom: &str) -> AppResult<PathBuf> {
        if !custom.trim().is_empty() {
            return Ok(PathBuf::from(custom.trim()));
        }
        let home = directories::BaseDirs::new()
            .ok_or_else(|| AppError::Config("无法解析 HOME 目录".into()))?
            .home_dir()
            .to_path_buf();
        Ok(home.join(".cc-switch").join("cc-switch.db"))
    }

    pub fn open(custom: &str) -> AppResult<Self> {
        let path = Self::resolve(custom)?;
        if !path.exists() {
            return Err(AppError::CcSwitch(format!(
                "未找到 cc-switch 数据库：{}\n请先安装 cc-switch（GUI 套餐切换器）：\n  https://github.com/farion1231/cc-switch/releases\n安装后启动一次 cc-switch、添加好 Claude Provider，再回到本工具刷新。",
                path.display()
            )));
        }
        Ok(Self { path })
    }

    /// 探测：返回路径是否存在 + Claude provider 数量；不存在时不报错。
    pub fn detect(custom: &str) -> AppResult<DetectResult> {
        let path = Self::resolve(custom)?;
        if !path.exists() {
            return Ok(DetectResult {
                installed: false,
                path: path.display().to_string(),
                claude_provider_count: 0,
                active_provider: None,
            });
        }
        let db = Self { path: path.clone() };
        let providers = db.list_claude_providers().unwrap_or_default();
        let active = providers
            .iter()
            .find(|p| p.is_current)
            .map(|p| p.name.clone());
        Ok(DetectResult {
            installed: true,
            path: path.display().to_string(),
            claude_provider_count: providers.len(),
            active_provider: active,
        })
    }

    fn connect_ro(&self) -> AppResult<Connection> {
        Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| AppError::CcSwitch(format!("打开 cc-switch.db 失败: {}", e)))
    }

    fn connect_rw(&self) -> AppResult<Connection> {
        Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| AppError::CcSwitch(format!("以可写方式打开 cc-switch.db 失败: {}", e)))
    }

    pub fn list_claude_providers(&self) -> AppResult<Vec<CcProvider>> {
        let conn = self.connect_ro()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, is_current, settings_config FROM providers \
                 WHERE app_type = ?1 ORDER BY sort_index, name",
            )
            .map_err(|e| AppError::CcSwitch(format!("准备查询失败: {}", e)))?;
        let rows = stmt
            .query_map(params![APP_TYPE_CLAUDE], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let is_current: bool = row.get::<_, i64>(2).map(|v| v != 0)?;
                let cfg: String = row.get(3)?;
                Ok((id, name, is_current, cfg))
            })
            .map_err(|e| AppError::CcSwitch(format!("查询失败: {}", e)))?;
        let mut out = Vec::new();
        for r in rows {
            let (id, name, is_current, cfg) =
                r.map_err(|e| AppError::CcSwitch(format!("读取行失败: {}", e)))?;
            let (base_url, auth_token) = parse_base_url_and_token(&cfg);
            out.push(CcProvider {
                id,
                name,
                is_current,
                base_url,
                auth_token,
                settings_config: cfg,
            });
        }
        Ok(out)
    }

    pub fn get_active_claude_provider(&self) -> AppResult<Option<CcProvider>> {
        Ok(self.list_claude_providers()?.into_iter().find(|p| p.is_current))
    }

    /// 切换激活 provider：把目标行 is_current=1，其余清零；并把对应 env 同步到 ~/.claude/settings.json。
    pub fn activate_claude(&self, provider_id: &str) -> AppResult<CcProvider> {
        let mut conn = self.connect_rw()?;
        let tx = conn
            .transaction()
            .map_err(|e| AppError::CcSwitch(format!("启动事务失败: {}", e)))?;

        // 1. 取目标 provider 的 settings_config
        let (name, settings_config): (String, String) = tx
            .query_row(
                "SELECT name, settings_config FROM providers \
                 WHERE id = ?1 AND app_type = ?2",
                params![provider_id, APP_TYPE_CLAUDE],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| {
                AppError::CcSwitch(format!("找不到目标 provider id={}: {}", provider_id, e))
            })?;

        // 2. 清零其他行的 is_current；置一目标行
        tx.execute(
            "UPDATE providers SET is_current = 0 WHERE app_type = ?1",
            params![APP_TYPE_CLAUDE],
        )
        .map_err(|e| AppError::CcSwitch(format!("更新 is_current=0 失败: {}", e)))?;
        tx.execute(
            "UPDATE providers SET is_current = 1 WHERE id = ?1 AND app_type = ?2",
            params![provider_id, APP_TYPE_CLAUDE],
        )
        .map_err(|e| AppError::CcSwitch(format!("更新 is_current=1 失败: {}", e)))?;

        tx.commit()
            .map_err(|e| AppError::CcSwitch(format!("提交事务失败: {}", e)))?;

        // 3. 同步 ~/.claude/settings.json（覆盖整体或仅合并 env 字段）
        write_claude_settings(&settings_config)?;

        let (base_url, auth_token) = parse_base_url_and_token(&settings_config);
        Ok(CcProvider {
            id: provider_id.to_string(),
            name,
            is_current: true,
            base_url,
            auth_token,
            settings_config,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectResult {
    pub installed: bool,
    pub path: String,
    pub claude_provider_count: usize,
    pub active_provider: Option<String>,
}

fn parse_base_url_and_token(settings_config_json: &str) -> (String, String) {
    let v: Value = match serde_json::from_str(settings_config_json) {
        Ok(v) => v,
        Err(_) => return (String::new(), String::new()),
    };
    let env = v.get("env");
    let base_url = env
        .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let auth_token = env
        .and_then(|e| e.get("ANTHROPIC_AUTH_TOKEN"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    (base_url, auth_token)
}

/// 把 cc-switch 的 settings_config（整段 JSON）写到 `~/.claude/settings.json`。
/// 行为与 cc-switch 自身一致：整体覆盖，因为这一段就是它管理的"Claude 配置文件"原始内容。
fn write_claude_settings(settings_config_json: &str) -> AppResult<()> {
    // 校验是合法 JSON
    let parsed: Value = serde_json::from_str(settings_config_json)
        .map_err(|e| AppError::CcSwitch(format!("settings_config 不是合法 JSON: {}", e)))?;

    let path = claude_settings_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let pretty = serde_json::to_string_pretty(&parsed)?;
    let tmp = match path.parent() {
        Some(parent) => parent.join(format!(
            ".{}.tmp",
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("settings.json")
        )),
        None => path.with_extension("tmp"),
    };
    std::fs::write(&tmp, pretty.as_bytes())?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

fn claude_settings_path() -> AppResult<PathBuf> {
    let home = directories::BaseDirs::new()
        .ok_or_else(|| AppError::Config("无法解析 HOME 目录".into()))?
        .home_dir()
        .to_path_buf();
    Ok(home.join(".claude").join("settings.json"))
}

#[allow(dead_code)]
pub fn default_db_path_display() -> String {
    match CcSwitchDb::resolve("") {
        Ok(p) => p.display().to_string(),
        Err(_) => "~/.cc-switch/cc-switch.db".to_string(),
    }
}

#[allow(dead_code)]
pub fn db_exists(custom: &str) -> bool {
    CcSwitchDb::resolve(custom)
        .map(|p| Path::new(&p).exists())
        .unwrap_or(false)
}
