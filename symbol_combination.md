# Symbol Combination 体系缺口分析与建设计划

## 目标 / Objective

让变量和中间符号成为领域模型的显式字段，有名字、有形状、有单位，可以被其他符号引用和组合。

当前 Kotlin 的建模形态：

```kotlin
class EdgeBandwidth(...) {
    lateinit var y: UIntVariable2            // 二维决策变量
    lateinit var bandwidth: LinearIntermediateSymbols1<Flt64>  // 一维派生中间符号

    fun register(model: LinearMetaModel<Flt64>): Try {
        y = UIntVariable2("y", Shape2(edges.size, services.size))
        // ... 设名称和范围 ...
        model.add(y)                         // 批量注册变量组合

        bandwidth = flatMap(                 // 从变量组合派生符号组合
            "bandwidth", edges,
            { e -> sum(y[e, _a]) },           // 通配下标聚合
            { (_, e) -> "$e" }
        )
        model.add(bandwidth)                 // 批量注册符号组合
    }
}
```

目标 Rust 建模形态：

```rust
pub struct EdgeBandwidth {
    pub y: VariableCombination2D<UContinuous>,
    pub bandwidth: LinearExpressionSymbols1<f64>,
}

impl EdgeBandwidth {
    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        self.y = VariableCombination2D::with_name_and_range_generator(...);
        model.register_combination(&self.y)?;

        self.bandwidth = flat_map1(
            "bandwidth", &edges,
            |e| self.y.sum_along_dimension(&[e], 1),  // 对第二维求和
            |_, e| format!("{}", e),
        );
        model.add_symbol_combination(&self.bandwidth)?;
        Ok(())
    }
}
```

## 架构愿景 / Architecture Vision

变量和中间符号不是构建完多项式就直接内联进约束的临时值，而是领域模型的持久化字段：

1. 变量组合（`VariableCombination<VT, S>`）已有基础设施，需要补批量注册入口
2. 符号组合（`SymbolCombination<Sym, S>`）完全缺失，需要新建
3. 符号组合需要 `map`/`flatMap` 工厂，从变量组合和领域对象派生
4. 符号组合需要批量注册入口
5. 变量组合和符号组合需要维度聚合能力（对应 Kotlin 的 `sum(y[e, _a])`）
6. 长期需要 `QuantitySymbolCombination`（带物理量）

## 当前 Rust 建模模式对比

### core demo 模式（不可取）

```rust
// 中间表达式内联成 Linear，不注册 IntermediateSymbol
let shipment = MultiArrayBuilder::new_by(shape, |_, vec| {
    Linear::new(
        stores.iter().map(|(s, _)| LinearMonomial::new(1.0, x_vars[&[w, s]].to_owned_symbol())).collect(),
        0.0,
    )
});
model.add_math_inequality(shipment[w].clone().le(cap), ...);
```

问题：
- 中间表达式没有 ID，无法被其他符号引用
- 无法做依赖追踪和影子价格提取
- 列生成场景无法复用

### framework demo2/4/gantt 模式（方向对了但缺组合）

```rust
// 逐个 Arc::new + add_symbol，符号是孤立的
let symbol = LinearExpressionSymbol::new(next_id, "estimate_total_weight", monomials, constant);
model.add_symbol(Arc::new(symbol))?;
```

问题：
- 符号没有组合容器，无法按形状整体管理
- 没有工厂方法，每个符号手动构造
- 无法用维度聚合语法组合引用

### framework gantt IndexedVariableArray 模式（变量侧已接近）

```rust
// 变量有组合容器 + 领域键映射 + 批量注册，但符号侧缺失
let indexed_vars = IndexedVariableArray1::<K, VT>::new("y", keys, model)?;
```

变量侧已接近目标形态，但符号侧没有对应的 `IndexedSymbolArray`。

## 缺口清单 / Gap List

### Gap 1: SymbolCombination 容器

Kotlin `SymbolCombination<Sym, S>` 是 `MultiArray<Sym, S>`，提供：
- 按形状存储中间符号
- 按索引 / 向量坐标访问符号
- 组 ID 和名称前缀
- `withIndex()` 遍历
- init 块里给 `LinearExpressionSymbol` / `QuadraticExpressionSymbol` 设 `_group` 和 `_index`（指向所属组合）

Rust 完全不存在。

需要新增：`ospf-rust-core/src/symbol/symbol_combination.rs`

```rust
pub struct SymbolCombination<Sym, S: AbstractShape>
where
    Sym: IntermediateSymbol,
{
    symbols: MultiArray<Arc<Sym>, S>,
    group_id: usize,
    name_prefix: String,
}
```

类型别名：

```rust
pub type LinearExpressionSymbols1<V> = SymbolCombination<LinearExpressionSymbol<V>, Shape<1>>;
pub type LinearExpressionSymbols2<V> = SymbolCombination<LinearExpressionSymbol<V>, Shape<2>>;
pub type LinearExpressionSymbols3<V> = SymbolCombination<LinearExpressionSymbol<V>, Shape<3>>;
pub type LinearExpressionSymbols4<V> = SymbolCombination<LinearExpressionSymbol<V>, Shape<4>>;
```

`group_id` 生成：使用 `ospf_rust_core::variable::new_group_id()` 全局递增生成器，和 `VariableCombination` 共享同一 ID 空间。这确保变量组合和符号组合的组 ID 不会冲突。

关于 Kotlin 的 `_group` / `_index` 设定：Kotlin 在 `SymbolCombination` 的 init 块里给每个 `LinearExpressionSymbol` 设内部字段 `_group = this` 和 `_index = i`，用于反向引用所属组合。Rust 端的 `LinearExpressionSymbol` 目前没有这些字段。Phase 1 暂不添加反向引用，因为：
- Rust 的 `Arc<dyn IntermediateSymbol>` 已经提供了符号 ID，可以通过 `SymbolCombination` 的索引映射实现等价查询
- 如果后续需要，可以给 `LinearExpressionSymbol` 增加可选的 `combination_group_id: Option<usize>` 和 `combination_index: Option<usize>` 字段

### Gap 2: SymbolCombination map/flatMap 工厂

Kotlin `flatMap` 是核心建模语法糖：

```kotlin
bandwidth = flatMap(
    "bandwidth", edges,
    { e -> sum(y[e, _a]) },
    { (_, e) -> "$e" }
)
```

Rust 需要对应的工厂函数。

#### 符号 ID 分配策略

每个 `LinearExpressionSymbol` 需要一个唯一 `id: u64`。Kotlin 使用构造函数内的自增 ID。Rust 端建议：

- 在 `flat_map` 工厂内使用 `crate::symbol::next_auto_intermediate_symbol_id()` 生成符号 ID
- 这是 `ospf-rust-core/src/symbol/intermediate_symbol.rs` 已有的全局原子递增 ID 生成器，起始值 `1_000_000_000`，与手写 ID 区分命名空间
- 工厂批量创建符号时，每个符号依次获取新 ID

#### 工厂函数签名

```rust
/// 一维工厂：从领域对象列表派生一维符号组合
/// 1D factory: derive 1D symbol combination from domain object list
///
/// # 参数 / Parameters
/// - `name`: 符号组合名称前缀 / Symbol combination name prefix
/// - `objs`: 领域对象切片 / Domain object slice
/// - `ctor`: 构造函数，接收领域对象引用，返回 `Linear<V>` / Constructor, receives domain object reference, returns `Linear<V>`
/// - `suffix`: 名称后缀函数，接收索引和对象引用 / Name suffix function, receives index and object reference
pub fn flat_map1<T, V>(
    name: &str,
    objs: &[T],
    ctor: impl Fn(&T) -> Linear<V>,
    suffix: impl Fn(usize, &T) -> String,
) -> LinearExpressionSymbols1<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,

/// 二维工厂：从两组领域对象派生二维符号组合
/// 2D factory: derive 2D symbol combination from two domain object lists
pub fn flat_map2<T1, T2, V>(
    name: &str,
    objs1: &[T1],
    objs2: &[T2],
    ctor: impl Fn(&T1, &T2) -> Linear<V>,
    suffix: impl Fn(usize, &T1, usize, &T2) -> String,
) -> LinearExpressionSymbols2<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,

/// 三维工厂
pub fn flat_map3<T1, T2, T3, V>(...) -> LinearExpressionSymbols3<V>
```

`map` 变体与 `flat_map` 的区别：
- `map`: ctor 返回单个值（如 `Flt64`），包装成常数项为该值的 `LinearExpressionSymbol`
- `flat_map`: ctor 返回 `Linear<V>` 多项式，包装成含单项式的 `LinearExpressionSymbol`

### Gap 3: SymbolCombination 批量注册

Kotlin `model.add(bandwidth)` 遍历符号组合批量注册。

Rust `add_symbols` 已接受 `Iterator<Item = Arc<dyn IntermediateSymbol<V>>>`，但 `SymbolCombination` 的元素是 `Arc<Sym>`，需要类型转换（`Arc<Sym>` where `Sym: IntermediateSymbol<V>` -> `Arc<dyn IntermediateSymbol<V>>`）。

建议新增：

