//! 火山方舟 OpenAPI 调用：支持 GetCodingPlanUsage / GetAFPUsage。
//!
//! - Host:    `open.volcengineapi.com`
//! - Method:  POST（控制台抓包是 POST + 空 body `{}`，OpenAPI 网关 GET 也兼容；这里统一 POST）
//! - Query:   `Action=<...>&Version=<...>`
//! - Body:    `{}`
//! - 鉴权:    火山引擎 Signature V4（HMAC-SHA256），Service = `ark`，Region 走配置项

use crate::config::ArkCredentials;
use crate::error::{AppError, AppResult};
use crate::sign::{self, SigningInput};
use crate::state::{PeriodUsage, QuotaSnapshot};
use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;

const SERVICE: &str = "ark";
const PATH: &str = "/";
const OPENAPI_HOST: &str = "open.volcengineapi.com";

#[async_trait]
pub trait QuotaProvider: Send + Sync {
    async fn fetch_quota(
        &self,
        creds: &ArkCredentials,
        action: &str,
        version: &str,
    ) -> AppResult<QuotaSnapshot>;
}

pub struct ArkClient {
    http: reqwest::Client,
}

impl ArkClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("volcano-coding-switcher/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("init http client"),
        }
    }

    fn host_for_region(_region: &str) -> String {
        OPENAPI_HOST.to_string()
    }

    async fn fetch_real(
        &self,
        creds: &ArkCredentials,
        action: &str,
        version: &str,
    ) -> AppResult<QuotaSnapshot> {
        if creds.access_key_id.is_empty() || creds.access_key_secret.is_empty() {
            return Err(AppError::Config("未配置 AccessKey ID / Secret".into()));
        }

        let host = Self::host_for_region(&creds.region);
        let region = if creds.region.is_empty() {
            "cn-beijing"
        } else {
            creds.region.as_str()
        };
        let canonical_query =
            sign::canonical_query(&[("Action", action), ("Version", version)]);

        let body: &[u8] = b"{}";

        let signed = sign::sign(&SigningInput {
            method: "POST",
            host: &host,
            path: PATH,
            canonical_query: &canonical_query,
            region,
            service: SERVICE,
            access_key_id: &creds.access_key_id,
            secret_access_key: &creds.access_key_secret,
            body,
            timestamp: Utc::now(),
        });

        let url = format!("https://{}{}?{}", host, PATH, canonical_query);
        tracing::info!(url = url.as_str(), action, "calling Volc OpenAPI");
        let resp = self
            .http
            .post(&url)
            .header("Host", &signed.host)
            .header("X-Date", &signed.x_date)
            .header("X-Content-Sha256", &signed.x_content_sha256)
            .header("Authorization", &signed.authorization)
            .header("Content-Type", "application/json")
            .body(body.to_vec())
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        tracing::info!(status = %status, body = text.as_str(), "Volc OpenAPI response");
        if !status.is_success() {
            return Err(AppError::Ark(format!("HTTP {}: {}", status, text)));
        }

        let envelope: ResponseEnvelope = serde_json::from_str(&text)
            .map_err(|e| AppError::Ark(format!("响应解析失败: {} ({})", e, text)))?;

        if let Some(err) = envelope.response_metadata.error {
            return Err(AppError::Ark(format!("{}: {}", err.code, err.message)));
        }
        if let Some(err) = envelope.error {
            return Err(AppError::Ark(format!("{}: {}", err.code, err.message)));
        }

        let result = envelope
            .result
            .ok_or_else(|| AppError::Ark(format!("响应缺少 Result 字段: {}", text)))?;

        let mut snapshot = parse_result(&result)?;
        snapshot.fetched_at = Utc::now().to_rfc3339();
        snapshot.raw_response = text;
        snapshot.source = "real".to_string();
        Ok(snapshot)
    }

    fn mock_snapshot() -> QuotaSnapshot {
        QuotaSnapshot {
            status: "Mock".to_string(),
            plan_type: "Mock".to_string(),
            update_timestamp: Utc::now().timestamp(),
            periods: vec![
                PeriodUsage {
                    level: "session".to_string(),
                    percent: 12.5,
                    reset_time: Utc::now().timestamp() + 3600,
                    ..Default::default()
                },
                PeriodUsage {
                    level: "weekly".to_string(),
                    percent: 46.7,
                    reset_time: Utc::now().timestamp() + 86_400 * 5,
                    ..Default::default()
                },
                PeriodUsage {
                    level: "monthly".to_string(),
                    percent: 28.2,
                    reset_time: Utc::now().timestamp() + 86_400 * 18,
                    ..Default::default()
                },
            ],
            fetched_at: Utc::now().to_rfc3339(),
            raw_response: "<mock>".to_string(),
            source: "mock".to_string(),
        }
    }
}

