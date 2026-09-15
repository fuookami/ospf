# Integrating OSPF with LLMs: Business Path

This page explains the problem, decision process, and capability lifecycle. Continue with the [technical path](./integrating-llms-technical) for components, typed IR, compilation, execution, and plan recovery. The original URL remains unchanged.

This tutorial develops a complete, small production-planning problem and shows how natural-language requests can query, change, and compare OSPF optimization models. No external business-system knowledge is required. The LLM proposes requests, the application validates and confirms them, and OSPF and the solver perform mathematical modeling and optimization.

The integration contract below is an application design, not a built-in OSPF chat or JSON compilation API. The model can be solved without an LLM. A live LLM is a replaceable boundary, not a dependency of deterministic regression tests.

The destination is **LLM as Runtime**: users express new queries, rules, and operation combinations during use. The application converts them to controlled intermediate representations, validates and executes them, then uses execution evidence to decide what deserves specialization. The LLM is a semantic frontend, not a replacement runtime for OSPF or the application executor.

Sections 1–5 establish a decision loop: intent, validation, what-if, comparison, and selection. Section 6 establishes a capability-formation loop: execution records, stable patterns, specialization, verified publication, invalidation, and fallback. Saving a prompt or template alone does not complete that loop. Registries, query plans, workflows, and state names below are tutorial application designs, not claims of built-in OSPF implementations.

## 1. Production planning domain model

### 1.1 Overview and dependent contexts

A factory produces A and B and maximizes daily profit subject to material and processing-time limits. This example has one production-planning context and no upstream context dependencies. All production can be sold. Inventory, changeovers, machine sequencing, and fixed costs are excluded. Processing time is an additive resource budget, not a calendar start or finish time.

### 1.2 Concepts and entities

The following deterministic inputs define the entire instance:

| Product $i$ | Unit profit $r_i$ (CNY/piece) | Material coefficient $a_i$ (kg/piece) | Processing coefficient $t_i$ (hours/piece) |
|---|---:|---:|---:|
| A | 30 | 2 | 1 |
| B | 50 | 3 | 2 |

**$M=120$**: daily material availability in kg. **$H=70$**: daily processing availability in hours. Unit profit is a given contribution margin; costs are not subtracted again.

### 1.3 Variables

**Decision variable $x_i$**: daily planned production, measured in pieces, with $x_i\in\mathbb Z_{\ge0}$ for every $i\in P$. A plan is not an observed production fact.

**Auxiliary variables**: none are required. Intermediate values and slacks below are evaluated directly from decision variables.

### 1.4 Predicates

**MinimumRequired**: a product is selected by a confirmed minimum-production rule in this scenario. This predicate determines which minimum constraints are added; it does not change product attributes.

### 1.5 Sets

**$P=\{A,B\}$**: all products. **$P^{\mathrm{min}}\subseteq P$**: products with a confirmed minimum requirement. This subset is empty in the baseline.

### 1.6 Intermediate values

**Material usage $U_M$**: planned quantities multiplied by their material coefficients, in kg.

$$
U_M=\sum_{i\in P}a_ix_i=2x_A+3x_B.
$$

**Processing usage $U_H$**: total planned resource hours.

$$
U_H=\sum_{i\in P}t_ix_i=x_A+2x_B.
$$

**Total profit $R$**: contribution profit in CNY.

$$
R=\sum_{i\in P}r_ix_i=30x_A+50x_B.
$$

**Remaining resources $S_M,S_H$**: material and hours remaining at a candidate solution. Negative values indicate violated resource constraints, not a feasible plan.

$$
S_M=M-U_M,\qquad S_H=H-U_H.
$$

### 1.7 Assertions

Inputs satisfy these properties; missing values must not default to zero:

$$
M,H\ge0,\qquad
\forall i\in P:\ a_i>0\land t_i>0\land r_i\ge0.
$$

Inputs must also be finite, product identifiers unique, and units compatible. A minimum quantity $q_i$ must be a nonnegative integer. Reject invalid inputs before building a model.

