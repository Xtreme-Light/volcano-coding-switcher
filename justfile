# 火山方舟 code_plan 切换器 - just 命令清单
# 使用: just <命令>

# 默认显示所有可用命令
default:
    @just --list

# 安装前端依赖
install:
    npm install

# 启动开发服务（Tauri + Vite）
dev:
    cargo tauri dev

# 构建生产版本
build:
    cargo tauri build

# macOS 签名构建（需要配置环境变量）
build-macos-signed:
    ./scripts/build-macos-signed.sh

# Rust 类型检查
check:
    cargo check --manifest-path src-tauri/Cargo.toml --tests --examples

# Rust 格式化检查
fmt-check:
    cargo fmt --check --manifest-path src-tauri/Cargo.toml

# Rust 格式化
fmt:
    cargo fmt --manifest-path src-tauri/Cargo.toml

# 前端类型检查 + 构建
frontend-build:
    npm run build

# 前端开发服务（仅 Vite，不含 Tauri）
frontend-dev:
    npm run dev

# 清理构建产物
clean:
    cargo clean --manifest-path src-tauri/Cargo.toml
    rm -rf dist

# 运行示例：检查 AFP 用量
example-afp account-id:
    cargo run --manifest-path src-tauri/Cargo.toml --example check_afp {{account-id}}

# 显示当前 git 状态
status:
    git status --short

# 提交并推送（需要提供提交信息）
commit message:
    git add -A
    git commit -m "{{message}}"
    git push

# 创建并推送 RC tag
tag-rc version:
    git tag -a v{{version}} -m "v{{version}}"
    git push origin v{{version}}