```rust
impl<V> MetaModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 批量注册符号组合 / Batch register symbol combination
    pub fn add_symbol_combination<Sym, S>(
        &mut self,
        combination: &SymbolCombination<Sym, S>,
    ) -> Result<()>
    where
        Sym: IntermediateSymbol<V>,
    {
        self.add_symbols(combination.iter_arc())
    }
}

impl<Sym, S> SymbolCombination<Sym, S>
where
    Sym: IntermediateSymbol,
{
    /// 返回 `Arc<dyn IntermediateSymbol>` 迭代器，用于批量注册
    /// Returns iterator of `Arc<dyn IntermediateSymbol>` for batch registration
    pub fn iter_arc(&self) -> impl Iterator<Item = Arc<dyn IntermediateSymbol>> + '_
    where
        Sym: 'static,
    {
        self.symbols.iter().cloned().map(|arc| arc as Arc<dyn IntermediateSymbol>)
    }
}
```

`BasicModel` 同样需要新增 `add_symbol_combination`。

### Gap 4: 变量组合批量注册入口

Kotlin `model.add(y)` 一行搞定。

Rust 当前需要 `model.register_variables::<VT, _>(y.iter().cloned())?` + `MultiArrayBuilder::from_list` 构建索引数组。

建议新增：

```rust
impl<V> MetaModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 批量注册变量组合，返回索引数组 / Batch register variable combination, returning index array
    pub fn register_combination<VT, S>(
        &mut self,
        combination: &VariableCombination<VT, S>,
    ) -> Result<MultiArray<usize, S>>
    where
        VT: VariableTypeTrait,
        VT::Value: crate::token::IntoValue<V>,
    {
        let shape = combination.shape().clone();
        let indices: Vec<usize> = combination
            .iter()
            .map(|var| self.register_variable(var.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(MultiArrayBuilder::from_list(shape, indices))
    }
}
```

### Gap 5: 维度聚合（sum across dimension）

Kotlin `sum(y[e, _a])` 把变量组合第二维所有元素求和，`_a` 是 `DummyIndex.All`。

Rust multiarray **已有完整的 DummyIndex 基础设施**：
- `DummyIndex` 支持 `Index`、`Range`（含全范围 `..`）、`IndexArray` 三种变体
- `Range(..)` 等价于 Kotlin 的 `All`，全范围语义已覆盖
- `dummy_expect![..]` / `dummy![..]` 宏提供 `_a` 等价的语法糖
- `MultiArray::view()` 支持基于 `DummyVector` 的切片视图
- `DummyAccessIterator` 支持按虚拟索引遍历多维数组子集

因此维度聚合不需要在 multiarray 层额外补 `All` 变体，只需要在 `VariableCombination` / `SymbolCombination` 上提供便利方法：

```rust
impl<VT: VariableTypeTrait, S: AbstractShape> VariableCombination<VT, S> {
    /// 对固定前缀索引的某维度求和，返回 Linear<V>
    /// Sum across a dimension with fixed prefix indices, returning Linear<V>
    ///
    /// 内部使用 DummyIndex::Range(..) + MultiArrayView 遍历该维度的所有元素，
    /// 对每个 VariableItem 调用 to_owned_symbol() 构造 LinearMonomial::new(1.0, symbol)，
    /// 最终返回求和后的 Linear<V>。
    ///
    /// # 参数 / Parameters
    /// - `fixed_indices`: 固定维度的索引值，长度必须等于 `S::NDIM - 1`
    /// - `dim`: 要求和的维度（0-based）
    pub fn sum_along_dimension<V>(
        &self,
        fixed_indices: &[usize],
        dim: usize,
    ) -> Linear<V>
    where
        V: Clone + Debug + Send + Sync + 'static,
    {
        // 构造 DummyVector：fixed 维度用 Index(i)，dim 维度用 Range(..)
        // 通过 MultiArrayView 遍历 dim 维度的所有元素
        // 对每个 VariableItem 调用 to_owned_symbol() 构造 LinearMonomial
        // 求和返回
    }
}

impl<Sym: IntermediateSymbol, S: AbstractShape> SymbolCombination<Sym, S> {
    /// 对固定前缀索引的某维度求和，返回 Linear<V>
    pub fn sum_along_dimension<V>(
        &self,
        fixed_indices: &[usize],
        dim: usize,
    ) -> Linear<V>
    where
        V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
        Sym: LinearIntermediateSymbol<V>,
    {
        // 类似 VariableCombination::sum_along_dimension
        // 但元素是 Arc<Sym>，需要通过 LinearIntermediateSymbol::to_linear_polynomial() 提取多项式后合并
    }
}
```

这些方法在 `flat_map` 工厂的 ctor 回调中使用，替代 Kotlin 的 `sum(y[e, _a])`。

长期可选：提供更通用的 `sum_slice` 方法，直接接受 `DummyVector` 切片描述。

### Gap 6: SymbolCombination 的多项式批量提取

Kotlin 的 `LinearIntermediateSymbol` 有 `toLinearPolynomial()` 方法，用于从符号提取多项式注册约束。Rust 端 `LinearIntermediateSymbol` trait 已有 `to_linear_polynomial()`。

当约束需要从 `SymbolCombination` 的某个元素提取多项式时，可以直接：

```rust
let poly = combination[i].to_linear_polynomial();
model.add_math_inequality(poly.le(rhs), "constraint_name");
```

不需要额外批量方法，因为约束注册通常是逐个的。但如果需要批量提取，可以在后续追加。

### Gap 7: QuantitySymbolCombination

Kotlin `QuantitySymbolCombination<Sym, S>` 是 `MultiArray<Quantity<Sym>, S>`。

这是长期目标，等 quantities 改造稳定后再做。

### Gap 8: 变量组合的范围和名称设定

Kotlin 可以在 `register` 里逐个设定：

```kotlin
y[e, s].name = "y_0_1"
y[e, s].range.leq(maxBandwidth)
```

Rust `VariableCombination` 已有 `with_name_and_range_generator`，可以构造时设好。但当前 demo1 连这个都没用，退化成手写 `Vec<Vec<usize>>`。

这不是基础设施缺口，是 demo 实现退化。

### Gap 9: demo4 bunch_compilation/model 文件拆解中断

`ospf-rust-example/src/framework/demo4/domain/bunch_compilation/model/` 目录下：

- `mod.rs` 包含完整的 `FlightCapacity`、`FleetBalance`（含 `FleetBalanceCheckpoint`、`FleetBalanceLimit`）、`FlightLink`、`Compilation` 四个 struct 及其 impl
- `flight_capacity.rs`、`fleet_balance.rs`、`flight_link.rs`、`compilation.rs` 四个文件已存在，但内容是 `mod.rs` 按行物理切割的片段，缺少文件头部（`use` 语句、struct 开头），无法独立编译
- `mod.rs` 没有 `mod flight_capacity;` 等子模块声明，未引用任何子文件

这是拆解到一半中断的状态：子文件存在但不完整，`mod.rs` 未声明子模块，编译时只使用 `mod.rs` 的内联内容。

修复方案：
1. 将 `mod.rs` 中的四个 struct 拆入对应子文件，每个子文件包含完整的 `use`、struct 定义和 impl
2. `mod.rs` 改为只包含 `pub mod flight_capacity;` 等模块声明和公共重导出
3. 验证编译通过

## Framework crate 改造范围 / Framework Crate Refactor Scope

结论：`ospf-rust-framework-bpp3d`、`ospf-rust-framework-csp1d`、`ospf-rust-framework-gantt-scheduling` 都应纳入改造计划。原因不是“代码能否编译”，而是它们已经承担优化模型装配职责，按 `.rules/framework-architecture.md` 应让 domain context / aggregation / model component 持有显式变量、中间值、派生表达式和结果解析引用。

这三个 crate 的问题层次不同：

1. Gantt Scheduling 最接近目标：变量侧已有 `IndexedVariableArray*` 包装 `VariableCombination`，但符号侧仍是 `Vec<Arc<LinearExpressionSymbol<f64>>>`，约束侧仍大量临时展开 `Linear`。
2. BPP3D 中等偏早期：有 context / aggregation / component 骨架，但 `VariableArray1/2` 只保存模型索引，`ExpressionArray1` 只保存 `(usize, f64)` 项列表，没有真正的中间符号字段。
3. CSP1D 最需要结构整理：列生成生命周期已经存在，但 `ProduceAggregation`、yield slack、length slack 主要保存 `Vec<usize>` / `Vec<Option<usize>>`，真实建模集中在 `domain/produce/mod.rs`，多个 `domain/*/model.rs` 仍是占位。

### 共享基础设施缺口

现有 `SymbolCombination` 是按 shape 管理的 dense multiarray，适合 demo1、Gantt 的常规二维/三维变量和符号。但三个 framework 还需要一层领域键适配：

1. `IndexedVariableCombination1/2/3<K, VT>`：持有 `VariableCombination<VT, Shape<N>>`、领域键到线性索引的映射，以及 `register_combination` 返回的模型索引数组。
2. `IndexedLinearExpressionSymbols1/2/3<K, V>`：持有 `LinearExpressionSymbolsN<V>`、领域键到线性索引的映射，并提供 `get_symbol`、`to_linear_polynomial`、`iter_symbols`。
3. `OptionalIndexedVariableArray` / `OptionalIndexedSymbolArray`：覆盖 Gantt `switch_time_symbols: Vec<Option<Arc<_>>>`、CSP1D yield/length slack 这种“部分需求才建变量/符号”的稀疏字段。
4. `AppendableVariablePool<K, VT>`：覆盖 CSP1D、BPP3D RMP、Gantt iterative 中“列生成新增列”的生命周期。它需要保存变量项、模型索引、active/retired 状态，并在 `add_columns` 时追加变量，在 `remove_columns` 时固定变量范围或标记退役。
5. `AppendableSymbolPool<K, V>`：覆盖列生成中随列池刷新的一维派生符号，例如 plan demand fulfillment、material usage、machine usage、task/bunch compilation 统计符号。