### 1.8 Constraints

**Material capacity [材料容量]**: planned consumption cannot exceed daily supply.

$$
\text{s.t.}\quad 2x_A+3x_B\le120.
$$

**Processing capacity [加工容量]**: processing usage cannot exceed available hours.

$$
\text{s.t.}\quad x_A+2x_B\le H.
$$

**Minimum production [最低产量]**: add hard constraints only for confirmed requirements.

$$
\text{s.t.}\quad x_i\ge q_i,\qquad\forall i\in P^{\mathrm{min}}.
$$

There are no soft constraints in this example. Changing a penalty cannot bypass material capacity or a confirmed minimum requirement.

### 1.9 Objective and actual solver model

Maximize total profit. The complete baseline sent to the solver is:

$$
\begin{aligned}
\max\quad &30x_A+50x_B\\
\text{s.t.}\quad &2x_A+3x_B\le120,\\
&x_A+2x_B\le70,\\
&x_A,x_B\in\mathbb Z_{\ge0}.
\end{aligned}
$$

Intermediate values are expressions, not additional equality rows or auxiliary columns. Confirmed minimum requirements add rows. An hours what-if changes only the processing right-hand side in its own scenario copy.

### 1.10 Algorithm references

This integer linear program requires no additional domain algorithm. Finite enumeration below independently checks the small instance; configured OSPF solver adapters handle actual optimization.

### 1.11 Ubiquitous language

| Term | Definition |
|---|---|
| Candidate request | An LLM-proposed operation not yet accepted by the application |
| Plan | Production quantities obtained for frozen inputs and rule versions |
| What-if | A separate parameter or rule modification that preserves the original plan |
| Evidence | Data traceable to a scenario, model element, and actual result |

### 1.12 Design decisions

Integer quantities prevent fractional products. A single day and two products make every result independently reproducible. Allowlisted operations prevent execution of arbitrary generated source code. The application owns the semantic catalog and authorization; these are not mathematical solver constraints.

### 1.13 Change log

| Version | Change | Reason |
|---|---|---|
| 1 | Baseline model and controlled dialogue workflow | Demonstrate a standalone, verifiable OSPF–LLM integration |

## 2. From optimization to interaction

```text
User request + authorized catalog and scenario snapshot
  → LLM: structured candidate
  → Application: parsing, semantic checks, confirmation, version and permission checks
  → OSPF: model construction and solver invocation
  → Application: result parsing, intermediate values, scenario differences
  → LLM: evidence-backed explanation
```

The LLM does not certify optimality or write production state. Valid JSON can still misinterpret “at least,” “additional,” or “today.” Confirm business meaning: feasibility does not prove faithful translation.

The dialogue has explicit scenario inheritance:

| Step | Intent | Context |
|---|---|---|
| S0 | What production maximizes profit? | Baseline, 70 hours, no minimum requirements |
| S1 | Produce at least 20 A | Add $x_A\ge20$ to S0 |
| S2 | What if we add 10 hours? | Branch from S1, set $H=80$, retain the A minimum |
| S3 | Also produce at least 30 B | Branch from S1 with $H=70$, retain the A minimum; do not inherit S2 |

## 3. Exposing model semantics to the LLM

Provide a small catalog rather than asking the LLM to guess variable indices:

| Stable identifier | Meaning and unit | Allowed operations |
|---|---|---|
| `production[A]`, `production[B]` | Daily planned quantities, integer pieces | Query, propose minimum |
| `material.used` | $U_M$, kg | Query |
| `processing.used` | $U_H$, hours | Query |
| `profit` | $R$, CNY | Query |
| `processing.capacity` | $H$, hours | Change in a what-if copy |

Bind business date, time zone, input version, catalog version, and confirmed rules. Only authorized fields and scenarios enter the LLM context. Logs should not retain credentials or unrestricted raw business inputs.

