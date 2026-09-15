# Integrating OSPF with LLMs: Technical Path

This page turns the [business path](./integrating-llms) into implementable components. The same two nonnegative integer quantities maximize $30x_A+50x_B$, subject to $2x_A+3x_B\le120$ and $x_A+2x_B\le70$. Requiring 20 A adds $x_A\ge20$. Additional hours change a separate what-if input, not the official plan.

Distinguish **OSPF API fragments**, **example application types**, and **runtime protocols still to implement**. This is not a published integration SDK or a runnable LLM service, persistent registry, or production publisher. The business page defines the model and numerical acceptance; this page defines component connections, object lifetimes, and failure boundaries.

## 1. Components and dependency direction

| Component | Input → output | Excluded responsibility |
|---|---|---|
| Catalog / Projection | Catalog version, authenticated identity → authorized fields | No solver variables or trust in request-provided roles |
| LLM Adapter | Authorized context, request → untrusted candidate DTO | No repository writes or capability publication |
| Decoder / Validator | DTO, catalog, policy → validated IR or errors | Valid JSON is not sufficient |
| Compiler | Validated IR, symbol descriptors → bindable plan | No previous-run variables or undeclared approximations |
| math symbolic AST | Mathematical nodes and symbol bindings → evaluation or representation conversion | No replacement for field authorization, business versions, or backend checks |
| OSPF Adapter | Frozen inputs, plan, current variables → solve report | No business approval or promotion decisions |
| Result / Evidence Mapper | Report, field bindings → evidence and comparison | No rewriting backend statuses or fabricating IIS |
| Runtime / Registry | Shape, dependencies, run records → plan selection and lifecycle | No validation bypass or cached optimum posing as a plan |

LLM providers, solvers, and registries are replaceable application boundaries. Only authorized application services trigger execution.

```text
Request → authorized context → LLM candidate → strict decoding → typed IR → validation
                                                                           ↓
                                          canonicalization → compatible plan lookup
                                               ├─ hit: bind specialized plan
                                               └─ miss: generic compilation
                                                                           ↓
                                               fresh model/read-only snapshot → evidence

Run records → promotion policy → asynchronous build → shadow verification → atomic publish
Dependency change → invalidation → generic fallback or explicit error → new version
```

## 2. Connecting fields to numeric and symbolic evaluation

The catalog stores stable semantics, not runtime object addresses:

| Field | Numeric entry | Current-model binding | Capability |
|---|---|---|---|
| `production[A]` | Read solved A quantity | Current $x_A$ | Query, minimum constraint |
| `material.used` | Evaluate $2x_A+3x_B$ with frozen coefficients | Current $2x_A+3x_B$ | Query, resource constraint |
| `processing.used` | Evaluate $x_A+2x_B$ | Current $x_A+2x_B$ | Query |
| `profit` | Evaluate $30x_A+50x_B$ | Current objective expression | Query |

Each build creates a fresh `ModelBindingContext` mapping field ID and product dimension to this model's variables or expressions. It lasts only as long as its model. Templates store field IDs, parameter slots, and relations, resolving bindings again on execution.

Numeric and symbolic entries consume one immutable coefficient definition. Substitute known quantities into expressions and compare with independent numeric evaluation. Profit coefficients, capacity values, and current solutions do not become shape parameters by accident. Validate capabilities, units, dimensions, business windows, and permissions separately: visibility does not imply constraint access.

### 2.1 The math component's symbolic AST

The math component's symbolic AST supplies structured mathematics for business IR, avoiding a second general expression engine just for the LLM. See [Symbolic Expressions and Symbolic Operations](./symbolic-expressions) for nodes, evaluation, and partial evaluation, and [Compiler-Like Architecture](./compiler-architecture) for backend conversion.

This example retains three boundaries: business IR records objects, units, time, and authorization for producing at least 20 items of A today; mathematics records $x_A\ge20$; the core model binds this run's integer variables and constraints. AST nodes neither authorize field access nor establish backend support.

For $2x_A+3x_B\le120$, check units and field provenance first. Binding plan $(30,20)$ gives material usage of 120 kg; retaining unknowns produces linear coefficients $[2,3]$ and RHS 120. Simple templates may construct polynomials directly, without generating text or routing every query and workflow through mathematical ASTs.

### 2.2 ASTs and runtime specialization