这些适配器不应放进 `ospf-rust-core`，因为 core 只应该知道 shape 和符号系统，不应该知道领域键、列池生命周期或 application context。建议放在 `ospf-rust-framework/src/model/`，再由 BPP3D、CSP1D、Gantt 复用。Gantt 现有 `task_compilation/adapter.rs` 可作为迁移参考，但最终应避免每个 framework 各自维护一套索引变量数组。

### BPP3D 现状与改造点

代表文件：

- `ospf-rust-framework-bpp3d/src/domain/layer_assignment/model/variable_array1.rs`
- `ospf-rust-framework-bpp3d/src/domain/layer_assignment/model/variable_array2.rs`
- `ospf-rust-framework-bpp3d/src/domain/layer_assignment/model/expression_array.rs`
- `ospf-rust-framework-bpp3d/src/domain/layer_assignment/service/assignment.rs`
- `ospf-rust-framework-bpp3d/src/domain/layer_assignment/service/load.rs`
- `ospf-rust-framework-bpp3d/src/domain/layer_assignment/service/capacity.rs`
- `ospf-rust-framework-bpp3d/src/domain/layer_assignment/service/limits/*.rs`
- `ospf-rust-framework-bpp3d/src/domain/item/model/continuous_radius/component.rs`

当前问题：

1. `VariableArray1/2` 只保存 `HashMap<K, usize>`，没有保存 `VariableCombination` 和 `VariableItem`，变量不是领域模型的显式字段。
2. `ExpressionArray1<K>` 只保存 `HashMap<K, Vec<(usize, f64)>>`，不是 `IntermediateSymbol`，不能被其他符号组合引用。
3. `Load.load`、`Load.over_load`、`Load.less_load`、`Capacity.load_weight`、`Capacity.load_volume`、`Capacity.load_depth` 是“表达式项缓存”，不是显式中间符号。
4. `DemandConstraint` 等 limits 里存在 `.or(Some(...))` 的模型索引 fallback，隐含依赖变量注册顺序，和显式字段愿景冲突。
5. `continuous_radius/component.rs` 直接注册 radius、radius_squared、segment、lambda 变量并直接添加约束/目标，缺少显式的半径变量族和 PWL 中间符号字段。

目标形态：

```rust
pub struct ImpreciseAssignment<V, U> {
    pub layers: Vec<BinLayer<V, U>>,
    pub x: IndexedVariableCombination1<usize, UContinuous>,
    pub usage: IndexedLinearExpressionSymbols1<usize, f64>,
}

pub struct Capacity {
    pub load_weight: IndexedLinearExpressionSymbols1<usize, f64>,
    pub load_volume: IndexedLinearExpressionSymbols1<usize, f64>,
    pub load_depth: IndexedLinearExpressionSymbols1<usize, f64>,
}
```

改造策略：

1. 先在 framework 公共层引入 indexed variable/symbol 适配器，替换 BPP3D 自定义 `VariableArray1/2` 的内部实现，但保留原 public 方法作为兼容桥。
2. 将 `ExpressionArray1` 标记为迁移期兼容类型，新代码不再写入 `(usize, f64)`，而是写入 `IndexedLinearExpressionSymbols1`。
3. `assignment.rs` 中 `x` / `v` 改为显式 `IndexedVariableCombination*` 字段，注册时走 `model.register_combination`。
4. `load.rs` 和 `capacity.rs` 的表达式字段改为符号组合，limits 从 `symbol.to_linear_polynomial()` 注册约束。
5. 清理 `DemandConstraint`、`BinCapacityConstraint`、`BinDepthConstraint` 等 limits 的模型索引 fallback。
6. `continuous_radius` 拆出 `ContinuousRadiusVariables` 和 `ContinuousRadiusSymbols`，保留 PWL 约束注册逻辑，但变量族成为 component 字段。

### CSP1D 现状与改造点

代表文件：

- `ospf-rust-framework-csp1d/src/application/model.rs`
- `ospf-rust-framework-csp1d/src/application/service.rs`
- `ospf-rust-framework-csp1d/src/domain/produce/mod.rs`
- `ospf-rust-framework-csp1d/src/domain/produce/model.rs`
- `ospf-rust-framework-csp1d/src/domain/yield/model.rs`
- `ospf-rust-framework-csp1d/src/domain/wasting_minimization/model.rs`
- `ospf-rust-framework-csp1d/src/domain/length_assignment/model.rs`

当前问题：

1. `Csp1dAssignment` 只有 `x: Vec<usize>`，不是显式变量组合；它还位于 application model。
2. `ProduceAggregation` 保存 `plan_variable_indices: Vec<usize>` 和 `plans_iteration_variable_indices: Vec<Vec<usize>>`，列变量没有显式变量池。
3. `YieldSlackAggregation` 的 `under_production` / `over_production`、`LengthSlackAggregation` 的 `assigned_length` / `over_length` 是 `Vec<Option<usize>>`，缺失变量字段、变量名、单位口径和注册索引的统一封装。
4. `DemandConstraintPipeline`、`MaterialConstraintPipeline`、`MachineConstraintPipeline`、`YieldConstraintPipeline`、`LengthConstraintPipeline` 直接扫描 plan 并拼 `(usize, f64)`，没有 demand fulfillment、material usage、machine usage、yield balance、length balance 等显式中间符号。
5. `domain/produce/mod.rs` 同时承载模型、pipeline、shadow price、context、builder、extraction，多个 `domain/*/model.rs` 仍是占位，和 framework 架构规范不一致。

目标形态：

```rust
pub struct ProduceAggregation<V: SolveValue> {
    pub cutting_plans: Vec<CuttingPlan<V>>,
    pub x: AppendableVariablePool<usize, UInteger>,
    pub demand_fulfillment: IndexedLinearExpressionSymbols1<Csp1dShadowPriceKey, f64>,
    pub material_usage: IndexedLinearExpressionSymbols1<String, f64>,
    pub machine_usage: IndexedLinearExpressionSymbols1<String, f64>,
}

pub struct YieldSlackAggregation<V: SolveValue> {
    pub under_production: OptionalIndexedVariableArray<usize, UContinuous>,
    pub over_production: OptionalIndexedVariableArray<usize, UContinuous>,
    pub balance: IndexedLinearExpressionSymbols1<usize, f64>,
}
```

改造策略：

1. 先新增 `PlanUsageVariablePool`，内部基于 `AppendableVariablePool`，覆盖 LP/MILP 两种变量类型差异和列生成 add/remove 生命周期。
2. `ProduceAggregation::register` 不再返回裸 `Vec<usize>`，而是填充 `x` 变量池；`plan_variable_index` 仅作为兼容只读 API。
3. 将 yield/length slack 变量替换为 optional indexed variable 字段，并把变量注册下沉到各自 aggregation。
4. 为 demand/material/machine/yield/length 派生显式 `LinearExpressionSymbols` 字段，shadow price extractor 从符号/约束元数据读取，而不是重新扫描相同业务逻辑。
5. 拆分 `domain/produce/mod.rs`：至少拆出 `aggregation.rs`、`context.rs`、`pipeline/*.rs`、`shadow_price.rs`、`extraction.rs`，并让 `model.rs` 不再是占位。
6. `application/service.rs` 保留求解编排、恢复、warm start、状态映射；不再承载建模细节或 token 回填细节。

### Gantt Scheduling 现状与改造点

代表文件：

- `ospf-rust-framework-gantt-scheduling/src/domain/task_compilation/adapter.rs`
- `ospf-rust-framework-gantt-scheduling/src/domain/task_compilation/model.rs`
- `ospf-rust-framework-gantt-scheduling/src/domain/task_compilation/iterative.rs`
- `ospf-rust-framework-gantt-scheduling/src/domain/task_compilation/service/limits.rs`
- `ospf-rust-framework-gantt-scheduling/src/domain/capacity_scheduling/model.rs`
- `ospf-rust-framework-gantt-scheduling/src/domain/produce/model/usage.rs`
- `ospf-rust-framework-gantt-scheduling/src/domain/resource/model/usage.rs`
- `ospf-rust-framework-gantt-scheduling/src/domain/resource/model/storage_usage.rs`
- `ospf-rust-framework-gantt-scheduling/src/domain/resource/model/connection_usage.rs`
- `ospf-rust-framework-gantt-scheduling/src/domain/bunch_compilation/model.rs`
- `ospf-rust-framework-gantt-scheduling/src/domain/bunch_compilation/iterative.rs`

当前问题：