“Produce at least 20” requires clarification of the product. “At least 20 kg of A” cannot silently become 20 pieces. Product identifiers are restricted to A and B; user text cannot access arbitrary properties or invoke functions.

### 3.1 The field catalog is a type contract

For `production[A]`, record the stable field ID `production.quantity`, product dimension A, integer type, `piece` unit, `planned` state role, business day, and `query/minimumConstraint` capabilities. A and B are stable product IDs in this example. Display labels can be translated; identifiers do not change with language.

A catalog snapshot says what a field meant, an input snapshot says which data existed, and an authorization projection says what this actor may access now. Keep these distinct; one revision cannot substitute for all three. A field capability is not a grant of permission.

The expression $U_M=2x_A+3x_B$ has three consumers: OSPF uses it in a resource constraint, numeric evaluation substitutes plan quantities to obtain kg, and the LLM references its catalog entry and result evidence. All use the same coefficients and units, rather than reinventing a formula in a prompt.

### 3.2 Queries and constraints have different IRs

“Show S1 material and processing usage” does not require another solve. Resolve S1 to a fixed successful run and read its result dataset. A minimal query candidate is:

```json
{
  "schemaVersion": 1,
  "operation": "queryResult",
  "sourceRunId": "run-S1-1",
  "catalogVersion": "production-v1",
  "select": ["material.used", "processing.used"],
  "maxRows": 1
}
```

`run-S1-1` is illustrative; the application assigns real IDs. Check run ownership, result snapshot, and catalog version. Apply the stricter of requested and service budgets. Authorization covers projection, filtering, sorting, and aggregation, not just visible output columns.

A query predicate selects existing records; a mathematical constraint restricts future feasible solutions. Filtering on `material.used <= 120` does not change the optimization model. A constraint must bind to this run's symbolic expression. An infeasible run has no feasible-plan values: distinguish legitimate empty results, missing results, and unknown values instead of substituting zero. Missing historical sources are errors, not invitations to read current results.

## 4. From natural language to a constraint

“Produce at least 20 pieces of A” means:

$$
\forall x\in\mathcal F_{\mathrm{accepted}}:\ x_A\ge20.
$$

Here $\mathcal F_{\mathrm{accepted}}$ is the set of allowed plans after adding the rule. Compilation adds the linear row $x_A\ge20$; it does not change the objective.

The following is a proposed application-level candidate contract, not an OSPF input API:

```json
{
  "schemaVersion": 1,
  "baseScenarioId": "S0",
  "catalogVersion": "production-v1",
  "operation": "minimumProduction",
  "product": "A",
  "quantity": 20,
  "unit": "piece"
}
```

The application strictly parses the candidate, rejects unknown fields, validates product, integer bounds, unit and versions, checks authorization, and asks for confirmation of the interpreted change. It then creates an immutable validated request. The LLM cannot grant approval, identity, or execution privileges through its output.

Compilation is a finite mapping: `minimumProduction(A,q)` adds $x_A\ge q$; B is analogous. `whatIfProcessingHours(delta)` computes $H'=H+\delta$ in an explicit baseline copy. `queryResult(field)` reads existing evidence for a selected scenario. Bound request size, numeric ranges, candidate counts, and solver budgets.

### 4.1 Technical implementation

Field registration, typed IR, constraint compilation, and Kotlin/Rust native modeling fragments are now in the [technical path](./integrating-llms-technical). This page retains business rules and complete mathematics; the technical page covers per-model binding, solve reports, parameterized plans, publication, and recovery.

## 5. Optimization and reproducible results

Finite integer enumeration independently verifies the following acceptance values. They are not presented as logs from an actual solver run:

| Scenario | $x_A$ | $x_B$ | Profit (CNY) | Material used / remaining (kg) | Hours used / remaining |
|---|---:|---:|---:|---|---|
| S0 | 30 | 20 | 1900 | 120 / 0 | 70 / 0 |
| S1 | 30 | 20 | 1900 | 120 / 0 | 70 / 0 |
| S2 | 21 | 26 | 1930 | 120 / 0 | 73 / 7 |
| S3 | — | — | — | Infeasible | Infeasible |

