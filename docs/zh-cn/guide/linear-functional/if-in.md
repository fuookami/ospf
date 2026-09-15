# 条件区间

## 契约

`IfInFunction<V>` 为一个线性多项式和两个标量端点返回二值的区间成员结果。目标区间是闭区间：`lower <= x <= upper` 时结果为 `1`，值安全地位于区间外时结果为 `0`。

实现对 `V : RealNumber<V> & NumberField<V>` 泛型化。它要求 `lower <= upper`，并且注册约束时 `x` 必须有有限范围。

## 定义与三值区间语义

定义两个差值：

$$
d_\mathrm{lower}=x-\mathrm{lower},
\qquad
d_\mathrm{upper}=\mathrm{upper}-x.
$$

两个差值都使用共享的 `GE` 关系分类。给定严格边界 $g$：

| Position of $x$ | Lower side | Upper side | Result |
| --- | --- | --- | --- |
| $x\le\mathrm{lower}-g$ | False | True or Undefined | 0 |
| $\mathrm{lower}-g<x<\mathrm{lower}$ | Undefined | True | Undefined |
| $\mathrm{lower}\le x\le\mathrm{upper}$ | True | True | 1 |
| $\mathrm{upper}<x<\mathrm{upper}+g$ | True | Undefined | Undefined |
| $x\ge\mathrm{upper}+g$ | True or Undefined | False | 0 |

两个端点都属于闭区间比较的真分支。如果 `lower == upper`，唯一端点仍然是真点。

## 边界、tolerance 与 Undefined

`classify(values)` 在任一侧为假时返回 `TruthValue.False`，两侧都为真时返回 `TruthValue.True`，其余情况返回 `TruthValue.Undefined`。`evaluate()` 将真/假映射为 `1`/`0`，将 undefined、输入缺失或校验失败映射为 `null`。

`strictBoundary` 默认取兼容参数 `tolerance`，其默认值为 `NONZERO_TOLERANCE = 1e-10`。`delta` 默认取 `strictBoundary`。这些参数描述离散条件间隔，不会把区间外侧的开放带变成区间成员。

## 当前 API

### Kotlin

源码：[`IfIn.kt`（`IfInFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/IfIn.kt)

```kotlin
IfInFunction(
    x: LinearPolynomial<V>,
    lower: V,
    upper: V,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "ifin",
    displayName: String? = null,
    bounds: ConditionBounds<V>? = null,
    conditionBounds: ConditionBounds<V>? = null,
    delta: V? = null
)
```

伴生 `invoke` 暴露相同参数。`bounds` 和 `conditionBounds` 是 `x` 有限范围的别名；`bigM` 为兼容性保留，不能替代该范围。

### Rust

源码：[`if_in.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_in.rs)

Rust 同名的 `IfInFunction` 不是 Kotlin 闭区间 API 的一一对应实现：它测试输入是否近似属于离散 `values` 集合，并接收显式 `big_m`。最接近的区间组合是为两侧分别创建 `ConditionalIfFunction`，再用 `IfInRangeFunction` 合并；调用 `IfInRangeFunction::registerable(id, name)` 可得到带已注册指示器和 AND 结果的 `RegisterableIfInRangeFunction`。区间校验要求两侧使用 `GreaterEqual`、一元变量条件系数符号相反、端点有序且范围有限。

```rust
IfInFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    values: Vec<V>,
    big_m: V,
) -> Self
ConditionalIfFunction::new(
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<Self>
IfInRangeFunction::new(
    lower: ConditionalIfFunction<V>,
    upper: ConditionalIfFunction<V>,
) -> Result<Self>
IfInRangeFunction::registerable(
    self,
    id: u64,
    name: impl AsRef<str>,
) -> Result<RegisterableIfInRangeFunction<V>>
```

## 求解器数学模型

Kotlin 构造 $q_l=x-lower$ 与 $q_u=upper-x$。对每一侧 $j\in\{l,u\}$，令 $L_j\le q_j\le U_j$、真阈值为 $T_j$、假阈值为 $F_j$、指标为 $a_j$，实际传入

$$
q_j+(L_j-T_j)a_j\ge L_j,
\qquad
q_j+(F_j-U_j)a_j\le F_j.
$$

然后注册 AND 结果 $y$：

$$
a_l+a_u\ge2y,
\qquad
y\le a_l,
\qquad
y\le a_u.
$$

Rust `RegisterableIfInRangeFunction` 同样使用两个关系指标和一个 AND 结果。Rust 较旧的同名 `IfInFunction` 表示离散列表成员关系，不是这里的区间模型。

## `evaluate()` 与求解器模型的差异

直接求值器使用精确求得的 `x` 和三值间隔。求解器使用范围驱动的指示约束；如果声明的 x 范围穿过区间外不可分类带，生成的模型可能不可行。显式有限 `ConditionBounds` 属于求解器契约，直接求值本身只需要输入值。

## 当前最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.IfInFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val x = RealVar("x")
    val xPoly = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, x)),
        constant = Flt64.zero
    )
    val function = IfInFunction(
        x = xPoly,
        lower = Flt64.zero,
        upper = Flt64(2.0),
        converter = IntoValue.Identity,
        strictBoundary = Flt64(0.1),
        conditionBounds = ConditionBounds(Flt64(-1.0), Flt64(3.0)),
        name = "ifin"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one)) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(-1.0))) == Flt64.zero)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(-0.05))) == null)
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, ConditionalIfFunction, IfInRangeFunction,
};

let bounds = ConditionBounds {
    lower: -2.0,
    upper: 2.0,
};
let lower = ConditionalIfFunction::new(
    Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0),
    ConditionRelation::GreaterEqual,
    0.1,
    bounds.clone(),
)
.expect("valid lower interval condition");
let upper = ConditionalIfFunction::new(
    Linear::new(vec![LinearMonomial::new(-1.0, 0)], 1.0),
    ConditionRelation::GreaterEqual,
    0.1,
    bounds,
)
.expect("valid upper interval condition");
let range = IfInRangeFunction::new(lower, upper).expect("valid closed interval");
let value = range.evaluate(&0.0, &0.0).expect("classifiable interval");
assert_eq!(value, Some(1.0));
```

:::

## Source and core tests

- [Implementation: `IfIn.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/IfIn.kt)
- [Core conditional registration test: `FunctionSymbolConditionalGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolConditionalGenericRegistrationTest.kt)
- [Core conditional regression test: `ConditionalFunctionRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ConditionalFunctionRegressionTest.kt)
- [Complete example: `ConditionalFunctionSolveRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ConditionalFunctionSolveRegressionTest.kt)
- [Rust 实现与单元测试：`if_in.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_in.rs)
- [Rust 条件回归：`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)
- [Rust 注册原子性与区间校验：`registration_atomicity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/registration_atomicity.rs)

## 相关页面

- [条件 IF](/guide/linear-functional/if)
- [If-Then](/guide/linear-functional/if-then)
- [选一约束](/guide/linear-functional/one-of)
