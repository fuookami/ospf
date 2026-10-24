# Gantt Network 桥接

[English](README.md)

本可选 crate 将 Network 资源路径映射为 Gantt 业务任务束，不注册第二套主模型、
不启动求解器，也不接管全局排程。Gantt 本体不依赖本 crate 或 Network。

`NetworkBunchGenerationContext::generate` 接收路径问题、计算预算、当前
`ColumnGenerationScope`、完整路径任务束提取器和独立无改进判定器。返回结果同时
保留原始状态、稳定弧身份回溯、诊断和 `ColumnGenerationBatch`。

输出截取后的空集合不是无改进证据。预算耗尽、中断、未经认证的支配、完整校验
拒绝或提取丢弃都不能认证穷尽。Gantt 提升下界前仍须独立验证当前模型绑定的
LP 报告，以及与该对偶严格匹配的完整定价域。

离线测试使用兼容的 nightly 工具链：

```powershell
cargo +nightly-2026-08-01 test -p ospf-rust-framework-gantt-network --no-default-features
```

桥接入口的交付不代表全部 Network 领域或 Demo4 端到端迁移已完成，具体见 Network
迁移说明。
