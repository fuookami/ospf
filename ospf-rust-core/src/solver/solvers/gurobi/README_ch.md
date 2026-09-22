# Gurobi 求解器说明

:us: [English](README.md) | :cn: 简体中文

## 前置依赖

`ospf-rust-core` 的 Gurobi 集成需要：

1. 本机已安装 Gurobi。
2. 已配置有效 Gurobi License。
3. Cargo feature 启用以下之一：
- `gurobi10`
- `gurobi11`
- `gurobi12`

## 主要能力

1. 支持 LP/MIP/QP/MIQP 求解。
2. 支持 stage callback 与 telemetry callback。
3. 支持 native callback 与 native observer。
4. 支持数值诊断与数值策略推荐。

## CP 边界

Gurobi 不提供 CP model component。Gantt task-compilation 先构造 immutable CP snapshot；本 backend
只消费精确的 `ExactLowering` MIP facade，并返回统一 `SolveReport`。原生 optional/variable-duration
interval 以及未经验证的 global-constraint 分解不会被声明为 native CP 能力。

## 原生回调语义

1. `add_native_callback` 为覆盖语义（后者覆盖前者）。
2. `add_native_observer` 为累加多播语义。
3. native observer 可返回 `Terminate` 请求终止求解。

## 函数符号的原生 lowering

函数符号默认展开为通用约束（`Eager` 策略）。当
`SolverConfig::resolved_function_expansion_policy`（或 `MetaModel::set_function_expansion_policy`）
选择 `DeferredNativeFirst` 时，符号可以只保留求解器无关的 `DeferredFunctionStructure`，由本 backend
写成 Gurobi 的 general constraint。

入口是 `GurobiSolver::solve_linear_with_native_lowering(mechanism, options)`，它执行让原生写入成为
可能的两阶段流程：

1. 按 `MechanismModel::linear_column_view()` 建列——列顺序与边界规则和随后的线性三角模型完全一致，
   因此阶段一建立的列**就是**最终模型的列；
2. 把容器与 writer registry 交给 `MechanismModel::lower_deferred_functions`：被 writer 认领的结构
   写成原生约束并从待展开列表丢弃，其余结构一次性物化为通用 fallback；writer 报错时向上传播，
   由调用方对整模型回退；
3. 转换为线性三角模型后，把行与目标装载进**同一个** SDK 模型再求解。

当前注册的 writer：

