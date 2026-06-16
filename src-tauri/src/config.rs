use crate::error::{AppError, AppResult};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const QUALIFIER: &str = "com";
const ORG: &str = "volcano";
const APP: &str = "coding-switcher";

/// 单组方舟 AK/SK；新版本里用作 ArkAccount 的内层数据。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArkCredentials {
    /// 火山方舟 API Key（保留字段，部分场景可使用 Bearer 鉴权；本工具默认走 AK/SK）。
    #[serde(default)]
    pub api_key: String,
    /// AccessKey ID（推荐填写）。
    #[serde(default)]
    pub access_key_id: String,
    /// AccessKey Secret（推荐填写）。
    #[serde(default)]
    pub access_key_secret: String,
    /// 区域，例如 cn-beijing。
    #[serde(default = "default_region")]
    pub region: String,
}

fn default_region() -> String {
    "cn-beijing".to_string()
}

/// 一个方舟账号，可被多个 cc-switch provider 绑定使用。
/// 每个账号自带"该账号要查 Code Plan 还是 AFP / OpenAPI Version"，
/// 因为不同账号可能购买不同类型的套餐。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArkAccount {
    /// 内部唯一 id（UUID 或时间戳形式）。
    pub id: String,
    /// 友好名，例如"主账号"、"团队账号"。
    pub name: String,
    /// 凭证。
    #[serde(flatten)]
    pub credentials: ArkCredentials,
    /// 此账号是否使用 Code Plan 接口；true → GetCodingPlanUsage，false → GetAFPUsage。
    #[serde(default = "default_use_coding_plan")]
    pub use_coding_plan: bool,
    /// OpenAPI 版本，默认 2024-01-01。
    #[serde(default = "default_version")]
    pub api_version: String,
}

impl ArkAccount {
    pub fn effective_action(&self) -> &'static str {
        if self.use_coding_plan {
            "GetCodingPlanUsage"
        } else {
            "GetAFPUsage"
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 多个方舟账号配置。
    #[serde(default)]
    pub accounts: Vec<ArkAccount>,
    /// cc-switch provider id → ark account id 的绑定关系。
    #[serde(default)]
    pub bindings: HashMap<String, String>,

    /// 临近限额触发切换的百分比阈值，0.0 - 1.0。
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    /// 后台轮询间隔（秒）。
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    /// 是否启用自动切换。
    #[serde(default)]
    pub auto_switch: bool,
    /// 当前生效的套餐名（仅展示用，权威来源是 cc-switch.db）。
    #[serde(default)]
    pub current_plan: String,
    /// cc-switch 数据库路径；留空则使用默认 `~/.cc-switch/cc-switch.db`。
    #[serde(default)]
    pub cc_switch_db_path: String,
    /// 切换成功后是否自动重启 cc-switch GUI（如果它正在运行）。
    #[serde(default = "default_restart_cc_switch")]
    pub restart_cc_switch_after_switch: bool,

    // ---- 兼容旧版本字段 ----
    /// 旧的全局 use_coding_plan，迁移时拷贝到 accounts。
    #[serde(default = "default_use_coding_plan", skip_serializing)]
    pub use_coding_plan: bool,
    /// 旧的全局 api_version，迁移时拷贝到 accounts。
    #[serde(default = "default_version", skip_serializing)]
    pub api_version: String,
    /// 旧的单一 AK/SK；首次加载会迁移为 accounts[0]。
    #[serde(default, skip_serializing_if = "is_empty_credentials")]
    pub credentials: ArkCredentials,
    /// 旧 plans 字段（已废弃，cc-switch 才是套餐源）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plans: Vec<serde_json::Value>,
    /// 旧字段：手动 Claude settings.json 路径，已不再使用。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub claude_settings_path: String,
    /// 旧字段：cc-switch 可执行文件路径（已废弃）。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cc_switch_bin: String,
    /// 旧字段：手动指定 Action 名（已被 use_coding_plan 替代）。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub api_action: String,
}

fn is_empty_credentials(c: &ArkCredentials) -> bool {
    c.api_key.is_empty()
        && c.access_key_id.is_empty()
        && c.access_key_secret.is_empty()
        && (c.region.is_empty() || c.region == default_region())
}

fn default_use_coding_plan() -> bool {
    true
}

fn default_restart_cc_switch() -> bool {
    true
}

fn default_version() -> String {
    "2024-01-01".to_string()
}

fn default_threshold() -> f64 {
    0.9
}

fn default_poll_interval() -> u64 {
    300
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            accounts: Vec::new(),
            bindings: HashMap::new(),
            threshold: default_threshold(),
            poll_interval_secs: default_poll_interval(),
            auto_switch: false,
            current_plan: String::new(),
            use_coding_plan: default_use_coding_plan(),
            api_version: default_version(),
            cc_switch_db_path: String::new(),
            restart_cc_switch_after_switch: default_restart_cc_switch(),
            credentials: ArkCredentials::default(),
            plans: Vec::new(),
            claude_settings_path: String::new(),
            cc_switch_bin: String::new(),
            api_action: String::new(),
        }
    }
}

impl AppConfig {
    /// 找出 cc-switch provider 对应的方舟账号（通过 bindings 索引）。
    pub fn account_for_provider(&self, provider_id: &str) -> Option<&ArkAccount> {
        let acc_id = self.bindings.get(provider_id)?;
        self.accounts.iter().find(|a| &a.id == acc_id)
    }

    pub fn config_path() -> AppResult<PathBuf> {
        let dirs = ProjectDirs::from(QUALIFIER, ORG, APP)
            .ok_or_else(|| AppError::Config("无法解析配置目录".into()))?;
        let dir = dirs.config_dir().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        Ok(dir.join("config.json"))
    }

    pub fn load() -> AppResult<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let mut cfg: AppConfig = serde_json::from_str(&content)?;
        cfg.migrate_legacy();
        Ok(cfg)
    }

    /// 把旧 credentials 字段迁移成 accounts[0]（如果 accounts 为空且旧字段非空）；
    /// 同时把旧的全局 use_coding_plan / api_version 下沉到每个 account。
    fn migrate_legacy(&mut self) {
        if self.accounts.is_empty() && !is_empty_credentials(&self.credentials) {
            self.accounts.push(ArkAccount {
                id: format!("acc-{}", chrono::Utc::now().timestamp_millis()),
                name: "默认账号".to_string(),
                credentials: self.credentials.clone(),
                use_coding_plan: self.use_coding_plan,
                api_version: self.api_version.clone(),
            });
        }
    }

    pub fn save(&self) -> AppResult<()> {
        let path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
