# Demo2 航空配载

简体中文 | [English](README.md)

`FullLoadApplication`、`PredistributionApplication`、`WeightRecommendationApplication`
统一委托 `Demo2ApplicationService`，共享完整领域注册，不再维护手写 master/sub 业务双模型。
`LoadingOrderApplication` 仍是独立的货物列表导出入口。

## 模型与扩展边界

- `domain/shared/loading_context.rs` 将显式 DTO 转成 BPP1D（B737/B757）或 BPP2D
  （B767/B747），注册稀疏分配、整体占位、布局，再将全部航空管线注册到同一个 `MetaModel`。
- `model_registration.rs` 只为既有航空管线提供投影；非法配对统一使用固定零列，不创建另一套 dense 决策。
- 业载、累积载重、包络、目标纵向力矩、横向平衡和可选宽体 CLIM 均使用含附加资源的完整重量。
- `recommendation.rs` 最大化完整业载，并对整数推荐重量建立两两硬均衡，按实际装载状态放松；
  允许实装／调整的舱位不能同时实装和推荐，保留固定 Kotlin 来源的可行域。
- 新规则应在领域 context/pipeline 中、`RegisteredDiscreteLoading::new` 之前注册；
  不在 solver callback 内拼业务模型，保证三种策略共享约束、范围与目标。

请求是明确的 **f64 适配边界**，不是泛型物理量 API。重量为 kg、力臂为 m、力矩及 CLIM 为 kg*m。
几何显式选择米、厘米或英寸，集中转换为 `Quantity<f64, Meter>`；共享装载重量使用
`Quantity<f64, Kilogram>`。不进行跨数值域证明提升。

## 输入合同

以 `Demo2Request::sample()` 为起点，填写 `request.loading`：

- `positions` 必须精确覆盖舱位 ID，并声明物理 `space_id`、正的显式几何边界、
  `loading_order` 和 `loading_order_scope`。窄体按纵向区间、宽体按矩形判交；接触不是重叠。
  不按面积或列表顺序猜几何、甲板或布局。
- `cargos` 可覆盖必装状态、合法舱位集合、物理空间和严格货物顺序。空合法集合就是不可分配。
  未覆盖时，预分配货物默认必装，另两模式默认可选；优先级本身不会凭空变成必装条件。
  显式注册的其他业务约束仍然生效。
- `loaded_items` 按身份固定货物。未知引用、重复固定归属和未知合法舱位均报错。
  禁用舱位不接收新货，但不删除合法固定货；stowage/adjustment 标志控制新实装资格。
- 固定占用、`empty_forbidden`、附加互斥、恰选一布局组全部显式声明。
  空禁在**三种模式一致**约束整体占位，不误当作件数限制；共享 BPP 合同允许空预留。
  固定占位不等于实际货物：actual-loaded 只由已分配货物推导，固定空预留仍可接收推荐重量。
- `loading.adjacent_positions` 使用稳定舱位身份，优先于旧的下标字段。
  注册与提取前按显式业务顺序归一化；仅重排请求列表不会改变已命名合同。
  优先顺序和来源提前位置集合均在显式顺序作用域内定义，重命名作用域不会创造整机装载先后。
- `predicate_weight` 显式开启有界连续预测资源；重量推荐模式的 `recommended_weight`
  开启有界**整数 kg** 推荐资源。不会从旧 `predicate_load_weight_min` 提示隐式开启资源。
  两者都接入真实容量和整体占位，正附加重量不能藏在未占用舱位。
- `max_clim` 为可选的显式宽体硬限制，与包络及目标力矩限制独立；窄体声明 CLIM 会被拒绝。

## 策略与证据

1. 普通 MILP：求解完整源快照，复核变量范围、整数性、全部航空行、布局与几何。
2. 安全 Benders：只划分同一展开源模型，全部整数决策（包括推荐重量）留在 master。
   证书或精确可表示能力不足时保留明确终态；允许回退时，使用独立 context／模型身份的
   全新等价完整 MILP 工厂，且仅使用总预算的剩余时间。输入、回调错误或取消不会触发回退。
3. 独立分区 CG：开启 `partition_column_generation` 并声明完整唯一的 `partitions`，
   不得同时优先 Benders。附加资源有显式舱位归属；独立补全阶段恢复整数性且冻结 assignment／occupancy。
   投影保留完整源行。这是根节点 CG 加受限整数池启发式，**不是 branch-and-price**；
   即使源问题可行，最终受限池也可能不可行。

### 完整源收尾与认证

`solve_policy.source_milp_finishing` 显式控制 Benders／CG 结束后的完整源求解：

| 策略 | 行为 |
| --- | --- |
| `SourceMilpFinishingPolicy::IfUnproven`（默认） | 尚无完整源最优或不可行／无界终局证明时，在剩余预算内使用**全新等价完整源 MILP**继续求解与认证。 |
| `SourceMilpFinishingPolicy::Disabled` | 不追加完整源求解，保留加速阶段真实结果、终止原因及算法报告；仍复核完整源身份、向量可行性和目标，不提升受限池证明。 |

