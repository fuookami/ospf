# 二次模型中的半连续标记

## 可用性

Kotlin 当前没有专用的二次 `SemiFunction`，也没有接收 `QuadraticPolynomial<V>` 的重载。唯一的类是标记 [`SemiFunction`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Semi.kt#L37-L107)。它只接收边界和转换器，不接收表达式参数。Rust 同时提供有实际功能的通用 `SemiFunction` 和直接接收二次输入的 `QuadraticSemiFunction`，API 见下文。

二次机制可以回退注册线性 `MathFunctionSymbolBase`（`MechanismModel.kt:1350-1355`），但这不会把 `SemiFunction` 变成二次正部函数。`SemiFunction` 没有辅助变量或约束，也不是二次表达式；它可以作为元数据与二次模型一起构造和保留，但加入它不会改变模型。

## 含义与边界

这个标记描述目标取值域

$$
y = 0 \quad\text{or}\quad lb \le y \le ub.
$$

构造函数为：

```kotlin
SemiFunction(
    lb: V? = null,
    ub: V? = null,
    converter: IntoValue<V>,
    name: String = "semi",
    displayName: String? = null
)
```

默认值是 `lb = 0`、`ub = 1e6`，并且要求 `lb <= ub`（`Semi.kt:37-50`）。`SemiFunction.from(variable, ...)` 可以从变量的有限范围推断缺失的边界（`Semi.kt:89-106`）。`helperVariables` 为空，`evaluate` 始终返回 `null`，注册方法为空操作（`Semi.kt:53-65`）。因此它不能表示二次多项式 `q` 的 `max(0,q)`。

## 当前 API

### Kotlin

下面是与当前二次冒烟测试一致的仅标记构造：

对于二次表达式，应在 `QuadraticMetaModel` 中显式写出所需的域约束，或使用确实接收 `QuadraticPolynomial` 的二次函数类；不要虚构或调用二次 `SemiFunction` 重载。

```kotlin
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.core.model.mechanism.QuadraticMetaModel
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SemiFunction

val model = QuadraticMetaModel<Flt64>(
    name = "quadratic-model-with-semi-marker",
    converter = IntoValue.Identity
)
val semi = SemiFunction(
    lb = Flt64.one,
    ub = Flt64(4.0),
    converter = IntoValue.Identity,
    name = "semi"
)
check(semi.helperVariables.isEmpty())
check(semi.evaluate(emptyMap()) == null)
// Keep `semi` as metadata; it has no quadratic expression or model constraints.
model.close()
```

### Rust

Rust 的 [`QuadraticSemiFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs) 是 Kotlin 当前缺少的直接二次对应物：

```rust
QuadraticSemiFunction::new(
    id: u64,
    name: &str,
    input: Quadratic<V>,
) -> QuadraticSemiFunction<V>
```

它通过 `QuadraticLinearFunction` 和精确的两个候选 `MaxFunction(input, 0)` 桥接 `input`。直接求值为 `max(input, 0)`，`result_variable` 返回内部的非负结果变量。有 token 边界时会为选择约束推导 Big-M。Rust 还提供独立的 [`SemiFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/semi.rs)，其 `new(id, name, lower, upper)` 建模带结果变量和指示变量的半连续变量；`try_from_variable`/`from_variable` 可以推导有限边界。两者都不是 Kotlin 的仅标记对象。

```rust
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticSemiFunction;

let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
let semi = QuadraticSemiFunction::new(17, "qsemi", input);
assert!(semi.result_variable().name().contains("qsemi_max"));
```

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.core.model.mechanism.QuadraticMetaModel
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SemiFunction

val model = QuadraticMetaModel<Flt64>(
    name = "quadratic-model-with-semi-marker",
    converter = IntoValue.Identity
)
val semi = SemiFunction(
    lb = Flt64.one,
    ub = Flt64(4.0),
    converter = IntoValue.Identity,
    name = "semi"
)
check(semi.helperVariables.isEmpty())
check(semi.evaluate(emptyMap()) == null)
// Keep `semi` as metadata; it has no quadratic expression or model constraints.
model.close()
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticSemiFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
let semi = QuadraticSemiFunction::new(18, "qsemi", input);
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(-1.0);
tokens.add_token(tx);
assert_eq!(semi.calculate_value(&tokens, false), Some(0.0));
```

:::

- 标记样例：[`SemiTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function/SemiTest.kt)

- Rust 二次实现与测试：[`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## 参考

- 标记实现：[`Semi.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Semi.kt)
- 二次回退分派：[`MechanismModel.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/model/mechanism/MechanismModel.kt#L1350-L1355)
- 标记样例：[`SemiTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function/SemiTest.kt)
