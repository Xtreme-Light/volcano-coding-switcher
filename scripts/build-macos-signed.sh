#!/usr/bin/env bash
set -euo pipefail

# macOS 签名 + 公证构建脚本。
# 使用前请把下面的占位值替换为你的 Apple Developer 信息，或在外部环境中提前导出同名变量。
# 注意：APPLE_PASSWORD 应使用 App-specific password，不是 Apple ID 登录密码。

export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:-Developer ID Application: Your Name (TEAMID)}"
export APPLE_ID="${APPLE_ID:-your-apple-id@example.com}"
export APPLE_PASSWORD="${APPLE_PASSWORD:-xxxx-xxxx-xxxx-xxxx}"
export APPLE_TEAM_ID="${APPLE_TEAM_ID:-TEAMID}"

# 如果你的 Apple ID 关联多个团队，取消注释并设置 provider short name。
# export APPLE_PROVIDER_SHORT_NAME="${APPLE_PROVIDER_SHORT_NAME:-TEAMID}"

# CI 场景可使用 base64 后的 .p12 证书；本机 Keychain 已安装证书时不需要设置。
# export APPLE_CERTIFICATE="${APPLE_CERTIFICATE:-base64-encoded-p12}"
# export APPLE_CERTIFICATE_PASSWORD="${APPLE_CERTIFICATE_PASSWORD:-p12-password}"

for name in APPLE_SIGNING_IDENTITY APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
  value="${!name}"
  case "$value" in
    ""|"Developer ID Application: Your Name (TEAMID)"|"your-apple-id@example.com"|"xxxx-xxxx-xxxx-xxxx"|"TEAMID")
      echo "请先设置真实的 $name" >&2
      exit 1
      ;;
  esac
done

if ! command -v cargo >/dev/null 2>&1; then
  echo "未找到 cargo" >&2
  exit 1
fi

cargo tauri build
