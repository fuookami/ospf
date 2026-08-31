# Kotlin ↔ Rust Demo 一致性检查报告

## 目标
重新阅读 Kotlin 的 example 实现（core_demo + framework_demo），对照 Rust 版本（ospf-rust-example/src/core + framework/demo1~4），检查每个决策对象的 **变量** 和 **中间值** 是否与 Kotlin **语义一致**。最终达到完全一致。

## 检查范围
- core demo 1-17 的变量类型、中间符号类型/数量/命名
- framework demo1 的 4 个 model struct (EdgeBandwidth, NodeBandwidth, ServiceBandwidth, Assignment)
- framework demo2/3/4 的模型 struct 简要检查（因 Phase 9 后未主动迁移，以旧 API 风格运行）

---

## 1. Core Demo 逐项检查清单

| Demo | Kotlin 变量 | Rust 变量 | 一致? | Kotlin 符号 | Rust 符号 | 一致? | 差异说明 |
|------|-------------|-----------|-------|-------------|-----------|-------|---------|
| 1 | `x: BinVariable1` | `select: VariableCombination1D<Binary>` | ✅ 语义等价 | `capital, liability, profit: LinearExpressionSymbol<Flt64>` (3独立) | `metrics: SymbolCombination<...,Shape<1>>` (3合1) | ✅ | Kotlin 3个独立字段，Rust 1个组合。功能等价。 |
| 2 | `x: BinVariable2` | `x_vars: VariableCombination2D<Binary>` (局部) | ✅ | `cost: LES`, `assignmentCompany/Product: LinearExpressionSymbols1` | `cost, assignment_company, assignment_product: SymbolCombination` (局部) | ✅ | 命名风格不同（camelCase vs snake_case），结构等价。 |
| 3 | `x: UIntVariable1` | `x: VariableCombination1D<UContinuous>` | ❌ 类型不匹配 | `cost: LinearIntermediateSymbol`, `yield: LinearIntermediateSymbols1` | `cost, yields: SymbolCombination` | ✅ | **Kotlin x 是 UIntVariable1（整数）**，Rust 是 UContinuous（连续）。 |
| 4 | `x: RealVariable1` | `x: VariableCombination1D<UContinuous>` | ✅ | `profit: LinearIntermediateSymbol`, `use: LinearIntermediateSymbols1` | `profit, usage: SymbolCombination` | ⚠️ 命名 | `use` vs `usage` 命名不一致，应统一。 |
| 5 | `x: BinVariable1` | `x: VariableCombination1D<Binary>` | ✅ | `cargoWeight, cargoValue: LinearIntermediateSymbol` | `total_value, total_weight: SymbolCombination` | ⚠️ 命名 | Rust 命名 `total_value/total_weight`，Kotlin `cargoWeight/cargoValue`。 |
| 6 | `x: UIntVariable1` | `x: VariableCombination1D<UInteger>` | ✅ | `cargoWeight, cargoValue: LinearIntermediateSymbol` | `total_value, total_weight: SymbolCombination` | ⚠️ 命名 | 同 demo5，命名不一致。 |
| 7 | `x: UIntVariable2` | `x_vars: VariableCombination2D<UContinuous>` (局部) | ❌ 类型不匹配 | `cost: LinearIntermediateSymbol`, `shipment/purchase: LinearIntermediateSymbols1` | `cost, shipment, purchase: SymbolCombination` (局部) | ✅ | **Kotlin x 是 UIntVariable2（整数）**，Rust 是 UContinuous（连续）。 |
| 8 | `x: UIntVariable1` | `x: VariableCombination1D<UInteger>` | ✅ | `profit: LinearIntermediateSymbol`, `manHours: LinearIntermediateSymbols1` | `profit_expr, man_hours_exprs: SymbolCombination` | ✅ 语义等价 | 命名风格不同（profit_expr vs profit），结构等价。 |
| 9 | `x,y: IntVar` | `x,y: VariableCombination1D<Integer>` | ✅ | `dx,dy,distance: LinearIntermediateSymbols1` | `dx,dy: VariableCombination1D<UContinuous>` + `distance_expr: SymbolCombination` | ❌ **结构差异** | **Kotlin 的 dx/dy 是从 x,y 派生的中间符号**（绝对值约束 linearization）。Rust 将 dx/dy 建模为**独立的决策变量**然后通过约束关联。这是根本性的建模方法差异。 |
| 10 | `x: BinVariable2, u: IntVariable1` | `x_vars: VariableCombination2D<Binary>`, `u_vars: VariableCombination1D<Integer>` (局部) | ✅ | `distance: LinearIntermediateSymbol`, `depart/reached: LinearIntermediateSymbols1` | `distance, depart, reached: SymbolCombination` (局部) | ✅ | 局部变量风格，结构等价。 |
| 11 | `x: UIntVariable2, flow: UIntVar` | `arc_vars: VariableCombination2D<UContinuous>`, `flow_vars: VariableCombination1D<UContinuous>` (局部) | ❌ 类型不匹配 | `flowIn/flowOut: LinearIntermediateSymbols1` | `flow_out_expr, flow_in_expr: SymbolCombination` (局部) | ⚠️ 命名 | **Kotlin x 是 UIntVariable2（整数）**，Rust UContinuous。命名 flowIn/flowOut vs flow_in_expr/flow_out_expr。 |
| 12 | `x: UIntVariable1` | `x: VariableCombination1D<UContinuous>` | ❌ 类型不匹配 | `assignment/premium: LinearIntermediateSymbols1`, `risk: LES`, `yield: LinearIntermediateSymbol` | `yield_expr, funds_expr, risk_expr, activation_expr, premium_rate_expr, premium_min_expr` (struct) | ❌ **符号数量/命名不匹配** | Rust 多出 `activation_expr`, `premium_rate_expr`, `premium_min_expr`。Kotlin 只有 assignment/premium/risk/yield 4个。需要核实 Rust 是否正确扩展或是否应移除。 |
| 13 | `x,y: UIntVariable2` | `x_vars,y_vars: VariableCombination2D<UContinuous>` (局部) | ❌ 类型不匹配 | `trans/receive: LinearIntermediateSymbols1`, `cost: LinearIntermediateSymbol` | `cost_expr, trans_expr, receive_expr` (局部) | ✅ | Kotlin UIntVariable2 vs Rust UContinuous。 |
| 14 | `x: UIntVariable2` | `x_vars: VariableCombination2D<UContinuous>` (局部) | ❌ 类型不匹配 | `cost: LinearIntermediateSymbol`, `transOut/transIn: LinearIntermediateSymbols1` | `cost_expr, trans_out_expr, trans_in_expr` (局部) | ⚠️ 命名 | Kotlin `transOut/transIn` vs Rust `trans_out_expr/trans_in_expr`。 |
| 15 | `x: UIntVariable3, y: Map<DC,PctVariable1>` | `x_vars: VariableCombination3D<UContinuous>`, `y_for_center: VariableCombination1D<UContinuous>` (局部) | ❌ 类型不匹配 + 稀疏 vs 统一 | `receive/demand/trans: LinearIntermediateSymbols2`, `cost: LinearIntermediateSymbol` | `cost, receive, demand, trans: SymbolCombination` (局部) | ✅ 语义等价 | Kotlin UIntVariable3 vs UContinuous。Kotlin y 是 `Map<DC,PctVariable1>`（稀疏，按 DistributionCenter 分组），Rust 是统一 VariableCombination1D。 |
| 16 | `x: UIntVariable2` | `x_vars: VariableCombination2D<UContinuous>` (局部) | ❌ 类型不匹配 | `produce/supply: LinearIntermediateSymbols1`, `delayDeliveryCost/storageCost/produceCost: LinearIntermediateSymbol` | `produce_cost_expr/storage_cost_expr/delay_cost_expr/supply_expr/produce_expr` (局部) | ✅ 语义等价 | Kotlin 6个符号名 vs Rust 5个。 |
| 17 | `x: BinVariable3, s: URealVariable2` | `x_vars: VariableCombination3D<Binary>`, `s_vars: VariableCombination2D<UContinuous>` | ✅ | `origin/destination/service/capacity: LinearIntermediateSymbols1`, `inFlow/outFlow: LinearIntermediateSymbols2` | `vehicle_usage_cost/transportation_cost/origin/destination/in_flow/out_flow/service/capacity` (局部) | ✅ | Rust 多出 `vehicle_usage_cost` 和 `transportation_cost`（目标函数分解）。 |

