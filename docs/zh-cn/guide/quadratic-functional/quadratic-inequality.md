# 二次不等式指示函数

`QuadraticInequalityFunction` 在有界二次多项式之上组合线性不等式指示函数。对输入多项式 $p(x)$、右侧常数 $r$ 与比较关系：

$$
y=\mathbf{1}[p(x)\ \text{sign}\ r],\qquad y\in\{0,1\}。
$$

真正含二次项的左侧表达式先由共享基类 `QuadraticFunctionSymbol<V>` 绑定到桥接变量，随后线性 `InequalityFunction` 在该桥接上施加 Big-M 指示编码。

## 契约

- 输入：`lhs: QuadraticPolynomial<V>`、标量 `rhs: V` 与 `Comparison` 比较关系；支持 `LE`、`LT`、`GE`、`GT`、`EQ`、`NE`。
- 标量 `rhs` 永远不会获得桥接；只有二次的 `lhs` 注册桥接变量 `${name}_input_0` 及一条精确二次等式。
- `createFunction` 返回 `InequalityFunction`，并透传相同的 `rhs`、`sign`、`bigM`、`tolerance`、`strictBoundary`，因此辅助变量为桥接变量加上 `${name}_flag`（`EQ`/`NE` 时还有 `${name}_side`）。
- 直接求值保留线性页面的间隔语义：`tolerance`（默认 $10^{-6}$）是 `LE`/`GE` 的间隔及 `EQ`/`NE` 的零容差；`strictBoundary`（默认 $0.5$）是 `LT`/`GT` 真分支的最小差值及 `EQ`/`NE` 的带外边界。在间隔内求值返回 `null`。
- 泛型值要求 `V : RealNumber<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。
- 公式可能是非凸 MIQCP（二次输入通过等式约束等于一个变量）；需要求解器支持非凸二次约束。

## 定义与数学模型

令 $d=p(x)-r$，并把请求的关系归一化为真分支的 $q\ge T$ 与假分支的 $q\le F$ —— 与线性不等式指示函数相同（`GT`/`LE` 使用 $q=d$，`LT`/`GE` 使用 $q=-d$）。对结果 $y\in\{0,1\}$，实现提交两条 Big-M 约束

$$
q-M_1y\le F,\qquad q-M_2y\ge T-M_2,
$$

其中间隔松弛乘子在 `GT`/`LT` 时为 $M_1=M,\ M_2=M+g$，在 `LE`/`GE` 时为 $M_1=M+g,\ M_2=M$，$g$ 对 `LE`/`GE` 取 `tolerance`、对 `LT`/`GT` 取 `strictBoundary`。因此 $y=1\Rightarrow q\ge T$、$y=0\Rightarrow q\le F$，开区间 $(F,T)$ 被有意设为不可行。`EQ` 与 `NE` 改用共享的四行零/非零 Big-M 编码并引入侧二值变量：`EQ` 指示等于非零指示的补集，`NE` 指示就是非零指示本身。

## 求解器数学模型

### Kotlin

令输入为 $p(x)$，有限值域为 $[L,U]$。基类提交恰好一条二次等式

$$
p(x)-b=0,\qquad b\in[\tilde L,\tilde U],
$$

其中 $b$ 是桥接变量 `${name}_input_0`，值域是捕获的输入范围，仅在浮点可表示性需要时放宽。线性 `InequalityFunction` 随后对 $d=b-rhs$ 应用上述归一化，并提交两条 Big-M 约束（`EQ`/`NE` 时为四行零带编码）。`bigM` 缺省时按有限的 lhs-rhs 值域推导。

### Rust

Rust 把桥接 `QuadraticLinearFunction`（结果列 `{name}_bridge_lin_y`）与建立在该列上的内层 `InequalityFunction` 组合，内层使用调用方传入的 `right`、`kind` 与 `big_m`。机制路径提交桥接的二次等式以及内层指示约束；当 token 边界可用时，内层 Big-M 会按原始二次输入的边界重新推导，且不低于策略最小值。`EQ` 与 `NE` 在内层函数中分配侧二值变量。直接求值按各关系以固定容差 $\varepsilon=16\varepsilon_{f64}$ 比较。

## 当前 API

### Kotlin

源码：[`QuadraticInequality.kt`（`QuadraticInequalityFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticInequality.kt)，组合自基类 [`QuadraticFunctionSymbol.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionSymbol.kt)。

```kotlin
QuadraticInequalityFunction(
    lhs: QuadraticPolynomial<V>,
    rhs: V,
    sign: Comparison,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_inequality",
    displayName: String? = null
)
```

线性侧的全部参数（`bigM`、`tolerance`、`strictBoundary`）都透传给组合的 `InequalityFunction`。

### Rust

Rust 的 [`QuadraticInequalityFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs) 接收平展的 `Quadratic<V>`、标量右侧值、显式 `InequalityKind` 与 Big-M：

