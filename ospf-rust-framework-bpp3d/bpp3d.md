# ospf-rust-framework-bpp3d 迁移交接文档

## 1. 总目标

将 `E:\workspace\ospf\ospf-kotlin\ospf-kotlin-framework-bpp3d` 迁移到 Rust workspace 中的 crate：

```text
E:\workspace\ospf\ospf-rust\ospf-rust-framework-bpp3d
```

迁移目标不是逐文件翻译 Kotlin，而是用 Rust 风格复刻 BPP3D 领域框架能力，并严格对齐当前 Rust 项目的 framework 架构规范：

1. 以 `ospf_rust_core::model::MetaModel` 为模型装配轴心。
2. 以 context / aggregation / model component / pipeline 组织优化建模能力。
3. application 层只负责流程编排、solver 选择、状态映射、CSV/renderer 协议适配、trace/KPI 组织，不直接堆积变量、约束、目标和 token 解析逻辑。
4. 普通 RMP、列生成 LP、最终 MILP 尽量共享同一套 context / aggregation / pipeline 注册路径。
5. public API 保持泛型数值和物理量边界，求解器适配边界内再落到 `f64`。
6. 不照搬 Kotlin 的 `QuantityPoint*`、`QuantityVector*`、`QuantityPlacement*` 类型体系；Rust 版必须基于泛型、trait、关联类型和 newtype 重新设计正交 typed geometry adapter。

## 2. 当前结论

当前尚未达到完整迁移总目标。

已达到当前阶段领域框架目标：crate 可编译、可测试，BPP3D domain/application/CSV/render/packing/solver backend adapter 的基础闭环已建立，dataset suite、diagnostics、trace replay、PWL continuous radius、fixture 护栏、业务规则聚合和语义化注册入口均已成型。

尚未达到完整目标的关键原因：

1. 复杂块、高级搜索和 Pattern/Pile/Historical generator 仍未形成完整真实策略。
2. solver dataset suite 仍未覆盖更大 Kotlin 回归数据目录和真实 solver feature 执行。
3. PatternedItem、PackageAttribute 和剩余 objectives/limits 仍未具备完整业务语义。
4. Kotlin README 范围能力仍未补齐。

## 3. 已完成事项摘要

已完成：

1. Rust crate 迁移基线、framework 架构边界和泛型/物理量基础设施。
2. BPP3D domain、application、CSV、packing、renderer 和 solver backend adapter 基础闭环。
3. MetaModel RMP/final 注册、one-round column generation、coverage、trace replay 和结构化 diagnostics。
4. serde dataset suite、fixture 元数据、suite-level batch report 和 fake/no-run/feature matrix 验收入口。
5. PWL continuous radius、PatternedItem/PackageAttribute 聚合 diagnostics、剩余 objective/limit MetaModel 语义入口。
6. 高级 generator guardrail、多物品 shadow-aware 候选和 Kotlin Gurobi CSV 代表样例适配。

当前基线：

```powershell
cargo test -p ospf-rust-framework-bpp3d --lib
```

默认 feature 下应通过 155 个 lib 测试；启用 `serde` feature 后应通过 178 个测试。

## 4. 下一轮目标

下一轮目标：把当前 fake/no-run/Kotlin 代表样例闭环推进到真实 solver、目录级数据桥接和更长链路验收，尽量一次性覆盖 backend survey、feature-gated smoke/no-run/fallback、Kotlin 数据目录批量迁入、RMP/final 与 column generation 更长链路、ComplexBlock/DFS/MLHS 更真实搜索、Pattern/Pile/Historical 历史层策略，以及 PatternedItem/PackageAttribute/objective/limit 的生产语义深化。

下一轮应尽量拓宽到：真实 Gurobi/SCIP 可用性探测与统一报告、Kotlin 数据目录递归桥接与 manifest 分组/标签、backend survey 与 fake/no-run 对比、RMP/final/column generation 长链路验收、复杂块/DFS/MLHS 多层搜索、Pattern/Pile/Historical 历史层复用、PatternedItem/PackageAttribute 业务约束落入模型、RestAmount/TailBinLoadingRate/BinLoadingOrder 更完整生产表达，以及 README/bpp3d 文档和完整命令验收。

## 5. 下一轮事项

1. 建立真实 solver backend survey/no-run/fallback 统一报告，覆盖 Gurobi/SCIP feature-gated 编译、可用性诊断和回退。
2. 扩展 Kotlin 数据集来源，支持递归目录桥接、manifest 分组/标签和更多 README 范围代表样例批量进入 suite。
3. 将 dataset suite 的 backend survey 与 fake one-round 结果对齐，输出可比较的 objective、coverage、render 和 diagnostics 汇总。
4. 将 PatternedItem/PackageAttribute 从聚合诊断推进到模型约束或 pipeline extension 入口。
5. 将 RestAmountMinimization、TailBinLoadingRateMinimization、BinLoadingOrderConstraint 从最小注册推进到更完整生产表达。
6. 将 ComplexBlock/DFS/MLHS/Pattern/Pile/Historical 推进到多层、多物品、历史层复用和 shadow-price 驱动候选。
7. 同步 README / README_ch / bpp3d.md，并保留默认、serde、async、no-run、scip no-run 验收。

