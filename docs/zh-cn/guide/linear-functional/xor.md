# 逻辑 XOR

## 契约

`XorFunction<V>` 统计输入多项式中的非零项。它的求值器仅在恰好一个输入非零时返回一，其他数量都返回零。因此它是“恰好一个”函数；对两个二值输入时与通常的 XOR 一致，但输入超过两个时不是奇偶 XOR。

当前求解器注册没有完全强制执行该求值契约。下面列出实际执行的不等式，以明确两者的差异。

## 定义与真值表

对于输入值 `p_i`，求值器计算：

$$
a_i =
\begin{cases}
1, & p_i\ne0 \\
0, & p_i=0
\end{cases}
\qquad
y_{\mathrm{eval}} =
\begin{cases}
1, & \sum_i a_i=1 \\
0, & \sum_i a_i\ne1
\end{cases}
$$

对于三个输入，当前求值器与最终约束所允许的求解器结果如下：

| 输入个数 `s=\sum_i a_i` | 求值器 | 奇偶 XOR | 求解器结果 |
| ---: | ---: | ---: | --- |
| 0 | 0 | 0 | 0 |
| 1 | 1 | 1 | 0 或 1 |
| 2 | 0 | 0 | 0 |
| 3 | 0 | 1 | 不可行 |

奇偶列仅用于说明当前函数为何不是奇偶 XOR。源码注释描述的是奇偶编码，但可执行的求值器与约束没有实现那个定义。

## 边界、tolerance 与 Undefined

`evaluate(values)` 将每个求值结果与 `converter.zero` 做精确比较；它不使用 `tolerance` 或 `strictBoundary`。缺少输入或多项式求值失败时返回 `null`。

注册约束为每个输入创建非零指示变量。给定容差 `t` 与严格边界 `g`，当前指示约束在指示变量为零时表示零值带 `|p_i|\le t`，在指示变量为一时表示外部值 `p_i\le-g` 或 `p_i\ge g`。当 `t<g` 时，开区间 `(-g,-t)` 与 `(t,g)` 内的值没有有效指示变量赋值。

默认 `tolerance` 是 `NONZERO_TOLERANCE = 1e-10`，默认 `strictBoundary` 是 `STRICT_BOUNDARY = NONZERO_TOLERANCE * 16 + 16 * 2^-52`。因此，极小的非零值可能在 `evaluate` 中被计为非零，却没有求解器赋值。

## 当前 API

### Kotlin

```kotlin
XorFunction(
    polynomials: List<LinearPolynomial<V>>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "xor",
    displayName: String? = null
)
```

伴随对象的 `invoke` 重载接受 `polynomials`、`converter`、`bigM`、`name` 与 `displayName`，但不暴露 `tolerance` 或 `strictBoundary`。必须设置这些边界时请直接使用构造函数。

### Rust

Rust 暴露 [`XorFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs)：

```rust
XorFunction::new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> XorFunction<V>
```

构造器要求至少两个输入，并创建 `result_variable()`、`indicator_variables()` 和 `side_variables()`。它没有逐实例的 `bigM`、tolerance 或 strict-boundary 参数，而是使用共享的指示策略。Rust 当前求值器在输入同时包含零和非零值时返回 `1`（二输入时就是通常 XOR），因此多于两个输入时既不是 Kotlin 的恰好一个求值器，也不是 parity XOR。

## 辅助变量与注册模型

对于 `name`，实现创建：

- 结果二进制变量 `name_xor`；
- 每个输入一个非零指示变量 `name_xor_nz{i}`；
- 每个输入一个二进制侧变量 `name_xor_side{i}`。

这些变量全部由 `helperVariables` 返回；`resultPolynomial` 是 `name_xor` 的单位系数多项式。每个输入的指示变量都通过共享的非零指示辅助函数注册，使用显式 `bigM` 或该多项式的默认 Big-M。

加入指示约束后，令 `s=\sum_i a_i` 且结果 `y` 为二进制变量，当前最终 XOR 不等式是：

$$
0\le s-y,
\qquad
s-y\le n-1,
\qquad
s+(n-1)y\le n.
$$

它们蕴含 `y=1\Rightarrow s=1`，但在 `s\le n-1` 时允许 `y=0`；特别是 `s=1` 并不强制 `y=1`，而 `s=n` 会不可行。这是高风险的求解器/求值器不一致，并不是奇偶行为。

## `evaluate()` 与求解器模型的差异

求值器实现的是精确的“恰好一个”计数，仅在输入缺少或求值失败时返回 `null`。求解器还会施加容差带和上述最终不等式。因此，某个赋值可能求值得到一但允许求解器结果为零；当所有输入都在非零带之外时，也可能使求解器模型不可行。不要把当前求解器编码描述为奇偶 XOR，或描述为完整的“恰好一个”等价约束。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.XorFunction
import fuookami.ospf.kotlin.core.variable.BinVar

class XorTest {
    @Test
    fun xorEvaluate() {
        val x = BinVar("x")
        val y = BinVar("y")
        val px = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
        val py = LinearPolynomial(listOf(LinearMonomial(Flt64.one, y)), Flt64.zero)
        val xor = XorFunction(listOf(px, py), converter = IntoValue.Identity, name = "xor")

        val r10 = xor.evaluate(mapOf(x to Flt64.one, y to Flt64.zero))
        val r11 = xor.evaluate(mapOf(x to Flt64.one, y to Flt64.one))
        assertTrue(r10 != null && (r10 eq Flt64.one))
        assertTrue(r11 != null && (r11 eq Flt64.zero))
    }
}
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::XorFunction;

let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let y = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
let xor = XorFunction::new(1, "xor", vec![x, y]);
assert_eq!(xor.indicator_variables().len(), 2);
let _result = xor.result_variable();
```

:::

Rust 源码与 parity 覆盖：[`and.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs) 和 [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)。

## 源码与 core 测试

- [实现：`And.kt`（包含 `XorFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/And.kt)
- [core 泛型注册测试：`FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- [完整示例：`XorTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/XorTest.kt)

## 相关页面

- [AND](/zh-cn/guide/linear-functional/and)
- [OR](/zh-cn/guide/linear-functional/or)
- [NOT](/zh-cn/guide/linear-functional/not)
- [One-of 约束](/zh-cn/guide/linear-functional/one-of)
- [二值化](/zh-cn/guide/linear-functional/bin)
