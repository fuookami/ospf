# 顺序统计

`ArgMinFunction`、`ArgMaxFunction`、`KthLargestFunction` 和 `TopKSumFunction` 用于选择或统计有限个线性表达式。它们组合精确的 `MinFunction`/`MaxFunction` 比较选择器，因此结果不依赖于把该结果作为目标最小化或最大化。

每个候选表达式都必须有有限范围，以构造精确选择约束。Kotlin 会在创建函数时验证有限界。Rust 在生成机制时从已注册的 token 域推导各候选范围，因此候选使用的变量必须声明有限范围。

## ArgMin 与 ArgMax

对候选值 $x_0,\ldots,x_{n-1}$，返回从 0 开始的索引：

$$
\operatorname{argmin}(x)=i\quad\text{其中 }i\in\arg\min_j x_j,
\qquad
\operatorname{argmax}(x)=i\quad\text{其中 }i\in\arg\max_j x_j.
$$

候选列表不能为空。若多个候选并列，模型可任选一个最优索引；直接语义求值则固定返回最小的并列索引。若索引还要参与表达式或约束，应使用函数返回的选择变量多项式。

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ArgMinFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.utils.functional.*

fun main() {
    val candidates = listOf(6.0, 2.0, 2.0).map {
        LinearPolynomial(emptyList(), Flt64(it))
    }
    val argmin = when (val created = ArgMinFunction(
        polynomials = candidates,
        converter = IntoValue.Identity,
        name = "cheapest"
    )) {
        is Ok -> created.value
        is Failed, is Fatal -> return
    }
    check(argmin.evaluate(emptyMap()) == Flt64.one)
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::{FunctionSymbol, function::ArgMinFunction};
use ospf_rust_core::token::VecTokenList;

fn main() {
    let candidates = vec![Linear::constant(6.0), Linear::constant(2.0), Linear::constant(2.0)];
    let argmin = ArgMinFunction::new(10, "cheapest", candidates).unwrap();
    let tokens = VecTokenList::<f64>::new();
    assert_eq!(FunctionSymbol::calculate_value(&argmin, &tokens, false), Some(1.0));
}
```

:::

若需返回最大值索引，使用同样输入形状的 `ArgMaxFunction`。模型在并列时仍可任取，直接求值固定取第一个并列最大项。

## 第 k 大值

`KthLargestFunction` 按从大到小的顺序返回从 0 开始编号的第 $k$ 个值：

$$
x_{(0)}\ge x_{(1)}\ge\cdots\ge x_{(n-1)},
\qquad y=x_{(k)},\quad 0\le k<n.
$$

候选列表不能为空，且 $k$ 必须小于列表长度。候选值相等时，重复排名仍返回相同数值，因此结果没有歧义。比较网络对任意目标方向都精确。

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.KthLargestFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.utils.functional.*

fun main() {
    val candidates = listOf(7.0, 4.0, 9.0, 2.0).map {
        LinearPolynomial(emptyList(), Flt64(it))
    }
    val thirdLargest = when (val created = KthLargestFunction(
        polynomials = candidates,
        k = 2,
        converter = IntoValue.Identity,
        name = "third_largest"
    )) {
        is Ok -> created.value
        is Failed, is Fatal -> return
    }
    check(thirdLargest.evaluate(emptyMap()) == Flt64(4.0))
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::{FunctionSymbol, function::KthLargestFunction};
use ospf_rust_core::token::VecTokenList;

fn main() {
    let candidates = vec![Linear::constant(7.0), Linear::constant(4.0), Linear::constant(9.0), Linear::constant(2.0)];
    let third_largest = KthLargestFunction::new(11, "third_largest", candidates, 2).unwrap();
    let tokens = VecTokenList::<f64>::new();
    assert_eq!(FunctionSymbol::calculate_value(&third_largest, &tokens, false), Some(4.0));
}
```

:::

## 最大 k 项之和

`TopKSumFunction` 返回

$$
y=\sum_{j=0}^{k-1}x_{(j)},\qquad 0\le k\le n.
$$

$k=0$ 时空和为 0；$k=n$ 时返回全部候选值之和。两端均受支持，空候选列表仅允许与 $k=0$ 同用。此函数使用精确比较网络，不依赖目标方向。

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.TopKSumFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.utils.functional.*

fun main() {
    val candidates = listOf(7.0, 4.0, 9.0, 2.0).map {
        LinearPolynomial(emptyList(), Flt64(it))
    }
    val topTwo = when (val created = TopKSumFunction(
        polynomials = candidates,
        k = 2,
        converter = IntoValue.Identity,
        name = "top_two_sum"
    )) {
        is Ok -> created.value
        is Failed, is Fatal -> return
    }
    check(topTwo.evaluate(emptyMap()) == Flt64(16.0))
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::{FunctionSymbol, function::TopKSumFunction};
use ospf_rust_core::token::VecTokenList;

fn main() {
    let candidates = vec![Linear::constant(7.0), Linear::constant(4.0), Linear::constant(9.0), Linear::constant(2.0)];
    let top_two = TopKSumFunction::new(12, "top_two_sum", candidates, 2).unwrap();
    let tokens = VecTokenList::<f64>::new();
    assert_eq!(FunctionSymbol::calculate_value(&top_two, &tokens, false), Some(16.0));
}
```

:::

## 相关函数

- [`ElementFunction`](./products-selection) 按整数索引从表中选择一项。
- [`MinFunction`](./min) 和 [`MaxFunction`](./max) 返回最小值或最大值，不返回索引。