| writer | schema | 接受的结构 | 原生调用 |
| --- | --- | --- | --- |
| `gurobi_abs` | `functions-abs-1` | 输入是「系数为 1 的单个单项式、常数项为 0」且参数列与结果列不同的 `AbsStructure` | `add_genconstr_abs` |
| `gurobi_max` | `functions-max-1` | 候选全部是「系数为 1 的单个单项式」且共享同一常数项、操作数列与结果列不同的 `MaxStructure` | `add_genconstr_max` |
| `gurobi_min` | `functions-min-1` | 准入规则与 `gurobi_max` 相同的 `MinStructure` | `add_genconstr_min` |
| `gurobi_pwl` | `functions-pwl-1` | 输入是「系数为 1 的单个单项式、常数项为 0」且点表至少 2 个有限、x 严格递增的 `SinStructure` / `CosStructure` / `LogisticStructure` | `add_genconstr_pwl` |
| `gurobi_indicator` | `functions-indicator-1` | 非严格与严格 kind 的 `InequalityStructure`（`Equal`/`NotEqual` 被拒绝：取假侧是析取，两条指示器表达不了） | `add_genconstr_indicator` |
| `gurobi_in_values` | `functions-in-values-1` | 值集合非空、输入是「系数 1 的单个单项式、常数项 0」且条件盒有限的 `InValuesStructure` | `add_genconstr_indicator`（每候选值 4 条）+ `add_genconstr_or`（聚合） |
| `gurobi_and` / `gurobi_or` | `functions-and-1` / `functions-or-1` | 模型层仅在**每个操作数都是直接二值变量**时才暴露结构的 `AndStructure` / `OrStructure` | `add_genconstr_and` / `add_genconstr_or` |
| `gurobi_binaryzation` | `functions-binaryzation-1` | 输入是「系数 1 的单个单项式、常数项 0」且输入盒有限的 `BinaryzationStructure`（Threshold 与 BigM 两种变体都支持；因即时形态与关系指示不同而独立成 writer） | `add_genconstr_indicator`（核心行）+ Big-M 冗余证明 |
| `gurobi_imply` | `functions-imply-1` | 自带两个内部子指示器（前提 / 结论）的 `ImplyStructure`。由于即时的耦合行 `r ≥ c` 是**无条件**行，用 3 条指示约束重建只在**二元域**上等价，因此 writer 会读 SDK 的 `grb::VarType` 校验结果列与两个子指示器列都是二元 | `add_genconstr_indicator`（每个子指示器 2 条 + 耦合 3 条）+ 每个子指示器的 Big-M 冗余证明 |
| `gurobi_conditional_value` | `functions-conditional-value-1` | `ConditionalThenStructure`（条件值符号：条件成立时 result = thenPoly，否则为 0）。折叠条件（分支已恒定）整体回退——Gurobi 没有「把列固定为常数」的一般约束 | 六条等式指示：条件关系（盒证明用符号自带显式有限 `ConditionBounds`）+ 对两个内部列各写的分支等式 |
| `gurobi_masking` | `functions-masking-1` | `MaskingStructure`（二值掩码）：掩码列必须二元、辅助列独占，且冻结 M 须满足 `M ≥ |x|`（对 SDK 输入盒验证；±1e100 显式处理） | 两条等式指示：`m=1 ⇒ y == x`、`m=0 ⇒ y == 0`（即时 `y ∓ M·m` 行由包含证明蕴含） |
| `gurobi_poly_mask` | `functions-poly-mask-1` | `MaskingWithPolyMaskStructure`：同上，掩码定义另写一条普通线性等式行 | 两条等式指示 + 一条线性等式 |
| `gurobi_if` | `functions-if-1` | `IfStructure`（分支选择：b=1 ⇒ result=t、b=0 ⇒ result=e）。即时条件行经分支等式归约，剩余两条义务（`M ≥ max|c|`、`M ≥ max|e−t|`）在 SDK 盒上证明 | 四条指示：条件两行（`b=0 ⇒ c=0`）+ 分支等式两条 |
| `gurobi_balance_ternary` | `functions-balance-ternary-1` | `BalanceTernaryzationStructure`（符号三分支：res=1/0/−1）。两条普通行（`res − pos + neg = 0`、`pos + neg ≤ 1`）经容器 `add_linear_row` 恒等替换；4 条 band 松弛在 SDK 盒上证明 | 两条普通行 + 4 条 band 指示（`pos=1 ⇒ input ≥ ε+sb`、`pos=0 ⇒ input ≤ ε`、`neg=1 ⇒ input ≤ −ε−sb`、`neg=0 ⇒ input ≥ −ε`） |
| `gurobi_not` | `functions-not-1` | `NotStructure`（逻辑非，仅直接二值输入；间接 nonzero-indicator 编码保持 EAGER）。两列都必须是二元列 | 一条普通等式行 `result + input = 1`，经容器 `add_linear_row` 恒等替换 |

> **标注（SEMI / 半连续）：** 半连续是**变量类型层**能力，不属于函数符号原生 writer 范畴：Rust 侧的
> `SemiFunction`/`SemiStructure`（`symbol/functions/semi.rs`）有延迟结构但未注册任何原生 writer，
> 始终以 EAGER 物化；Kotlin 侧的 `GurobiNativeSemi` 以 `GRB.SEMICONT`（边界 + 变量类型）原生承载。
> 未来如需对齐，应沿变量类型路径实现，而非 genconstr/行写入。

所有 writer 写入前都施加同一组门控：

- 结果列已被固定（`FunctionUsageSummary::forbids_native_write`）→ 拒绝：代换结果列会改变原生结构的含义；
- 辅助列被本函数自身关系行之外的引用触及（`helpers_are_exclusive` 为假）→ 拒绝：原生关系只约束结果列，
  分段选择列/分支列一旦被模型别处引用就会静默失去含义；
- writer 无法精确表达的形态同样拒绝：需要桥接列的一般仿射输入、常数项不一致的候选、操作数列与结果列
  重合。

