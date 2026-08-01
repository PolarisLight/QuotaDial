# QuotaDial

QuotaDial 是一个跨平台的 Codex 额度与 Token 监控应用。它以账号返回的额度为准，
同时读取当前电脑上的 Codex 会话记录，用于展示每个会话的 Token 构成和等效 API 费用。

## 数据范围

- **账号额度与每日 Token**：来自当前登录账号，覆盖使用同一账号的所有设备。
- **项目与会话**：来自当前设备的本地 Codex 会话文件。主表按项目路径汇总，展开项目后查看具体会话，不代表其他电脑上的会话明细。
- **子任务归并**：子代理/子任务的用量会归入所属的顶层会话，列表只显示顶层会话。
- **等效费用**：按照对应模型的公开 API 单价估算，仅用于比较，不是订阅账单或实际扣费。

## 本地隐私

应用只从会话文件提取会话关系、模型、时间、项目路径和 Token 计数。提示词、回复正文、
工具输出、推理内容和子代理昵称不会写入 SQLite。

## 开发

```bash
pnpm install
pnpm tauri dev
```

常用检查：

```bash
pnpm test
pnpm lint
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

## Windows

Windows 开发需要 Rust MSVC 工具链、Visual Studio C++ 生成工具和 WebView2。生成当前用户安装、不要求管理员权限的 NSIS 安装包：

```powershell
pnpm tauri:build:windows
```

安装包输出到 `src-tauri/target/release/bundle/nsis/`。应用会依次从 `QUOTADIAL_CODEX_PATH`、系统 PATH、WindowsApps、npm 全局目录，以及 VS Code 系列编辑器的 OpenAI 扩展中查找 Codex CLI；后台 app-server 使用无控制台窗口模式启动。
