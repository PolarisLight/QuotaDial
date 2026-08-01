<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <img src="./assets/readme/hero.svg" width="100%" alt="QuotaDial：账号额度、Token 用量与等效费用">
</p>

<p align="center">
  <a href="https://github.com/PolarisLight/QuotaDial/releases/tag/v0.1.1"><img src="https://img.shields.io/badge/release-v0.1.1-72D6A7?style=flat-square&labelColor=15382E" alt="Release v0.1.1"></a>
  <img src="https://img.shields.io/badge/macOS-Apple%20Silicon-F5F3EA?style=flat-square&labelColor=15382E" alt="macOS Apple Silicon">
  <img src="https://img.shields.io/badge/Windows-10%20%2F%2011-8FA7FF?style=flat-square&labelColor=15382E" alt="Windows 10 / 11">
  <img src="https://img.shields.io/badge/built%20with-Tauri-8FA7FF?style=flat-square&labelColor=15382E" alt="Built with Tauri">
</p>

QuotaDial 是一个用于理解 Codex 订阅用量的原生桌面仪表盘。它以当前登录账号返回的额度为准，记录剩余额度如何变化，再结合当前电脑的会话记录，提供 Token 明细与等效费用估算。

<p align="center">
  <img src="./assets/readme/dashboard.webp" width="100%" alt="QuotaDial 仪表盘，展示账号剩余额度、Token 消耗、耗尽预测与本机项目会话">
</p>

## 在额度耗尽前看见它

- **账号级额度**：直接使用当前 Codex 账号返回的额度，包含同一账号在其他设备上的活动。
- **消耗与预测**：同时查看下降的剩余额度曲线、每日 Token 柱状图、当前速度和预计耗尽时间。
- **会话级明细**：按顶层会话统计本月 Token，并将子代理用量归并到所属会话。
- **等效 API 费用**：根据模型公开 API 单价估算本机用量价值，分别处理缓存输入与输出价格。
- **原生菜单栏与 Windows 额度浮窗**：macOS 保留菜单栏工作流；Windows 将剩余百分比嵌入 QuotaDial 六边形表盘图标，并通过紧凑浮窗集中提供额度状态与操作。

## 每个数字代表什么

| 视图 | 数据来源 | 统计范围 |
| --- | --- | --- |
| 额度与重置时间 | 当前登录的 Codex 账号 | 使用该账号的所有设备 |
| 每日账号 Token | Codex 账号数据 | 账号提供数据时覆盖所有设备 |
| 会话列表与项目名称 | 本地 Codex 会话文件 | 仅当前电脑 |
| 等效费用 | 本地 Token 记录 × 模型公开价格 | 估算值，不是账单 |

QuotaDial 会明确区分这些数据边界：不会用本地会话 Token 反推账号额度，也不会把等效 API 费用描述成订阅实际支出。

## 本地优先

本地会话导入器只提取统计所需的信息：会话关系、模型、时间、项目路径和 Token 数量。提示词、回复正文、工具输出、推理内容和子代理昵称不会写入 QuotaDial 的 SQLite 数据库。

## 下载

[**下载 QuotaDial v0.1.1（Windows 10/11 x64）→**](https://github.com/PolarisLight/QuotaDial/releases/download/v0.1.1/QuotaDial_0.1.1_windows_x64_setup.exe)

[**下载 QuotaDial v0.1.0（Apple Silicon macOS）→**](https://github.com/PolarisLight/QuotaDial/releases/download/v0.1.0/QuotaDial_0.1.0_aarch64.dmg)

当前预览版尚未进行 Apple 公证或 Windows 代码签名。macOS 首次运行时可能需要右键点击 QuotaDial 并选择**打开**；Windows 可能显示 SmartScreen 提示。

## 开发

需要 Node.js、pnpm、Rust，以及 [Tauri 2](https://v2.tauri.app/start/prerequisites/) 对应平台的开发环境。

```bash
git clone https://github.com/PolarisLight/QuotaDial.git
cd QuotaDial/app
pnpm install
pnpm tauri dev
```

运行检查：

```bash
pnpm test
pnpm lint
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

Windows 会自动查找系统 PATH、WindowsApps、npm 全局目录，以及 VS Code、VS Code Insiders、Cursor 和 Windsurf 中随 OpenAI 扩展安装的 Codex CLI。需要指定自定义位置时，可设置 `QUOTADIAL_CODEX_PATH`。Windows 托盘图标会在品牌表盘内直接显示剩余额度百分比，左键或右键都会打开同一个额度浮窗；浮窗只读取内存中的轻量摘要，不再传输会话明细。开机启动时应用会保持在托盘中，不弹出主窗口。

数据契约和本地开发流程见 [App 开发说明](./app/README.md)。

## 路线图

- Claude Code 账号额度数据源与仪表盘切换
- Windows 代码签名与自动发布
- Apple Developer ID 签名和公证

QuotaDial 是独立开发的实用工具，与 OpenAI 或 Apple 不存在隶属或官方背书关系。
