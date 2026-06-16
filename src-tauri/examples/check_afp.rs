//! 命令行调用 Volc Ark OpenAPI 的小工具，默认走 GetCodingPlanUsage。
//!
//! 用法：
//!
//! ```bash
//! export VOLC_AK="AKLT..."
//! export VOLC_SK="..."
//! export VOLC_REGION="cn-beijing"            # 可选
//! export VOLC_ACTION="GetCodingPlanUsage"    # 可选；改成 GetAFPUsage 即可查 AFP
//! cargo run --manifest-path src-tauri/Cargo.toml --example check_afp
//! ```

use volcano_coding_switcher_lib::__test_support::*;

#[tokio::main]
async fn main() {
    let ak = std::env::var("VOLC_AK").expect("请设置 VOLC_AK 环境变量");
    let sk = std::env::var("VOLC_SK").expect("请设置 VOLC_SK 环境变量");
    let region = std::env::var("VOLC_REGION").unwrap_or_else(|_| "cn-beijing".into());

    let creds = ArkCredentials {
        api_key: String::new(),
        access_key_id: ak,
        access_key_secret: sk,
        region,
    };
    let client = ArkClient::new();
    let action = std::env::var("VOLC_ACTION").unwrap_or_else(|_| "GetCodingPlanUsage".to_string());
    let version = std::env::var("VOLC_VERSION").unwrap_or_else(|_| "2024-01-01".to_string());
    match client.fetch_quota(&creds, &action, &version).await {
        Ok(snap) => {
            println!("Status        = {:?}", snap.status);
            println!("PlanType      = {:?}", snap.plan_type);
            println!("UpdatedTs     = {}", snap.update_timestamp);
            for p in &snap.periods {
                println!(
                    "  [{:>10}] used={} quota={} percent={:.2}% reset={}",
                    p.level, p.used, p.quota, p.percent, p.reset_time
                );
            }
            println!("FetchedAt     = {}", snap.fetched_at);
            println!("Source        = {}", snap.source);
        }
        Err(e) => {
            eprintln!("调用失败: {e}");
            std::process::exit(1);
        }
    }
}