1. `IndexedVariableArray1/2/3` 已包装 `VariableCombination`，但仍逐个调用 `model.register_variable`，未复用 `MetaModel::register_combination`。
2. 符号字段大量是 `Vec<Arc<LinearExpressionSymbol<f64>>>` 或 `Vec<Option<Arc<_>>>`，例如 `task_assignment_symbols`、`operation_time_symbols`、`capacity_symbols`、`quantity_symbols`、`switch_time_symbols`。
3. `task_compilation/service/limits.rs` 有 `expand_symbols_to_polynomials(symbols: &[Arc<_>])`，说明约束层仍按孤立符号列表工作，没有组合视角。
4. iterative 模块因为符号不可变而整批重建 `Vec<Arc<_>>`，需要显式的 refresh/replace 策略，而不是让约束层理解重建细节。
5. produce/resource usage 模型虽然注册了 quantity symbols，但容量 slack、上下界 slack 仍常在 register 内临时构造。

目标形态：

```rust
pub struct TaskCompilation {
    pub x: IndexedVariableCombination2<usize, usize, Binary>,
    pub y: Option<IndexedVariableCombination1<usize, Binary>>,
    pub z: Option<IndexedVariableCombination1<usize, Binary>>,
    pub task_assignment: IndexedLinearExpressionSymbols1<usize, f64>,
    pub task_compilation: IndexedLinearExpressionSymbols1<usize, f64>,
    pub executor_compilation: IndexedLinearExpressionSymbols1<usize, f64>,
}
```

改造策略：

1. 将 `task_compilation/adapter.rs` 中的 `IndexedVariableArray*` 迁移到 `ospf-rust-framework` 公共 indexed adapter，Gantt 侧改为重导出或薄封装。
2. 新增 `IndexedLinearExpressionSymbols*`，替换 `Vec<Arc<LinearExpressionSymbol<f64>>>` 字段。
3. 对 `Vec<Option<Arc<_>>>` 的 switch time/masking/front/between 符号，引入 optional indexed symbol array，保留稀疏语义。
4. `capacity_scheduling` 的 `operation_time_symbols` 和 `capacity_symbols` 改为二维/三维符号组合，limits 通过组合字段取多项式。
5. produce/resource usage 的 `quantity_symbols` 改为 indexed symbol array，并把 over/less slack 也显式字段化。
6. `bunch_compilation` 和 `task_compilation` iterative 模块在 add/remove columns 时刷新 symbol pool，不再暴露裸 `Vec<Arc<_>>` 给 service/limits。

## Phase 1-2 复检记录 / Phase 1-2 Review Notes

另一个会话已完成并提交：

1. `312c878 feat(core): add SymbolCombination container with type aliases`
2. `00c7655 feat(core): add flat_map/map factory functions for SymbolCombination`
3. `88c5d3c feat(core): add register_combination and add_symbol_combination to MetaModel`

已验证：

1. `cargo test -p ospf-rust-core` 通过
2. `cargo check --workspace` 通过
3. `ospf-rust-core/src/symbol/mod.rs` 已导出 `symbol_combination` 和 `symbol_combination_factory`
4. `MetaModel::register_combination`、`MetaModel::add_symbol_combination`、`BasicModel::add_symbol_combination` 已存在

仍需跟进：

1. `SymbolCombination::with_name_generator` 当前包含 `let _ = name_gen;`，没有实际使用 `name_gen`，应在 Phase 1 修正或删除该 API，避免调用方误以为名称后缀会生效。
2. `SymbolCombination<V, Sym, S>` 当前把 `V` 放在 struct 泛型里，并通过 `PhantomData<V>` 保存；这能编译，但比原计划 `SymbolCombination<Sym, S>` 更重。后续如要支持非 `IntermediateSymbol<V>` 的通用符号组合，需要重新评估泛型位置。
3. `iter_arc`、`add_symbol_combination` 当前对 `V` 要求 `Add<Output = V> + Mul<Output = V>`。这可能来自 `LinearExpressionSymbol` 的实现约束，但 `IntermediateSymbol` 本身不要求这些 bound。后续应确认是否能把 bound 下沉到 `LinearExpressionSymbol` 专用路径，避免限制非线性或函数符号组合。
4. Phase 3 的 `sum_along_dimension` 尚未实现；framework 和 demo1 迁移前必须先完成它，或至少先提供基于 `DummyVector` 的 `sum_slice`。

## 计划 / Plan

### Phase 1: SymbolCombination 容器与工厂

#### 1.1 SymbolCombination 容器

新增 `ospf-rust-core/src/symbol/symbol_combination.rs`：

**struct 定义：**

```rust
pub struct SymbolCombination<Sym, S: AbstractShape>
where
    Sym: IntermediateSymbol,
{
    symbols: MultiArray<Arc<Sym>, S>,
    group_id: usize,
    name_prefix: String,
}
```

**构造方法：**

1. `new(shape, name, ctor)` — 基本构造，ctor 接收 `(linear_index, vector_coords) -> Sym`
2. `with_name_generator(shape, name, name_gen, ctor)` — 自定义名称后缀，ctor 接收 `(linear_index, vector_coords) -> Sym`

**group_id 生成：** 构造时调用 `crate::variable::new_group_id()` 全局递增，与 `VariableCombination` 共享 ID 空间。

**访问方法：**

1. `iter()` — `impl Iterator<Item = &Arc<Sym>>`
2. `iter_arc()` — `impl Iterator<Item = Arc<dyn IntermediateSymbol>>`（需要 `Sym: 'static`，用于批量注册）
3. `as_array()` — `&MultiArray<Arc<Sym>, S>`
4. `into_array()` — `MultiArray<Arc<Sym>, S>`
5. `shape()` — `&S`
6. `len()` — `usize`
7. `is_empty()` — `bool`
8. `group_id()` — `usize`
9. `name_prefix()` — `&str`

**Index 实现：**

- `impl<Sym, S> Index<usize> for SymbolCombination<Sym, S>` — 按线性索引访问
- `impl<Sym, S, const N: usize> Index<[usize; N]> for SymbolCombination<Sym, S>` — 按向量坐标访问（需要 `S: AbstractShape<NDIM = N>` 或类似约束）

**类型别名：**

```rust
pub type LinearExpressionSymbols1<V> = SymbolCombination<LinearExpressionSymbol<V>, Shape<1>>;
pub type LinearExpressionSymbols2<V> = SymbolCombination<LinearExpressionSymbol<V>, Shape<2>>;
pub type LinearExpressionSymbols3<V> = SymbolCombination<LinearExpressionSymbol<V>, Shape<3>>;
pub type LinearExpressionSymbols4<V> = SymbolCombination<LinearExpressionSymbol<V>, Shape<4>>;
```

**测试：**
- 创建 `LinearExpressionSymbols1<f64>`，验证按索引访问
- 创建 `LinearExpressionSymbols2<f64>`，验证按向量坐标访问
- 验证 `group_id` 唯一递增
- 验证 `iter_arc()` 返回的元素可以传给 `add_symbols`

#### 1.2 SymbolCombination 工厂

新增 `ospf-rust-core/src/symbol/symbol_combination_factory.rs`：

**符号 ID 分配：** 使用 `crate::symbol::next_auto_intermediate_symbol_id()` 全局原子递增，起始值 `1_000_000_000`。

**一维工厂：**

```rust
pub fn flat_map1<T, V>(name, objs, ctor, suffix) -> LinearExpressionSymbols1<V>
pub fn map1<T, V>(name, objs, ctor, suffix) -> LinearExpressionSymbols1<V>
```

- `flat_map1`: ctor 返回 `Linear<V>` 多项式
- `map1`: ctor 返回单个值 `V`，包装成常数项

**二维工厂：**

```rust
pub fn flat_map2<T1, T2, V>(name, objs1, objs2, ctor, suffix) -> LinearExpressionSymbols2<V>
pub fn map2<T1, T2, V>(name, objs1, objs2, ctor, suffix) -> LinearExpressionSymbols2<V>
```

**三维工厂：**

```rust
pub fn flat_map3<T1, T2, T3, V>(name, objs1, objs2, objs3, ctor, suffix) -> LinearExpressionSymbols3<V>
pub fn map3<T1, T2, T3, V>(name, objs1, objs2, objs3, ctor, suffix) -> LinearExpressionSymbols3<V>
```

**内部逻辑：**

1. 遍历领域对象列表的笛卡尔积
2. 对每个组合调用 ctor 获取 `Linear<V>`（或 `V`）
3. 调用 `next_auto_intermediate_symbol_id()` 获取符号 ID
4. 用 `suffix` 生成名称后缀，组合成 `"name_suffix"` 格式的符号名
5. 构造 `LinearExpressionSymbol::new(id, name, monomials, constant)`
6. 包装成 `Arc` 放入 `MultiArray`
7. 组装成 `SymbolCombination`

**测试：**
- `flat_map1` 从整数列表 + ctor 创建 `LinearExpressionSymbols1<f64>`
- `flat_map2` 从两组领域对象 + ctor 创建 `LinearExpressionSymbols2<f64>`，ctor 内引用 `VariableCombination2D` 元素
- 验证符号 ID 不重复
- 验证符号名称格式正确

### Phase 2: 批量注册入口

#### 2.1 MetaModel::register_combination

```rust
impl<V> MetaModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn register_combination<VT, S>(
        &mut self,
        combination: &VariableCombination<VT, S>,
    ) -> Result<MultiArray<usize, S>>
    where
        VT: VariableTypeTrait,
        VT::Value: crate::token::IntoValue<V>,
    {
        let shape = combination.shape().clone();
        let indices: Vec<usize> = combination
            .iter()
            .map(|var| self.register_variable(var.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(MultiArrayBuilder::from_list(shape, indices))
    }
}
```

