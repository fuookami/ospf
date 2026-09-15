# 二次正部函数

`QuadraticPositivePartFunction` 为 Kotlin 与 Rust 定义相同且无歧义的运算：

$$
y=\max\{p(x),0\}.
$$

它是正部函数，不是半连续变量。旧 Rust 名称 `QuadraticSemiFunction` 已直接删除；半连续取值域仍由独立的线性 `SemiFunction` API 表达。

## 求解器数学模型

### Kotlin

Kotlin 使用恒等式 $y=-\min\{-p(x),0\}$。令 $t=\min\{-p(x),0\}$，$u_0,u_1\in\{0,1\}$，其精确选择模型为：

$$
\begin{aligned}
t &\le -p(x),\\
t &\le 0,\\
t &\ge -p(x)-M(1-u_0),\\
t &\ge -M(1-u_1),\\
u_0+u_1 &=1,\\
y&=-t.
\end{aligned}
$$

公开的二次结果表达式是 `-resultVar`；`resultVar` 本身是内部最小值 $t$。

### Rust

Rust 先以 $b=p(x)$ 桥接二次输入，再应用两个候选值的精确最大值模型：

$$
\begin{aligned}
b&=p(x),\\
y&\ge b,\\
y&\ge0,\\
y&\le b+M(1-u_0),\\
y&\le M(1-u_1),\\
u_0+u_1&=1.
\end{aligned}
$$

两种内部公式不同，但结果与直接求值契约一致。

## API 与示例

::: code-group

```kotlin [Kotlin]
val p = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = -Flt64(4.0)
)
val positivePart = QuadraticPositivePartFunction(
    input = p,
    bigM = Flt64(100.0),
    converter = IntoValue.Identity,
    name = "positive_part"
)
// x = 1：max(1^2 - 4, 0) = 0
```

```rust [Rust]
let p = Quadratic::new(
    vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
    -4.0,
);
let positive_part = QuadraticPositivePartFunction::new(
    17,
    "positive_part",
    p,
);
// x = 1: max(1^2 - 4, 0) = 0
```

:::

Kotlin 可传入显式 `bigM`；为空时从多项式的有限范围推导候选专用值。Rust 在可能时从已注册 token 的范围推导选择约束的 Big-M，否则使用已配置的回退值。

## 求值与边界

直接求值在输入为负数时返回零，为正数时返回输入，在原点返回零。输入值缺失时返回 `null`/`None`。该算术函数没有基于容差的 `Undefined` 区域。

## 测试与源码

- Kotlin 源码：[`QuadraticPositivePart.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticPositivePart.kt)
- Kotlin 独立测试：[`QuadraticPositivePartFunctionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticPositivePartFunctionTest.kt)
- Rust 源码：[`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust 独立测试：[`function_symbol_quadratic_positive_part.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_positive_part.rs)

## 相关页面

- [线性半连续标记](../linear-functional/semi)
- [二次最小值](./quadratic-min)
