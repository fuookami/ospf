# 快速开始

本教程使用 Kotlin 和 Rust 构建并求解同一个小型整数生产分配模型。代码标签中的示例都是完整示例：声明变量、创建带名称的中间符号、添加资源约束和目标函数、调用后端，并输出求解状态和结果。

示例遵循当前 1.1.0 源码中的 API。后端安装单独说明，因为原生求解器库和许可证取决于运行机器。

## 安装

### Kotlin / Maven

当前 Kotlin 源码以 JVM 17 为目标，并使用 Maven 3 或更高版本。请在应用中加入 starter 和一个具体的求解器插件。starter 提供通用建模模块，但不会替应用选择原生求解器。

::: code-group

```xml [pom.xml]
<dependency>
    <groupId>io.github.fuookami.ospf.kotlin</groupId>
    <artifactId>ospf-kotlin-starter</artifactId>
    <version>1.1.0</version>
</dependency>

<dependency>
    <groupId>io.github.fuookami.ospf.kotlin.core.plugin</groupId>
    <artifactId>ospf-kotlin-core-plugin-scip</artifactId>
    <version>1.1.0</version>
</dependency>
```

```kotlin [Gradle 等价写法]
implementation("io.github.fuookami.ospf.kotlin.core.plugin:ospf-kotlin-core-plugin-scip:1.1.0")
implementation("io.github.fuookami.ospf.kotlin:ospf-kotlin-starter:1.1.0")
```

:::

当前 starter reactor 还包含一维、二维和三维装箱、一维和二维下料、甘特图排程以及网络排程等领域 starter。需要对应领域模型时再使用这些模块；当前源码模块列表中已经没有旧的 -jdk8 starter 构件名。ScipLinearSolver 来自 SCIP 插件。插件 README 说明了 JSCIP 绑定及所需的原生 SCIP 库；原生库可以由系统提供，也可以由 JAR 提供。

### Rust / Cargo

下面的依赖适用于 OSPF Rust workspace checkout，与仓库示例使用的路径一致。gurobi10 feature 会导出 GurobiSolver；请根据机器上安装的 Gurobi 绑定选择对应 feature。

::: code-group

```toml [Cargo.toml]
[dependencies]
ospf-rust-core = { path = "../ospf-rust-core", version = "1.1.0", features = ["gurobi10"] }
ospf-rust-multiarray = { path = "../ospf-rust-multiarray", version = "1.1.0" }
```

```toml [SCIP 替代方案]
[dependencies]
ospf-rust-core = { path = "../ospf-rust-core", version = "1.1.0", features = ["scip-bundled"] }
ospf-rust-multiarray = { path = "../ospf-rust-multiarray", version = "1.1.0" }
```

:::

下面的 Gurobi 标签需要原生 Gurobi 安装和有效许可证。Rust core README 列出了 gurobi10、gurobi11、gurobi12 以及 scip-bundled 选项和对应的后端设置。

## 建模

我们使用一种有限资源生产两种产品：

| 产品 | 单位利润 | 单位资源用量 | 最大产量 |
| --- | ---: | ---: | ---: |
| A | 3 | 2 | 4 |
| B | 5 | 3 | 3 |

令 $x_A,x_B$ 为产量，单位件，分别取整数域 $\{0,\ldots,4\}$ 和 $\{0,\ldots,3\}$。总利润 $P$ 和资源用量 $U$ 是中间值：

$$
P=3x_A+5x_B,\qquad U=2x_A+3x_B.
$$

约束和目标为：

$$
\begin{aligned}
\max\quad &P\\
\text{s.t.}\quad &U\le12,\\
&x_A\in\{0,\ldots,4\},\quad x_B\in\{0,\ldots,3\}.
\end{aligned}
$$

代码中的 profit 和 resource_used 分别对应 $P$ 和 $U$。容量约束引用 $U$，这些具名表达式也可供其他约束或报告复用。

枚举 $x_B=0,1,2,3$，对应最佳 $x_A$ 分别为 4、4、3、1，利润分别为 12、17、19、18。因此：

$$
x_A=3,\qquad x_B=2,\qquad U=12,\qquad P=19.
$$

## 构建并求解

下面的代码组标签使用当前的高层 API 实现同一模型。

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.multiarray.*
import fuookami.ospf.kotlin.math.*
import fuookami.ospf.kotlin.math.algebra.number.*
import fuookami.ospf.kotlin.math.symbol.operation.*
import fuookami.ospf.kotlin.math.symbol.polynomial.*
import fuookami.ospf.kotlin.core.model.mechanism.*
import fuookami.ospf.kotlin.core.solver.*
import fuookami.ospf.kotlin.core.solver.report.*
import fuookami.ospf.kotlin.core.solver.scip.*
import fuookami.ospf.kotlin.core.solver.value.*
import fuookami.ospf.kotlin.core.symbol.*
import fuookami.ospf.kotlin.core.variable.*

data class Product(
    val label: String,
    val profit: Flt64,
    val resource: Flt64,
    val maxUnits: UInt64
) : AutoIndexed(Product::class)

private val products = listOf(
    Product("A", Flt64(3.0), Flt64(2.0), UInt64(4)),
    Product("B", Flt64(5.0), Flt64(3.0), UInt64(3))
)

private suspend fun solve(
    solver: AbstractLinearSolver,
    model: LinearMetaModel<Flt64>
): Ret<SolveReport<Flt64>> {
    val mechanism = when (val result = solver.dump(
        model = model,
        registrationStatusCallBack = null,
        dumpingStatusCallBack = null
    )) {
        is Ok -> result.value
        is Failed -> return Failed(result.error)
        is Fatal -> return Fatal(result.errors)
    }
    val triad = solver.dump(mechanism)
    return solver(model = triad, solvingStatusCallBack = null)
}