**测试：** 注册 `VariableCombination2D<Binary>`，验证返回的 `MultiArray<usize, Shape<2>>` 索引连续且正确。

#### 2.2 MetaModel::add_symbol_combination

```rust
impl<V> MetaModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn add_symbol_combination<Sym, S>(
        &mut self,
        combination: &SymbolCombination<Sym, S>,
    ) -> Result<()>
    where
        Sym: IntermediateSymbol<V> + 'static,
    {
        self.add_symbols(combination.iter_arc())
    }
}
```

**测试：** 创建 `LinearExpressionSymbols1<f64>` 并 `add_symbol_combination`，验证符号出现在 model 中。

#### 2.3 BasicModel::add_symbol_combination

与 `MetaModel` 版本相同，委托给 `add_symbol`。

### Phase 3: 维度聚合

本阶段的关键接口约束：聚合方法必须返回 `crate::symbol::flatten::Linear<V>`（即 core 模型层的 flatten polynomial），不能返回 `ospf_rust_math::symbol::Linear<V>`。`flat_map*` 工厂、`LinearExpressionSymbol::new` 和后续 `to_linear_polynomial()` 都以 core flatten 类型为衔接点；如果这里返回 math 层 `Linear`，core demo 和 framework demo 的显式符号字段化会在 API 层断开。

#### 3.1 VariableCombination::sum_along_dimension

```rust
impl<VT: VariableTypeTrait, S: AbstractShape> VariableCombination<VT, S> {
    /// 对固定前缀索引的某维度求和，返回模型级 Linear<V>
    pub fn sum_along_dimension<V>(
        &self,
        fixed_indices: &[usize],
        dim: usize,
        index_array: &MultiArray<usize, S>,
        coefficient: V,
        zero: V,
    ) -> crate::symbol::flatten::Linear<V>
    where
        V: Clone + Debug + Send + Sync + 'static,
        VT: Clone,
    {
        // 1. 构造 DummyVector：fixed 维度用 Index(i)，dim 维度用 Range(..)
        // 2. 调用 MultiArray::view(&dummy_vector) 获取切片视图
        // 3. 遍历视图中的每个 VariableItem
        // 4. 用 index_array 将 VariableItem 映射到模型 token index
        // 5. 构造 crate::symbol::flatten::LinearMonomial::new(coefficient, model_index)
        // 6. 收集所有 monomial，构造 crate::symbol::flatten::Linear::new(monomials, zero)
    }
}
```

**测试：** 对 `VariableCombination2D<UContinuous>` 形状 [3, 4]，先调用 `model.register_combination(&vars)` 得到 `y_idx`，再调用 `sum_along_dimension(&[1], 1, &y_idx, 1.0_f64, 0.0_f64)`，应返回 4 个 core flatten `LinearMonomial`（对第 1 行的 4 个变量求和），且每个 monomial 的 `var_index` 来自 `y_idx`。

#### 3.2 SymbolCombination::sum_along_dimension

```rust
impl<Sym: IntermediateSymbol, S: AbstractShape> SymbolCombination<Sym, S> {
    /// 对固定前缀索引的某维度求和，返回模型级 Linear<V>
    pub fn sum_along_dimension<V>(
        &self,
        fixed_indices: &[usize],
        dim: usize,
        zero: V,
    ) -> crate::symbol::flatten::Linear<V>
    where
        V: Clone + Debug + Send + Sync + 'static + Add<Output = V>,
        Sym: LinearIntermediateSymbol<V> + 'static,
    {
        // 类似 VariableCombination::sum_along_dimension
        // 但元素是 Arc<Sym>，通过 LinearIntermediateSymbol::to_linear_polynomial() 提取多项式后合并
    }
}
```

**测试：** 对 `LinearExpressionSymbols2<f64>` 形状 [3, 4]，`sum_along_dimension(&[1], 1, 0.0_f64)` 应返回第 1 行 4 个符号的 core flatten 多项式之和。

#### 3.3 DummyIndex 对齐要求

`ospf-rust-multiarray/src/dummy_index.rs` 已提供 `DummyIndex::Range(..)`、`dummy![..]` / `dummy_expect![..]` 等 `_a` 等价能力。维度聚合的最终实现应优先复用这些切片语义，避免在多个组合类型里复制坐标匹配逻辑。

最低要求：

1. `VariableCombination::sum_along_dimension` 和 `SymbolCombination::sum_along_dimension` 的行为必须与 `DummyIndex::Range(..)` 切片一致。
2. 若第一版为快速落地使用 `enumerate()` 过滤坐标，也必须在测试中覆盖 `dim = 0`、`dim = 1` 和三维场景，后续再收敛到 `MultiArray::view()` 或通用 `sum_slice(DummyVector)`。
3. 长期追加 `sum_slice(dummy_vector, ...)`，直接接受 `DummyVector`，让调用方可以表达 `sum(y[_a, s])`、`sum(y[e, _a])`、`sum(y[_a, n, _a])` 等更复杂聚合。

### Phase 4: core demo1 建模示范重写

`ospf-rust-example/src/core/demo1.rs` 是最小建模示范，应先于 framework demo 改造。它不涉及 context / aggregation / pipeline，可以专门验证 core 层新 API 是否满足“变量和中间符号都是显式字段”的愿景。

#### 4.1 当前问题

当前 `core/demo1.rs` 已经使用 `VariableCombination1D<Binary>` 表示公司选择变量，但仍存在以下缺口：

1. 变量注册仍逐个 `meta_model.register_variable(var.clone())?`，没有展示 `register_combination`。
2. `company_metric_expression(...)` 返回 `ospf_rust_math::symbol::Linear<f64>`，中间表达式没有注册成 `IntermediateSymbol`。
3. `total_capital_expr`、`total_liability_expr`、`total_profit_expr` 是局部临时多项式，不是模型字段。
4. 约束和目标直接消费临时多项式，无法示范符号复用、依赖追踪和后续影子价格提取。

#### 4.2 目标模型字段

新增或内联一个小型显式模型结构：

```rust
pub struct PortfolioSelectionModel {
    pub select: VariableCombination1D<Binary>,
    pub select_idx: MultiArray1<usize>,
    pub metrics: LinearExpressionSymbols1<f64>,
}
```

`metrics` 使用一维符号组合表达三个派生量：

1. `total_capital`
2. `total_liability`
3. `total_profit`

也可以拆成三个命名字段，但第一版推荐 `LinearExpressionSymbols1<f64>`，因为它能集中展示 `flat_map1`、批量注册和按索引取符号的完整链路。

#### 4.3 注册流程

目标流程：

1. 用 `VariableCombination1D::with_name_generator` 或 `VariableCombination1D::new` 创建 `select`。
2. 用 `meta_model.register_combination(&select)?` 批量注册变量，并保存 `select_idx`。
3. 定义 `PortfolioMetric` 或等价数据结构，描述 capital / liability / profit 的名称和系数函数。
4. 用 `flat_map1("portfolio_metric", &metrics, |metric| metric_expression(...), suffix)` 创建 `LinearExpressionSymbols1<f64>`。
5. `metric_expression` 必须返回 `crate::symbol::flatten::Linear<f64>`，单项式的变量索引来自 `select_idx`。
6. 调用 `meta_model.add_symbol_combination(&portfolio.metrics)?` 批量注册中间符号。
7. 约束和目标统一从 `portfolio.metrics[...] .to_linear_polynomial()` 提取多项式。

#### 4.4 约束和目标改写

当前方式：

```rust
meta_model.add_math_inequality(total_capital_expr.ge(min_capital), "capital_constraint");
meta_model.set_math_linear_objective(total_profit_expr, ObjectiveCategory::Maximum, "total_profit")?;
```

目标方式：

```rust
let total_capital = portfolio.metrics[0].to_linear_polynomial();
let total_liability = portfolio.metrics[1].to_linear_polynomial();
let total_profit = portfolio.metrics[2].to_linear_polynomial();

meta_model.add_inequality(
    LinearInequality::greater_equal(total_capital, min_capital),
    "capital_constraint",
)?;
meta_model.add_inequality(
    LinearInequality::less_equal(total_liability, max_liability),
    "liability_constraint",
)?;
meta_model.set_objective_category(ObjectiveCategory::Maximum);
meta_model.add_sub_objective(SubObjective::new(
    ObjectiveCategory::Maximum,
    total_profit,
    "total_profit",
));
```

实际代码需引入 `LinearInequality` 和 `SubObjective` 等当前 core API。关键要求是约束和目标从显式 `LinearExpressionSymbol` 提取 core flatten 多项式，而不是直接消费局部 math expression。

#### 4.5 验证点

1. `core/demo1.rs` 不再把 `ospf_rust_math::symbol::Linear` 作为最终约束/目标输入。
2. 文件中至少有一个 `VariableCombination` + `LinearExpressionSymbols` + `add_symbol_combination` 的完整闭环。
3. `cargo run -p ospf-rust-example -- core:demo1` 的目标值、选中公司和测试断言与重写前一致。

### Phase 5: framework demo1 重写

#### 5.1 EdgeBandwidth

当前：

```rust
pub struct EdgeBandwidth {
    pub y_idx: Vec<Vec<usize>>,  // 退化：手动索引
}
```