The S1 minimum does not change the optimum. S2 uses only three additional hours and increases profit by 30 CNY. This does not imply a constant marginal profit for every extra hour.

The optima also admit direct proofs. For S0/S1, $R=10U_M+10U_H\le1900$, attained at $(30,20)$. For S2, $R=(50U_M-10x_A)/3\le5800/3$. Integer production makes profit a multiple of 10, so $R\le1930$, attained at $(21,26)$. These are proofs for this small instance, not replacements for general solver reports.

If a user asks “Why is material unused?” in S1, correct the premise: all 120 kg are used. For “Why are seven hours unused?” in S2, report full material consumption and cite the re-optimization result. A binding constraint alone is not a causal proof or an integer-program shadow price.

### 5.1 Infeasibility and statuses

S3 yields a directly checkable contradiction:

$$
x_A\ge20\land x_B\ge30
\Rightarrow 2x_A+3x_B\ge130>120.
$$

This is an independently verified argument, not a solver-provided IIS. Suggest reconsidering minimum quantities or authorizing different resource assumptions, but do not delete hard constraints automatically. Raising hours to 80 would not resolve the material contradiction.

Record problem status, termination reason, and solution presence separately. A feasible incumbent without an optimality proof should retain available bounds and gap. A timeout without a solution is not proven infeasibility. Report unsupported diagnostics or missing evidence explicitly rather than manufacturing evidence from natural-language logs.

### 5.2 Evidence for explanations

The application can expose `scenarioId`, frozen input and rule versions, actual solve status, variable values, objective components, and constraint `lhs/rhs/slack`. Give evidence stable identifiers such as `S1/material.capacity`. Quantitative explanations cite these identifiers. Missing values must not become zero. Solver version, termination reason, and gap can only be populated from actual execution.

### 5.3 Separate candidates, runs, and official plans

S0–S3 are readable scenario IDs, not four published business versions. Introduce these application records:

| Record | Example | Lifecycle |
|---|---|---|
| Official plan | `plan-v1` references a selected S1 run | New versions require selection and approval |
| Iteration | `iteration-1` freezes the hash of `plan-v1` | Contains S2 and S3 candidates |
| Candidate | `candidate-S2`, `candidate-S3` | Immutable baseline and changes; no official version number |
| Run | `run-S2-1`, `run-S2-2` | One ID per attempt; retries retain old reports |
| Capability version | `compare-hours-v1` | A reusable operation from Section 6, not a production plan |

Confirming the meaning of “at least 20 A,” authorizing a simulation, adopting its result, and publishing a reusable capability are different decisions. They must not share one LLM-writable `approved` flag.

Run S2 and S3 against the same `plan-v1`. Before selecting S2, recheck baseline freshness, the selected run's candidate hash, feasibility, and approval. Use an expected version and idempotency key to create a unique `plan-v2`. S3 remains an infeasible historical candidate. A concurrent selection conflict requires renewed comparison, not two overwrites of the official plan. This describes plan registration, not production dispatch.

### 5.4 Impact reasoning precedes corrective candidates

S3 evidence already proves a minimum material requirement of 130 kg. The application can expose an impact result containing A/B, reason `MINIMUM_EXCEEDS_MATERIAL`, evidence references, unresolved questions, and allowed actions `ASK_CLARIFICATION/CREATE_WHAT_IF`. The LLM may explain and combine proposals; “we could increase material” must not become “material has already increased.”

Correct field spelling only when the mapping is unambiguous; clarify uncertain meaning. A timeout can lead to a new attempt within authorized budgets. Changing hard constraints requires a new candidate and renewed confirmation. Bound correction rounds, elapsed time, LLM calls, and solve attempts, then hand over to a human. Majority agreement among LLM responses is neither proof nor approval.

