# Framework Demo4 - 航班排班调度

:us: [English](README.md) | :cn: 简体中文

## 简介

demo4 演示基于 gantt_scheduling 框架的**航班排班调度**。它建模航班恢复场景，在满足值勤时间限制、连接规则和机队平衡约束的前提下将飞机分配到航班任务。该示例还包含泛型物理量示例，展示如何在排程各维度使用框架类型。

## 作用范围

- 建模飞机的类型、容量和成本费率。
- 定义航班任务、航段和恢复场景。
- 生成飞行任务束（可行值勤序列）。
- 编译任务束为航班链接排程。
- 应用机队平衡和容量约束。
- 使用泛型物理量表示时间、成本、资源容量和切换。

## 领域模型

### task/ - 飞行任务

| 结构体 | 说明 |
| --- | --- |
| FlightTaskImpl | 实现 TaskTrait，包含时间窗、执行者、状态 |
| Aircraft | 实现 ExecutorTrait，包含类型、容量、成本费率 |
| FlightLeg | 具体航班任务，含出发/到达机场 |
| FlightTaskBunch | 任务束（列生成中的列） |
| Airport | 机场，含 ICAO 代码、类型、中转时间 |
| FlightLegPlan | 航班计划，含计划/预计/实际时间 |

### crew/ - 机组

| 结构体 | 说明 |
| --- | --- |
| Crew | 机组成员组合 |
| Pilot | 飞行员及其职级 |
| CrewMan | 机组成员及其职级 |

### passenger/ - 旅客

| 结构体 | 说明 |
| --- | --- |
| Passenger | 旅客及航线 |
| PassengerAmount | 旅客数量符号注册 |
| PassengerCancel | 旅客取消符号注册 |
| PassengerChange | 旅客变更符号注册（class_change + flight_change） |

### rule/ - 规则

| 结构体 | 说明 |
| --- | --- |
| FlowControl | 流量控制限制 |
| Link | 航班链接（连接时间） |
| Restriction | 业务限制 |

### bunch_generation/ - Bunch 生成（核心算法）

| 结构体 | 说明 |
| --- | --- |
| Graph | 路线图（Root -> Task -> End） |
| Node | 图节点：Root、Task{task_id, time, index}、End |
| Edge | 节点间有向边 |
| FlightTaskReverse | 可逆任务对管理 |
| RouteGraphGenerator | BFS 路线图构建 |
| FlightTaskBunchGenerator | Label Setting 定价算法 |
| InitialFlightTaskBunchGenerator | 初始束生成 |
| AggregationInitializer | 初始化编排 |

### bunch_compilation/ - Bunch 编译

| 结构体 | 说明 |
| --- | --- |
| Compilation | 编译结果（bunch_id, flights, aircraft_type, cost） |
| FleetBalance | 机队平衡，含检查点和松弛变量 |
| FlightCapacity | 航班旅客/货物容量符号 |
| FlightLink | 航班链接，含连接时间和松弛变量 |

### bunch_selection/ - Bunch 选择

| 结构体 | 说明 |
| --- | --- |
| BranchAndPriceAlgorithm | 分支定价求解器，含迭代上限和容差 |

## 列生成流程

`
                    +---------------------------+
                    |    AggregationInitializer  |
                    |  1. 构建 FlightTaskReverse |
                    |  2. 构建 RouteGraph (BFS)  |
                    |  3. 生成初始束             |
                    +---------------------------+
                                |
                                v
+----------+    +-----------------------+    +------------------+
|  主问题  |    |     定价              |    |  编译            |
|  (RMP)   |<---|  (Label Setting)      |--->|  (约束注册)      |
|          |    |                       |    |                  |
+----------+    +-----------------------+    +------------------+
     |                   ^                         |
     | Shadow Price      |                         |
     +-------------------+                         |
     |                                             |
     | 更新列                                       |
     +---------------------------------------------+
`

1. **初始化**（AggregationInitializer）：
   - 从可逆任务对构建 FlightTaskReverse。
   - 为每架飞机从所在机场出发，通过 BFS 生成 RouteGraph。
   - 通过 InitialFlightTaskBunchGenerator 生成初始束，确保 locked task 被覆盖。