目标：

```rust
pub struct EdgeBandwidth {
    pub y: VariableCombination2D<UContinuous>,
    pub y_idx: MultiArray2<usize>,  // register_combination 返回值
    pub bandwidth: LinearExpressionSymbols1<f64>,
}
```

改动要点：
- `y` 用 `VariableCombination2D::with_name_and_range_generator` 创建，设名称和范围
- `y_idx` 由 `model.register_combination(&self.y)?` 返回
- `bandwidth` 由 `flat_map1("bandwidth", edges, |e| self.y.sum_along_dimension(&[e], 1), ...)` 创建
- `model.add_symbol_combination(&self.bandwidth)?` 注册

#### 5.2 Assignment

当前：

```rust
pub struct Assignment {
    pub x_idx: Vec<Vec<usize>>,  // 退化
}
```

目标：

```rust
pub struct Assignment {
    pub x: VariableCombination2D<Binary>,
    pub x_idx: MultiArray2<usize>,
    pub node_assignment: LinearExpressionSymbols1<f64>,
    pub service_assignment: LinearExpressionSymbols1<f64>,
}
```

#### 5.3 ServiceBandwidth

目标：

```rust
pub struct ServiceBandwidth {
    pub in_degree: LinearExpressionSymbols2<f64>,
    pub out_degree: LinearExpressionSymbols2<f64>,
    pub out_flow: LinearExpressionSymbols2<f64>,
}
```

#### 5.4 NodeBandwidth

目标：

```rust
pub struct NodeBandwidth {
    pub in_degree: LinearExpressionSymbols1<f64>,
    pub out_degree: LinearExpressionSymbols1<f64>,
    pub out_flow: LinearExpressionSymbols1<f64>,
}
```

#### 5.5 约束文件改动

所有 limits 文件（`edge_bandwidth_constraint.rs`、`service_capacity_constraint.rs`、`transfer_node_bandwidth_constraint.rs`、`bandwidth_cost_objective.rs`、`demand_constraint.rs`）需要从组合字段引用符号。

当前方式（手动索引）：

```rust
let var_idx = y_idx[e][s];
coefficients.push((var_idx, 1.0));
```

目标方式（从符号组合提取多项式）：

```rust
// 方式 1：直接从符号提取多项式
let poly = aggregation.edge_bandwidth.bandwidth[e].to_linear_polynomial();
model.add_math_inequality(poly.le(max_bandwidth), "bandwidth_constraint");

// 方式 2：从变量组合构造单项式
let symbol = aggregation.edge_bandwidth.y[&[e, s]].to_owned_symbol();
coefficients.push(LinearMonomial::new(1.0, symbol));
```

### Phase 6: framework 共享 indexed adapter

在 `ospf-rust-framework` 公共层新增领域键索引适配器，避免 BPP3D、CSP1D、Gantt 各自维护一套不兼容的变量/符号数组。

#### 6.1 IndexedVariableCombination

新增：

- `IndexedVariableCombination1<K, VT>`
- `IndexedVariableCombination2<K1, K2, VT>`
- `IndexedVariableCombination3<K1, K2, K3, VT>`

职责：

1. 保存 `VariableCombination<VT, Shape<N>>`
2. 保存领域键到线性索引的映射
3. 保存 `model.register_combination(&combination)` 返回的模型索引数组
4. 暴露 `variable_item(...)`、`model_index(...)`、`combination()`、`model_indices()`
5. 支持 name/range generator，覆盖业务上界和变量命名

#### 6.2 IndexedLinearExpressionSymbols

新增：

- `IndexedLinearExpressionSymbols1<K, V>`
- `IndexedLinearExpressionSymbols2<K1, K2, V>`
- `IndexedLinearExpressionSymbols3<K1, K2, K3, V>`

职责：

1. 保存 `LinearExpressionSymbolsN<V>`
2. 保存领域键到线性索引的映射
3. 暴露 `symbol(...)`、`to_linear_polynomial(...)`、`combination()`
4. 注册时走 `model.add_symbol_combination`

#### 6.3 Optional indexed arrays

新增稀疏适配器：

- `OptionalIndexedVariableArray<K, VT>`
- `OptionalIndexedLinearExpressionSymbols<K, V>`

用于“有些领域键不建变量/符号”的场景：

- CSP1D under/over slack
- CSP1D assigned/over length
- Gantt switch time symbols
- Gantt masking/front/between 类稀疏符号

#### 6.4 Appendable pools

新增列生成适配器：

- `AppendableVariablePool<K, VT>`
- `AppendableSymbolPool<K, V>`

职责：

1. `register_initial` 注册初始列变量/符号
2. `add_columns` 追加变量/符号并记录 iteration
3. `remove_columns` 固定变量范围或标记 retired
4. 提供 active iterator，供约束刷新和结果提取使用
5. 保留模型索引查找 API，兼容旧 pipeline 逐步迁移

### Phase 7: BPP3D 建模字段显式化

#### 7.1 替换 layer_assignment 变量数组

将以下文件中的 `VariableArray1/2` 内部替换为 shared `IndexedVariableCombination*`：

- `ospf-rust-framework-bpp3d/src/domain/layer_assignment/model/variable_array1.rs`
- `ospf-rust-framework-bpp3d/src/domain/layer_assignment/model/variable_array2.rs`
- `ospf-rust-framework-bpp3d/src/domain/layer_assignment/service/assignment.rs`

兼容期可以保留 `index()` 方法，但其实现必须委托给 `model_index()`，不得再依赖手写注册顺序。

#### 7.2 替换 ExpressionArray1

将 `ExpressionArray1<K>` 从 `(usize, f64)` 缓存迁移为 `IndexedLinearExpressionSymbols1<K, f64>` 兼容包装。

改动目标：

- `Load.load`
- `Load.over_load`
- `Load.less_load`
- `Capacity.load_weight`
- `Capacity.load_volume`
- `Capacity.load_depth`

#### 7.3 重写 limits 引用方式

修改 `layer_assignment/service/limits/*.rs`：

1. 从 assignment/load/capacity 的显式字段取变量或符号
2. 使用 `to_linear_polynomial()` 注册约束和目标
3. 删除 `.or(Some(...))` 这种基于注册顺序的 fallback
4. shadow price key 与约束 metadata 继续保留

#### 7.4 continuous_radius component 字段化

在 `domain/item/model/continuous_radius/component.rs` 拆出：

- `ContinuousRadiusVariables`
- `ContinuousRadiusPiecewiseVariables`
- `ContinuousRadiusSymbols`

radius、radius_squared、segment、lambda 都应成为字段，而不是 `register_one_solver_variable` 的局部返回索引。

### Phase 8: CSP1D 列生成建模字段显式化

#### 8.1 ProduceAggregation 变量池

将 `ProduceAggregation` 的：

- `plan_variable_indices: Vec<usize>`
- `plans_iteration_variable_indices: Vec<Vec<usize>>`

替换为：

- `x: PlanUsageVariablePool<V>`

其中 `PlanUsageVariablePool` 内部根据 `Csp1dModelingMode` 选择 LP 连续变量或 MILP 无符号整数变量。

兼容期保留：

- `plan_variable_indices()`
- `plans_iteration_variable_indices()`
- `plan_variable_index(index)`

但这些方法必须只读代理到 `x`。

#### 8.2 Yield/Length slack 变量字段化

将：

- `YieldSlackAggregation.under_production`
- `YieldSlackAggregation.over_production`
- `LengthSlackAggregation.assigned_length`
- `LengthSlackAggregation.over_length`

迁移为 optional indexed variable 字段，并将 `register_yield_variables` / `register_length_variables` 下沉到对应 aggregation。

#### 8.3 派生符号字段化

为 Produce/Yield/Length 建立显式中间符号：

- demand fulfillment
- material usage
- machine batch usage
- machine capacity usage
- yield balance
- over production bound expression
- assigned length
- over length link

pipeline 约束从这些符号提取多项式，不再重复扫描 `cutting_plans` 拼 `(usize, f64)`。

#### 8.4 拆分 produce/mod.rs

将 `ospf-rust-framework-csp1d/src/domain/produce/mod.rs` 拆为：

- `aggregation.rs`
- `context.rs`
- `pipeline/demand.rs`
- `pipeline/material.rs`
- `pipeline/machine.rs`
- `pipeline/yield.rs`
- `pipeline/length.rs`
- `shadow_price.rs`
- `extraction.rs`
- `builder.rs`

并填充当前占位文件：

- `domain/produce/model.rs`
- `domain/yield/model.rs`
- `domain/wasting_minimization/model.rs`
- `domain/length_assignment/model.rs`

### Phase 9: Gantt Scheduling 符号组合迁移

#### 9.1 变量 adapter 收敛

将 `task_compilation/adapter.rs` 的 `IndexedVariableArray1/2/3` 迁移为 shared `IndexedVariableCombination1/2/3` 的薄封装或重导出。

注册逻辑改为：

```rust
let indices = model.register_combination(indexed.combination())?;
indexed.bind_model_indices(indices);
```

#### 9.2 task_compilation 符号字段迁移

替换：

- `task_assignment_symbols`
- `task_compilation_symbols`
- `executor_compilation_symbols`
- `estimate_start_time_symbols`
- `estimate_end_time_symbols`
- `switch_time_symbols`