Retain structured candidates, context hashes, parent candidate/run IDs, correction reasons, and new reports. Deterministic replay does not require another LLM call. Regenerated explanations need not be textually identical; wording differences do not imply different mathematical results.

## 6. From interactive optimization to LLM as Runtime

### 6.1 Why one successful conversation is insufficient

The application can answer a question, but similar requests may repeatedly incur LLM calls, interpretation, and what-if orchestration. Users repeatedly ask for remaining resources, specify different minimum quantities, or compare different hours increases. Parameter values change while operation structure stays stable.

Extend the same example with a second loop:

```text
Natural language → candidate IR → validation → generic execution → run records
                                                   ↑                  ↓
                                              safe fallback     stable shapes
                                                   ↑                  ↓
                                         invalidation ← specialized capability
                                                                  ↑
                                                        verification/publication
                                                                  ↓
                                                     bind new parameters and execute
```

Compilation first means translating semantics into reusable execution plans, not machine code. The LLM helps express requests that previously required individual development; the application runtime decides which verified combinations to retain. Unknown formulas still need domain development. Natural language alone cannot create a correct new capability.

### 6.2 Three kinds of reusable artifacts

| Recurring request | Parameterized IR | Specialized artifact | Work still required |
|---|---|---|---|
| Read a run's remaining resources | Fixed dataset, fields, result mapping; parameter `runId` | Read-only query plan | Authorize and read the new snapshot, enforce budgets |
| Produce at least a quantity of a product | `MinimumProduction(product,q,date)` | Constraint template and verified compilation rule | Validate parameters, units, scope; bind this model's OSPF variables |
| Compare additional processing hours | `CompareHours(base,deltaList)` | Query–candidate–solve–compare workflow plan | Solve each candidate again, compare, and approve |

A query plan caches how to read data, not a query result. A constraint template caches how to build a constraint, not runtime variables across models. A workflow caches orchestration, not an optimal solution. A cache hit proves neither feasibility nor optimality for new inputs.

This example can query in-memory result snapshots and needs no database. A database extension must compile validated queries into parameterized adapter plans, not accept LLM-generated SQL. Generated code artifacts require additional performance evidence, isolation, and release governance; they are not prerequisites here.

### 6.3 Requests, shapes, and execution evidence

“At least 20 A” and “at least 25 A” are distinct requests sharing the shape `MinimumProduction(product:ProductId,q:Integer[piece])`. Product can also be a typed parameter if A and B share capability and authorization applicability rules. Do not erase algorithm placement, units, time semantics, or policy differences to increase reuse.

| Identifier | Contents | Boundary |
|---|---|---|
| `requestHash` | Canonical concrete request, parameters, baseline | Shape hashes cannot replace historical requests |
| `shapeHash` | Operation, fields, types, parameter positions, units, temporal and result semantics | Excludes sensitive values; does not imply identical results |
| `dependencyKey` | Catalog, policy, compiler, adapter, model-semantics versions | Not an input-data hash |
| `runId/resultHash` | One execution and its canonical result | Not a plan version |

Use deterministic structured serialization, not natural-language text or object `toString()`. Sort only nodes proven semantically commutative. Plans and metrics exclude raw parameter values; concrete-request audit data has separate access and retention policies. Empty results have an explicit canonical representation distinct from missing results.

Append execution path, shape, selected plan version if any, source snapshot, latency, row count, errors, result hash, and available costs. Requests not yet normalized may lack a shape; do not fabricate hits. Optimization runs also retain actual solver reports; metrics do not replace those reports.

### 6.4 Let execution evidence justify specialization

When real records show repeated comparisons, first separate time spent in the LLM, validation, model construction, and optimization. Cached orchestration may reduce the earlier stages; it does not establish faster integer optimization.

Promotion considers frequency, semantic stability, errors, human corrections, tail latency, build cost, and expected reuse. This estimate can guide the decision; it is not a measured result:

$$
N(C_{\mathrm{generic}}-C_{\mathrm{specialized}})
>C_{\mathrm{build}}+C_{\mathrm{verify}}+C_{\mathrm{maintain}}.
$$

$N$ is expected reuse, with costs measured consistently. A frequent but cheap operation may not deserve specialization. This tiny instance cannot establish production performance benefits. Limit plan counts, build queues, and total budgets so an expensive outlier cannot trigger unlimited compilation.

### 6.5 Verify and publish a specialized capability

The application registry stores immutable plan versions: `planId/version`, parameterized IR, parameter schema, result mapping, dependencies, digest, verification evidence, scope, publisher, and fallback reference. A possible lifecycle is:

```text
CANDIDATE → COMPILING → VERIFYING → ACTIVE
                 └────────┴──────→ FAILED
ACTIVE → STALE / REVOKED
```

Append state events. Rebuilding creates a new version instead of overwriting the stale one. Only verified versions can become ACTIVE through an atomic switch. Verification covers:

1. Queries: identical rows, ordering, nulls, pagination, authorization, and budget errors on frozen data.
2. Constraints: equivalent variable bindings, coefficients, relations, bounds, and domains, including boundary and unit counterexamples.
3. Workflows: equivalent candidates and comparisons under fixed node contracts, baseline, and parameters; no official writes in the sandbox.
4. Performance: measured time and resource costs for both paths against predeclared thresholds, not averages alone.

Start in shadow mode: return generic results while specialized execution supplies comparison evidence only. Enable narrowly after correctness, failure semantics, and benefits pass; monitor p95/p99, errors, and fallbacks. Finite tests do not prove equivalence for all inputs. Deterministic compilation over restricted syntax, declared applicability, and property tests jointly define the trust boundary.

### 6.6 Execute the next request and invalidate safely

For “compare 5, 10, and 15 additional hours,” the LLM may extract parameters, or the user may directly invoke the published structured operation. Revalidate authorization, parameters, and baseline, then look up an applicable plan using `shapeHash + dependencyKey`. A hit selects the specialized path; a miss selects the fully validated generic path. Asynchronous construction must not block the request.

Removing fields, changing unit semantics, upgrading policies, or incompatible compiler changes mark affected plans STALE. Revalidate dependencies and digests on restart to catch missed events. If the generic path also cannot interpret the new fields, return a capability gap rather than claiming successful fallback. Only compatible, non-revoked versions are eligible rollback targets.

If processing units change from hours to minutes, an old plan must not continue treating 10 as ten hours: require explicit conversion or a verified new version. A daily change in the value of $H$ is new input binding and need not invalidate a parameterized plan, but still requires solving again. Historical replay retains its original catalog and input snapshots.

### 6.7 Compose governed operations into a workflow

Register node contracts for batch hours comparisons, rather than exposing arbitrary function names:

| Node | Typed input → output | Boundary |
|---|---|---|
| ReadBaseline | Plan version → frozen snapshot | Authorized read-only access |
| CreateHoursCandidates | Snapshot, hours deltas → candidates | Independent copies, count limit |
| SolveCandidates | Validated candidates → reports | Solve budget, cancellation, retained failures |
| CompareResults | Baseline and candidate reports → comparison | Actual feasibility and KPI semantics |

Validate graph types, units, node versions, reachability, and budgets. Wait for required reports before comparison. Keep timeouts and no-solution rows as statuses, not zero-profit ranking entries. Sandbox optimization can be real, but approval, official-plan publication, and dispatch are not Agent-composable nodes.

An adaptation package for “hours comparisons under minimum-production requirements” can reference the workflow version, minimum template version, catalog dependencies, parameter bounds, and verification records. It reuses governed capabilities with new parameters without adding an endpoint for every wording. Package publication and plan adoption require separate approval. Changed dependencies invalidate joint verification; do not publish only half the required dependencies.

### 6.8 Understand the limits of capability evolution

