# 乘积、选择与查表

这些函数为乘积和索引选择提供线性求解模型。它们的保证不同：有界整数乘积和二值选择是精确建模，一般连续乘积使用 McCormick 松弛。

## 有界整数乘积

对有界整数变量 $n$ 和有界线性表达式 $x$，`IntegerProductFunction` 表示

$$
y=nx.
$$

函数将整数平移到零点后用二进制变量编码，并约束编码不超过其取值域宽度；每个编码位与表达式的乘积均通过掩码精确表达，因此得到精确 MILP。整数上下界必须有限、有序且为整数，表达式 $x$ 也必须具有有限界。Kotlin 尽可能从已声明的变量范围推导界；显式传入的 `ConditionBounds` 会作为输入域约束注册，因此必须确实覆盖业务域。Rust 使用扁平表达式，构造器要求显式提供表达式界；传入界必须对所有可行输入成立。

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.IntegerProductFunction
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.Int64
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val units = IntVar("units")
    units.range.geq(Int64.zero)
    units.range.leq(Int64(10))
    val price = RealVar("price")
    price.range.geq(Flt64.zero)
    price.range.leq(Flt64(8.0))
    val priceExpression = LinearPolynomial(
        listOf(LinearMonomial(Flt64.one, price)), Flt64.zero
    )
    val total = IntegerProductFunction(
        integer = units,
        input = priceExpression,
        converter = IntoValue.Identity,
        inputBounds = ConditionBounds(Flt64.zero, Flt64(8.0)),
        name = "total"
    )
}
```

```rust [Rust]
use std::sync::Arc;

use ospf_rust_core::model::BasicModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::IntegerProductFunction;
use ospf_rust_core::variable::{ContinuousVariableItem, IntegerVariableItem, VariableId, VariableRange};

fn main() {
    let mut model = BasicModel::new("integer_product");
    let units = IntegerVariableItem::with_range(
        VariableId::standalone(1), "units", VariableRange::bounded(0.0, 10.0)
    );
    let _units_index = model.register_variable(units.clone()).unwrap();
    let price = ContinuousVariableItem::with_range(
        VariableId::standalone(2), "price", VariableRange::bounded(0.0, 8.0)
    );
    let price_index = model.register_variable(price.clone()).unwrap();
    let price_expression = Linear::new(vec![LinearMonomial::new(1.0, price_index)], 0.0);
    let total = IntegerProductFunction::new(
        1, "total", units, price_expression, (0.0, 8.0)
    ).unwrap();
    model.add_symbol(Arc::new(total)).unwrap();
}
```

:::

## 二值分支选择

`SelectFunction`（别名 `IfThenElseFunction`）按二值变量 $b$ 在两个线性表达式间选择：

$$
y=\begin{cases}a,&b=1,\\c,&b=0.\end{cases}
\qquad y=c+b(a-c).
$$

该 MILP 表达精确等价。两个分支都必须具有有限范围，供掩码约束使用。Kotlin 可推导范围，也可提供 `thenBounds`、`otherwiseBounds`；显式范围会作为域约束加入模型。Rust 构造器要求显式传入两侧范围。非二值选择变量无效，直接求值时也不会产生合法分支值。

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SelectFunction
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val selected = SelectFunction(
        mask = BinVar("use_premium"),
        then = LinearPolynomial(emptyList(), Flt64(12.0)),
        otherwise = LinearPolynomial(emptyList(), Flt64(8.0)),
        converter = IntoValue.Identity,
        name = "selected_cost"
    )
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::SelectFunction;
use ospf_rust_core::variable::{BinaryVariableItem, VariableId};

fn main() {
    let selector = BinaryVariableItem::create(VariableId::standalone(3), "use_premium");
    let selected = SelectFunction::new(
        3, "selected_cost", selector,
        Linear::constant(12.0), (12.0, 12.0),
        Linear::constant(8.0), (8.0, 8.0),
    ).unwrap();
}
```

:::

## McCormick 乘积包络