### 差异汇总（Core Demo）

**类型不匹配（变量）**：Demo 3, 7, 11, 12, 13, 14, 15 — Kotlin 使用 `UIntVariable1/2/3`（无符号整数），Rust 使用 `UContinuous`（无符号连续）。
→ 影响范围：7 个 demo，是最广泛的差异。

**结构差异（建模）**：Demo 9 — dx/dy 在 Kotlin 中是中间符号，在 Rust 中是独立决策变量。

**符号数量和命名**：
- Demo 4: `use` vs `usage`
- Demo 5/6: `cargoWeight/cargoValue` vs `total_value/total_weight`
- Demo 11: `flowIn/flowOut` vs `flow_in_expr/flow_out_expr`
- Demo 12: Rust 额外多了 `activation_expr`, `premium_rate_expr`, `premium_min_expr`
- Demo 16: Kotlin 3个 cost 符号 vs Rust 语义等价

**字段化 vs 局部变量**：
- 有显式 struct 字段：Demo 1, 3, 4, 5, 6, 8, 9, 12（PortfolioModel/BlendingModel/ProductionModel 等）
- 纯局部变量（无 struct）：Demo 2, 7, 10, 11, 13, 14, 15, 16, 17
- Kotlin 所有 demo 都使用 `class` + `lateinit var` 字段
- 建议统一为 struct 字段风格

---

## 2. Framework Demo 1 检查清单

| Model | Kotlin 变量 | Rust 变量 | 一致? | Kotlin 符号 | Rust 符号 | 一致? | 差异 |
|-------|-------------|-----------|-------|-------------|-----------|-------|------|
| EdgeBandwidth | `y: UIntVariable2` | `y: VariableCombination<UContinuous, Shape<2>>` | ❌ **整数 vs 连续** | `bandwidth: LinearIntermediateSymbols1` | `bandwidth: SymbolCombination<...,Shape<1>>` | ✅ | 变量类型不匹配。 |
| NodeBandwidth | (无变量) | (无变量) | ✅ | `inDegree/outDegree/outFlow: LinearIntermediateSymbols1` | `in_degree/out_degree/out_flow: SymbolCombination<...,Shape<1>>` | ✅ | 完全一致。 |
| ServiceBandwidth | (无变量) | (无变量) | ✅ | `inDegree/outDegree/outFlow: LinearIntermediateSymbols2` | `in_degree/out_degree/out_flow: SymbolCombination<...,Shape<2>>` | ✅ | 完全一致。 |
| Assignment | `x: BinVariable2` | `x: VariableCombination<Binary, Shape<2>>` | ✅ | `nodeAssignment/serviceAssignment: LinearIntermediateSymbols1` | `node_assignment/service_assignment: SymbolCombination<...,Shape<1>>` | ✅ | 完全一致。 |

---

## 3. Framework Demo 2/3/4 简要检查

| Framework | 状态 | 说明 |
|-----------|------|------|
| demo2 (stowage) | ⚠️ 未使用新 API | 使用旧式 `Vec<Vec<usize>>` 索引风格，不在本次 scope 内。 |
| demo3 (CSP1D) | ✅ 已迁移 (Phase 9) | 确认使用 SymbolCombination 新 API。 |
| demo4 (gantt) | ✅ 已迁移 (Phase 9) | 确认使用 SymbolCombination 新 API。 |

---

## 4. 差异分类

### 4.1 必须修复（语义不一致）

| # | 差异 | 影响 demo | 修复难度 | 说明 |
|---|------|-----------|---------|------|
| M1 | **UIntVariable -> UContinuous 类型不匹配** | 3, 7, 11, 12, 13, 14, 15 | 低 | 将 `UContinuous` 改为 `UInteger`（与 Kotlin UIntVariable 语义一致） |
| M2 | **Demo 9 dx/dy 建模方法差异** | 9 | 高 | 需要将 dx/dy 从独立决策变量改为从 x,y 派生的中间符号（绝对值 linearization），重写建模逻辑。 |

### 4.2 建议修复（命名/风格统一）

| # | 差异 | 影响 demo | 建议 |
|---|------|-----------|------|
| S1 | Demo 5/6: `total_value/total_weight` -> `cargo_value/cargo_weight` | 5, 6 | 重命名以匹配 Kotlin |
| S2 | Demo 4: `usage` -> `use` | 4 | 重命名以匹配 Kotlin |
| S3 | Demo 8: `profit_expr/man_hours_exprs` -> `profit/man_hours` | 8 | 去掉 `_expr` 后缀 |
| S4 | Demo 11: `flow_in_expr/flow_out_expr` -> `flow_in/flow_out` | 11 | 去掉 `_expr` 后缀 |
| S5 | 局部变量 -> struct 字段 | 2, 7, 10, 11, 13, 14, 15, 16, 17 | 仿照 demo1/3/4/5/6/8/9/12，将模型字段封装为结构体 |
| S6 | Demo 12: 核实 `activation_expr`, `premium_rate_expr`, `premium_min_expr` | 12 | 如 Kotlin 无这些字段，考虑是否移除 |
| S7 | Framework EdgeBandwidth: `UContinuous` -> `UInteger` | FW1 | 与 Kotlin 类型一致 |
| S8 | 统一命名风格 | 全部 | 确认 Rust 命名风格与 Kotlin 一致（snake_case 已满足） |

---



## 4a. 中间符号组合方式一致性检查

