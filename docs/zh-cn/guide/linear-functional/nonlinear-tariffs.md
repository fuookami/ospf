# 非线性近似与阶梯计费

平滑非线性函数和分档成本可以用分段线性函数（PWL）纳入线性模型。模型在给定断点之间使用线性插值；工厂不会自动计算或证明近似误差上界。

## 非线性函数

工厂提供以下函数的 PWL 近似：

| 函数 | Kotlin 符号 | Rust 符号 | 定义域要求 |
|---|---|---|---|
| 指数 $e^x$ | `ExpFunction` | `ExpFunction` | 有限区间 |
| 自然对数 $\ln x$ | `LogFunction` | `LogFunction` | 严格正区间 |
| 倒数 $1/x$ | `ReciprocalFunction` | `ReciprocalFunction` | 区间不得包含零 |
| 幂 $x^p$ | `PowerFunction` | `PowerFunction` | 指定指数对应的实值必须有限 |
| 平滑 logistic $1/(1+e^{-s(x-m)})$ | `LogisticApproximationFunction` | `LogisticApproximationFunction` | 有限区间且 $s>0$ |

可以选择等距划分区间，也可以提供有限且严格递增的断点。增加分段数可能改善近似质量，但仅凭分段数无法推出普适误差保证。函数的 `evaluate` 与模型使用相同的插值；`originalValue`/`original_value` 单独求原数学函数值，便于比较。

`SigmoidFunction` 的含义不同：它是具有边界间隔的离散关系指示函数，取值为 0 或 1。需要平滑 logistic 曲线时使用 `LogisticApproximationFunction`。

```kotlin
val exp = ExpFunction.create(x, Flt64(-2.0), Flt64(2.0), segments = 16, converter = converter)
val log = LogFunction.create(x, Flt64(0.1), Flt64(4.0), segments = 16, converter = converter)
val reciprocal = ReciprocalFunction.create(x, Flt64(1.0), Flt64(5.0), segments = 16, converter = converter)
val power = PowerFunction.create(x, Flt64(0.5), Flt64(0.0), Flt64(4.0), segments = 16, converter = converter)
val logistic = LogisticApproximationFunction.create(x, Flt64(-6.0), Flt64(6.0), segments = 24, converter = converter)
```

```rust
let exp = ExpFunction::with_segments(1, "exp", x.clone(), -2.0, 2.0, 16)?;
let log = LogFunction::with_segments(2, "log", x.clone(), 0.1, 4.0, 16)?;
let reciprocal = ReciprocalFunction::with_segments(3, "reciprocal", x.clone(), 1.0, 5.0, 16)?;
let power = PowerFunction::with_exponent_segments(4, "sqrt", x.clone(), 0.5, 0.0, 4.0, 16)?;
let logistic = LogisticApproximationFunction::with_segments(5, "logistic", x, -6.0, 6.0, 24)?;
```

## 增量阶梯费率

增量计费对每个数量档新增的部分按该档边际费率计价。给定断点 $0=b_0<b_1<\cdots<b_n$，总成本在断点处连续：

$$
C(q)=\sum_{i=0}^{n-1} r_i\,\max(0,\min(q,b_{i+1})-b_i),
$$

定义在配置的数量域内。费率必须有限且非负。Kotlin 还支持设置首个断点处的 `baseCost`；Rust 工厂固定从数量零、费用零开始。

```kotlin
val incremental = IncrementalTariff.create(
    x = quantity,
    breakpoints = listOf(Flt64.zero, Flt64(10.0), Flt64(25.0)),
    marginalRates = listOf(Flt64(2.0), Flt64(1.5)),
    converter = converter
)
```

```rust
let incremental = TariffFunctions::incremental(
    6, "incremental", quantity,
    vec![0.0, 10.0, 25.0], vec![2.0, 1.5]
)?;
```

## 全量折扣

全量折扣选择一个档位费率，并将其作用于全部数量。费率必须非负且不递增。内部断点归入右侧档位；每个断点左侧会有一段由正 `boundaryGap` 指定的不可行区间，且该间隔必须小于每档宽度。配置的断点限定有限数量域。

```kotlin
val discount = AllUnitsDiscountFunction.create(
    x = quantity,
    breakpoints = listOf(Flt64.zero, Flt64(10.0), Flt64(25.0)),
    rates = listOf(Flt64(2.0), Flt64(1.5)),
    boundaryGap = Flt64(0.01),
    converter = converter
)
```

```rust
let discount = TariffFunctions::all_units_discount(
    7, "discount", quantity,
    vec![0.0, 10.0, 25.0], vec![2.0, 1.5], 0.01
)?;
```

边界间隔用于使连续输入的档位归属明确，也是对可行数量域的实际限制。取值应符合业务单位和数据精度。

## 固定费用

固定费用组合表示启动成本加单位活动成本：

$$
C=fz+cq,\qquad z\in\{0,1\}。
$$

提供活动量联动时，还会施加 $0\le q\le Uz$，并可选施加 $q\ge Lz$。若 `minimumActive` 为零，启动变量取 1 时活动量仍可为零；若启动必须表示正活动量，应设正的最小值。不提供活动量表达式时，由调用方决定是否启用，也允许零产量时支付固定费用。

```kotlin
val fixed = FixedChargeFunction.create(
    activation = open,
    fixedCost = Flt64(25.0),
    activity = quantity,
    unitCost = Flt64(3.0),
    minimumActive = Flt64(1.0),
    maximumActivity = Flt64(100.0),
    converter = converter
)
```

```rust
use std::collections::HashMap;
use ospf_rust_core::model::BasicModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::FixedChargeFunction;
use ospf_rust_core::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange};

let mut model = BasicModel::new("fixed_charge");
let open = BinaryVariableItem::create(VariableId::standalone(1), "open");
model.register_variable(open.clone())?;
let quantity_var = ContinuousVariableItem::with_range(
    VariableId::standalone(2), "quantity", VariableRange::bounded(0.0, 100.0)
);
let quantity_index = model.register_variable(quantity_var)?;
let quantity = Linear::new(vec![LinearMonomial::new(1.0, quantity_index)], 0.0);
let fixed = FixedChargeFunction::new(open, quantity, 3.0, 25.0, 100.0)?
    .with_minimum_active_quantity(1.0)?;
let symbol_to_index = model.tokens().iter()
    .map(|token| (token.id().unique_id() as usize, token.solver_index))
    .collect::<HashMap<_, _>>();
let cost = fixed.cost_expression(&symbol_to_index)?;
let linking_rows = fixed.linking_constraints(&symbol_to_index)?;
```

Rust API 分别公开成本表达式和联动约束行，供组合进外围模型。传入从已注册 token ID 到最终 solver index 的映射；Kotlin 返回函数符号，由模型注册其结果变量和约束。