2. **定价**（FlightTaskBunchGenerator）：
   - 使用 Label Setting 算法遍历路线图。
   - 沿出边扩展标签，累加任务成本和 shadow price 扣减。
   - 支配剪枝：在同一末端节点，仅保留时间和 reduced cost 均不劣于其他标签的标签。
   - 输出 reduced cost < 0 的束。

3. **主问题**（RMP）：
   - 用当前列求解受限主问题。
   - 从机队平衡和航班链接约束中提取 shadow price。

4. **编译**（BunchCompilationContext）：
   - 注册机队平衡约束：每个机场到达航班 - 出发航班 = 期望平衡值。
   - 注册航班链接约束：连续航班之间连接时间 >= 最小连接时间。
   - 注册航班容量约束：每个航班的旅客/货物上限。

5. **迭代**：
   - 将 shadow price 反馈给定价步骤。
   - 添加 reduced cost < 0 的新列。
   - 重复直到无改进列，然后执行最终 MILP 求解。

### 模块间边界

| 模块 | 承担 | 不承担 |
| --- | --- | --- |
| unch_generation | 路线图、初始束、定价 | 主问题约束、机队平衡、结果解析 |
| unch_compilation | 主问题约束注册、机队平衡、航班链接 | Label Setting、路线图、reduced cost |
| unch_selection | 分支定价编排、shadow price 提取、加列 | 具体定价逻辑 |

## Shadow Price 与 Reduced Cost

### Shadow Price

**Shadow price**（对偶值）是主问题约束的边际成本，衡量约束放宽一单位带来的目标改善。在列生成语境中：

- 每个**机队平衡约束**的 shadow price 反映在某机场多要求或少要求一架飞机的边际成本。
- 每个**航班链接约束**的 shadow price 反映连接要求的边际成本。

Shadow price 在每次迭代求解 RMP 后提取，传入定价子问题。

### Reduced Cost

**Reduced cost** 衡量候选列（束）对当前 RMP 解的改进潜力：

`
reduced_cost = original_cost - sum(shadow_price_i * contribution_i)
`

其中：
- original_cost 是束的总运营成本（燃油、机组、延误等）。
- contribution_i 是该束对约束 i 的贡献（如覆盖航班 i、在机场 j 使用机型 k）。
- shadow_price_i 是约束 i 的对偶值。

**判定规则**：
- 若 
educed_cost < 0，添加该列可改善目标。FlightTaskBunchGenerator 将其输出。
- 若所有候选列的 
educed_cost >= 0，当前解已最优，迭代终止。

### 支配剪枝

Label Setting 中，同一末端节点上的两个标签做比较。标签 A **支配**标签 B 当：
- A 访问的任务数不少于 B。
- A 的 reduced cost 不大于 B。
- B 访问的每个任务也在 A 的访问集中。

被支配的标签被剪枝，避免冗余探索。

## 泛型物理量示例

pp.rs 入口演示如何使用框架泛型类型：

| 类型 | 说明 |
| --- | --- |
| TimeRange | 带起止时间的时间区间 |
| Cost | 成本数量 |
| TimeWindow | 带时长和时刻值转换的时间窗 |
| WorkingCalendar | 工作日历，支持实际时间范围查询 |
| FlightHour / FlightCycle | 飞行小时和飞行循环 |

## 使用方式

`powershell
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo4
`

## 测试

`powershell
cargo test -p ospf-rust-example --features backend-gurobi -- bunch_generation
`

覆盖场景：
- Graph：节点/边操作、路径查询
- FlightTaskReverse：可逆对、对称检测
- RouteGraphGenerator：路线图构建、可行性检查、对换边
- FlightTaskBunchGenerator：负 reduced cost 列生成、无可行列、支配剪枝
- InitialFlightTaskBunchGenerator：locked task 覆盖、空束

## 相关模块

- [OSPF Rust Example README](../../README_ch.md)
- [OSPF Rust Root README](../../../README_ch.md)
