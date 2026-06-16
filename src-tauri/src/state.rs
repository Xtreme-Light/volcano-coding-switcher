use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 单个用量周期，统一表示 AFP / Coding Plan 的周期。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PeriodUsage {
    /// 周期标识：AFP 用 `FiveHour/Daily/Weekly/Monthly`；Coding Plan 用 `session/weekly/monthly`。
    #[serde(default)]
    pub level: String,
    /// 已用百分比（0~100）；如果接口仅返回百分比则用此字段。
    #[serde(default)]
    pub percent: f64,
    /// 已用绝对值（AFP 接口提供）。
    #[serde(default)]
    pub used: u64,
    /// 上限绝对值（AFP 接口提供）。
    #[serde(default)]
    pub quota: u64,
    /// 订阅时间，Unix 秒（0 表示未提供）。
    #[serde(default)]
    pub subscribe_time: i64,
    /// 周期重置时间，Unix 秒（0 表示无）。
    #[serde(default)]
    pub reset_time: i64,
}

impl PeriodUsage {
    /// 0~1 比例，优先用绝对值，否则用百分比。
    pub fn ratio(&self) -> f64 {
        if self.quota > 0 {
            self.used as f64 / self.quota as f64
        } else {
            (self.percent / 100.0).clamp(0.0, 1.0)
        }
    }

    pub fn is_active(&self) -> bool {
        self.quota > 0 || self.percent > 0.0 || self.reset_time > 0
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuotaSnapshot {
    /// Coding Plan 状态（Running / Suspended ...），AFP 模式下保持空字符串。
    #[serde(default)]
    pub status: String,
    /// AFP 接口返回的 PlanType（兼容老逻辑）。
    #[serde(default)]
    pub plan_type: String,
    /// 接口给出的更新时间戳（Unix 秒）。
    #[serde(default)]
    pub update_timestamp: i64,
    /// 通用周期列表。
    #[serde(default)]
    pub periods: Vec<PeriodUsage>,
    /// 上次成功获取的时间（ISO8601）。
    #[serde(default)]
    pub fetched_at: String,
    /// 原始响应 JSON（调试用）。
    #[serde(default)]
    pub raw_response: String,
    /// 数据来源：`real` / `mock`。
    #[serde(default)]
    pub source: String,
}

impl QuotaSnapshot {
    /// 取所有"活跃"周期中使用率最高的一个，作为是否触发切换的依据。
    pub fn peak_ratio(&self) -> f64 {
        self.periods
            .iter()
            .filter(|p| p.is_active())
            .map(|p| p.ratio())
            .fold(0.0_f64, f64::max)
    }

    /// 取"近5小时"周期（Code Plan 的 session / AFP 的 FiveHour）的使用率。
    /// 用于套餐列表展示和"选最低用量套餐"切换策略。
    /// 找不到该周期时回退到 peak_ratio()，保证总有可用值。
    pub fn short_term_ratio(&self) -> f64 {
        for p in &self.periods {
            if !p.is_active() {
                continue;
            }
            let lvl = p.level.to_lowercase();
            if lvl == "session" || lvl == "fivehour" {
                return p.ratio();
            }
        }
        self.peak_ratio()
    }

    /// 触发阈值时给出最关键的周期标识，用于通知文案。
    pub fn peak_label(&self) -> Option<String> {
        let mut best: Option<(String, f64)> = None;
        for period in &self.periods {
            if !period.is_active() {
                continue;
            }
            let r = period.ratio();
            if best.as_ref().map(|(_, br)| r > *br).unwrap_or(true) {
                best = Some((period.level.clone(), r));
            }
        }
        best.map(|(l, _)| l)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
    pub last_quota: Arc<RwLock<Option<QuotaSnapshot>>>,
}
