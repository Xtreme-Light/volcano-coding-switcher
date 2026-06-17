//! 读取 cc-switch provider 列表，并通过 cc-switch-cli 执行切换。
//!
//! provider 列表仍从兼容的 SQLite 数据库 `~/.cc-switch/cc-switch.db` 读取，
//! 但切换不再直接写数据库和 Claude settings，而是调用官方 CLI：
//! `cc-switch --app claude provider switch <id>`。

use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const APP_TYPE_CLAUDE: &str = "claude";
const CLI_BIN: &str = "cc-switch";
pub const CLI_INSTALL_URL: &str = "https://github.com/SaladDay/cc-switch-cli";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcProvider {
    pub id: String,
    pub name: String,
    pub is_current: bool,
    pub base_url: String,
    pub auth_token: String,
    /// 原始 settings_config JSON 字符串。
    pub settings_config: String,
}

pub struct CcSwitchCli {
    pub path: PathBuf,
}

impl CcSwitchCli {
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
                "未找到 cc-switch 数据库：{}\n请先安装并运行 cc-switch-cli，添加好 Claude Provider 后再回到本工具刷新。\n安装地址：{}",
                path.display(),
                CLI_INSTALL_URL
            )));
        }
        Ok(Self { path })
    }

    /// 探测：返回 CLI 是否可用、数据库路径是否存在、Claude provider 数量；不存在时不报错。
    pub fn detect(custom: &str) -> AppResult<DetectResult> {
        let cli = detect_cli();
        let path = Self::resolve(custom)?;
        if !path.exists() {
            return Ok(DetectResult {
                cli_installed: cli.installed,
                cli_error: cli.error,
                install_url: CLI_INSTALL_URL.to_string(),
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
            cli_installed: cli.installed,
            cli_error: cli.error,
            install_url: CLI_INSTALL_URL.to_string(),
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
        Ok(self
            .list_claude_providers()?
            .into_iter()
            .find(|p| p.is_current))
    }

    /// 通过 cc-switch-cli 切换激活 provider。
    pub fn activate_claude(&self, provider_id: &str) -> AppResult<CcProvider> {
        let provider = self
            .list_claude_providers()?
            .into_iter()
            .find(|p| p.id == provider_id)
            .ok_or_else(|| AppError::CcSwitch(format!("找不到目标 provider id={}", provider_id)))?;

        let output = Command::new(CLI_BIN)
            .args(["--app", APP_TYPE_CLAUDE, "provider", "switch", provider_id])
            .output()
            .map_err(|e| {
                AppError::CcSwitch(format!(
                    "无法执行 cc-switch-cli：{}。请确认已安装 cc-switch 并在 PATH 中。安装地址：{}",
                    e, CLI_INSTALL_URL
                ))
            })?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let detail = if stderr.is_empty() { stdout } else { stderr };
            return Err(AppError::CcSwitch(format!(
                "cc-switch-cli 切换失败（exit={}）：{}",
                output.status, detail
            )));
        }

        Ok(CcProvider {
            is_current: true,
            ..provider
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectResult {
    pub cli_installed: bool,
    pub cli_error: Option<String>,
    pub install_url: String,
    pub installed: bool,
    pub path: String,
    pub claude_provider_count: usize,
    pub active_provider: Option<String>,
}

struct CliDetect {
    installed: bool,
    error: Option<String>,
}

fn detect_cli() -> CliDetect {
    match Command::new(CLI_BIN).arg("--help").output() {
        Ok(output) if output.status.success() => CliDetect {
            installed: true,
            error: None,
        },
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let detail = if stderr.is_empty() { stdout } else { stderr };
            CliDetect {
                installed: false,
                error: Some(detail),
            }
        }
        Err(err) => CliDetect {
            installed: false,
            error: Some(err.to_string()),
        },
    }
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

#[allow(dead_code)]
pub fn default_db_path_display() -> String {
    match CcSwitchCli::resolve("") {
        Ok(p) => p.display().to_string(),
        Err(_) => "~/.cc-switch/cc-switch.db".to_string(),
    }
}

#[allow(dead_code)]
pub fn db_exists(custom: &str) -> bool {
    CcSwitchCli::resolve(custom)
        .map(|p| Path::new(&p).exists())
        .unwrap_or(false)
}