`gurobi_pwl` 还多一道只对分段线性写入适用的门控——**范围证明**：即时展开把 `x` 写成断点的凸组合
（`x = Σ λᵢ xᵢ`），等于把输入钉在 `[x₀, xₙ]` 内；而原生 PWL 约束会在区间外**外推**。因此 writer 写前从
SDK 模型读取输入列的**实际界**（`Model::get_obj_attr(attr::LB/UB, &var)`，且必须先 `Model::update()`——
新建列在 Gurobi 的 lazy update 模式下是 pending 对象，属性读取会失败），只有 `lb ≥ x₀ - 1e-9` 且
`ub ≤ xₙ + 1e-9` 才原生写入，否则回退。**读 SDK 的实际界而不是令牌声明界**正是关键：求解模型才是权威。

`gurobi_indicator` 还多一道领域专属门控——**Big-M 冗余证明**。关系指示的即时展开对每个指示值写两条行：
一条核心行（原生指示器用同一个 `INDICATOR_TOLERANCE` 精确复现）与一条被 Big-M 松弛的行。只写核心行会
**放大可行域**，因此 writer 写入前必须在条件变量的**实际盒**上证明松弛行恒成立：从 SDK 界
（`get_obj_attr(attr::LB/UB)`）算出 `s = Σ cₖxₖ + constant` 的 `s_min`/`s_max`，并**显式与 `grb::INFINITY`
比较**——因为 Gurobi 用**有限值 `±1e100`** 表示无穷界；盒无界或读不到界即回退。容差为具名常量
`GUROBI_INDICATOR_BIG_M_TOLERANCE`。**只由 Big-M 编码的行绝不被静默丢弃。**

被拒绝不是错误：同一次 lowering 会把它回退为通用展开。而**写入失败**（区别于拒绝）会触发**整模型回退**：
此时已建好的 SDK 模型可能含有无法逐条回滚的原生约束，求解器丢弃它、用同一份机制模型沿通用路径重建，
求解仍然成功，报告则如实记录全部结构因写入失败而回退。

实现要点：

- `GurobiNativeContainer` **按值持有** Gurobi 模型：writer registry 要求容器类型满足 `'static`，
  借用式容器带生命周期参数无法满足；写入结束后用 `into_parts()` 取回模型继续装行。
- 原生路径**不新增也不删除任何公开列**；未被 writer 认领的结构仍然得到 fallback 行，因此不会出现
  「模型局部原生、关系却没写完整」的情况。
- 原生写入的参数是**输入单项式指向的列**，不是符号的分支指示列；两者混淆会写出语义错误的关系，
  而这种错误只有真实求解测试才能发现。`plan_abs_native` 刻意保持不依赖 SDK，因此准入判定在没有
  许可证的机器上也能测。

该路径的验收必须是**真实求解**（因此需要上面的许可证）：

```bash
cargo test -p ospf-rust-core --features gurobi10 --test gurobi_native_function_lowering
cargo test -p ospf-rust-core --features gurobi10 --test native_release_matrix -- --include-ignored
```

第一条命令断言：ABS 结构恰好被原生写入一次、没有为它物化任何 fallback、真实解满足 `y = |x|` 与
模型自身约束、且 EAGER 与原生两条路径的可行性一致。当前哪些符号提供结构见 core README 的
[函数符号支持矩阵](../../../../README_ch.md#函数符号支持矩阵)。

## 最小验证命令

在 workspace 根目录执行：

```bash
cargo test -p ospf-rust-core gurobi_native_observer_integration --features gurobi10 -- --nocapture
cargo test -p ospf-rust-core gurobi_telemetry_callback_integration --features gurobi10 -- --nocapture
cargo test -p ospf-rust-core gurobi_stage_callback_integration --features gurobi10 -- --nocapture
```

当前 workspace 按本机 Gurobi 10 环境验证。若使用 Gurobi 11/12，
请将 `gurobi10` 替换为 `gurobi11`/`gurobi12`。

共享 native contract 还会比较 report identity、best bound、gap、solution value 和 constraint residual。
缺少许可证属于 `LICENSE` 错误（包括原生错误码 `10009`），不得当作环境跳过后成功。详见
core README 中的 [Solver 原生验收矩阵](../../../../README_ch.md#solver-原生验收矩阵)。