| Demo | Kotlin 组合方式 | Rust 组合方式 | 一致? | 差异说明 |
|------|----------------|---------------|-------|---------|
| 1 | `sum(companies) { it.capital * x[it] }` 等3个独立 | `flat_map1` + coefficient * x_idx | ✅ | 结构等价（3合1 vs 3独立，但语义相同） |
| 2 | `flatSum(companies) { c -> sum(stores) { x[c, s] } }` -> `cost`, `assignmentCompany`, `assignmentProduct` | `flat_map1` 3个 | ✅ | 组合方式一致 |
| 3 | `cost = sum(m.cost * x[m])` (标量); `yield[p] = sum(m where m.yield contains p) { m.yield * x[m] }` (一维) | 一致 | ✅ | 组合方式一致 |
| 4 | `profit = sum(p.profit * x[p])` (标量); `use[m] = sum(ps where m) { p.use * x[p] }` (一维) | 一致 | ✅ | 组合方式一致 |
| 5/6 | `cargoValue = sum(c.value * x[c])`, `cargoWeight = sum(c.weight * x[c])` | 一致 | ✅ | 组合方式一致 |
| 7 | `cost = sum(w.stores map s) { x[w,s] }`; `shipment[w] = sum(s) where w.cost contains s { x[w,s] }`; `purchase[s] = sum(w) where w.cost contains s { x[w,s] }` | 一致 | ✅ | 组合方式一致 |
| 8 | `profit = sum(p.profit * x[p])`; `manHours[e] = sum(products) { e.manHours[p] * x[p] }` | 一致 | ✅ | 组合方式一致 |
| 9 | `dx[i] = x - s_i.x` (中间符号, 绝对值 linearization); `dy[i] = y - s_i.y`; `distance[i] = dx[i] + dy[i]` | ❌ **结构差异** | Rust 将 dx/dy 作为独立决策变量而非从 x,y 派生的中间符号。这是本质性建模差异。 |
| 10 | `distance = sum(cities) sum(x[i,j]*dist)`; `depart[i] = sum j x[i,j]`; `reached[j] = sum i x[i,j]` | 一致 | ✅ | 组合方式一致 |
| 11 | `flowIn[j] = sum i x[i,j]`; `flowOut[i] = sum j x[i,j]`; 目标直接用 `flow` 变量 | 基本一致 | ⚠️ | Rust 额外用 `flow_obj` 符号包裹 `flow_idx[0]`（多余），Kotlin 直接 `maximize(flow)` |
| 12 | `assignment[i] = Binaryzation(x[i])` (非线性); `premium[i] = max(p.premium*x[i], p.minPremium*assignment[i])` (非线性); `risk = sum(p.risk*x[p]/funds)`; `yield = sum(p.yield*x[p] - premium[p])` | ❌ **组合管线差异** | Rust 已有 `BinaryzationFunction`/`MaxFunction`，且 `FunctionSymbol: IntermediateSymbol` 可作为中间符号注册；当前 demo 仍用**6 个线性符号**+额外约束模拟。缺口不是 function symbol 本身，而是 demo/`SymbolCombination` 工厂未直接使用 `FunctionSymbol` 组合。 |
| 13 | `trans[c] = sum d x[d,c]` (对 dealers 求和); `receive[d] = sum c x[d,c]` (对 centers 求和); `cost = sum d sum c (x+y)*cost` | 一致 | ✅ | 组合方式一致 |
| 14 | `transOut[i] = sum j x[i,j]`; `transIn[j] = sum i x[i,j]`; `cost = sum arcs x*cost` | 一致 | ✅ | 组合方式一致 |
| 15 | `receive[d,c] = sum m x[m,d,c]`; `demand[d,c] = demands - replacedDemand + replacedToDemand`; `trans[m,c] = sum d x[m,d,c]`; `cost = 标量 sum x*cost` | 基本一致 | ⚠️ | Rust 的 `demand` 符号包含范围逻辑（`replacedDemand` / `replacedToDemand` 中间计算），需要确认是否与 Kotlin 的 filter-map 逻辑等价。 |
| 16 | `produce[p] = sum j x[p,j]`; `supply[p] = sum i x[i,p]`; 3个 cost 符号 | 一致 | ✅ | 组合方式一致。 |
| 17 | `origin[v] = sum OriginNodes x[n1,_,v]` **按类型过滤**; `destination[v] = sum EndNodes x[_,n2,v]`; `inFlow[n,v]` 分支: OriginNode/EndNode 时为 empty; `service[n,v]` = sum x[n,_,_] (排除 OriginNode); `capacity[v] = sum DemandNodes demand*x[_,n2,v]` | ❌ **过滤逻辑差异** | Rust: origin 硬编码 `nodes[0]` = origin(OriginNode的假设), destination 硬编码 `nodes[last]`。inFlow/outFlow 对所有统一构建 flat 1D (k=v*n+v) 不分 origin/end。service 也不排除 origin。capacity 用 `continue if NodeKind::Demand` 过滤。**Kotlin 的类型过滤语义没有完全对齐**。 |

### 关键发现总结

| # | 差异 | 影响 demo | 严重程度 |
|---|------|-----------|---------|
| C1 | **Demo 9**: dx/dy 不是中间符号而是独立变量 | 9 | ❌ 语义 |
| C2 | **Demo 12**: 已有 Binaryzation/Max function symbol，但 demo 未通过 `SymbolCombination` 直接组合使用 | 12 | ❌ 组合管线 |
| C3 | **Demo 17**: type-based filtering (filterIsInstance) 被 hardcoded index 替代 | 17 | ❌ 语义 |
| C4 | **Demo 11**: 多余的 `flow_obj` 包装符号 | 11 | ⚠️ 风格 |
| C5 | **Demo 15**: demand 符号的中间计算等价性未确认 | 15 | ⚠️ 待核实 |
| C6 | **flat_map1 API**: Rust 的闭包只有 `&T` 没有索引，导致 demo 内大量 `iter().position()` 查找 | 多个 | ❌ API |

## 5. 改进计划

### Phase A：类型修复（M1 + S7）
- [x] Demo 3: `UContinuous` -> `UInteger`
- [x] Demo 7: `UContinuous` -> `UInteger`
- [x] Demo 11: `UContinuous` -> `UInteger`
- [x] Demo 12: `UContinuous` -> `UInteger`
- [x] Demo 13: `UContinuous` -> `UInteger`
- [x] Demo 14: `UContinuous` -> `UInteger`
- [x] Demo 15: `UContinuous` -> `UInteger`
- [x] FW1 EdgeBandwidth.y: `UContinuous` -> `UInteger`

### Phase B：Demo 9 结构重构（M2）
- [x] 将 dx/dy 从独立决策变量改为中间符号（从 x,y 派生，绝对值 linearization 约束）
- [x] 更新约束注册逻辑
- [x] 更新求解后读取逻辑

### Phase C：命名统一（S1-S4, S8）
- [x] Demo 4: `usage` -> `use`
- [x] Demo 5: `total_value/total_weight` -> `cargo_value/cargo_weight`
- [x] Demo 6: `total_value/total_weight` -> `cargo_value/cargo_weight`
- [x] Demo 8: 去掉 `_expr` 后缀
- [x] Demo 11: 去掉 `_expr` 后缀

### Phase D：字段化改进（S5）
- [x] Demo 2: 封装为结构体
- [x] Demo 7: 封装为结构体
- [x] Demo 10: 封装为结构体
- [x] Demo 11: 封装为结构体
- [x] Demo 13: 封装为结构体
- [x] Demo 14: 封装为结构体
- [x] Demo 15: 封装为结构体
- [x] Demo 16: 封装为结构体
- [x] Demo 17: 封装为结构体

### Phase E：Demo 12 符号清理（S6）
- [x] 对比 Kotlin Demo12 完整的约束/目标实现，确认 `activation_expr`, `premium_rate_expr`, `premium_min_expr` 是否多余
  - **结论：不冗余，是 Kotlin 非线性函数的线性化。** Rust 已有 `BinaryzationFunction`/`MaxFunction`，且 `FunctionSymbol` 继承 `IntermediateSymbol`，可以作为 `Arc<dyn IntermediateSymbol<_>>` 注册；当前缺口是 demo/组合工厂没有直接构造并注册这些 function symbols，因此保留了手动线性化。
  - `activation_expr` -> 重命名为 `assignment_expr`，对应 Kotlin 的 `assignment = Binaryzation(x[i])`
  - `premium_rate_expr` + `premium_min_expr` -> 对应 Kotlin 的 `premium = max(rate*x, minPrem*a)`，通过两个下界约束线性化
  - 已添加完整的模块级文档注释和字段级注释，说明 Kotlin 4 符号到 Rust 6 符号的映射关系
- [x] 保持与 Kotlin 一致：struct 字段已重命名对齐 Kotlin 命名（`assignment_expr`），保留线性化所需的所有符号

---

### Phase F：组合方式修复（C1-C5）
- [x] Demo 9: 将 dx/dy 改为从 x,y 派生的中间符号，使用绝对值 linearization 约束
- [x] Demo 11: 移除多余的 `flow_obj` 包装符号，直接以 `flow` 变量作目标
- [x] Demo 12: 已在 demo 中显式注释为手写线性化版本（Phase E），模块文档说明 Kotlin 4 符号到 Rust 6 符号的映射
- [x] Demo 15: 对照 Kotlin 重新核对 `demand` 符号的 replacement 逻辑等价性
- [x] Demo 17: 用类型 enum 而非硬编码下标过滤 OriginNode/EndNode/DemandNode；inFlow/outFlow/service 在符号构建时按 NodeKind 分支处理（与 Kotlin 一致）

