# BPP3D 层分配 Context

[English](README.md)

该 context 对应 Kotlin `bpp3d-domain-layer-assignment-context`。

## 职责

layer_assignment 持有 RMP 和 final MILP 赋值组件、需求负载表达式、容量表达式、动态层列生命周期、约束、目标和 shadow price 提取。

## 文件结构

- `model.rs` 是公开建模 helper shim；variable array、expression array、solution extraction 和 component trait 拆在 `model/` 下。
- `service/mod.rs` 是公开 service shim；value adapter、assignment model、load/capacity model、iterative context、aggregation 和 context registration 拆在同级文件。
- `service/limits.rs` 是公开 limits shim；每个 Kotlin-style limit/objective 位于 `service/limits/` 下。

## 扩展点

新增约束或目标应在 `service/limits/` 下实现 `Pipeline<MetaModel<f64>>`。add/remove/hide/fix/flush 的列生命周期行为保留在 `IterativeLayerAssignmentContext`。

## 验证

运行 `cargo test -p ospf-rust-framework-bpp3d --lib`；涉及 solver 集成时同步运行 serde feature check 和 backend `--no-run` 构建。
