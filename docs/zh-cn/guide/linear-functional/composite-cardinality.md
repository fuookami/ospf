# 组合函数与基数约束

许多常见线性化场景由已有基础函数组合而成。OSPF 为常见模式提供了直接函数符号；若只需要可行性，也可以直接添加纯约束行。

## 正部、区间裁剪与死区

对有界线性表达式 $x$，正部函数为

$$
\max(x,0),
$$

`PositivePartFunction` 精确返回该值。`ClampFunction` 计算 $\min(\max(x,\ell),u)$，其中 $[\ell,u]$ 是有限有序区间。`DeadZoneFunction` 对非负宽度 $\delta$ 计算 $\max(|x|-\delta,0)$。这些 MILP 表达依赖有限输入界，对任意目标方向都精确。

```kotlin
val positive = PositivePartFunction(input, converter, name = "overage")
val clipped = ClampFunction(input, lower, upper, converter, name = "clipped")
val deadZone = DeadZoneFunction(input, delta, converter, name = "tolerance")
```

```rust
let positive = PositivePartFunction::new(1, "overage", input.clone())?;
let clipped = ClampFunction::new(2, "clipped", input.clone(), 0.0, 100.0)?;
let dead_zone = DeadZoneFunction::new(3, "tolerance", input, 2.0)?;
```

Kotlin 构造器返回用于校验的 `Ret<...>`，Rust 返回 `Result<...>`。两种实现都要求精确线性化的输入界有限。Kotlin 可从变量范围推导，支持显式界的函数也可接受用户给定的范围；Rust 在生成机制时验证已注册 token 的变量域。

## 互补约束

`ComplementarityFunction` 要求两个非负有界表达式不能同时为正：

$$
x\ge 0,\quad y\ge 0,\quad xy=0。
$$

实现通过二值选择器和有限上界构成精确 MILP 析取。该函数只施加约束：语义求值在两个输入都为正时返回 1（表示违反约束），满足条件时返回 0。内部选择器用于表示可行分支；当两个表达式都为零时，任一分支都成立。

```kotlin
val exclusive = ComplementarityFunction(
    x = x,
    y = y,
    converter = converter,
    xBounds = ConditionBounds(Flt64.zero, Flt64(10.0)),
    yBounds = ConditionBounds(Flt64.zero, Flt64(10.0)),
    name = "exclusive"
)
```

```rust
let exclusive = ComplementarityFunction::new(
    4, "exclusive", x, y, (0.0, 10.0), (0.0, 10.0)
)?;
```

两个区间都必须有限、有序、非负，并覆盖所有可行输入。Kotlin 的显式边界会作为输入域约束加入模型，因此可约束原本无界的表达式；Rust 要求已注册 token 的变量域包含在声明边界内。

## 距离与极差

对等长向量，`L1DistanceFunction` 计算 $\sum_i |x_i-y_i|$，`LInfinityDistanceFunction` 计算 $\max_i |x_i-y_i|$。`RangeFunction` 对非空列表计算 $\max_i x_i-\min_i x_i$。这些函数组合精确绝对值和极值选择器，因此作为约束、最小化目标或最大化目标都保持正确。所有输入都需要有限范围；L∞ 和极差的输入集合不得为空。

```kotlin
val l1 = L1DistanceFunction(left, right, converter, name = "l1")
val linf = LInfinityDistanceFunction(left, right, converter, name = "linf")
val spread = RangeFunction(inputs, converter, name = "spread")
```

```rust
let l1 = L1DistanceFunction::new(5, "l1", left.clone(), right.clone())?;
let linf = LInfinityDistanceFunction::new(6, "linf", left, right)?;
let spread = RangeFunction::new(7, "spread", values)?;
```

## 至多与恰好计数

当 $b_i$ 是二值变量时，计数本身就是线性表达式 $\sum_i b_i$。若只关心可行性，直接添加普通线性行即可：

$$
\sum_i b_i\le k,\qquad \sum_i b_i=k。
$$

Kotlin 提供 `atMostConstraints(...)` 和 `exactlyConstraints(...)` 来创建纯约束行。`AtMostFunction` 与 `ExactlyFunction` 则精确返回计数是否满足上限或指定总数的 0/1 指示。Rust 对应函数也提供指示形式；纯计数条件同样可以直接写成线性约束。输入必须是二值变量，计数必须是 $[0,n]$ 内的整数。

```kotlin
val atMost = AtMostFunction(indicators, limit = 2, converter = converter)
val exactly = ExactlyFunction(indicators, amount = 1, converter = converter)
val rows = atMostConstraints(indicators, limit = 2, converter = converter)
```

```rust
use std::collections::HashMap;
use std::sync::Arc;

use ospf_rust_core::model::BasicModel;
use ospf_rust_core::symbol::function::{
    at_most_constraints, exactly_constraints, AtMostFunction, ExactlyFunction,
};
use ospf_rust_core::variable::{BinaryVariableItem, VariableId};

let mut model = BasicModel::new("cardinality");
let indicators = vec![
    BinaryVariableItem::create(VariableId::standalone(1), "b0"),
    BinaryVariableItem::create(VariableId::standalone(2), "b1"),
    BinaryVariableItem::create(VariableId::standalone(3), "b2"),
];
for indicator in &indicators {
    model.register_variable(indicator.clone())?;
}
let at_most = Arc::new(AtMostFunction::new(8, "at_most", indicators.clone(), 2)?);
let exactly = Arc::new(ExactlyFunction::new(9, "exactly", indicators.clone(), 1)?);
model.add_symbol(at_most)?;
model.add_symbol(exactly)?;
let symbol_to_index = model.tokens().iter()
    .map(|token| (token.id().unique_id() as usize, token.solver_index))
    .collect::<HashMap<_, _>>();
let pure_at_most = at_most_constraints::<f64>(
    &indicators, 2, &symbol_to_index, "pure_at_most"
)?;
let pure_exactly = exactly_constraints::<f64>(
    &indicators, 1, &symbol_to_index, "pure_exactly"
)?;
```

Rust 的 `at_most_constraints` 和 `exactly_constraints` 接收已注册的 `symbol_to_index` 映射并返回 `Result` 行；缺少指示变量列时会报错。映射键使用变量 ID 的 `unique_id()`，值使用最终的 `solver_index`。若只需要计数条件，这些纯行可避免引入辅助满足指示变量。

## 如何选择表达

只需要满足条件时优先使用纯线性行；后续还要使用数值结果时使用函数符号；非线性输入运算可直接使用精确组合函数。精确性依赖有限边界覆盖完整可行域；界限过窄或不成立都会改变模型本身。