| New request | Handling |
|---|---|
| Require at least 15 B instead | Validate parameters against existing fields and compilation rules |
| Query slack, then compare several hours increases | Compose registered query, solve, and comparison nodes |
| Introduce a yield curve dependent on quantity | Undeclared formula: return a capability gap for modeling and development |
| Ignore material capacity and dispatch immediately | Reject unsafe or unauthorized behavior, not a configuration opportunity |

LLM as Runtime therefore brings open intent into a restricted domain language and forms executable, reusable, revocable capabilities through use. One-off requests can stay temporary. Stable rules can become templates or adaptation packages; sufficiently verified common behavior can enter normally maintained product functionality. This completes the transition from software answering questions to software forming capabilities under control.

### 6.9 A trace that can be accepted step by step

This is an expected application acceptance trace, not an executed log. The same production model suffices; no order, equipment, or external business system is required.

| Step | Input and action | Recorded output or fact |
|---|---|---|
| 1. First request | Compare ten additional hours from S1 using `production-v1` | Candidate, frozen input, generic run, S2 comparison; official plan unchanged |
| 2. Repeated use | Different hours deltas, retaining minimum requirements | Distinct request hashes, the same applicable shape, independent runs; insufficient frequency means no promotion |
| 3. Capability candidate | Observations satisfy the predeclared promotion policy | `compare-hours-v1` parameter contract, dependencies, build job, and reason |
| 4. Verification | Execute both paths on identical historical inputs | Comparison evidence, failure cases, measured costs; a mismatch means FAILED |
| 5. Publication | Verification and publication authorization pass | Immutable capability version and ACTIVE event, not a new production plan |
| 6. Next invocation | Compare 5, 10, and 15 additional hours from S1 | Renew authorization, bind parameters, execute the published workflow; solve every candidate again |
| 7. Plan adoption | Select a successful candidate meeting adoption policy | Separate approval and official plan version; capability version unchanged |
| 8. Dependency change | Processing field's unit contract changes | Old capability becomes STALE; compatible generic path or capability error; asynchronously verify a new version |
| 9. Restart and replay | Restore registry and inspect historical runs | Revalidate dependencies for current execution; historical records retain original versions and results |

Step 1 without execution evidence for steps 3–9 is LLM-assisted what-if. Calling a manually authored template is parameterized capability reuse. The LLM as Runtime loop discussed here additionally uses execution facts to select, verify, publish, and invalidate capabilities. OSPF supplies general solve reports and mathematical evidence; the example application owns the registry and capability lifecycle.

## 7. Testing, reproduction, and extensions

These complete standard-library-only programs check the numerical fixtures; they do not replace OSPF modeling and solver integration. Save the selected block as `Main.kt` or `main.rs`. Successful assertions produce no output.

::: code-group

```kotlin [Kotlin]
data class Plan(val a: Int, val b: Int) {
    val material get() = 2 * a + 3 * b
    val hours get() = a + 2 * b
    val profit get() = 30 * a + 50 * b
}

// Independent oracle for this fixed 120 kg instance only.
fun oracle(minA: Int, minB: Int, hours: Int): Plan? {
    require(minA >= 0 && minB >= 0 && hours >= 0)
    return (0..60).flatMap { a -> (0..40).map { b -> Plan(a, b) } }
        .filter { it.a >= minA && it.b >= minB &&
            it.material <= 120 && it.hours <= hours }
        .maxByOrNull { it.profit }
}

fun main() {
    check(oracle(0, 0, 70) == Plan(30, 20))
    check(oracle(20, 0, 70) == Plan(30, 20))
    check(oracle(20, 0, 80) == Plan(21, 26))
    check(oracle(20, 30, 70) == null)
}
```