suspend fun main() {
    val model = LinearMetaModel<Flt64>(
        name = "production_allocation",
        converter = IntoValue.Identity
    )
    try {
        val x = UIntVariable1("production", Shape1(products.size))
        for (product in products) {
            x[product].name = x.name + "_" + product.label
            x[product].range.leq(product.maxUnits)
        }
        model.add(x)

        val profit = LinearExpressionSymbol(
            sum(products.map { product -> product.profit * x[product] }),
            name = "profit"
        )
        val resourceUsed = LinearExpressionSymbol(
            sum(products.map { product -> product.resource * x[product] }),
            name = "resource_used"
        )
        model.add(profit)
        model.add(resourceUsed)
        model.addConstraint(
            resourceUsed leq Flt64(12.0),
            name = "resource_limit"
        )
        model.maximize(profit, "profit")

        when (val result = solve(ScipLinearSolver(), model)) {
            is Ok -> {
                println("problemStatus: " + result.value.problemStatus)
                println("terminationReason: " + result.value.terminationReason)
                println("solutionPresence: " + result.value.solutionPresence)
                if (result.value.solution != null) {
                    model.tokens.setSolution(result.value.values)
                    println("objective: " + result.value.solution?.objective)
                    println("A: " + model.tokens.find(x[products[0]])?.result)
                    println("B: " + model.tokens.find(x[products[1]])?.result)
                }
            }
            is Failed -> println("solve failed: " + result.error)
            is Fatal -> println("solve failed: " + result.errors)
        }
    } finally {
        model.close()
    }
}
```

```rust [Rust]
use std::error::Error;
use std::sync::Arc;

use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use ospf_rust_core::symbol::{
    next_auto_intermediate_symbol_id, LinearExpressionSymbol, LinearIntermediateSymbol,
};
use ospf_rust_core::variable::{UInteger, VariableCombination1D, VariableRange};
use ospf_rust_multiarray::{MultiArray, Shape};

fn coefficients(symbol: &LinearExpressionSymbol<f64>) -> Vec<(usize, f64)> {
    symbol
        .to_linear_polynomial()
        .monomials()
        .iter()
        .map(|monomial| (monomial.var_index(), *monomial.coefficient()))
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut model = MetaModel::<f64>::new("production_allocation");
    let production = VariableCombination1D::<UInteger>::with_range_generator(
        Shape::new([2]),
        "production",
        |index, _| VariableRange::bounded(0.0, if index == 0 { 4.0 } else { 3.0 }),
    );
    let x: MultiArray<usize, Shape<1>> = model.register_combination(&production)?;

    let profit = LinearExpressionSymbol::new(
        next_auto_intermediate_symbol_id(),
        "profit",
        vec![
            LinearMonomial::new(3.0, x[0]),
            LinearMonomial::new(5.0, x[1]),
        ],
        0.0,
    );
    let resource_used = LinearExpressionSymbol::new(
        next_auto_intermediate_symbol_id(),
        "resource_used",
        vec![
            LinearMonomial::new(2.0, x[0]),
            LinearMonomial::new(3.0, x[1]),
        ],
        0.0,
    );

    let profit_coefficients = coefficients(&profit);
    let resource_coefficients = coefficients(&resource_used);
    model.add_symbol(Arc::new(profit))?;
    model.add_symbol(Arc::new(resource_used))?;
    model.add_linear_constraint(
        &resource_coefficients,
        ConstraintRelation::LessEqual,
        12.0,
        "resource_limit",
    )?;
    model.add_linear_objective(&profit_coefficients, "profit");
    model.set_objective_category(ObjectiveCategory::Maximum);

    let report = model.solve_linear_report_with(&GurobiSolver::new())?;
    println!("problem_status: {:?}", report.problem_status);
    println!("termination_reason: {:?}", report.termination_reason);
    println!("solution_presence: {:?}", report.solution_presence);
    if let Some(solution) = report.solution.as_ref() {
        println!("objective: {:?}", solution.objective);
        println!("A: {:?}", solution.values.get(x[0]));
        println!("B: {:?}", solution.values.get(x[1]));
    }
    Ok(())
}
```

:::

### 读取报告

Kotlin 求解器返回 `Ret<SolveReport<Flt64>>`。示例完整处理 `Ok`、`Failed` 和 `Fatal`，然后读取独立的 `problemStatus`、`terminationReason` 和 `solutionPresence` 字段，再读取目标值和变量值。Rust API 的 `solve_linear_report_with` 返回 `Result<SolveReport<f64>>`，对应字段为 `problem_status`、`termination_reason`、`solution_presence` 和 solution。

后端成功并证明最优时，结果应与手工枚举一致：A = 3、B = 2、资源用量 = 12、目标值 = 19。枚举文本和证明细节属于后端报告数据，因此应检查输出的状态和解存在性字段，不要把可行 incumbent 直接当作最优性证明。

## 后端设置与源码参考

Kotlin 标签中的 ScipLinearSolver 需要 SCIP 插件以及原生 SCIP/JSCIP 设置。Rust 标签中的 GurobiSolver 受 feature 控制，需要匹配的原生 Gurobi runtime 和许可证。由于原生环境依赖机器配置，本教程代码没有在当前文档 workspace 中执行；上面的数值结果来自对展示模型的手工枚举，不是本地求解器运行结果的声明。

确切的模块和示例目录请参考：

- [Kotlin starter README](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-starters/ospf-kotlin-starter/README.md)
- [Kotlin SCIP 插件 README](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-scip/README.md)
- [Kotlin 核心分配示例](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo8.kt)
- [Rust core solver README](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/README.md)
- [Rust 分配示例](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo8.rs)
