# 领域层

:us: [English](README.md) | :cn: 简体中文

本目录包含 Gantt Scheduling framework 的领域建模层，将 Kotlin Gantt Scheduling 各领域子模块映射为 Rust 的 context、aggregation、model component 与 pipeline 结构。

## 职责

- 定义任务、执行器、资源、物料、产能和任务束抽象。
- 向 `MetaModel` 注册变量、中间符号、约束和目标。
- 为普通 MILP、列生成 LP 和 final MILP 流程提供可复用模型组件。
- 暴露成本策略、可行性策略、影子价格和动态模型生命周期扩展点。

## 模块

- [`task`](task/README_ch.md)：任务、执行器、分配、任务计划、任务束、成本、求解器值适配器和影子价格 key。
- [`task_compilation`](task_compilation/README_ch.md)：任务编译模型组件、时间变量、切换变量、完工时间、解分析和限制。
- [`task_generation`](task_generation/README_ch.md)：从 Kotlin 映射保留的任务生成扩展点。
- [`bunch_generation`](bunch_generation/README_ch.md)：定价图、标签算法和任务束生成器。
- [`bunch_compilation`](bunch_compilation/README_ch.md)：任务束列生成主问题组件。
- [`capacity_scheduling`](capacity_scheduling/README_ch.md)：产能动作、产能编译、带序产能编译、产能列和限制。
- [`produce`](produce/README_ch.md)：物料需求、储备、生产任务、产出使用、消耗使用和数量目标。
- [`resource`](resource/README_ch.md)：执行资源、存储资源、连接资源、资源使用量、容量、松弛和资源限制。
- [`common`](common/README_ch.md)：共享约束索引和动态模型生命周期重导出。

## Public API

- `task`
- `task_compilation`
- `task_generation`
- `bunch_generation`
- `bunch_compilation`
- `capacity_scheduling`
- `produce`
- `resource`
- `common`

## 扩展点

领域模块应持有优化语义。应用层可以组合这些能力，但不应重复变量注册、约束构造或解提取逻辑。

## 生命周期与数据流

task、resource 和 produce 模型定义共享词汇；compilation context 注册主问题变量和表达式；generation context 创建 pricing column；capacity 与 bunch context 将任务流程接入 solver 迭代；common helper 复用动态模型状态和约束索引。

## 验证

领域覆盖使用 `cargo test -p ospf-rust-framework-gantt-scheduling --lib`。修改 context 注册、pipeline 约束或 solution extraction 时应补充聚焦测试。

## 相关目录

- [`../application`](../application/README_ch.md)
- [`../infrastructure`](../infrastructure/README_ch.md)
