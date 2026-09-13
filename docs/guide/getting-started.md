# Quickstart

This walkthrough builds and solves the same small integer production-allocation model in Kotlin and Rust. The tabs are complete examples: they declare variables, create named intermediate symbols, add the resource constraint and objective, call a backend, and print the solve status and result.

The snippets follow the current 1.1.0 source APIs. Backend installation is intentionally kept separate because native solver libraries and licenses depend on the machine.

## Installation

### Kotlin / Maven

The current Kotlin source targets JVM 17 and uses Maven 3 or newer. Add the starter and one concrete solver plugin to the application. The starter supplies the common modeling modules; it does not select a native solver for you.

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

```kotlin [Gradle equivalent]
implementation("io.github.fuookami.ospf.kotlin.core.plugin:ospf-kotlin-core-plugin-scip:1.1.0")
implementation("io.github.fuookami.ospf.kotlin:ospf-kotlin-starter:1.1.0")
```

:::

The current starter reactor also contains domain starters for one-, two- and three-dimensional bin packing, one- and two-dimensional cutting stock, Gantt scheduling, and network scheduling. Use those modules when their domain model is a better fit; the old -jdk8 starter artifact names are not part of the current source module list. ScipLinearSolver comes from the SCIP plugin. Its README documents the JSCIP binding and the required native SCIP library, which can be supplied by the system or by a JAR.

### Rust / Cargo

The following dependencies are for a checkout of the OSPF Rust workspace, matching the paths used by the repository examples. The gurobi10 feature exposes GurobiSolver; choose the feature that matches the Gurobi binding installed on the machine.

::: code-group

```toml [Cargo.toml]
[dependencies]
ospf-rust-core = { path = "../ospf-rust-core", version = "1.1.0", features = ["gurobi10"] }
ospf-rust-multiarray = { path = "../ospf-rust-multiarray", version = "1.1.0" }
```

```toml [SCIP alternative]
[dependencies]
ospf-rust-core = { path = "../ospf-rust-core", version = "1.1.0", features = ["scip-bundled"] }
ospf-rust-multiarray = { path = "../ospf-rust-multiarray", version = "1.1.0" }
```

:::

The Gurobi tab below requires a native Gurobi installation and a valid license. The Rust core README lists the gurobi10, gurobi11, gurobi12, and scip-bundled options and their backend-specific setup.

## Model the problem

We make two products from one limited resource:

| Product | Profit per unit | Resource per unit | Maximum units |
| --- | ---: | ---: | ---: |
| A | 3 | 2 | 4 |
| B | 5 | 3 | 3 |

Let $x_A,x_B$ be production quantities in items with integer domains $\{0,\ldots,4\}$ and $\{0,\ldots,3\}$. Total profit $P$ and resource usage $U$ are intermediate values:

$$
P=3x_A+5x_B,\qquad U=2x_A+3x_B.
$$

The constraint and objective are:

$$
\begin{aligned}
\max\quad &P\\
\text{s.t.}\quad &U\le12,\\
&x_A\in\{0,\ldots,4\},\quad x_B\in\{0,\ldots,3\}.
\end{aligned}
$$

In code, profit and resource_used correspond to $P$ and $U$. The capacity constraint references $U$; these named expressions can also serve other constraints or reports.

Enumerating $x_B=0,1,2,3$ gives best corresponding $x_A$ values 4, 4, 3, 1 and profits 12, 17, 19, 18. Therefore:

$$
x_A=3,\qquad x_B=2,\qquad U=12,\qquad P=19.
$$

## Build and solve

The following code-group tabs implement that formulation with the current high-level APIs.

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

### Read the report

The Kotlin solver returns `Ret<SolveReport<Flt64>>`. The example handles `Ok`, `Failed`, and `Fatal`, then reads the independent `problemStatus`, `terminationReason`, and `solutionPresence` fields before reading the objective and variable values. The Rust API returns `Result<SolveReport<f64>>` from `solve_linear_report_with`; its corresponding fields are `problem_status`, `termination_reason`, `solution_presence`, and solution.

For a successful optimal backend run, the values should agree with the hand enumeration: A = 3, B = 2, resource use = 12, and objective = 19. The enum text and proof details are backend/report data, so inspect the printed status and solution-presence fields instead of assuming that a feasible incumbent is already an optimality proof.

## Backend setup and source references

ScipLinearSolver in the Kotlin tab needs the SCIP plugin plus its native SCIP/JSCIP setup. GurobiSolver in the Rust tab is feature-gated and needs the matching native Gurobi runtime and license. The snippets have not been executed in this documentation workspace because those native environments are machine-specific; the numeric result above is the manually enumerated result of the displayed model, not a claim of a local solver run.

For the exact module and example layouts, see:

- [Kotlin starter README](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-starters/ospf-kotlin-starter/README.md)
- [Kotlin SCIP plugin README](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core-plugin/ospf-kotlin-core-plugin-scip/README.md)
- [Kotlin core allocation example](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo8.kt)
- [Rust core solver README](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/README.md)
- [Rust allocation example](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo8.rs)