给定 $x\in[L_x,U_x]$、$z\in[L_z,U_z]$ 和结果变量 $w$，`McCormickEnvelopeFunction` 添加四条不等式：

$$
\begin{aligned}
w&\ge L_xz+L_zx-L_xL_z, & w&\ge U_xz+U_zx-U_xU_z,\\
w&\le U_xz+L_zx-U_xL_z, & w&\le L_xz+U_zx-L_xU_z.
\end{aligned}
$$

对一般连续因子，这些不等式给出有界矩形域上图的凸包松弛；它们不保证每个内部点都满足 $w=xz$。因子必须有正确的有限界。Kotlin 要求显式提供 `ConditionBounds`；Rust 要求显式传入界对。普通求值从模型结果变量读取 $w$；单独的乘积求值方法才会计算真实的算术乘积。

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.McCormickEnvelopeFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val x = RealVar("x").also { it.range.geq(Flt64.zero); it.range.leq(Flt64(2.0)) }
    val z = RealVar("z").also { it.range.geq(Flt64.zero); it.range.leq(Flt64(3.0)) }
    val xExpression = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
    val zExpression = LinearPolynomial(listOf(LinearMonomial(Flt64.one, z)), Flt64.zero)
    val envelope = McCormickEnvelopeFunction(
        left = xExpression,
        right = zExpression,
        leftBounds = ConditionBounds(Flt64.zero, Flt64(2.0)),
        rightBounds = ConditionBounds(Flt64.zero, Flt64(3.0)),
        converter = IntoValue.Identity,
        name = "xz_envelope"
    )
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::McCormickEnvelopeFunction;

fn main() {
    let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
    let z = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
    let envelope = McCormickEnvelopeFunction::new(
        4, "xz_envelope", x, z, (0.0, 2.0), (0.0, 3.0)
    ).unwrap();
}
```

:::

## 整数索引查表

`ElementFunction`（别名 `LookupFunction`）按整数索引 $i$ 选择表中的表达式：

$$
y=v_{i-\ell},\qquad \ell\le i<\ell+n,
$$

其中 $\ell$ 是 `lowerIndex`，表有 $n$ 项。索引必须是有限界的整数变量，每个表项表达式也须有有限界，以便构造 one-hot 选择与掩码。Kotlin 可推导各项范围，也可以通过 `valueBounds` 提供；Rust 用 `ElementValue(expression, lower_bound, upper_bound)` 描述每个表项。索引越界时模型不可行，直接求值返回 `null`/`None`。首个索引可以为负数。无论表项数值是否相同，具体索引都唯一决定被选中的表项。

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ElementFunction
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.number.Int64

fun main() {
    val grade = IntVar("grade")
    grade.range.geq(Int64(-1))
    grade.range.leq(Int64(1))
    val lookup = ElementFunction.fromConstants(
        index = grade,
        values = listOf(Flt64(10.0), Flt64(14.0), Flt64(19.0)),
        lowerIndex = -1,
        converter = IntoValue.Identity,
        name = "grade_cost"
    )
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::{ElementFunction, ElementValue};
use ospf_rust_core::variable::{IntegerVariableItem, VariableId, VariableRange};

fn main() {
    let grade = IntegerVariableItem::with_range(
        VariableId::standalone(5), "grade", VariableRange::bounded(-1.0, 1.0)
    );
    let table = [10.0, 14.0, 19.0].into_iter()
        .map(|value| ElementValue::new(Linear::constant(value), value, value))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let lookup = ElementFunction::new(5, "grade_cost", grade, table, -1).unwrap();
}
```

:::

## 相关函数

- [`ProductFunction`](../quadratic-functional/product) 将两个线性表达式展开为二次多项式，不会创建线性化乘积变量。
- [`ArgMin`、`ArgMax`、`KthLargest` 与 `TopKSum`](./order-statistics) 提供精确的选择与排序统计。
- [`McCormickEnvelopeFunction`](../quadratic-functional/linear-compositions) 可用于二次模型，但仍是连续乘积的松弛。