目标类型为 indexed symbol array 或 optional indexed symbol array。

`service/limits.rs` 中 `from_symbols(&[Arc<_>])` 迁移为 `from_symbol_combination(...)` 或直接接收 indexed symbol array。

#### 9.3 capacity_scheduling 符号字段迁移

替换：

- `operation_time_symbols`
- `capacity_symbols`

根据 action/executor/slot 维度改为二维或三维 `IndexedLinearExpressionSymbols`。

#### 9.4 produce/resource usage 符号字段迁移

替换以下文件中的 `quantity_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>`：

- `domain/produce/model/usage.rs`
- `domain/resource/model/usage.rs`
- `domain/resource/model/storage_usage.rs`
- `domain/resource/model/connection_usage.rs`

同时把 over/less slack 符号也做成显式 optional indexed symbol 字段。

#### 9.5 iterative 模块刷新策略

`task_compilation/iterative.rs`、`bunch_compilation/iterative.rs` 不再向外暴露裸 `Vec<Arc<_>>`。新增：

- `refresh_symbols(model)`
- `replace_symbol_pool(model)`
- `active_symbols()`

约束刷新只依赖 symbol pool 的稳定 API。

### Phase 10: QuantitySymbolCombination（长期）

等 quantities UnitConversionValue 改造稳定后再做。

Kotlin `QuantitySymbolCombination<Sym, S>` 是 `MultiArray<Quantity<Sym>, S>`，每个元素带物理单位。

设计草图：

```rust
pub struct QuantitySymbolCombination<Sym, S: AbstractShape> {
    symbols: MultiArray<Quantity<Arc<Sym>, Unit>, S>,
    group_id: usize,
    name_prefix: String,
}
```

### Phase 11: demo4 bunch_compilation/model 文件拆解修复

修复 `ospf-rust-example/src/framework/demo4/domain/bunch_compilation/model/` 的拆解中断问题。

**当前状态：**

- `mod.rs` 包含完整内联代码（`FlightCapacity`、`FleetBalance` 及辅助类型、`FlightLink`、`Compilation`），无子模块声明
- `flight_capacity.rs`、`fleet_balance.rs`、`flight_link.rs`、`compilation.rs` 已存在，但内容是 `mod.rs` 的物理切割片段，缺少头部，无法独立编译

**修复步骤：**

1. **`flight_capacity.rs`**：补充完整文件头（`use` 语句），包含 `FlightCapacity` struct 及其 impl
2. **`fleet_balance.rs`**：补充完整文件头，包含 `FleetBalanceCheckpoint`、`FleetBalanceLimit`、`FleetBalance` 三个 struct 及 `FleetBalance` 的 impl
3. **`flight_link.rs`**：补充完整文件头，包含 `FlightLink` struct 及其 impl
4. **`compilation.rs`**：补充完整文件头，包含 `Compilation` struct 及其 impl
5. **`mod.rs`**：清空内联代码，改为模块声明和公共重导出：

```rust
pub mod compilation;
pub mod fleet_balance;
pub mod flight_capacity;
pub mod flight_link;

pub use compilation::Compilation;
pub use fleet_balance::{FleetBalance, FleetBalanceCheckpoint, FleetBalanceLimit};
pub use flight_capacity::FlightCapacity;
pub use flight_link::FlightLink;
```

6. 验证 `cargo check -p ospf-rust-example` 编译通过，确认上层引用不受影响

## 修改清单 / File Changes

### ospf-rust-core

新增文件：
- `src/symbol/symbol_combination.rs`
- `src/symbol/symbol_combination_factory.rs`

修改文件：
- `src/symbol/mod.rs` — 增加 `pub mod symbol_combination` 和 `pub mod symbol_combination_factory`，重导出类型别名和工厂函数
- `src/model/meta_model.rs` — 增加 `register_combination` 和 `add_symbol_combination`
- `src/model/basic_model.rs` — 增加 `add_symbol_combination`

### ospf-rust-example/src/core/demo1

修改文件：
- `demo1.rs` — 改为 `VariableCombination1D<Binary>` + `LinearExpressionSymbols1<f64>` 的显式建模示范，使用 `register_combination` 和 `add_symbol_combination`

### ospf-rust-example/src/framework/demo1

修改文件：
- `route_context/model/assignment.rs` — 改用 `VariableCombination2D<Binary>` + `LinearExpressionSymbols1<f64>`
- `route_context/model/graph.rs` — 可能需要适配
- `bandwidth_context/model/edge_bandwidth.rs` — 改用 `VariableCombination2D<UContinuous>` + `LinearExpressionSymbols1<f64>`
- `bandwidth_context/model/node_bandwidth.rs` — 改用 `LinearExpressionSymbols1<f64>`
- `bandwidth_context/model/service_bandwidth.rs` — 改用 `LinearExpressionSymbols2<f64>`
- `bandwidth_context/service/limits/edge_bandwidth_constraint.rs` — 从符号组合引用
- `bandwidth_context/service/limits/service_capacity_constraint.rs` — 从符号组合引用
- `bandwidth_context/service/limits/transfer_node_bandwidth_constraint.rs` — 从符号组合引用
- `bandwidth_context/service/limits/bandwidth_cost_objective.rs` — 从符号组合引用
- `bandwidth_context/service/limits/demand_constraint.rs` — 从符号组合引用
- `bandwidth_context/aggregation.rs` — 适配新字段类型
- `bandwidth_context/bandwidth_context.rs` — 适配新注册方式
- `route_context/service/pipeline_list_generator.rs` — 适配新字段类型

### ospf-rust-example/src/framework/demo4

修改文件：
- `domain/bunch_compilation/model/mod.rs` — 清空内联代码，改为模块声明和重导出
- `domain/bunch_compilation/model/flight_capacity.rs` — 补充完整文件头和 `FlightCapacity` 定义
- `domain/bunch_compilation/model/fleet_balance.rs` — 补充完整文件头和 `FleetBalance*` 定义
- `domain/bunch_compilation/model/flight_link.rs` — 补充完整文件头和 `FlightLink` 定义
- `domain/bunch_compilation/model/compilation.rs` — 补充完整文件头和 `Compilation` 定义

### ospf-rust-framework

新增文件：
- `src/model/indexed_variable_combination.rs` — shared indexed variable combination adapters
- `src/model/indexed_symbol_combination.rs` — shared indexed linear expression symbol adapters
- `src/model/optional_indexed_array.rs` — optional/sparse variable and symbol arrays
- `src/model/appendable_pool.rs` — column-generation variable/symbol pools

修改文件：
- `src/model/mod.rs` — 导出 indexed adapters 和 appendable pools
- `src/lib.rs` — 如当前导出结构需要，增加公共重导出

### ospf-rust-framework-bpp3d

修改文件：
- `src/domain/layer_assignment/model/variable_array1.rs` — 迁移到 `IndexedVariableCombination1`
- `src/domain/layer_assignment/model/variable_array2.rs` — 迁移到 `IndexedVariableCombination2`
- `src/domain/layer_assignment/model/expression_array.rs` — 迁移到 `IndexedLinearExpressionSymbols1`
- `src/domain/layer_assignment/service/assignment.rs` — `x` / `v` 成为显式变量组合字段
- `src/domain/layer_assignment/service/load.rs` — load/over/less load 成为显式符号字段
- `src/domain/layer_assignment/service/capacity.rs` — load weight/volume/depth 成为显式符号字段
- `src/domain/layer_assignment/service/limits/*.rs` — 从变量/符号组合注册约束，删除索引 fallback
- `src/domain/item/model/continuous_radius/component.rs` — 拆出连续半径变量族和 PWL 符号字段
- `src/domain/layer_assignment/model/tests.rs`、`service/limits/tests.rs` — 同步测试显式字段和兼容 API

### ospf-rust-framework-csp1d

修改文件：
- `src/application/model.rs` — `Csp1dAssignment` 不再持有裸 `Vec<usize>`，或迁移到 domain 层兼容包装
- `src/application/service.rs` — 保留编排职责，移除建模细节依赖
- `src/domain/produce/mod.rs` — 拆分为聚合、上下文、pipeline、shadow price、extraction 子模块
- `src/domain/produce/model.rs` — 填充 ProduceAggregation / variable pool 相关模型
- `src/domain/yield/model.rs` — 填充 YieldSlackAggregation 显式变量/符号字段
- `src/domain/wasting_minimization/model.rs` — 填充 waste objective/result 所需模型字段
- `src/domain/length_assignment/model.rs` — 填充 LengthSlackAggregation 显式变量/符号字段

新增文件：
- `src/domain/produce/aggregation.rs`
- `src/domain/produce/context.rs`
- `src/domain/produce/pipeline/demand.rs`
- `src/domain/produce/pipeline/material.rs`
- `src/domain/produce/pipeline/machine.rs`
- `src/domain/produce/pipeline/yield.rs`
- `src/domain/produce/pipeline/length.rs`
- `src/domain/produce/shadow_price.rs`
- `src/domain/produce/extraction.rs`
- `src/domain/produce/builder.rs`

### ospf-rust-framework-gantt-scheduling

