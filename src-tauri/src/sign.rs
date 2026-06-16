//! 火山引擎 OpenAPI Signature V4（HMAC-SHA256）。
//!
//! 参考：<https://www.volcengine.com/docs/6369/67269>
//!
//! 实现要点：
//! - Algorithm: `HMAC-SHA256`
//! - Credential Scope: `<YYYYMMDD>/<region>/<service>/request`
//! - Signed Headers: 至少包含 `host`、`x-date`、`x-content-sha256`
//! - Payload Hash: SHA256(请求体)，GET 请求为空字符串的 SHA256
//! - Query 字符串需按 key 排序并按 RFC3986 编码

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub struct SigningInput<'a> {
    pub method: &'a str,
    pub host: &'a str,
    pub path: &'a str,
    /// 已经按 key 字典序排好且 URL 编码过的 query。
    pub canonical_query: &'a str,
    pub region: &'a str,
    pub service: &'a str,
    pub access_key_id: &'a str,
    pub secret_access_key: &'a str,
    pub body: &'a [u8],
    pub timestamp: DateTime<Utc>,
}

pub struct SignedHeaders {
    pub host: String,
    pub x_date: String,
    pub x_content_sha256: String,
    pub authorization: String,
}

pub fn sign(input: &SigningInput) -> SignedHeaders {
    let x_date = input.timestamp.format("%Y%m%dT%H%M%SZ").to_string();
    let short_date = input.timestamp.format("%Y%m%d").to_string();

    let payload_hash = hex_sha256(input.body);

    // 1. CanonicalRequest
    let signed_headers_list = "host;x-content-sha256;x-date";
    let canonical_headers = format!(
        "host:{}\nx-content-sha256:{}\nx-date:{}\n",
        input.host, payload_hash, x_date
    );
    let canonical_request = format!(
        "{method}\n{path}\n{query}\n{headers}\n{signed}\n{hash}",
        method = input.method,
        path = input.path,
        query = input.canonical_query,
        headers = canonical_headers,
        signed = signed_headers_list,
        hash = payload_hash,
    );

    // 2. StringToSign
    let credential_scope = format!("{}/{}/{}/request", short_date, input.region, input.service);
    let string_to_sign = format!(
        "HMAC-SHA256\n{}\n{}\n{}",
        x_date,
        credential_scope,
        hex_sha256(canonical_request.as_bytes()),
    );

    // 3. SigningKey: kSecret -> kDate -> kRegion -> kService -> kSigning
    let k_date = hmac(input.secret_access_key.as_bytes(), short_date.as_bytes());
    let k_region = hmac(&k_date, input.region.as_bytes());
    let k_service = hmac(&k_region, input.service.as_bytes());
    let k_signing = hmac(&k_service, b"request");

    let signature = hex::encode(hmac(&k_signing, string_to_sign.as_bytes()));

    let authorization = format!(
        "HMAC-SHA256 Credential={ak}/{scope}, SignedHeaders={signed}, Signature={sig}",
        ak = input.access_key_id,
        scope = credential_scope,
        signed = signed_headers_list,
        sig = signature,
    );

    SignedHeaders {
        host: input.host.to_string(),
        x_date,
        x_content_sha256: payload_hash,
        authorization,
    }
}

fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// 把查询参数列表按 key 字典序排序并按 RFC3986 编码后拼接。
pub fn canonical_query(pairs: &[(&str, &str)]) -> String {
    let mut sorted: Vec<(&str, &str)> = pairs.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    sorted
        .into_iter()
        .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn signs_get_afp_usage() {
        let ts = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let query = canonical_query(&[("Action", "GetAFPUsage"), ("Version", "2024-01-01")]);
        let signed = sign(&SigningInput {
            method: "GET",
            host: "ark.cn-beijing.volces.com",
            path: "/",
            canonical_query: &query,
            region: "cn-beijing",
            service: "ark",
            access_key_id: "AKLT_TEST",
            secret_access_key: "secret",
            body: b"",
            timestamp: ts,
        });
        assert_eq!(signed.x_date, "20250101T000000Z");
        // 空 payload 的 SHA256
        assert_eq!(
            signed.x_content_sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert!(signed.authorization.starts_with(
            "HMAC-SHA256 Credential=AKLT_TEST/20250101/cn-beijing/ark/request, "
        ));
        assert!(signed
            .authorization
            .contains("SignedHeaders=host;x-content-sha256;x-date"));
    }

    #[test]
    fn canonical_query_is_sorted_and_encoded() {
        let q = canonical_query(&[("Version", "2024-01-01"), ("Action", "GetAFPUsage")]);
        assert_eq!(q, "Action=GetAFPUsage&Version=2024-01-01");
    }
}