Reusable plans retain expression structure and stable field descriptions, then bind parameters and current-model variables at execution. The expression layer governs general symbolic caching; runtime dependencies additionally include catalogs, authorization scope, business windows, and publication versions.

Equal expression hashes do not imply equal scenarios or authorization. An evaluation-plan cache is not an optimal-solution cache. Representing $x_Ax_B$ does not make it valid in this linear example: explicitly choose an appropriate modeling path or reject it, never silently drop the product.

## 3. LLM output is not an internal domain object

Adapter inputs contain projected catalogs, baseline references, context hashes, allowed operations, and budgets. Apply response-size limits before JSON parsing, then validate schema version, unknown properties, enums, integer overflow, required fields, and array length. Do not truncate fractional quantities. Class names, URLs, SQL, and scripts cannot designate executable operations.

Map boundary DTOs to these closed types. String IDs simplify the example; production code should distinguish RunId and BaselineId types. These declarations are not a JSON decoder. Defensively copy collections or use immutable collections to prevent modification after validation.

::: code-group

```kotlin [Kotlin]
enum class Product { A, B }
enum class Metric { MaterialUsed, HoursUsed, Profit }
enum class UnitCode { Piece, Hour }
data class Literal(val value: Long, val unit: UnitCode)

sealed interface QueryIR {
    data class ReadRun(val runId: String, val fields: List<Metric>) : QueryIR
}
sealed interface ConstraintIR {
    data class Minimum(val product: Product, val quantity: Literal) : ConstraintIR
}
sealed interface WorkflowIR {
    data class CompareHours(
        val baselineId: String,
        val deltas: List<Literal>
    ) : WorkflowIR
}

enum class ParameterType { ProductId, NonnegativePieces }
data class MinimumPlan(
    val compilerVersion: String,
    val parameters: List<ParameterType> =
        listOf(ParameterType.ProductId, ParameterType.NonnegativePieces)
)
// Application plan data only: no OSPF variable object is cached.
```

```rust [Rust]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Product { A, B }
#[derive(Clone, Copy, Debug)]
enum Metric { MaterialUsed, HoursUsed, Profit }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnitCode { Piece, Hour }
#[derive(Clone, Copy, Debug)]
struct Literal { value: i64, unit: UnitCode }

enum QueryIR {
    ReadRun { run_id: String, fields: Vec<Metric> },
}
enum ConstraintIR {
    Minimum { product: Product, quantity: Literal },
}
enum WorkflowIR {
    CompareHours { baseline_id: String, deltas: Vec<Literal> },
}

enum ParameterType { ProductId, NonnegativePieces }
struct MinimumPlan {
    compiler_version: String,
    parameters: Vec<ParameterType>,
}
// Application plan data only: no OSPF variable object is cached.
```

:::


Queries read completed runs, constraints restrict future solutions, and workflows compose registered operations. A `ReadRun` cannot become a solver constraint; filtering historical records with `Minimum` does not build an optimization model.

The validator creates a `ValidatedRequest` unavailable to direct DTO construction, binding request hash, catalog/policy versions, baseline, authenticated identity, and confirmation records. Restrict construction to the validator. Recheck freshness and authorization before execution; possessing this object is not perpetual permission.

## 4. Validation, normalization, and compilation

### 4.1 Compiling a minimum requirement

For `Minimum(A, Literal(20, Piece))`:

1. Resolve A to a registered product and planned-quantity field.
2. Check a nonnegative integer piece quantity within declared service limits.
3. Check frozen business day, baseline, and support for the main integer linear model.
4. Check authorization and confirmation of meaning.
5. Normalize to `GE(Field(production.quantity,A),Integer(20,piece))`.
6. Bind A in this model and emit coefficient 1 with right-hand side 20.

When a native API uses floating-point coefficients, check exact representability before converting integers. Arbitrary 64-bit integers must not be converted unconditionally.

| Example application error | Example | Action |
|---|---|---|
| UNKNOWN_FIELD | Unregistered product C | Reject or clarify; no fuzzy automatic binding |
| UNIT_MISMATCH | 20 kg used as pieces | Require explicit conversion or correction |
| STALE_CONTEXT | Changed baseline or catalog | Refresh and validate again |
| UNSUPPORTED_EXPRESSION | Undeclared yield function | Return a capability gap, not silent linearization |
| FORBIDDEN_OPERATION | Bypass approval and dispatch | Reject rather than ask the LLM to retry |
| BUDGET_EXCEEDED | Excessive candidates or elapsed time | Stop and hand over |