启用收尾时，`loading.acceleration_time_ratio` 严格位于 0 与 1 之间，默认 0.5：加速阶段
最多使用进入阶段时剩余预算的一半，其余预留给源认证。关闭收尾时忽略该比例、不为收尾
预留时间，加速阶段可使用全部剩余预算（仍受 CG 自身 `column_generation.time_limit` 限制）。
工厂、复核与求解始终计入同一总时限。直接 MILP 不受收尾策略影响。

纯 Benders／CG 对比可这样配置；关闭收尾不保证能获得方案或全局最优证明：

```rust
request.solve_policy.source_milp_finishing = SourceMilpFinishingPolicy::Disabled;
request.solve_policy.benders_fallback_to_milp = false; // 同时关闭 Benders 内部回退 / Also disable internal Benders fallback.
```

既有源可行向量经完整复核后，按已核对等价的列绑定作为 warm start；不搬运旧 token、
对偶或受限池证明。源收尾无解、限时或得到更差 incumbent 时保留更好的已验证方案。
实际收尾报告独立保存在 `algorithm_reports`；只有完整源求解给出可靠证明，最终状态才为
Optimal。与已验证方案矛盾的不可行／更差最优报告会被拒绝，不静默覆盖已有证据。

启用收尾时，CG 受限池无解、缺少 LP 最优／对偶证据，以及 Benders 能力不足，均可进入完整源收尾。
关闭收尾后，已有方案仍按完整源模型复核；无方案时保留真实未知／限时等终态，未被算法
内部处理的后端／能力错误直接传播，不通过隐式源求解恢复。
已有但无效的证书、错误映射、建模／输入／回调合同错误仍直接报错，不放宽证书门禁。
旧 `benders_fallback_to_milp` 仅控制加速阶段内部提前回退，与收尾策略独立：即使关闭收尾，
该开关为 true 时仍允许 Benders 内部普通 MILP 回退。若要禁止 Benders 后所有普通 MILP，须同时关闭两者。
取消／外部中断及总预算耗尽不启动新求解；加速阶段自己的局部时限不等于总预算耗尽。
源认证后端失败时保留已验证方案并记录明确失败终态，不伪造最优证明。

实际启动收尾时，`framework_path` 使用 `benders_source_milp` 或
`partition_column_generation_source_milp`；若内部 fallback 已获得完整源终局证明则不重复求解。
启用收尾实现完整源问题的求解／认证闭环，但不将根 CG 本身称为 branch-and-price。

`solve_report` 保留实际源问题状态、终止原因、解存在性、证明与审计身份；
`algorithm_reports` 独立保留受限、补全、master/sub 和 fallback 实际报告。
Benders／CG 阶段的源方案保守标为 Feasible；完整源收尾可以提供独立可靠最优证明。
只有 `solve_report.is_optimal()` 为真才输出 Optimal；源 Infeasible／Unbounded 及时间、
取消等终态不会统一抹成 NoSolution，受限池不可行也不会提升成源不可行。

`objective` 是包含常数一次的**最小化口径完整源目标**，不是单纯货重或受限主问题目标。
旧 quality overrides 仅根据实际可得下界、割数等生成观察诊断，不虚构轨迹、不丢 incumbent、
不覆盖安全回退策略。安全固定追索未实现的旧停滞开关明确报告为 inactive。

## 回调与结果

三入口保留 `execute(request)`，新增 `execute_with_handle` 和
`execute_with_options(Demo2ExecutionOptions)`。选项支持共享取消句柄、阶段心跳、
完成回调和最终渲染。回调位于后端恢复边界之外，错误直接传播；最终提取后只完成通知一次。

分配、已选布局、整体占位、附加 kg 重量、render 和 KPI 均来自**最终保留的实际注册**，
涵盖 fallback 或被保留的限时 incumbent，不从中间 master 或被淘汰的回退候选提取。
`with_render=false` 不生成渲染，但保留 KPI 与求解证据。

## 验证

配置可用的 Gurobi 10 runtime／license 后：

```powershell
cargo +nightly-2026-08-01 test -p ospf-rust-example --features backend-gurobi framework::demo2 -- --test-threads=1
cargo +nightly-2026-08-01 run -p ospf-rust-example --features backend-gurobi -- framework:demo2
```

`migration_tests.rs` 覆盖三模式／策略、显式宽体几何、稀疏／固定／禁用资格、目标力矩与 CLIM、
整数推荐和硬均衡、请求重排、全新回退、独立 CG、真实报告、限时方案、回调失败与最终 render／KPI。
根节点 CG 回归明确区分源可行性与受限池证明，不把“正常返回”冒充完整最优性。