修改文件：
- `src/domain/task_compilation/adapter.rs` — 收敛到 shared indexed adapters
- `src/domain/task_compilation/model.rs` — task/executor/time/switch 符号字段改为 indexed symbol arrays
- `src/domain/task_compilation/iterative.rs` — 使用 appendable/refreshable symbol pool
- `src/domain/task_compilation/service/limits.rs` — 约束入口接收 symbol combination，不再接收裸 `Vec<Arc<_>>`
- `src/domain/capacity_scheduling/model.rs` — operation/capacity 符号改为 indexed symbol arrays
- `src/domain/capacity_scheduling/service/limits.rs` — 从组合字段取多项式
- `src/domain/produce/model/usage.rs` — quantity/slack 符号字段组合化
- `src/domain/produce/service/limits.rs` — 从组合字段取多项式
- `src/domain/resource/model/usage.rs` — quantity/slack 符号字段组合化
- `src/domain/resource/model/storage_usage.rs` — quantity/slack 符号字段组合化
- `src/domain/resource/model/connection_usage.rs` — quantity/slack 符号字段组合化
- `src/domain/resource/service/limits.rs` — 从组合字段取多项式
- `src/domain/bunch_compilation/model.rs` — bunch/task/executor compilation 符号组合化
- `src/domain/bunch_compilation/iterative.rs` — 使用 appendable/refreshable symbol pool

## 验收标准 / Acceptance Criteria

### 基础设施

1. `SymbolCombination<LinearExpressionSymbol<f64>, Shape<1>>` 可编译、可构造、可按索引访问
2. `SymbolCombination` 的 `group_id` 全局唯一递增，与 `VariableCombination` 不冲突
3. `flat_map1` 工厂能从领域对象列表 + ctor 回调创建 `LinearExpressionSymbols1<f64>`
4. `flat_map2` 工厂能创建 `LinearExpressionSymbols2<f64>`，ctor 回调可引用 `VariableCombination2D` 的元素
5. `flat_map1`/`flat_map2` 内部使用 `next_auto_intermediate_symbol_id()` 分配符号 ID，ID 不重复
6. `model.register_combination(&y)` 返回 `MultiArray<usize, Shape<2>>`
7. `model.add_symbol_combination(&bandwidth)` 批量注册符号组合
8. `VariableCombination::sum_along_dimension` 能对某维度求和生成 core flatten `Linear<f64>`，并通过 `register_combination` 返回的 index array 写入模型 token index
9. `SymbolCombination::sum_along_dimension` 能对某维度求和生成 core flatten `Linear<f64>`
10. 维度聚合行为与 `DummyIndex::Range(..)` 切片一致；第一版若未直接使用 `MultiArrayView`，必须有覆盖等价行为的测试，并在后续补 `sum_slice(DummyVector)`

### core demo1

11. `ospf-rust-example/src/core/demo1.rs` 使用 `register_combination(&select)` 批量注册变量，不再逐个注册变量作为主流程
12. `core/demo1.rs` 中 capital/liability/profit 至少有一个统一的 `LinearExpressionSymbols1<f64>` 显式符号组合字段
13. `core/demo1.rs` 调用 `model.add_symbol_combination(&metrics)` 注册中间符号
14. `core/demo1.rs` 的约束和目标从 `metrics[i].to_linear_polynomial()` 提取多项式，不再以内联 `ospf_rust_math::symbol::Linear` 作为最终输入
15. `cargo run -p ospf-rust-example -- core:demo1` 结果与重写前一致

### framework demo1

16. `EdgeBandwidth` 的 `y` 是 `VariableCombination2D<UContinuous>` 显式字段
17. `EdgeBandwidth` 的 `bandwidth` 是 `LinearExpressionSymbols1<f64>` 显式字段
18. `Assignment` 的 `x` 是 `VariableCombination2D<Binary>` 显式字段
19. `Assignment` 的 `node_assignment` / `service_assignment` 是 `LinearExpressionSymbols1<f64>` 显式字段
20. `ServiceBandwidth` 的 `in_degree` / `out_degree` / `out_flow` 是 `LinearExpressionSymbols2<f64>` 显式字段
21. `NodeBandwidth` 的 `in_degree` / `out_degree` / `out_flow` 是 `LinearExpressionSymbols1<f64>` 显式字段
22. framework demo1 所有约束通过符号组合引用，不再手动拼 `Vec<Vec<usize>>` 索引
23. `cargo run -p ospf-rust-example -- framework:demo1` 求解结果与重写前一致（目标值相同，变量赋值一致）

### demo4

24. `bunch_compilation/model/mod.rs` 只包含模块声明和重导出，无内联 struct/impl
25. `flight_capacity.rs`、`fleet_balance.rs`、`flight_link.rs`、`compilation.rs` 各自包含完整可编译内容
26. 上层对 `FlightCapacity`、`FleetBalance`、`FlightLink`、`Compilation` 的引用不受影响

### framework shared adapters

27. `IndexedVariableCombination1/2/3` 能创建变量组合、绑定模型索引，并按领域键返回 `VariableItem` 和模型索引
28. `IndexedLinearExpressionSymbols1/2/3` 能创建符号组合、批量注册，并按领域键返回 `LinearExpressionSymbol`
29. optional indexed arrays 能表达“部分键没有变量/符号”，并且不会在约束注册时产生伪索引
30. appendable pool 能覆盖 initial/add/remove 生命周期，移除列时能固定变量为 0 或标记 retired

### BPP3D

31. `ImpreciseAssignment.x`、`PreciseAssignment.x`、`PreciseAssignment.v` 底层都是 `VariableCombination`
32. `Load` 和 `Capacity` 的派生量是 `IndexedLinearExpressionSymbols`，不是 `(usize, f64)` 缓存
33. layer_assignment limits 不再使用基于注册顺序的 `.or(Some(...))` fallback
34. `continuous_radius` 的 radius、radius_squared、segment、lambda 是 component 字段，可用于约束和结果提取
35. `cargo test -p ospf-rust-framework-bpp3d` 通过

### CSP1D

36. `ProduceAggregation` 使用 plan usage variable pool，不再直接持有可变裸 `Vec<usize>` 作为主状态
37. yield/length slack 使用 optional indexed variable arrays
38. demand/material/machine/yield/length 派生量有显式 symbol 字段，pipeline 从 symbol 字段注册约束
39. `domain/produce/mod.rs` 拆分完成，`domain/*/model.rs` 不再只是占位
40. add_columns/remove_columns 后变量池、符号池、约束刷新、目标刷新一致
41. `cargo test -p ospf-rust-framework-csp1d` 通过

### Gantt Scheduling

42. `task_compilation/adapter.rs` 不再维护独立变量数组实现，而是复用 shared indexed adapters
43. task/capacity/produce/resource/bunch compilation 的符号字段不再是裸 `Vec<Arc<LinearExpressionSymbol<f64>>>`
44. sparse switch/masking/front/between 符号使用 optional indexed symbol array
45. iterative add/remove columns 通过 symbol pool refresh API 完成，约束层不感知裸符号重建
46. `cargo test -p ospf-rust-framework-gantt-scheduling` 通过

### 编译与测试

47. 全 workspace 编译通过（`cargo check --workspace`）
48. quantities 测试通过（`cargo test -p ospf-rust-quantities`）
49. core 测试通过（`cargo test -p ospf-rust-core`）
50. example 测试通过（`cargo test -p ospf-rust-example`）
51. framework 公共层测试通过（`cargo test -p ospf-rust-framework`）

## 测试命令 / Test Commands

```powershell
cargo check -p ospf-rust-core
cargo test -p ospf-rust-core
cargo test -p ospf-rust-framework
cargo test -p ospf-rust-framework-bpp3d
cargo test -p ospf-rust-framework-csp1d
cargo test -p ospf-rust-framework-gantt-scheduling
cargo check --workspace
cargo test -p ospf-rust-example
cargo run -p ospf-rust-example -- core:demo1
cargo run -p ospf-rust-example -- framework:demo1
```

## 提交顺序 / Commit Order

1. `feat(core): add SymbolCombination container with type aliases`
2. `feat(core): add flat_map/map factory functions for SymbolCombination`
3. `feat(core): add register_combination and add_symbol_combination to MetaModel`
4. `feat(core): add sum_along_dimension to VariableCombination and SymbolCombination`
5. `refactor(example-core-demo1): rewrite model with VariableCombination and SymbolCombination`
6. `refactor(example-framework-demo1): rewrite model structs with VariableCombination and SymbolCombination`
7. `refactor(example-framework-demo1): rewrite constraint registration to use symbol combination references`
8. `feat(framework): add indexed variable and symbol combination adapters`
9. `feat(framework): add optional indexed arrays and appendable pools`
10. `refactor(bpp3d): migrate layer assignment variables to indexed combinations`
11. `refactor(bpp3d): migrate layer assignment expressions to symbol combinations`
12. `refactor(bpp3d): make continuous radius variables explicit fields`
13. `refactor(csp1d): migrate produce aggregation to appendable variable pool`
14. `refactor(csp1d): make yield and length slack variables explicit fields`
15. `refactor(csp1d): split produce modeling context and pipelines`
16. `refactor(gantt): reuse shared indexed variable adapters`
17. `refactor(gantt): migrate task and capacity symbols to symbol combinations`
18. `refactor(gantt): migrate produce and resource usage symbols`
19. `refactor(gantt): refresh iterative symbol pools through explicit adapters`
20. `fix(example-demo4): complete bunch_compilation model file split`