### Phase G：API 增强（C6）
- [x] 为 `flat_map1/2/3` 增加索引版本（`flat_map1_indexed` 或 `ctor: Fn(usize, &T) -> Linear<V>`），消除 demo 中的 `Cell::new` 闭包计数器和 `iter().position(...)` 查找
- [x] 同步更新所有现有 demo 使用新签名
- [x] 评估是否提供 `sum_along` / `sum_filter` 高阶组合函数，匹配 Kotlin 的 `sum(x[i, _a])` 语法糖

### Phase H：FunctionSymbol 组合管线接入（下一轮计划）
- [x] 明确 `FunctionSymbol` 到 `IntermediateSymbol` 的 demo 接入路径，避免继续把 trait 层误判为缺口
  - **结论：** `BinaryzationFunction`/`MaxFunction`/`AbsFunction` 均实现 `IntermediateSymbol<V>`，可直接用于 `SymbolCombination`
  - 测试验证：`test_binaryzation_function_in_symbol_combination` + `test_add_symbol_combination_with_binaryzation_function`
- [x] 评估并补齐 `add_symbol_combination` / 组合工厂对 `FunctionSymbol` 的直接接入能力
  - **结论：** `add_symbol_combination` 已支持 `FunctionSymbol` 类型（通过 `IntermediateSymbol` trait bound）
  - 测试验证：`test_add_symbol_combination_with_binaryzation_function`
- [x] 让 demo12 / demo9 这类场景优先使用 `BinaryzationFunction` / `MaxFunction` / `AbsFunction` 的组合入口，而不是手写线性化
  - **demo12**: `assignment_expr` → `BinaryzationFunction`, `premium_rate_expr + premium_min_expr` → `MaxFunction`
  - **demo9**: `dx/dy` VariableCombination → `AbsFunction`
- [x] 为 function symbol 提供与 Kotlin `LinearFunctionSymbolAdapter` 等价的最小桥接层，或确认现有 `Arc<dyn IntermediateSymbol<_>>` 注册路径已经足够
  - **结论：** `LinearFunctionSymbol<V>` trait 已存在，组合 `FunctionSymbol<V> + LinearIntermediateSymbol<V>`
  - 自动实现：`impl<T, V> LinearFunctionSymbol<V> for T where T: FunctionSymbol<V> + LinearIntermediateSymbol<V>`
- [x] 明确 `AbsFunction` / `piecewise` 只需要实现 `FunctionSymbol + LinearIntermediateSymbol`，`to_quadratic_polynomial()` 只是二次型视图，不应误写成 `QuadraticFunctionSymbol` 的新实现目标
  - **结论：** `AbsFunction` 和 `MaxFunction` 均实现 `IntermediateSymbol<V>` + `LinearIntermediateSymbol<V>`，可直接用于 `SymbolCombination`
- [x] 复核 demo12 的 4 符号 Kotlin 结构与 Rust 实现的最终收敛点，明确哪些是抽象层，哪些是线性化落地层
  - **Kotlin 4 符号**: assignment (Binaryzation), premium (Max), risk, yield
  - **Rust 5 符号**: assignment_fn (BinaryzationFunction), premium_fn (MaxFunction), risk, yield, funds (辅助)
  - **收敛点**: BinaryzationFunction/MaxFunction 自动产生 mechanism 约束，与 Kotlin 的 Binaryzation/Max 语义等价
  - **差异**: Rust 多出 `funds_expr`（预算聚合约束），这是 Kotlin 隐式处理的

## 6. 验收标准

## 7. 阶段总览

| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase A | 类型修复（UContinuous -> UInteger） | ✅ 已完 |
| Phase B | Demo 9 结构重构 | ✅ 已完 |
| Phase C | 命名统一 | ✅ 已完 |
| Phase D | 字段化改进 | ✅ 已完 |
| Phase E | Demo 12 符号清理 | ✅ 已完 |
| Phase F | 组合方式修复 | ✅ 已完 |
| Phase G | API 增强（flat_map1_indexed 等） | ✅ 已完 |
| Phase H | FunctionSymbol 组合管线接入 | ✅ 已完 |
| Phase I | functions 目录与 Kotlin 对齐 | ✅ 已完 |
| Phase J | Framework Model 改造（BPP3D / CSP1D / GanttScheduling） | ✅ 已完 |
| Phase K | Example 手写线性化替换为 FunctionSymbol | ✅ 已完 |


1. **编译通过**：`cargo check --workspace` 无错误（已知的 demo2 gurobi 错误除外）
2. **类型一致**：所有 Kotlin `UIntVariable` 对应 Rust `UInteger`，`RealVariable` 对应 `UContinuous`，`BinVariable` 对应 `Binary`
3. **符号一致**：每个 demo 的中间符号数量/类型/命名与 Kotlin 一致（或可证明为合理扩展）
4. **建模一致**：Demo 9 的 dx/dy 使用与 Kotlin 相同的方法派生（中间符号，而非独立变量）
5. **结构体封装**：所有 core demo 将模型变量作为 struct 字段而非纯局部变量
6. **FunctionSymbol 接入**：Demo 9 / Demo 12 使用 `AbsFunction`、`BinaryzationFunction`、`MaxFunction` 作为中间符号组合入口，手写线性化仅保留在 function symbol 的 mechanism 层
7. **Quadratic 视图**：`AbsFunction` / `piecewise` 不要求额外实现 `QuadraticFunctionSymbol`；只要 `LinearIntermediateSymbol` 视图和 `mechanism_constraints` 正确即可
### Phase I：functions 目录与 Kotlin 对齐

#### 背景

当前 ospf-rust-core/src/symbol/functions/ 目录与 Kotlin 版本 ospf-kotlin-core/.../symbol/function/ 存在结构差异：Rust 有多个合并文件（slack.rs、trigonometric.rs、piecewise.rs 等），同时缺少 Kotlin 已有的多个类型（IfFunction、IfInFunction、ImplyFunction 等），另有 Rust 多出的重复类型。

**架构说明**：
- Rust 有两个入口目录：functions/（真实文件所在）与 function/（仅含 mod.rs，通过 #[path = "..."] 转发到 functions/ 下的文件，function/mod.rs 做 pub use 统一导出）。
- functions/ 下的 mod.rs 是兼容转发入口，内容为 pub use crate::symbol::function::*;。
- Phase I 的修改仅涉及 functions/ 下的拆分/补缺，两个 mod.rs 的转发映射在末尾一次性对齐。

#### 模块重新划分（37 个 Kotlin .kt -> 37 个 Rust .rs）

**1. 拆分合并文件**

| 当前 Rust 文件 | 拆分为 | 对应 Kotlin | 策略 |
|---------------|--------|-------------|------|
| min_max.rs | max.rs + 删除 min_max.rs | Max.kt + Min.kt（Kotlin Min 在 Max.kt 内） | 文件名以 Kotlin 文件名为准；MinFunction 仍放 max.rs 内，不拆分 |
| max_min.rs | min_max.rs（重命名） | MinMax.kt | 纯重命名，内容不变 |
| logic.rs | and.rs（重命名） | And.kt | 内容不变（AndFunction, OrFunction, NotFunction, XorFunction 都在同一文件） |
| slack.rs | slack.rs + slack_range.rs | Slack.kt + SlackRange.kt | SlackFunction 留在 slack.rs；SlackRangeFunction 拆到 slack_range.rs |
| trigonometric.rs | sin.rs + cos.rs | Sin.kt + Cos.kt | SinFunction 拆到 sin.rs；CosFunction 拆到 cos.rs |
| piecewise.rs | univariate_linear_piecewise.rs + bivariate_linear_piecewise.rs | UnivariateLinearPiecewise.kt + BivariateLinearPiecewise.kt | 按 Kotlin 拆为两个独立文件 |

**2. balance_ternary.rs 重命名**
- balance_ternary.rs -> balance_ternaryzation.rs（与 Kotlin BalanceTernaryzation.kt 一致）