Technical retries create new attempts; business-constraint changes create new candidates. Record parents, reasons, and budget consumption. Retries cannot expand permissions.

### 4.2 The OSPF model boundary

The native fragments below move here from the business page. Kotlin's enclosing application must still implement error propagation; comments are not executable checks. Compilation and real-solver integration tests remain acceptance work.


These model-building fragments use the respective core APIs; they are not complete LLM clients or standalone solver programs. The Kotlin fragment shows S1 and leaves result propagation to its enclosing application: every registration and constraint result must be checked immediately, aborting on failure. Rust propagates modeling errors with `?`. For S2, rebuild with 80 hours rather than retaining both the old and new capacity rows.

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.model.basic.ObjectCategory
import fuookami.ospf.kotlin.core.model.mechanism.LinearMetaModel
import fuookami.ospf.kotlin.core.variable.IntVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val model = LinearMetaModel<Flt64>(
    name = "production",
    objectCategory = ObjectCategory.Maximum
)
val xA = IntVar("xA")
val xB = IntVar("xB")
val registration = model.add(listOf(xA, xB))
// Check registration before proceeding; propagate any error.

fun p(vararg terms: LinearMonomial<Flt64>) =
    LinearPolynomial<Flt64>(terms.toList(), Flt64.zero)
fun constant(value: Double) =
    LinearPolynomial<Flt64>(emptyList(), Flt64(value))

val material = p(
    LinearMonomial(Flt64(2.0), xA),
    LinearMonomial(Flt64(3.0), xB)
)
val hours = p(
    LinearMonomial(Flt64.one, xA),
    LinearMonomial(Flt64(2.0), xB)
)
val profit = p(
    LinearMonomial(Flt64(30.0), xA),
    LinearMonomial(Flt64(50.0), xB)
)

// Check each result before continuing to the next operation.
val nonnegativeA = model.addConstraint(
    LinearInequality(p(LinearMonomial(Flt64.one, xA)),
        constant(0.0), Comparison.GE), name = "xA_nonnegative"
)
val nonnegativeB = model.addConstraint(
    LinearInequality(p(LinearMonomial(Flt64.one, xB)),
        constant(0.0), Comparison.GE), name = "xB_nonnegative"
)
val materialRow = model.addConstraint(
    LinearInequality(material, constant(120.0), Comparison.LE),
    name = "material.capacity"
)
val hoursRow = model.addConstraint(
    LinearInequality(hours, constant(70.0), Comparison.LE),
    name = "processing.capacity"
)
val objective = model.maximize(profit, name = "profit")

// S1 only: after candidate validation and confirmation.
val minimumA = model.addConstraint(
    LinearInequality(p(LinearMonomial(Flt64.one, xA)),
        constant(20.0), Comparison.GE), name = "minimum.A"
)
```

```rust [Rust]
use ospf_rust_core::error::Result;
use ospf_rust_core::model::{
    LinearExpressionBuilder, MetaModel, ObjectiveCategory,
};
use ospf_rust_core::variable::{IntegerVariableItem, VariableRange};

