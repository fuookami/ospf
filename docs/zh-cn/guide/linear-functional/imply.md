# 蕴含

`ImplyFunction` 为两个线性表达式建模蕴含 `A ⇒ B`，其中两个表达式都解释为相对于零的关系。它使用共享的离散条件判定器和三值边界语义。

> [!WARNING]
> `ImplyFunction` 不是任意 solver 符号上的通用布尔代数。注册要求条件范围有限、有序；旧的 `bigM` 参数即使传入也会校验，但不能替代这些范围。

## 契约

- `antecedent` 和 `consequent` 是 `LinearPolynomial<V>` 值。
- `relation`（默认 `Comparison.GT`）分别应用于两个多项式与零的比较。
- `evaluate` 对 true 返回 one、对 false 返回 zero、对未定义条件或缺失/非法的必需输入返回 `null`。
- `strictBoundary` 是真假分支的间隔；未提供 `strictBoundary` 时，兼容参数 `tolerance` 用作该业务间隔；`delta` 默认取 `strictBoundary`。
- 可选的 `antecedentBounds` 和 `consequentBounds` 必须在 solver 注册时为有限且有序。

## 定义与数学模型

经典逻辑为：

$$
A\Rightarrow B\equiv\lnot A\lor B.
$$

对于每个关系，共享判定器使用间隔 $g=\text{strictBoundary}$。默认 `GT` 关系下，差值 $d\ge g$ 为 true，$d\le0$ 为 false，$0<d<g$ 为 undefined。`GE`、`LT` 和 `LE` 使用相应的同一矩阵。

蕴含结果为：

| 前件 | 后件 | 结果 |
| --- | --- | --- |
| False | 不检查 | True |
| True | True | True |
| True | False | False |
| Undefined | 不检查 | Undefined |

## 实现、辅助变量与约束

实现创建名称 `name` 后接 `_ant_nz` 和 `name` 后接 `_con_nz` 的二值指标。注册时校验并规范化两个有限条件范围；在可能时折叠常量分支，并生成共享的关系指标约束。false 前件会短路求值，但注册仍会预检后件范围，因此可能在写入 token 或约束前失败。

## 当前 API

### Kotlin

源码：[`Imply.kt`（`ImplyFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Imply.kt#L229-L1148)

```kotlin
ImplyFunction(
    antecedent: LinearPolynomial<V>,
    consequent: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "imply",
    displayName: String? = null,
    relation: Comparison = Comparison.GT,
    antecedentBounds: ConditionBounds<V>? = null,
    consequentBounds: ConditionBounds<V>? = null,
    delta: V? = null
)
```

### Rust

源码：[`imply.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/imply.rs)

Rust 保留了针对两个 `LinearInequality<V>` 和显式 `big_m` 的旧版 `ImplyFunction`；其直接求值使用普通的布尔不等式判定。与 Kotlin 有限范围、三值蕴含最接近的是 `ConditionalImplyFunction`：从两个 `ConditionalIfFunction` 描述器构造（或使用 `from_parts`），为每一侧指定关系、严格边界和有限范围。前件为假时短路为真；前件未定义，或到达的后件未定义时，结果映射为 `None`。

```rust
ImplyFunction::new(
    id: u64,
    name: &str,
    premise: LinearInequality<V>,
    consequence: LinearInequality<V>,
    big_m: V,
) -> Self
ConditionalImplyFunction::from_parts(
    id: u64,
    name: &str,
    premise: Linear<V>,
    premise_relation: ConditionRelation,
    premise_strict_boundary: V,
    premise_bounds: ConditionBounds<V>,
    consequence: Linear<V>,
    consequence_relation: ConditionRelation,
    consequence_strict_boundary: V,
    consequence_bounds: ConditionBounds<V>,
) -> Result<Self>
ConditionalImplyFunction::evaluate(
    &self,
    premise_difference: &V,
    consequence_difference: &V,
) -> Result<Option<V>>
```

## evaluate 与 solver 的差异

直接求值先判定前件：前件 false 时返回 true 且不读取后件；前件 true 时判定后件；前件 undefined 时返回 `null`。solver 注册构造共享指标约束，执行范围校验与常量折叠，并使用二值指标作为模型结果。因此直接求值的短路不会绕过注册阶段的范围错误。

## 边界、tolerance 与 Undefined

缺少前件输入会导致判定失败；只有在前件为 true 后，缺少后件输入才有影响。落在判定器间隔内的值是 `TruthValue.Undefined`，所以 `evaluate` 返回 `null`。非有限或 sentinel 条件范围、逆序范围、非正 `strictBoundary`/`delta` 以及不合适的 Big-M 都会通过 Result API 使注册失败。

## 当前最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.ImplyFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val antecedent = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val consequent = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, y)), Flt64.zero
)
val implication = ImplyFunction(
    antecedent = antecedent,
    consequent = consequent,
    converter = IntoValue.Identity,
    strictBoundary = Flt64(0.1),
    antecedentBounds = ConditionBounds(Flt64(-10.0), Flt64(10.0)),
    consequentBounds = ConditionBounds(Flt64(-10.0), Flt64(10.0)),
    name = "imply"
)
val value = implication.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64.zero, y to Flt64.zero)
)
check(value == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, ConditionalImplyFunction,
};

let function = ConditionalImplyFunction::from_parts(
    1,
    "imply",
    Linear::new(vec![], 1.0),
    ConditionRelation::Greater,
    0.1,
    ConditionBounds {
        lower: -2.0,
        upper: 2.0,
    },
    Linear::new(vec![], 1.0),
    ConditionRelation::Greater,
    0.1,
    ConditionBounds {
        lower: -2.0,
        upper: 2.0,
    },
)
.expect("valid conditional implication");
let value = function
    .evaluate(&1.0, &1.0)
    .expect("classifiable implication");
assert_eq!(value, Some(1.0));
```

:::

## 测试与示例

- Core 回归测试：[`ImplyFunctionRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ImplyFunctionRegressionTest.kt)
- 示例目录（当前没有专门的蕴含文件）：[linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)
- Rust 实现与单元测试：[`imply.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/imply.rs)
- Rust 条件回归：[`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)

## 相关页面

- [条件 IF](./if)
- [条件 If-Then](./if-then)
- [Sigmoid](./sigmoid)