**3. 补充缺失类型文件**

| 新文件 | Kotlin 对应 | 内容 |
|-------|-------------|------|
| ceiling.rs | Ceiling.kt | CeilingFunction -- 独立 struct，不包装 RoundingFunction |
| floor.rs | Floor.kt | FloorFunction -- 独立 struct，不包装 RoundingFunction |
| if_function.rs | If.kt | IfFunction -- if 是 Rust 关键字，文件名用 if_function.rs，导出类型名为 IfFunction |
| if_in.rs | IfIn.kt | IfInFunction |
| imply.rs | Imply.kt | ImplyFunction |
| satisfied_amount_inequality.rs | SatisfiedAmountInequality.kt | 5 个类型：AnyFunction、AllFunction、AtLeastInequalityFunction、NotAllFunction、NumerableFunction |

**4. 补充现有文件的缺失类型**

| 文件 | 补充类型 | Kotlin 对应 |
|------|---------|-------------|
| masking.rs | MaskingWithPolyMaskFunction | Masking.kt 中的 MaskingWithPolyMaskFunction |

**5. quadratic_function.rs 拆分策略**

Kotlin 有 4 个独立 QuadraticXxx.kt 文件，其余 14 个 Quadratic* 类型在 Kotlin 没有独立文件，分散在各自基础函数的 .kt 文件中。

按以下两步拆分：

**第一步：拆出 4 个独立文件（对应 Kotlin 同名文件）**

| 拆分文件 | 包含类型 | Kotlin 对应 |
|---------|---------|-------------|
| quadratic_linear.rs | QuadraticLinearFunction | QuadraticLinear.kt |
| quadratic_min.rs | QuadraticMinFunction | QuadraticMin.kt |
| quadratic_masking_range.rs | QuadraticMaskingRangeFunction | QuadraticMaskingRange.kt |
| quadratic_in_step_range.rs | QuadraticInStepRangeFunction | QuadraticInStepRange.kt |

**第二步：其余 14 个 Quadratic* 类型随各自基础函数文件拆分/迁移**

| Quadratic 类型 | 目标文件 | Kotlin 位置 |
|---------------|---------|-------------|
| QuadraticBinaryzationFunction | binaryzation.rs（合并） | Binaryzation.kt（内联） |
| QuadraticInequalityFunction | inequality.rs（合并） | Inequality.kt（内联） |
| QuadraticRoundingFunction | rounding.rs（合并） | Rounding.kt（内联） |
| QuadraticModFunction | mod_function.rs（合并） | Mod.kt（内联） |
| QuadraticMaxFunction | max.rs（合并） | Max.kt（内联） |
| QuadraticSlackFunction | slack.rs（合并） | Slack.kt（内联） |
| QuadraticSlackRangeFunction | slack_range.rs（合并） | SlackRange.kt（内联） |
| QuadraticMaskingFunction | masking.rs（合并） | Masking.kt（内联，注意不与 masking_range.rs 混淆） |
| QuadraticSinFunction | sin.rs（合并） | Sin.kt（内联） |
| QuadraticCosFunction | cos.rs（合并） | Cos.kt（内联） |
| QuadraticUnivariateLinearPiecewiseFunction | univariate_linear_piecewise.rs（合并） | UnivariateLinearPiecewise.kt（内联） |
| QuadraticBivariateLinearPiecewiseFunction | bivariate_linear_piecewise.rs（合并） | BivariateLinearPiecewise.kt（内联） |
| QuadraticSemiFunction | semi.rs（合并） | Semi.kt（内联） |
| QuadraticSigmoidFunction | sigmoid.rs（合并） | Sigmoid.kt（内联） |

**实施顺序**：先拆 4 个独立的，其余 14 个随各自基础文件合并时迁移。所有迁移完成后删除 quadratic_function.rs。

**6. 补充 LinearFunctionSymbolAdapter**

Kotlin function/FunctionSymbol.kt 包含：
- MathFunctionSymbol -- function symbol 的数学接口（trait）
- MathFunctionSymbolBase -- 提供默认实现的基础抽象类
- HasResultPolynomial -- 结果多项式访问接口
- LinearFunctionSymbolAdapter -- 将 FunctionSymbol 适配为 LinearFunctionSymbol 的桥接 struct

**确认动作**：Phase I 启动前，先搜索 Rust 中 FunctionSymbol trait 的实际定义位置，再决定 LinearFunctionSymbolAdapter 放哪个文件。

**7. 补充 BigM.kt 公共 API**

Kotlin BigM.kt 导出以下公共工具：
- defaultBigM -- 默认大 M 值
- LinearPolynomialBounds -- 边界计算工具
- ensurePositiveBigM -- 确保大 M 为正
- positiveIndicatorConstraints -- 正向指示约束生成
- nonnegativeIndicatorConstraints -- 非负指示约束生成
- negativeIndicatorConstraints -- 负向指示约束生成
- nonzeroIndicatorConstraints -- 非零指示约束生成

Rust 当前 big_m.rs 内容主要是 token 推断的私有路径 + BigMPolicy 枚举。需将其提升为 public API 工具库，让其他 function symbol 能直接使用。

**确认动作**：Phase I 启动前，对照 Kotlin BigM.kt 逐一确认 API 签名。

#### 修改清单（按执行顺序）

1. 拆分合并文件
   - min_max.rs -> max.rs（重命名 + 调整导出）
   - max_min.rs -> min_max.rs（重命名）
   - logic.rs -> and.rs（重命名）
   - slack.rs -> slack.rs + slack_range.rs（拆分）
   - trigonometric.rs -> sin.rs + cos.rs（拆分）
   - piecewise.rs -> univariate_linear_piecewise.rs + bivariate_linear_piecewise.rs（拆分）
2. balance_ternary.rs -> balance_ternaryzation.rs（重命名）
3. 补充缺失类型：创建 6 个新文件
   - ceiling.rs、floor.rs、if_function.rs、if_in.rs、imply.rs、satisfied_amount_inequality.rs
4. 补充 masking.rs：添加 MaskingWithPolyMaskFunction
5. 拆分 quadratic_function.rs：
   - 先拆出 4 个 Kotlin 同名独立文件
   - 其余 14 个随各自基础文件合并
   - 最后删除 quadratic_function.rs
6. 补充 LinearFunctionSymbolAdapter
7. 补充 BigM.kt 公共 API
8. 对齐 function/mod.rs 和 functions/mod.rs 的转发映射

#### 验收标准

1. 目录结构与 Kotlin 逐文件对应（37 个 .rs 对 37 个 .kt），已知名称冲突（mod.rs -> mod_function.rs）以注释说明
2. cargo check -p ospf-rust-core 编译通过
3. 所有拆分后文件保持原功能（测试通过或确认无回归）
4. LinearFunctionSymbolAdapter 可用，可被 Phase H 接入
5. BigM 公共 API 可被其他 function symbol 使用
6. function/mod.rs 和 functions/mod.rs 转发映射完整无遗漏
### Phase J：Framework Model 改造（BPP3D / CSP1D / GanttScheduling）

#### 背景

三个 framework（bpp3d / csp1d / gantt-scheduling）在建模方式上不完全对齐 Kotlin 版本，存在程度不同的偏差：

| Framework | Rust 当前模式 | Kotlin 模式 | 偏差程度 |
|-----------|-------------|-------------|---------|
| BPP3D | ExpressionArray1<K> 存 Vec<(usize, f64)> + 各 constraint pipeline 自行计算 raw terms | LinearExpressionSymbol / LinearIntermediateSymbols2 注册到模型，flush + asMutable 增量更新 | 严重 |
| CSP1D | DerivedPlanExpressionSymbols 用 flat_map1 构建 LinearExpressionSymbols1，但 pipelines 立即 extract_terms_from_symbol 拆回 raw terms | demandQuantity/materialQuantity/machineBatchQuantity/machineCapacityQuantity 注册为模型内 LinearExpressionSymbols1，迭代时增量更新 | 中等 |
| GanttScheduling | 部分符号注册到模型（Arc<LinearExpressionSymbol<f64>>），同时存在 pending_contributions: Vec<Vec<(usize, f64)>> 裸系数混合模式 | 直接构建和注册 LinearExpressionSymbol 到模型 | 局部 |