```rust
QuadraticInequalityFunction::new(
    id: u64,
    name: &str,
    input: Quadratic<V>,
    right: V,
    kind: InequalityKind,
    big_m: V,
) -> Self

QuadraticInequalityFunction::less_equal(id: u64, name: &str, input: Quadratic<V>, right: V, big_m: V) -> Self
QuadraticInequalityFunction::greater_equal(id: u64, name: &str, input: Quadratic<V>, right: V, big_m: V) -> Self
```

`InequalityKind` 包含 `LessEqual`、`GreaterEqual`、`Less`、`Greater`、`Equal`、`NotEqual`；`result_variable()` 返回二值指示变量。Rust 没有 `tolerance`/`strictBoundary` 构造参数：直接求值使用上述固定 epsilon，机制路径使用显式传入或推断的 Big-M。

## evaluate 与 solver 的差异

直接求值按关系的间隔分类（`LE`/`GE` 用 `tolerance`，`LT`/`GT` 用 `strictBoundary`，`EQ`/`NE` 用距离带），在间隔内返回 `null`。求解器注册使用相同阈值的 Big-M 约束；间隔在那里是不可行，而不是未定义。`NE` 在两条路径上都完整支持。Rust 的直接求值用固定 $\varepsilon=16\varepsilon_{f64}$ 比较取代可配置间隔。

## 最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticInequalityFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val x = RealVar("x").also {
    it.range.geq(Flt64(-2.0))
    it.range.leq(Flt64(2.0))
}
val lhs = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64.zero
)
val inequality = QuadraticInequalityFunction(
    lhs = lhs,
    rhs = Flt64.one,
    sign = Comparison.LE,
    converter = IntoValue.Identity,
    name = "quadratic_inequality"
)
val satisfied = inequality.evaluate(
    values = mapOf<Symbol, Flt64>(x to Flt64.one),
    tokenTable = null,
    converter = IntoValue.Identity,
    zeroIfNone = false
)
check(satisfied == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticInequalityFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(2.0);
tokens.add_token(tx);
let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
let inequality = QuadraticInequalityFunction::less_equal(1, "quadratic_inequality", input, 5.0, 10.0);
assert_eq!(inequality.calculate_value(&tokens, false), Some(1.0));
```

:::

## 测试与参考

- Kotlin 组合、注册与求值覆盖：[`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt)
- Rust 专门契约测试：[`function_symbol_quadratic_inequality.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_inequality.rs)；端到端求解覆盖：[`gurobi_quadratic_model_integration.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_quadratic_model_integration.rs)（`gurobi_solves_quadratic_inequality_with_non_linear_input`）。

## 相关页面

- [不等式指示函数](../linear-functional/inequality)：底层线性指示语义。
- [二次线性](./quadratic-linear)：共享的二次桥接。
- [二次区间指示](./quadratic-in-step-range)：二次输入上的区间隶属指示。