## 6. 下一轮计划

1. 先梳理可用 solver feature 和 Kotlin 数据目录，补齐 backend survey / no-run / fallback 诊断。
2. 再扩展 fixture suite 数据来源和 batch report 对比字段，保证真实/fake/no-run 路径共用同一报告结构。
3. 接着把 PatternedItem/PackageAttribute 约束接入 pipeline 或 extension point。
4. 然后深化 RestAmount/TailBinLoadingRate/BinLoadingOrder 的生产目标/约束表达。
5. 再推进 ComplexBlock/DFS/MLHS/Pattern/Pile/Historical 的多层搜索和历史层策略。
6. 最后跑完整命令验收，更新 README / bpp3d.md，并提交 BPP3D 变更。

## 7. 下一轮清单

必须完成：

1. 真实 solver backend survey/no-run/fallback 报告。
2. Kotlin 数据目录递归桥接或更大代表样例批量迁入。
3. solver feature matrix run/no-run diagnostics。
4. PatternedItem/PackageAttribute 模型约束或 extension point。
5. RestAmountMinimization、TailBinLoadingRateMinimization、BinLoadingOrderConstraint 生产语义深化。
6. ComplexBlock/DFS/MLHS/Pattern/Pile/Historical 多层搜索和历史层策略。
7. solver feature 下的 compile/no-run 验收。
8. README / README_ch / bpp3d.md 更新。

暂不要求：

1. 全量 Kotlin dataset suite 一次性完全对齐。
2. 所有复杂块生成和高级搜索策略达到最优级。
3. Pattern/Pile/Historical layer generator 的最优级完整策略。
4. 任意三维圆柱旋转。

## 8. 下一轮验收标准

功能验收：

1. Kotlin dataset 子集或目录可由 crate manifest/suite 加载，并能批量产生 load/materialize/survey/no-run/fake/backend suite-level 报告。
2. CSV materialized request 能通过 solver-backed application flow，并验证 coverage coefficient 被 RMP/final 使用。
3. RMP LP 能返回 objective 和 demand shadow price，shadow price key 与 demand entry 顺序稳定。
4. 至少一轮 shadow price 驱动的 layer generation/add-columns 可验证，新增层带 placement/block trace 或明确 diagnostics。
5. final MILP 能提取 selected layer/bin assignment，并能从 placement/block trace 生成 packing/render 或 structured diagnostics。
6. PatternedItem/PackageAttribute 业务规则能输出稳定校验结果并接入 fixture diagnostics。
7. RestAmount/TailBinLoadingRate/BinLoadingOrder 生产语义有 MetaModel 测试覆盖。
8. ComplexBlock/DFS/MLHS/Pattern/Pile/Historical 至少有一个更真实候选策略，并保留 request-context diagnostics。
9. CSV/serde fixture 能同时验证 render DTO、diagnostics、PWL、多 fixture suite、feature matrix 和 fake one-round suite。
10. placement/block trace replay 继续由 domain/adapter service 承接，application 只负责流程编排。
11. solver 后端不可用时，feature-gated 编译、feature matrix diagnostics 和 fake/no-run fallback 验收仍明确通过。

命令验收：

```powershell
cargo check -p ospf-rust-framework-bpp3d
cargo test -p ospf-rust-framework-bpp3d --lib
cargo test -p ospf-rust-framework-bpp3d --no-run
cargo test -p ospf-rust-framework-bpp3d --features serde
cargo check -p ospf-rust-framework-bpp3d --features async
cargo test -p ospf-rust-framework-bpp3d --features scip --no-run
```

## 9. 完整迁移完成定义

完整迁移完成仍以以下条件为准：

1. Kotlin README 范围内所有子模块均有 Rust 实现或明确不支持说明。
2. RMP LP、final MILP 和后续 column generation 共享 context / aggregation / pipeline 注册路径。
3. Cuboid、Axis3.Y Cylinder、guarded Axis3.X/Z Cylinder 的 generated/final/render 路径均有测试。
4. PWL continuous radius 注册、提取、renderer 回写闭环可验证。
5. application 层没有大量直接 `model.add_*` 或 token 解析逻辑。
6. CSV dataset suite 在对应 solver feature 下通过。

## 10. 长期延后项

1. 任意三维圆柱旋转。
2. cuboid-only DFS/MLHS 对圆柱的完全 shape-polymorphic 支持。
3. PatternLayerGenerator / PileLayerGenerator / HistoricalLayerGenerator。
4. ComplexBlockGenerator / DFS / MLHS 完整实现。
5. RestAmountMinimization / TailBinLoadingRateMinimization / BinLoadingOrderConstraint。
6. PatternedItem 完整语义。
7. PackageAttribute 完整语义。
8. Gurobi/SCIP 全量 dataset suite。