#### BPP3D 改造清单

**核心问题**：逐文件替换 ExpressionArray1（HashMap 裸系数）为 LinearExpressionSymbols1/2。

Rust 当前文件列表（与建模相关）：

| 文件 | 当前做法 | 改造方向 |
|------|---------|---------|
| src/domain/layer_assignment/model/expression_array.rs | ExpressionArray1<K> — HashMap<K, Vec<(usize, f64)>> | **废弃**，全量迁移到 LinearExpressionSymbols1 |
| src/domain/layer_assignment/model/variable_array1.rs | VariableArray1<K> — 索引管理 | **废弃**，全量迁移到 VariableCombination1D 或 AppendableVariablePool |
| src/domain/layer_assignment/model/variable_array2.rs | VariableArray2 — 索引管理 | **废弃**，全量迁移到 VariableCombination2D |
| src/domain/layer_assignment/service/assignment.rs | ImpreciseAssignment / PreciseAssignment struct | **重写**：按 Kotlin Assignment.kt 对齐 |
| src/domain/layer_assignment/service/limits/demand_constraint.rs | imprecise_layer_terms / precise_layer_terms 返回 Vec<(usize, f64)> | **改造**：引用已注册符号而非计算 raw terms |
| src/domain/layer_assignment/service/limits/bin_capacity_constraint.rs | 类似 raw terms 模式 | **需检查**：应引用已注册 volume/load 符号 |
| src/domain/layer_assignment/service/limits/bin_depth_constraint.rs | 类似 raw terms 模式 | **需检查**：应引用已注册符号 |
| src/domain/layer_assignment/service/limits/volume_minimization.rs | 类似 raw terms 模式 | **需检查**：应引用已注册 volume 符号 |
| src/domain/layer_assignment/service/limits/bin_amount_minimization.rs | 类似 raw terms 模式 | **需检查** |
| src/domain/layer_assignment/service/limits/tail_bin_assignment_constraint.rs | 类似 raw terms 模式 | **需检查** |
| src/domain/layer_assignment/service/limits/precise_assignment_activation_constraint.rs | 类似 raw terms 模式 | **需检查** |
| src/domain/layer_assignment/service/load.rs | 加载逻辑 | **需检查**：Kotlin 有 Load.kt 的 demand_coverage_coefficient 等 |

**Kotlin 对齐目标**：
- ImpreciseAssignment 对照 Kotlin 版本：x: List<UIntVariable1> + volume: LinearExpressionSymbol<FltX>（已注册）+ addColumns 用 flush + asMutable
- PreciseAssignment 对照 Kotlin 版本：x: UIntVariable2 + u: LinearIntermediateSymbols2（BinaryzationFunction）+ v: LinearIntermediateSymbols1（BinaryzationFunction）+ tail: BinVariable1
- Capacity 对照 Kotlin Capacity.kt 中的中间符号设计

#### CSP1D 改造清单

**核心问题**：给 LinearExpressionSymbols1 加上"注册到模型 + 列生成增量更新"的生命周期。

Rust 当前文件列表：

| 文件 | 当前做法 | 改造方向 |
|------|---------|---------|
| src/domain/produce/model.rs | DerivedPlanExpressionSymbols 构建后立刻拆 extract_terms_from_symbol | **改造**：保持 LinearExpressionSymbols1 注册到模型而非拆散 |
| src/domain/produce/aggregation.rs | ProduceAggregation 管理 raw variable pool | **改造**：添加 batch_symbols: Vec<LinearExpressionSymbols1> 匹配 Kotlin 的 _batch: MutableList<LinearExpressionSymbols1> |
| src/domain/produce/builder.rs | 注册 Produce 到模型 | **改造**：添加 LinearExpressionSymbols1 注册 |
| src/domain/produce/extraction.rs | solution extraction 直接遍历 raw 数据 | **保留**：提取逻辑不涉及中间符号，可保持不变 |
| src/domain/produce/pipeline/demand.rs | 每次 register 重建 symbol → 拆 raw → add_linear_constraint | **改造**：引用已注册的符号而非重建 |
| src/domain/produce/pipeline/material.rs | 同上 | **改造** |
| src/domain/produce/pipeline/machine.rs | 同上 | **改造** |
| src/domain/produce/pipeline/yield.rs | 产出目标 | **检查** |
| src/domain/produce/pipeline/length.rs | 长度约束目标 | **检查** |
| src/domain/cutting_plan_generation/model.rs | 切割方案生成模型 | **检查** |

**Kotlin 对齐目标**：
- 初始注册创建 x + batch = LinearExpressionSymbols1("batch", ...) 注册到模型
- demandQuantity = LinearExpressionSymbols1("demandQuantity", ...) 注册到模型
- materialQuantity = LinearExpressionSymbols1("materialQuantity", ...) 注册到模型
- machineBatchQuantity = LinearExpressionSymbols1("machineBatchQuantity", ...) 注册到模型
- machineCapacityQuantity = LinearExpressionSymbols1("machineCapacityQuantity", ...) 注册到模型
- 约束 pipeline 直接引用这些符号（而非重建 → 拆 raw）
- addColumns：创建新 x_i + 新 batch_i 符号，对已有符号 flush + asMutable +=

#### GanttScheduling 改造清单

**核心问题**：消除 pending_contributions 裸系数混合模式。

Rust 当前相关文件：

| 文件 | 当前做法 | 改造方向 |
|------|---------|---------|
| src/domain/resource/model/connection_usage.rs | pending_contributions: Vec<Vec<(usize, f64)>> + add_connection(slot, x_model_index, coefficient) | **改造**：直接用 LinearMonomial 构建符号，消除 raw index 层 |
| src/domain/resource/model/storage_usage.rs | 同上模式 | **改造** |
| src/domain/resource/model/usage.rs | 同上模式（出现 3 次不同 ResourceUsage） | **改造** |
| src/domain/capacity_scheduling/model.rs | 同上 pending_* 模式 | **改造** |
| src/domain/capacity_scheduling/service/limits.rs | 约束注册 | **检查** |
| src/domain/task_compilation/model.rs | 任务编译模型 | **检查** |
| src/domain/produce/service/limits.rs | 产出约束 | **检查** |
| src/domain/resource/service/limits.rs | 资源约束 | **检查** |

**Kotlin 对齐目标**：
- 消除 pending_contributions：改为约束/目标注册时直接从 task_bunch 的 ConnectionResource.usedQuantityQuantity 构建 LinearMonomial 再累计到 LinearExpressionSymbol
- 保持已注册的 Arc<LinearExpressionSymbol<f64>> 模式不变，只替换符号构建方式
- Resource.kt 的 usedQuantityQuantity 抽象是计算 Quantity<V> — Rust 需确保 usedQuantityQuantity 返回的 Quantity 可用于符号构建

#### Phase J 修改清单（按执行顺序）

