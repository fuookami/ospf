# 搜索骨架

[English](README.md) | 中文

骨架只负责生命周期：建立初始状态、推进一步、把状态摊平为个体。循环、档案、终止与观测者只实现一次，位于 `drive`。

## 循环

```text
state = initialize(model, iteration)
archive.update(members(state))
iteration.next(true)
while !stopping.should_stop(iteration):
    state = advance(model, iteration, state)
    improved = archive.update(members(state))
    broadcast observers(iteration, members(state), improved)
    model.flush()
    iteration.next(improved)
return archive.individuals()
```

`improved` 是停滞信号的唯一来源；`iteration.next(improved)` 据此维护 `not_better_iteration`。

## 骨架

| 骨架 | 状态 | 一步 | 典型算法 |
| --- | --- | --- | --- |
| `PopulationRunner` | 种群 | 更新 → 变异 → 环境选择 | GA、DE、GWO、WOA、HS 等 |
| `TrajectoryRunner` | 单个个体 | 邻域 → 接受判据 | 爬山、模拟退火、邻域搜索 |
| `ParticleSwarmRunner` | 粒子（位置、速度、个体最优） | 速度更新 | PSO、APSO |
| `IslandRunner` | 多个种群 | 各岛推进一代 + 周期迁移 | 岛屿 GA |
| `Nsga2Runner` / `Nsga3Runner` | 多目标种群 | 繁殖 → 非支配排序 → 拥挤距离 / 参考点 | NSGA-II、NSGA-III |

`PopulationRunner` 另外暴露 `initialize_island` / `step_island`，`IslandRunner` 正是借此**组合**它，而不是重写种群逻辑。

## 约定

- 串行执行：个体逐个推进，固定 seed 可复现同一次运行。
- `drive` 接收熵源并传给 `advance`；骨架自身不持有随机性。
- 档案是唯一的收敛载体，骨架不另存「最优」。
- 每次推进后广播观测者，包括最初一次的档案更新。

## 相关文档

- [模块总览](../README_ch.md)
- [算子族](../operator/README_ch.md)
