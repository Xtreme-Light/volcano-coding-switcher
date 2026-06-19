# 火山方舟 code_plan 切换器

> 查看火山方舟 Code Plan / AFP 用量，临近上限时自动通过 [cc-switch](https://github.com/farion1231/cc-switch) 切换到用量最低的套餐。

## 项目背景

[火山方舟](https://www.volcengine.com/product/ark) 的 Code Plan 套餐有多个用量周期（近 5 小时 / 近 1 周 / 近 1 月），任一周期达到上限都会限流。当你在 [Claude Code](https://docs.anthropic.com/en/docs/claude-code) 里重度使用时，很容易在不知不觉中撞上限额。

[cc-switch](https://github.com/farion1231/cc-switch) 是一个管理多个 Claude Code Provider（不同 API Key / Base URL）的桌面工具，但它本身不会感知用量、也不会自动切换。

本工具把两者结合起来：

- 定时查询火山方舟 OpenAPI 获取各周期用量
- 系统托盘图标实时显示当前用量（绿 ≤80% / 橙 >80% / 红 ≥100%）
- 临近阈值时自动在 cc-switch 的所有已绑定套餐里挑"近 5 小时用量最低"的那个切换过去
- 支持多账号、多套餐绑定，不同账号可配不同接口类型（Code Plan / AFP）

## 功能特性

- **用量监控**：展示近 5 小时 / 近 1 周 / 近 1 月三个周期的使用率与重置倒计时
- **多账号管理**：可配置多组方舟 AK/SK，每个账号独立选择 Code Plan 或 AFP 接口
- **套餐绑定**：把 cc-switch 里的每个套餐绑到某个方舟账号，未绑定的套餐在切换时自动跳过
- **自动切换**：后台轮询，达到阈值时自动切换到"近 5 小时用量最低"的套餐
- **手动切换**：主页可一键查询所有套餐用量并手动切换
- **托盘图标**：动态生成的进度环图标，颜色随用量变化；悬浮显示当前峰值
- **跨平台**：支持 macOS、Windows、Linux

## 截图

> 主界面左侧显示当前账号用量，右侧显示 cc-switch 套餐列表与各套餐用量。

## 安装

前往 [Releases](../../releases) 下载对应平台的安装包：

| 平台 | 安装包 |
|------|--------|
| macOS | `.dmg` |
| Windows | `.msi` 或 `.exe` |
| Linux | `.AppImage` / `.deb` / `.rpm` |

### 前置依赖

- 已安装 [cc-switch-cli](https://github.com/SaladDay/cc-switch-cli) 并添加至少一个 Claude Provider
- 拥有火山方舟账号并购买了 Code Plan 或 AFP 套餐
- 获取账号的 AccessKey ID / AccessKey Secret（在[火山引擎控制台](https://console.volcengine.com/iam/keymanage/)创建）

## 用法

1. **启动本应用**，首次打开会检测 cc-switch-cli 是否已安装：
   - 未安装时会显示阻断提示并提供 GitHub 安装链接
   - 已安装 CLI 但未初始化数据库时会提示先添加 Provider
2. **打开设置**（右上角齿轮）：
   - **方舟账号**：新增账号，填入 AK / SK / 区域，勾选是否为 Code Plan
   - **账号 ↔ 套餐 绑定**：把 cc-switch 里的每个套餐绑到对应的方舟账号
   - **切换策略**：设置阈值（默认 90%）和轮询间隔（默认 300 秒），可开启自动切换
   - **cc-switch 集成**：确认 cc-switch 数据库路径（默认 `~/.cc-switch/cc-switch.db`）
3. **回到主页**：
   - 用量区域右上角下拉可切换查看不同账号的用量
   - 点击"刷新"立即拉取当前账号用量
   - cc-switch 状态卡片点击"查询所有用量"可查看所有套餐的近 5 小时用量，并提示建议切换的套餐
4. **后台运行**：关闭窗口会最小化到系统托盘，后台继续轮询；托盘右键菜单可手动刷新或切换

## 工作原理

```
┌─────────────────────────────────────────────────────────────┐
│                    本工具 (Tauri 2 + Rust)                   │
│                                                             │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐    │
│  │  用量轮询     │──▶│  阈值判断     │──▶│  自动切换     │    │
│  │  (monitor.rs) │   │  peak_ratio  │   │  lowest 5h   │    │
│  └──────┬───────┘   └──────────────┘   └──────┬───────┘    │
│         │                                       │            │
│         ▼                                       ▼            │
│  ┌──────────────┐                      ┌──────────────┐     │
│  │ 火山方舟 API  │                      │ cc-switch-cli│     │
│  │ GetCodingPlan │                      │ provider     │     │
│  │ Usage         │                      │ switch <id>  │     │
│  └──────────────┘                      └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

- **用量查询**：通过火山引擎 Signature V4 签名调用 `GetCodingPlanUsage`（POST）或 `GetAFPUsage`（GET）OpenAPI
- **cc-switch 集成**：通过官方 CLI `cc-switch --app claude provider switch <id>` 执行切换，不再直接写数据库
- **进程重启**：切换后可选自动重启 cc-switch GUI（因为它不会热加载数据库变更）

## 开发

### 技术栈

- **后端**：Rust + Tauri 2.0
- **前端**：React 18 + TypeScript + Tailwind CSS + Vite
- **依赖**：rusqlite (bundled)、reqwest、image、tokio、chrono

### 本地开发

```bash
# 安装前端依赖
npm install

# 开发模式（同时启动 Vite 和 Tauri）
npm run tauri dev
# 或
cargo tauri dev

# 构建
npm run tauri build
```

### 项目结构

```
.
├── src/                      # 前端 React 代码
│   ├── components/           # UI 组件
│   ├── state/                # React hooks
│   ├── api.ts                # Tauri 命令封装
│   └── App.tsx
├── src-tauri/                # 后端 Rust 代码
│   ├── src/
│   │   ├── ark.rs            # 火山方舟 API 客户端
│   │   ├── sign.rs           # Signature V4 签名
│   │   ├── cc_switch_cli.rs  # cc-switch CLI 调用
│   │   ├── cc_switch_proc.rs # cc-switch 进程重启
│   │   ├── commands.rs       # Tauri 命令
│   │   ├── config.rs         # 配置管理
│   │   ├── monitor.rs        # 后台轮询与自动切换
│   │   ├── tray.rs           # 系统托盘
│   │   └── tray_icon.rs      # 托盘图标生成
│   ├── Cargo.toml
│   └── tauri.conf.json
├── scripts/
│   └── build-macos-signed.sh # macOS 签名构建脚本
├── package.json
└── tailwind.config.js
```

## 致谢

- [cc-switch-cli](https://github.com/SaladDay/cc-switch-cli) — Claude Code Provider 切换器 CLI，本工具强依赖它
- [Tauri](https://tauri.app/) — 跨平台桌面应用框架
- [火山引擎方舟](https://www.volcengine.com/product/ark) — 大模型推理平台

## License

MIT