1. **BPP3D**
   - (1.1) 废除 expression_array.rs、variable_array1.rs、variable_array2.rs：不再引入 include!
   - (1.2) 重写 ImpreciseAssignment 和 PreciseAssignment struct，对照 Kotlin Assignment.kt
   - (1.3) 为 ImpreciseAssignment 添加 volume: LinearExpressionSymbol<f64> 符号字段，注册到模型
   - (1.4) 实现 addColumns：创建新变量组 x_i 后 flush + asMutable += 更新已有符号
   - (1.5) 为 PreciseAssignment 添加 u: LinearIntermediateSymbols2 + v: LinearIntermediateSymbols1（BinaryzationFunction）+ tail，对照 Kotlin
   - (1.6) 改造各 limits/*.rs 约束管线：引用已注册符号而非计算 raw terms
   - (1.7) 对照 Kotlin Capacity.kt / Load.kt 补充中间符号设计
   - (1.8) 更新 consumer（context / service / tests）

2. **CSP1D**
   - (2.1) ProduceAggregation 改造：添加 batch_symbols 字段，初始注册创建并注册 x 变量
   - (2.2) 创建并注册 demand_fulfillment / material_usage / machine_batch_usage / machine_capacity_usage 为 LinearExpressionSymbols1
   - (2.3) 管线改造：DemandConstraintPipeline / MaterialConstraintPipeline / MachineConstraintPipeline / Yield / Length 约束管线不再重建 DerivedPlanExpressionSymbols，直接引用已注册符号
   - (2.4) addColumns 实现（ProduceAggregation）：新建 x_i 变量组 + batch_i 符号组，对 demand_fulfillment 等已有符号做 flush + asMutable +=
   - (2.5) 如 DerivedPlanExpressionSymbols 无新使用者，则删除或降级
   - (2.6) 更新 consumer

3. **GanttScheduling**
   - (3.1) 替换 pending_contributions: Vec<Vec<(usize, f64)>>：约束注册时改为直接构建 Linear::new(monomials, 0.0) 而非先收集 raw indices
   - (3.2) connection_usage.rs / storage_usage.rs / usage.rs（3 个变体）：消除 add_connection / add_usage 的 raw index 路径
   - (3.3) capacity_scheduling/model.rs：消除 pending_* 模式
   - (3.4) 确保 Resource.kt 的 usedQuantityQuantity 抽象在 Rust 中能返回可符号化的值
   - (3.5) 更新 tests

#### 验收标准

1. **BPP3D**：ExpressionArray1 被彻底移除；ImpreciseAssignment.volume 注册为 LinearExpressionSymbol；PreciseAssignment.u/v 使用 BinaryzationFunction 对齐 Kotlin；约束管线引用已注册符号；cargo check -p ospf-rust-framework-bpp3d 通过
2. **CSP1D**：demand_fulfillment / material_usage / machine_batch_usage / machine_capacity_usage 注册为模型内 LinearExpressionSymbols1；pipeline 直接引用这些符号；addColumns 用 flush + asMutable 增量更新；cargo check -p ospf-rust-framework-csp1d 通过
3. **GanttScheduling**：所有 pending_contributions / Vec<Vec<(usize, f64)>> 被消除；符号构建直接使用 LinearMonomial 而非 raw index；cargo check -p ospf-rust-framework-gantt-scheduling 通过
4. **整体**：满足 3 个 framework 各自的验收标准，无编译错误

### Phase K：Example 手写线性化替换为 FunctionSymbol

#### 背景

Phase B/E/F 已识别出 core demo 中的 `BinaryzationFunction`/`MaxFunction`/`AbsFunction` 缺口，并在 Phase H 中规划 FunctionSymbol 接入路径。但 example 中还存在大量"应当用 function symbol 而当前用手写线性化"的位置，特别是 framework demo2，对照 Kotlin 的 `Load.kt`/`Envelope.kt`/`RelativeOrder.kt`/`SequentialLoading.kt`/`HorizontalStabilizer.kt` 等使用 function symbol 的地方，Rust 全部退化为 raw 约束或简化连续近似。

Phase K 系统地列出这些位置，分 core 和 framework 两类。

#### 关键确认前置

Phase H 与 Phase I 需先到位（提供 `LinearFunctionSymbolAdapter` 桥接 + 完整 functions 目录），Phase K 才有对齐目标。Phase K 的清单按 demo / 文件粒度列出，可独立执行每一项。

#### Core Demo 清单

| Demo | 当前做法 | 应替换为 | Kotlin 对照 | 优先级 |
|------|---------|---------|-------------|--------|
| 9 | dx/dy 作为独立决策变量 + Big-M 约束（4 条约束 / settlement）| `AbsFunction`（2 个中间符号） | `val dx = abs(x - settlement.x); val dy = abs(y - settlement.y); val distance = dx + dy` | P0 |
| 12 | assignment_expr / premium_rate_expr / premium_min_expr 手写 Big-M 线性化（6 个符号面） | `BinaryzationFunction` 替代 assignment + `MaxFunction` 替代 premium | `val assignment = Binaryzation(x[i]); val premium = max(rate*x[i], minPrem*a[i])` | P0 |

##### Demo 9 改造细节

**当前代码**（`ospf-rust-example/src/core/demo9.rs`）：
- struct `LocationModel` 字段：`dx: VariableCombination1D<UContinuous>` + `dy: VariableCombination1D<UContinuous>` + `distance_expr: SymbolCombination<...>`
- 注册阶段：注册独立的 dx/dy 决策变量
- 约束阶段：每个 settlement 注册 4 条 raw `add_linear_constraint` 实现 `dx >= |x - settlement.x|` 与 `dy >= |y - settlement.y|`

**目标对齐**：
- 删除 dx/dy 作为决策变量
- 用 `AbsFunction` 创建中间符号 `dx[i] = AbsFunction(x - settlement.x)`，`dy[i] = AbsFunction(y - settlement.y)`
- distance 直接是 `dx[i] + dy[i]` 作为 `LinearExpressionSymbol`
- AbsFunction 内部自行处理 mechanism constraints，约束注册阶段不再写 4 条 raw 约束
- 字段调整为：`dx: SymbolCombination<f64, Arc<dyn FunctionSymbol<f64>>, Shape<1>>` 或等价的具体类型

##### Demo 12 改造细节

**当前代码**（`ospf-rust-example/src/core/demo12.rs`）：
- struct `PortfolioModel` 字段：`assignment_expr` / `premium_rate_expr` / `premium_min_expr` / `funds_expr` / `risk_expr` / `yield_expr`（6 个表达式符号）
- 模块文档已注明 "linearization of Kotlin Binaryzation + Max"
- 约束阶段：每个 product 注册 3 条 raw 约束（assignment / premium_rate_lower_bound / premium_min_lower_bound）

**目标对齐**：
- 用 `BinaryzationFunction` 替代 assignment_expr：`assignment[i] = BinaryzationFunction(x[i])` 一个符号即可，不需要 funds * a_i 系数
- 用 `MaxFunction` 替代 premium_rate_expr + premium_min_expr：`premium[i] = MaxFunction(rate*x[i], minPrem*a[i])` 一个符号即可
- 字段从 6 个表达式符号缩减到 4 个对齐 Kotlin（assignment / premium / risk / yield）
- 移除手写的 Big-M / max 下界约束，由 function symbol 的 mechanism 层自动生成

#### Framework Demo 2 清单

Kotlin framework demo2 大量使用 function symbol，当前 Rust demo2 全部退化为手写线性化或简化近似。

##### 1. Stowage Domain — `Load.kt` 对照

| Kotlin 符号 | FunctionSymbol | Rust 当前 | 改造方向 |
|-------------|---------------|-----------|---------|
| `predicateLoadWeightSlack[j]` | `SlackFunction` | `load.rs` 未实现 | 补充注册 `SlackFunction` 为弧立符号 |
| `loadAmount[j]` | `LinearExpressionSymbol` | `load.rs` 已实现线性符号 | **保留**（本身就是线性符号，无需 function） |
| `full[j]` | `BinaryzationFunction` | `load.rs` 注释 "连续近似"（简化） | 替换为 `BinaryzationFunction` |
| `estimateLoadWeight[j]` | `LinearExpressionSymbol` | `load.rs` 已实现线性符号 | **保留** |
| `actualLoadWeight[j]` | `LinearExpressionSymbol` | `load.rs` 已实现线性符号 | **保留** |
| `y[j]` `z[j]` 中间符号 | `SameAsFunction` / `BinaryzationFunction` / `IfFunction` | `load.rs` 作为独立决策变量 | 替换为 `SameAsFunction` / `IfFunction` 组合 |
| `if loadingScheme[k]` 条件 | `IfFunction` | 无实现 | 补充 `IfFunction` |

**Kotlin Load.kt 关键代码**（需对齐的核心模式）：
```
predictLoadWeight[i] = IfFunction(
    condition = loadAmount[i] geq 1,
    then_ = SameAsFunction(estimateLoadWeight[i]),
    else_ = BinaryzationFunction(y[i])
)
actualLoadWeight[i] = SameAsFunction(predictLoadWeight[i])
```

##### 2. Stowage Domain — `Position.kt` 对照

| Kotlin | FunctionSymbol | Rust 当前 | 改造方向 |
|--------|---------------|-----------|---------|
| `capacityUsage[j][t]` | `UnivariateLinearPiecewiseFunction` | `position.rs` 未实现 | 补充 |

##### 3. Airworthiness Security Domain — `Envelope.kt` 对照

| Kotlin | FunctionSymbol | Rust 当前 | 改造方向 |
|--------|---------------|-----------|---------|
| `envelope[point]` | `UnivariateLinearPiecewiseFunction` | `envelope.rs` 线性近似 | 替换为 `UnivariateLinearPiecewiseFunction` |

##### 4. MAC Domain — `HorizontalStabilizer.kt` 对照

| Kotlin | FunctionSymbol | Rust 当前 | 改造方向 |
|--------|---------------|-----------|---------|
| `hsSlack` | `SlackFunction` | `horizontal_stabilizer.rs` 空符号（无项） | 替换为完整 `SlackFunction` |
| `hsTrim` | `AbsFunction` | 未实现 | 补充 `AbsFunction` |

##### 5. Redundancy Domain — `Redundancy.kt` 对照

| Kotlin | FunctionSymbol | Rust 当前 | 改造方向 |
|--------|---------------|-----------|---------|
| `redundancySlack` | `SlackFunction` | `redundancy.rs` 仅数据 struct | 补充注册 `SlackFunction` |

##### 6. Loading Effectiveness Domain — `SequentialLoading.kt` `TrailerLoading.kt` `TransferAdjacentLoading.kt`

| Kotlin | FunctionSymbol | Rust 当前 | 改造方向 |
|--------|---------------|-----------|---------|
| `loading[i]` | `IfFunction` | `sequential_loading.rs` / `trailer_loading.rs` / `transfer_adjacent_loading.rs` 未建模 | 补充 `IfFunction` |

##### 7. Express Effectiveness Domain — `RelativeOrder.kt` 对照

| Kotlin | FunctionSymbol | Rust 当前 | 改造方向 |
|--------|---------------|-----------|---------|
| `order[i]` | `IfFunction` | `relative_order.rs` 仅 precedence map | 补充 `IfFunction` |

##### 8. Soft Security Domain — `DivideEmptyLoading.kt`

| Kotlin | FunctionSymbol | Rust 当前 | 改造方向 |
|--------|---------------|-----------|---------|
| `emptyFlag[i]` | `IfFunction` | `divide_empty_loading_limit.rs` 约束注册 | 补充 `IfFunction` |

#### Framework Demo 4 清单

##### 9. Passenger Domain — `PassengerFlightChangeConstraint.kt`

| Kotlin | FunctionSymbol | Rust 当前 | 改造方向 |
|--------|---------------|-----------|---------|
| 旅客改签条件 | `IfFunction` | `passenger_change.rs` 仅数据 struct | 补充 `IfFunction` |

#### Framework Demo 1 清单

Demo 1（带宽分配）建模相对简单，目前 `EdgeBandwidth` / `NodeBandwidth` / `ServiceBandwidth` / `Assignment` 都是线性表达式符号，无 function symbol 需求。

但建议复检 4 个 model 文件：
- `bandwidth_context/model/edge_bandwidth.rs`
- `bandwidth_context/model/node_bandwidth.rs`
- `bandwidth_context/model/service_bandwidth.rs`
- `route_context/model/assignment.rs`

确认是否有"带条件分配""活跃指示"等 Kotlin 用 `BinaryzationFunction` / `IfFunction` 而 Rust 退化为 raw 约束的位置。

#### Framework Demo 3 清单

Framework demo3 是 csp1d 切割库存的 thin wrapper，无独立建模代码，所有 function symbol 需求由 framework-csp1d 本身承担（属于 Phase J 范围）。

#### Phase K 修改清单（按执行顺序）

**前置条件**：Phase H 完成 `LinearFunctionSymbolAdapter` 桥接，Phase I 完成 functions 目录的拆分与补缺（`IfFunction` / `IfInFunction` / `ImplyFunction` 等新增类型）。

1. **Core Demo 改造**（P0，2 个文件）
   - (1.1) `core/demo9.rs`：dx/dy 改用 `AbsFunction`，移除手写 4 条 lower bound 约束
   - (1.2) `core/demo12.rs`：`assignment_expr` → `BinaryzationFunction`，`premium_rate_expr + premium_min_expr` → `MaxFunction`，6 个符号缩减为 4 个

2. **Framework Demo 2 改造**（P1，约 12 个文件）
   - (2.1) `stowage/model/load.rs`：补充 `SlackFunction` / `BinaryzationFunction` / `SameAsFunction` / `IfFunction` 中间符号；`full[j]` 移除 "连续近似"
   - (2.2) `stowage/model/position.rs`：补充 `UnivariateLinearPiecewiseFunction`
   - (2.3) `airworthiness_security/model/envelope.rs`：替换为 `UnivariateLinearPiecewiseFunction`
   - (2.4) `mac/model/horizontal_stabilizer.rs`：替换为完整 `SlackFunction` + `AbsFunction`
   - (2.5) `redundancy/model/redundancy.rs`：补充 `SlackFunction`
   - (2.6) `loading_effectiveness/model/sequential_loading.rs`：补充 `IfFunction`
   - (2.7) `loading_effectiveness/model/trailer_loading.rs`：补充 `IfFunction`
   - (2.8) `loading_effectiveness/model/transfer_adjacent_loading.rs`：补充 `IfFunction`
   - (2.9) `express_effectiveness/model/relative_order.rs`：补充 `IfFunction`
   - (2.10) `soft_security/service/limits/divide_empty_loading_limit.rs`：补充 `IfFunction`
   - (2.11) 改造对应的 `service/limits/*.rs` 约束管线：原本 `add_linear_constraint` 调用替换为引用已注册 function symbol 的 `result_variable()` 或直接用符号 polynomial

3. **Framework Demo 4 改造**（P2）
   - (3.1) `passenger/model/passenger_change.rs` + 对应 limits：补充 `IfFunction`

4. **Framework Demo 1 复检**（P3）
   - (4.1) 复检 4 个 model 文件，识别是否存在条件分配 / 活跃指示等需要 function symbol 的位置

#### 验收标准

1. **Demo 9**：dx/dy 不再作为决策变量；`AbsFunction` 注册到模型并可解出与原版本一致的结果；通过 `cargo check` 与运行时对比
2. **Demo 12**：4 个中间符号（assignment / premium / risk / yield）对齐 Kotlin；Big-M 由 `BinaryzationFunction` mechanism 自动产生；运行时 KPI 与原 6 符号版本一致
3. **Framework Demo 2**：每个目标文件中至少出现一个 function symbol；与 Kotlin 同名文件类型对齐；模块文档说明哪些 Kotlin function symbol 对应哪些 Rust function symbol
4. **整体**：Phase H 已交付的 `LinearFunctionSymbolAdapter` 在所有 example 中实际被消费；不再出现 "连续近似" / "Big-M 线性化" 这类绕过 function symbol 的注释；`cargo check --workspace` 通过
