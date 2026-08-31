# 资源领域

:us: [English](README.md) | :cn: 简体中文

本目录建模执行资源、存储资源和连接资源，对应 Kotlin `gantt-scheduling-domain-resource-context`。

## 职责

- 定义资源 trait 和具体资源类型。
- 定义资源使用量、松弛和容量模型组件。
- 提供资源容量约束和数量目标。

## 模块

- `model/`：资源 trait、容量、使用量和松弛组件。
- `service/limits.rs`：资源容量约束和数量目标。

## 公共 API

- `ResourceCapacity`
- `ResourceSlack`
- `ResourceTrait`
- `ExecutionResourceTrait`
- `StorageResourceTrait`
- `ConnectionResourceTrait`
- `BasicExecutionResource`
- `BasicStorageResource`
- `BasicConnectionResource`
- `ResourceUsage`
- `StorageResourceUsage`
- `ConnectionResourceUsage`
- `ResourceCapacityConstraint`
- `ResourceOverQuantityMinimization`
- `ResourceLessQuantityMinimization`

## 扩展点

资源语义通过 resource trait、执行/存储/连接资源模型、resource usage component、slack component 和资源数量 limit pipeline 扩展。物料产出与消耗保留在 `produce`。

## 生命周期与数据流

resource definition 提供容量与身份，usage component 将资源消耗注册到 `MetaModel`，slack component 表达受控违约，limit pipeline 约束或优化资源数量。

## 验证

修改 resource trait、usage/slack component、capacity constraint 或 resource quantity objective 时运行 `cargo test -p ospf-rust-framework-gantt-scheduling --lib`。

## 相关目录

- [`../produce`](../produce/README_ch.md)
- [`../task`](../task/README_ch.md)
- [`../../application`](../../application/README_ch.md)