```rust [Rust]
#[derive(Debug, PartialEq, Eq)]
struct Plan { a: u32, b: u32 }

impl Plan {
    fn material(&self) -> u32 { 2 * self.a + 3 * self.b }
    fn hours(&self) -> u32 { self.a + 2 * self.b }
    fn profit(&self) -> u32 { 30 * self.a + 50 * self.b }
}

// Independent oracle for this fixed 120 kg instance only.
fn oracle(min_a: u32, min_b: u32, hours: u32) -> Option<Plan> {
    (0..=60)
        .flat_map(|a| (0..=40).map(move |b| Plan { a, b }))
        .filter(|p| p.a >= min_a && p.b >= min_b
            && p.material() <= 120 && p.hours() <= hours)
        .max_by_key(Plan::profit)
}

fn main() {
    assert_eq!(oracle(0, 0, 70), Some(Plan { a: 30, b: 20 }));
    assert_eq!(oracle(20, 0, 70), Some(Plan { a: 30, b: 20 }));
    assert_eq!(oracle(20, 0, 80), Some(Plan { a: 21, b: 26 }));
    assert_eq!(oracle(20, 30, 70), None);
}
```

:::


Deterministic tests do not call a live LLM. Replay fixed candidates to verify parsing, validation, and constraint mapping, then use a real solver to check results. Evaluate LLM understanding, clarification, and refusal behavior separately.

| Test | Expected outcome |
|---|---|
| Baseline and S1 | Feasible, optimal profit 1900 |
| S2 retaining the A minimum | Optimal profit 1930, not 2000 from accidentally dropping the minimum |
| S3 | Infeasible; preserve the source of the three contradictory constraints |
| Unknown product, negative/fractional pieces, wrong unit | Reject before modeling |
| Stale baseline, unauthorized change, forged approval | Reject before execution |
| Duplicate request | Idempotent handling; do not add hours twice |
| Simulated timeout without a solution | Do not describe as proven infeasibility |
| Template versus generic path | Equivalent constraints and objective semantics |

### 7.1 Additional acceptance for LLM as Runtime

These are application-runtime acceptance requirements, not claims that this tutorial implements a registry, database, or publishing service:

| Scenario | Required behavior |
|---|---|
| Minimum changes from 20 to 25 | Reusable shape, distinct concrete request hashes, renewed parameter validation |
| Read-only query changes to a minimum constraint | Different execution shapes |
| Incorrect specialized shadow result | No publication; retain generic response and difference evidence |
| Unit or authorization policy changes | Invalidate before execution; cache hits cannot bypass checks |
| Restart or missed invalidation event | Revalidate dependencies and digest instead of blindly restoring ACTIVE |
| Concurrent publishing workers | Expected version/CAS permits one effective publication |
| Same parameters, different historical inputs | No incorrect result reuse; read the corresponding snapshot and solve again |
| Build failure or revoked plan | Compatible generic execution or explicit capability error |
| Candidate selection versus template publication | Separate authorization and versions; no implicit coupling |
| Sandbox workflow | Optimization may run, but no official-plan or external writes |

Measure candidate validity, evidence coverage, unapproved hard-constraint changes, hits and fallbacks, generic/specialized p95/p99, build cost, and LLM cost. Passing numerical fixtures proves the small model's results only. Claiming a runtime capability loop additionally requires execution evidence for registry persistence, state transitions, real path comparisons, invalidation, and recovery.

Enumerate integer points with $0\le x_A\le60$ and $0\le x_B\le40$, retain those satisfying every constraint, and maximize profit to obtain an independent oracle. Compare feasibility, intermediate values, and optimal objective across languages. For general problems with multiple optima, identical variable vectors are not required.

A live LLM adapter only needs an “authorized context and request → candidate” interface. Read credentials from the runtime environment and set timeout and retry budgets. No particular vendor, SDK, or tool protocol is required. Fixed-candidate replay can verify the deterministic workflow first.

This tutorial does not implement production dispatch, long-term memory, or automatic learning of objective weights. For further organization, see [Use Domain Driven Design Architecture](./use-ddd-architecture). For translation correctness, see [Deductive Logic Expression](./deductive-logic-expression) and [Formal Design and Formal Verification](./formal-design-and-formal-verification).
