# 算子族

[English](README.md) | 中文

算子是元启发式中可替换的部分。每个算子族都是一个 trait 加若干无状态实现；随机性与熵源一律由调用方注入，
算子自身从不持有。

## 算子族

| 模块 | 契约 | 职责 |
| --- | --- | --- |
| `initializer` | `Initializer` | 生成初始解 |
| `fitness` | `FitnessEvaluator` | 把解转成个体并拒绝非有限目标 |
| `ranking` | `Ranking` | 个体排序 |
| `boundary` | `BoundaryHandler` | 截断、反射或环绕越界值 |
| `topology` | `Topology` | 定义邻域关系 |
| `diversity` | `DiversityMetric` | 度量种群分布 |
| `acceptance` | `Acceptance` | 判断候选解是否取代当前解 |
| `stochastic` | `LevyFlight`、`NoiseStep` | 重尾步长与逐维噪声 |
| `evolution` | `ParameterSource` | 参数随迭代演化 |
| `neighborhood` | `Neighborhood` | 生成单解邻域 |
| `survivor` | `SurvivorSelection` | 合并父代与子代 |
| `migration` | `IslandMigration` | 岛屿间交换个体 |
| `linear` | `SymmetricEigen` | 对称矩阵特征分解（Jacobi） |
| `update` | `SolutionUpdate` | 交叉与位置更新 |
| `mutation` | `Mutation` | 逐基因变异 |
| `rl` | `StateDiscretizer`、`ActionSelector`、`QTable` | 强化学习驱动的选择 |
| `multi_objective` | `Dominance`、`FastNonDominatedSort`、`CrowdingDistance`、`ParetoArchive`、`ReferencePoints` | Pareto 工具 |
| `archive` | `Archive` | 维护最优集合与改进信号 |
| `stopping` | `StoppingCriterion` | 决定何时停止 |

## 共享工具

`operator/mod.rs` 提供所有算子族共用的样板：`compare_strictly`（双向比较规范化）、`value_at` / `set_value_at`
（规范顺序读写）、`bounds_of`、`clamp_to`、`clamped_value`、`pick_two`、`unit`、`should_fire`、
`uniform_between` 与 `clamp_unit`。

## 约定

- 算子无状态；熵源永远是参数。
- 非有限值在评估入口被拒绝、在边界处理处被夹紧，绝不进入档案。
- 位置语义一律经 `VariableModel::variable_order()`。
- 环境选择用「逐个挑最优」而不是排序，因为模型比较器可能不具传递性。

## 相关文档

- [模块总览](../README_ch.md)
- [骨架说明](../runner/README_ch.md)
