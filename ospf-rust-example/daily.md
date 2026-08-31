# ospf-rust-example Daily

## 任务主题
在 `ospf-rust-example` 中实现 `framework-demo4`（参考 Kotlin demo4，缺失部分以 `E:\workspace\fsra-proof` 为准）。

## 事项拆解
1. 规则与基线确认
- 读取并遵循仓库规则：`.clinerules/chore.md`。
- 确认目标范围：仅 `ospf-rust-example` 的 framework demo4。
- 产出：模块映射清单（Kotlin/demo4 -> Rust/demo4）。

2. 模块骨架与入口接入
- 新建 `src/framework/demo4` 目录与 `mod.rs`。
- 接入 `src/framework/mod.rs` 导出 `run_demo4`。
- 接入 `src/main.rs` 命令 `framework:demo4`。
- 产出：`cargo check` 可通过，命令入口可执行。

3. 基础数据模型与 DTO
- 落地 `infrastructure/dto` 的 `Input`/`Output`。
- 落地 `task/rule/passenger` 的核心模型最小集。
- 定义统一错误类型与结果返回约定。
- 产出：可完成最小初始化与样例数据加载。

4. Context 初始化链路
- `FlightTaskContext`：完成聚合初始化。
- `RuleContext`：完成可行性/连接时间/最早起飞/成本计算接口。
- `BunchCompilationContext`：完成 register/construct 基础流程。
- `PassengerContext`：完成 register/construct 与成本叠加接口。
- 产出：上下文初始化与模型注册流程可跑通。

5. 列生成主流程（MVP）
- 实现简化循环：`RMP(LP) -> SP -> addColumns -> IP校验`。
- 先不引入复杂 fix/keep/remove 规则，只保留必要收敛条件。
- 产出：单案例可迭代并结束，能得到可解释输出。

6. 约束与策略补全
- `bunch_compilation`：补 `FlightLink/FleetBalance` 等核心限制。
- `passenger`：补容量、改签、取消约束与相关目标项。
- 引入 `free executor` 选择与局部/全局加列策略。
- 产出：与 fsra 流程语义一致的可用版本。

7. 稳定性与可观测性
- 增加迭代统计（主问题建模/求解耗时、子问题耗时、列数）。
- 增加关键日志与失败分支错误信息。
- 产出：可定位问题的运行日志与性能摘要。

8. 测试与文档
- 新增 smoke test：`framework:demo4` 可运行。
- 新增核心回归：影子价格提取、加列、约束生效。
- 更新 `README.md` 与 `README_ch.md`（双向超链接）。
- 产出：可复现运行步骤与测试说明。

## 分阶段计划

### Phase 1（骨架可编译）
- 完成事项 1-2。
- 验收：`cargo check -p ospf-rust-example` 通过；`framework:demo4` 命令可进入 demo4。

### Phase 2（最小可运行）
- 完成事项 3-5。
- 验收：demo4 可跑通一组样例，得到可解析输出。

### Phase 3（能力补全）
- 完成事项 6-7。
- 验收：关键约束、列管理策略、统计日志完整。

### Phase 4（收尾交付）
- 完成事项 8。
- 验收：测试通过，README 中英文完成并互链。

## 今日执行顺序（建议）
1. 建立 `demo4` 骨架并接入入口。
2. 先打通 `Context` 初始化与最小 RMP/SP 闭环。
3. 再逐步补约束和策略，期间持续加测试。
4. 最后整理文档与示例命令。

## 风险与对策
- 风险：Kotlin demo4 本身不完整。
- 对策：以 `fsra-proof` 同名上下文和服务作为行为来源。

- 风险：一次性迁移范围过大导致长时间不可运行。
- 对策：坚持 MVP 先行，按 Phase 分段提交可运行版本。

- 风险：求解器差异导致结果波动。
- 对策：先固定样例和容差，测试以“可行+约束满足+趋势”验收。

## 依赖确认与当前决策（2026-03-29）
- 已确认 Kotlin `framework_demo/demo4` 依赖 `gantt-scheduling`：
- 代码层面：`domain/bunch_compilation/BunchCompilationContext.kt` 直接 import/继承 `fuookami.ospf.kotlin.framework.gantt_scheduling...`。
- 构建层面：`ospf-kotlin-example/pom.xml` 依赖 `ospf-kotlin-starter-gantt-scheduling`。
- 当前决策：`ospf-rust-example` 的 `framework-demo4` 暂缓实现，先不进入 Phase 2+ 开发。
- 保留状态：已完成 Phase 1（入口与骨架接入，编译通过）。