#[async_trait]
impl QuotaProvider for ArkClient {
    async fn fetch_quota(
        &self,
        creds: &ArkCredentials,
        action: &str,
        version: &str,
    ) -> AppResult<QuotaSnapshot> {
        if creds.access_key_id.is_empty() || creds.access_key_secret.is_empty() {
            tracing::warn!("未配置 AK/SK，返回 Mock 数据");
            return Ok(Self::mock_snapshot());
        }
        self.fetch_real(creds, action, version).await
    }
}

/// 把 Result 字段解析为 QuotaSnapshot，兼容 GetCodingPlanUsage / GetAFPUsage。
fn parse_result(result: &Value) -> AppResult<QuotaSnapshot> {
    let mut snapshot = QuotaSnapshot::default();

    // GetCodingPlanUsage 风格
    if let Some(arr) = result.get("QuotaUsage").and_then(|v| v.as_array()) {
        if let Some(s) = result.get("Status").and_then(|v| v.as_str()) {
            snapshot.status = s.to_string();
        }
        if let Some(t) = result.get("UpdateTimestamp").and_then(|v| v.as_i64()) {
            snapshot.update_timestamp = t;
        }
        for item in arr {
            snapshot.periods.push(PeriodUsage {
                level: item
                    .get("Level")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                percent: item.get("Percent").and_then(|v| v.as_f64()).unwrap_or(0.0),
                reset_time: item
                    .get("ResetTimestamp")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                ..Default::default()
            });
        }
        return Ok(snapshot);
    }

    // GetAFPUsage 风格：包含 AFPxxx 字段
    if let Some(plan_type) = result.get("PlanType").and_then(|v| v.as_str()) {
        snapshot.plan_type = plan_type.to_string();
    }
    let mappings = [
        ("AFPFiveHour", "FiveHour"),
        ("AFPDaily", "Daily"),
        ("AFPWeekly", "Weekly"),
        ("AFPMonthly", "Monthly"),
    ];
    for (key, label) in mappings {
        if let Some(obj) = result.get(key) {
            snapshot.periods.push(PeriodUsage {
                level: label.to_string(),
                used: obj.get("Used").and_then(|v| v.as_u64()).unwrap_or(0),
                quota: obj.get("Quota").and_then(|v| v.as_u64()).unwrap_or(0),
                subscribe_time: obj
                    .get("SubscribeTime")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                reset_time: obj.get("ResetTime").and_then(|v| v.as_i64()).unwrap_or(0),
                ..Default::default()
            });
        }
    }
    Ok(snapshot)
}

// ---- 响应外层 ----

#[derive(Debug, Deserialize)]
struct ResponseEnvelope {
    #[serde(rename = "ResponseMetadata", default)]
    response_metadata: ResponseMetadata,
    #[serde(rename = "Result", default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<OpenAiError>,
}

#[derive(Debug, Default, Deserialize)]
struct ResponseMetadata {
    #[serde(rename = "Error", default)]
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    #[serde(rename = "Code", default)]
    code: String,
    #[serde(rename = "Message", default)]
    message: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiError {
    #[serde(default)]
    code: String,
    #[serde(default)]
    message: String,
}
