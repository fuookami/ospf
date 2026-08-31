# 资源领域

[English](README.md)

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

## 相关目录

- [`../produce`](../produce/README_ch.md)
- [`../task`](../task/README_ch.md)
- [`../../application`](../../application/README_ch.md)