// Inputs here have already passed application validation.
fn build_model(
    hours: f64,
    min_a: Option<u32>,
    min_b: Option<u32>,
) -> Result<MetaModel<f64>> {
    let mut model = MetaModel::<f64>::new("production");
    // Bounds follow from the fixed 120 kg material capacity.
    let a = model.register_variable(IntegerVariableItem::auto_with_range(
        "xA", VariableRange::bounded(0.0, 60.0),
    ))?;
    let b = model.register_variable(IntegerVariableItem::auto_with_range(
        "xB", VariableRange::bounded(0.0, 40.0),
    ))?;
    model.add_linear_constraint_input(
        LinearExpressionBuilder::new().term(a, 2.0).term(b, 3.0)
            .le(120.0, "material.capacity"),
    )?;
    model.add_linear_constraint_input(
        LinearExpressionBuilder::new().term(a, 1.0).term(b, 2.0)
            .le(hours, "processing.capacity"),
    )?;
    if let Some(q) = min_a {
        model.add_linear_constraint_input(
            LinearExpressionBuilder::new().term(a, 1.0)
                .ge(f64::from(q), "minimum.A"),
        )?;
    }
    if let Some(q) = min_b {
        model.add_linear_constraint_input(
            LinearExpressionBuilder::new().term(b, 1.0)
                .ge(f64::from(q), "minimum.B"),
        )?;
    }
    model.set_linear_objective_input(
        LinearExpressionBuilder::new().term(a, 30.0).term(b, 50.0)
            .maximize("profit").category(ObjectiveCategory::Maximum),
    );
    Ok(model)
}
// S0: build_model(70.0, None, None)
// S1: build_model(70.0, Some(20), None)
// S2: build_model(80.0, Some(20), None)
// S3: build_model(70.0, Some(20), Some(30))
```

:::

Kotlin's explicit nonnegative rows encode the variable domains. Rust sets nonnegative ranges and adds redundant upper bounds implied by material capacity. These representations have the same feasible set; neither introduces auxiliary variables.

API references: [Kotlin MetaModel](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/model/mechanism/MetaModel.kt), [Rust MetaModel](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/model/meta_model.rs). These link to API implementations, not a separately published tutorial example.

Kotlin's solver flow lowers the meta model to a mechanism model and then a solver model before invocation; do not invent a `model.solve()` shortcut. Rust exposes `meta_model.solve_report(&solver)`. Configure the actual backend and propagate its report. SCIP/Gurobi require the corresponding runtime dependencies and, where applicable, a license. These snippets have been checked against source APIs, but have not been compiled or run with an OSPF solver as part of this documentation change.

## 5. Execution, results, and evidence

S1 adds the minimum row. S2 copies parameters from the same frozen input, replaces the hours right-hand side with 80, and creates fresh variables and a model. Adding an 80-hour row while retaining the 70-hour row does not implement that what-if.

The execution service configures an explicit solver, time limit, and cancellation signal and retains actual reports and failures. Map quantities through the variable binding table and evaluate intermediate values using the same model inputs. Stable constraint and field IDs connect `material.capacity` to its business explanation.

Store problem status, termination reason, and solution presence independently. Evaluate plan KPIs only when a usable solution exists: a no-solution report is not a zero-production plan. Preserve diagnostic provenance, capabilities, and missing reasons. Label the example's independent contradiction proof separately from solver-provided IIS.

The explanation adapter receives authorized evidence only: run IDs, resource lhs/rhs/slack, objective components, and baseline differences. Natural language is not a status field. Provider timeouts must not discard completed solver results; structured tables remain available.

## 6. Producing a cacheable plan from IR

### 6.1 Parameter extraction and dependency keys

`Minimum(A,20)` and `Minimum(A,25)` remain distinct concrete requests but share a parameterized shape:

```text
shape = Minimum(product: ProductId, quantity: NonnegativeInteger[piece])
parameters(request1) = [A, 20]
parameters(request2) = [A, 25]
plan = resolve product field → emit GE coefficient 1, RHS parameter[1]
```

The shape fixes operation version, types, units, temporal semantics, and algorithm placement. Request hashes additionally contain actual parameters and input references. Matching shapes do not authorize reuse across incompatible catalogs or policies.

Registry selection uses `tenantScope + shapeHash + dependencyKey`. Dependencies include catalog semantics, policy, model semantics, compiler, and adapter versions; database queries also include source schema versions. Identity comes from the session, not a user-supplied shape string.

Plans retain parameter slots, resolved descriptors, and binding instructions—not variable indices, solver handles, or old results. Binding order is versioned and cannot depend on map iteration. After registering fresh variables, use only indices returned by the current context.

### 6.2 Two execution paths

This language-neutral pseudocode uses example ports, not OSPF API methods:

```text
execute(candidate, authenticatedSession):
    validated = validate(candidate, authenticatedSession, frozenCatalog)
    shape, parameters = canonicalizeAndExtract(validated)
    key = (authorizedScope, shape.hash, dependencyVersions)
    plan = registry.findActive(key)
    if plan exists and verifyDigestAndDependencies(plan):
        executable = plan
        path = Specialized
    else:
        executable = genericCompiler.compile(validated)
        path = Generic
    recheckAuthorizationAndVersions(validated, executable)
    context = createFreshExecutionContext(validated.baseline)
    result = bindAndExecute(executable, parameters, context)
    appendRun(path, key, planVersionIfUsed, inputRefs, result)
    return result
