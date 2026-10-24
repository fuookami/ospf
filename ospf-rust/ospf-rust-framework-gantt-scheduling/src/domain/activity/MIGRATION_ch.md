# Kotlin 3f15899 迁移说明

[English](MIGRATION.md) | 简体中文

固定来源 `3f15899e6398c878f641a7f48ccb2e5491bf4c9f`，相对父提交
`c66cd7fa1779848819ba25bf7fa34ba54733c3c9`；不包含后续 Kotlin 变更。

- 活动输入、完整开始域、联合资源段、四端点依赖、存在性、基线、Running/Completed
  事实及业务行位于 Gantt `domain::activity`，不引入重复内核或新 crate。
- 有符号事件库存支持独立物料、期初/仓储边界、固定未来到货及候选相对流；不是 BOM/MRP。
- 嵌入式与独立应用共用事务注册和完整编译/提取，不新增活动列生成或连续排程。
- 自定义 `SchedulingSolverValueAdapter` 必须新增可失败的 `try_into_value`、`try_from_value`。
  旧方法保留兼容既有调用方，但 activity 不调用；`preserves_f64_identity` 默认 false，
  即使显式声明 true，也核对本次实际系数，不把声明当证书。
- 输入与领域结果保留 V；非 f64 或潜在有损模型不转移编译模型证明、界或无 incumbent
  结论。消费目标/证明前须阅读[API 指南](README_ch.md)中的口径与范围说明。
- 原生 SCIP 用例覆盖柔性车间、联合资源日历、Running 恢复、冻结不可行、物料等待和显式取消。
  不因缺少 runtime 条件早退；测试日志和进度计划仅本地保留，不作为源码或历史验收数字交付。

非目标：连续时间精确优化、抢占、部分交付、顺序准备、自动物料替代、活动定价树，
以及补全旧 resource/produce 注册占位。
