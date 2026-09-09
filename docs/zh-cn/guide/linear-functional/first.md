# 首个非零索引

`FirstFunction` 返回输入多项式中值严格大于 `epsilon` 的第一个元素的从零开始索引。如果没有输入通过该判定，则返回输入数量。

## 契约

- 输入：有序、通常应非空的 `List<LinearPolynomial<V>>`。
- 输出：$[0,n]$ 中的数值索引，以线性多项式 `result` 暴露。
- 选择条件：`p_i > epsilon`，不是 `abs(p_i) > epsilon`；负值永远不会被选中。
- `epsilon` 是 `Flt64` 参数（默认 `1e-6`），系数和结果通过转换器使用泛型 `V`。
- 直接 `evaluate` 遇到缺失输入值时返回 `null`。

## 定义与数学模型

设输入值为 $p_0,\ldots,p_{n-1}$，则

$$
b_i=\mathbf{1}[p_i>\varepsilon].
$$

设 $y_i$ 为首个命中 one-hot 标志：

$$
y_i=\mathbf{1}\!\left[b_i=1\land\sum_{j<i}y_j=0\right].
$$

返回索引为

$$
r=\sum_{i=0}^{n-1} i\,y_i+n\left(1-\sum_{i=0}^{n-1}y_i\right).
$$

因此全为 false 时返回 $n$，而不是 `null`。

## 实现、辅助变量与约束

实现为每个输入创建一个 `BinaryzationFunction`，并创建名称 `name` 后接 `_first` 的二值数组，每个输入一个元素。二值化标志和首个命中标志通过上界、下界和单调性约束连接；`result` 就是上面的加权表达式。因此每个输入多项式都需要有效的 Big-M 范围（由二值化辅助函数推导，或在该辅助路径中显式提供）。

## 当前 API

### Kotlin

源码：[`First.kt`（`FirstFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/First.kt#L53-L215)

```kotlin
FirstFunction(
    polynomials: List<LinearPolynomial<V>>,
    epsilon: Flt64 = Flt64(1e-6),
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

### Rust

源码：[`first.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/first.rs)

Rust 有同名辅助函数，但不是 Kotlin 阈值函数的一一对应替代。`FirstFunction::new` 接收候选 `Linear<V>` 值以及等长的、已经计算好的 `Vec<BinaryVariableItem>` 条件变量；它没有 `epsilon` 参数，也不会在内部创建 `BinaryzationFunction`。公开访问器为 `result_variable()`、`polynomials()` 和 `condition_variables()`。没有任何条件命中时，直接求值返回 `None`（`zero_if_none` 为 true 时返回零），机理约束也将结果限定为零；而 Kotlin 返回输入数量 `n`。若需要 Kotlin 的阈值契约，应为每个输入组合一个 `BinaryzationFunction`，再传入其指示变量。

```rust
FirstFunction::new(
    id: u64,
    name: &str,
    polynomials: Vec<Linear<V>>,
    conditions: Vec<BinaryVariableItem>,
) -> Self
FirstFunction::result_variable(&self) -> &ContinuousVariableItem
FirstFunction::polynomials(&self) -> &[Linear<V>]
FirstFunction::condition_variables(&self) -> &[BinaryVariableItem]
```

## evaluate 与 solver 的差异

直接求值从索引 0 扫描列表并使用调用者的 `epsilon`。solver 注册先构造二值化函数；其当前约束容差是共享的 `NONZERO_TOLERANCE`，再连接首个命中数组。因此当 `epsilon` 非默认值时，阈值附近的直接求值和 solver 判定可能不同。

## 边界、tolerance 与 Undefined

首个命中结果不是布尔值，也不是从 1 开始的索引。由于使用严格 `>`，等于 `epsilon` 时不会选中。空列表不是有用的模型输入，调用者应至少提供一个多项式。直接求值遇到缺失符号时返回 `null`；Big-M 无法推导或多项式值非法时，底层二值化注册失败。

## 当前最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.FirstFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x0 = RealVar("x0")
val x1 = RealVar("x1")
val first = FirstFunction(
    polynomials = listOf(
        LinearPolynomial(listOf(LinearMonomial(Flt64.one, x0)), Flt64.zero),
        LinearPolynomial(listOf(LinearMonomial(Flt64.one, x1)), Flt64.zero)
    ),
    converter = IntoValue.Identity,
    name = "first"
)
val value = first.evaluate(
    mapOf<Symbol, Flt64>(x0 to Flt64.zero, x1 to Flt64.two)
)
check(value != null && value == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::FirstFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{BinaryVariableItem, VariableId};

let c0 = BinaryVariableItem::create(VariableId::standalone(1), "c0");
let c1 = BinaryVariableItem::create(VariableId::standalone(2), "c1");
let function = FirstFunction::new(
    1,
    "first",
    vec![Linear::new(vec![], 10.0), Linear::new(vec![], 20.0)],
    vec![c0.clone(), c1.clone()],
);
let mut tokens = VecTokenList::new();
let token0 = Token::from_generic(c0.clone(), c0.index());
token0.set_result(0.0);
tokens.add_token(token0);
let token1 = Token::from_generic(c1.clone(), c1.index());
token1.set_result(1.0);
tokens.add_token(token1);
let value = <FirstFunction as FunctionSymbol>::calculate_value(&function, &tokens, false);
assert_eq!(value, Some(20.0));
```

:::

## 测试与示例

- Core 测试：[`FirstFunctionGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FirstFunctionGenericEvaluateTest.kt)
- 示例目录（当前没有专门的首个索引文件）：[linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)
- Rust 实现与求值：[`first.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/first.rs)、[`p0_evaluation_tests.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/p0_evaluation_tests.rs)

## 相关页面

- [二值化](./bin)
- [条件 IF](./if)
- [一元分段线性](./ulp)