```

An invalid registry entry does not make a candidate valid. Generic compilation also rejects unknown or unsupported fields. Before specialized execution begins, switching to the generic path can be safe. If execution has begun or completion is uncertain, establish its status before retrying. Even read-only solves retain new run IDs and parent references; official side effects require a separate idempotency protocol.

Specialization can reduce repeated parsing, field resolution, and orchestration; it does not automatically reduce integer-program search. Measure LLM, parsing, validation, modeling, and solving time separately.

## 7. Queries and workflows on the same runtime

### 7.1 Query plans

`ReadRun(runId,fields)` binds a source snapshot without creating an OSPF model. Compile fixed field accessors and result mappings, then check run ownership, field permissions, and budgets on execution. A plan ID is not a result-cache key.

The first version needs neither joins nor SQL. A database adapter registers field-to-column mappings and pushes down only provably equivalent expressions. Keep residual evaluation or reject unsupported expressions. Equivalence includes nulls, row grain, ordering, pagination, and authorization.

### 7.2 Workflow plans

Expand `CompareHours(base,deltas)` into registered ReadBaseline, CreateHoursCandidates, SolveCandidates, and CompareResults nodes. Validate acyclicity, port types, hour units, candidate limits, node versions, and result dependencies.

Fix one baseline for the execution context. Each delta creates an independent candidate instead of accumulating on the previous candidate. Solves may run concurrently, but aggregate by candidate ID and retain timeout, cancellation, and no-solution statuses. Missing reports must not become zero-profit ranking entries.

Use real pure-computation and solver ports in the sandbox. Do not register official publication, dispatch, or external-write ports. Missing ports mean rejection, not reflective lookup of similarly named handlers.

## 8. Asynchronous builds, publication, and recovery

Separate immutable `PlanVersion`, append-only `PlanEvent`, and an atomically updated `ActivePointer`. Versions retain parameter schemas, canonical IR, dependencies, digest, verification references, creator, and rollback references. The active pointer is an index, not historical truth.

1. Aggregate shape observations within tenant scope; create an idempotent build job when policy passes.
2. Deduplicate jobs by shape and dependencies; do not synchronously compile in the request thread.
3. Verify candidates against fixed samples and counterexamples; shadow output does not change the generic response.
4. Recheck dependencies before publication; switch the active pointer using expected version/CAS and record the event.
5. Retain failed-version diagnostics and prevent partial ACTIVE state. Compatible older versions or generic execution remain available.
6. Invalidate through dependency indexes for catalog, policy, or schema changes; duplicate events are idempotent.
7. On restart, reload active pointers and verify dependencies and digests before use.
8. Historical replay selects exact versions and original snapshots; current execution requires current compatibility.

Multi-instance publication requires transactions or an equivalent consistency protocol; a file cache is not a publication lock. Revocation and rollback append facts rather than overwrite history. Explicit invocation by capability ID still passes the same authorization boundary.

## 9. Verification layers and delivery boundary

| Layer | Required tests | Insufficient substitute |
|---|---|---|
| Decoder | Unknown fields, overflow, nesting limits, invalid units | Valid JSON examples only |
| Validator | Permissions, time scope, stale context, confirmation | LLM assertions of validity |
| Compiler | Coefficients, relations, bounds, and domains from IR | Successful model allocation |
| Binding | One template bound to two fresh, isolated models | Reusing first-run indices |
| Solver adapter | S0/S1=1900, S2=1930, S3 infeasible, termination semantics | Enumeration posing as a real backend |
| Runtime | Hits/misses, revocation, restart, concurrent publication, fallback | In-memory map lookup only |
| Equivalence | Values, authorization, budgets, and errors across paths | Hash-only or latency-only comparison |
| LLM adapter | Generation, clarification, malformed output, timeout, cost | Fixtures replacing live-provider evaluation |

Implement in this order: fixed-candidate decoding, validation, IR compilation, actual OSPF solving, evidence mapping, generic run records, parameterized plans, registry and shadow checks, publication, invalidation, and recovery. Keep every stage independently replayable. Computing a request hash is not a completed simulation.

These types and algorithms define an implementation contract. The current deliverable is documentation and API fragments, not a complete runtime or real-solver integration tests. Subsequent engineering should add separate implementation and test files in the Kotlin/Rust example repositories, then link verified runnable entry points here. Do not link nonexistent examples in advance.

Return to the [business path](./integrating-llms) for the full mathematical model, scenario comparisons, and nine-step capability-formation trace.
