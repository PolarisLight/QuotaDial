# QuotaDial

QuotaDial 是一个跨平台的 Codex 额度与 Token 监控应用。它以账号返回的额度为准，
同时读取当前电脑上的 Codex 会话记录，用于展示每个会话的 Token 构成和等效 API 费用。

## 数据范围

- **账号额度与每日 Token**：来自当前登录账号，覆盖使用同一账号的所有设备。
- **会话详情**：来自当前设备的本地 Codex 会话文件，不代表其他电脑上的会话明细。
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
