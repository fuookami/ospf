# Using Domain-Driven Design (DDD) Architecture

Domain-driven design in OSPF is not the mechanical act of splitting one large model into several files. It organizes **domain knowledge, mathematical expressions, and software responsibilities** into bounded contexts. Each context owns its variables, named intermediate values, objectives, and constraints, and participates in application orchestration through stable semantic interfaces.

This approach is suitable for optimization systems that must evolve over time, combine multiple business concerns, or reuse the same domain knowledge across monolithic MILP, column-generation, and Benders solution paths. A one-off teaching model can still use a solver API or the script-style OSPF Core interface directly.

## 1. From a business problem to contexts

Start with relatively independent decision topics in the business language, then choose boundaries. Do not begin by creating packages for formula types or solver objects.

For example, a service-placement problem contains two dependent but nearly orthogonal contexts:

| Context | Question it answers | Semantics it publishes |
|---|---|---|
| Route | Which services are used and where are they placed? | Whether a node hosts a service; whether a service is active |
| Bandwidth | How does flow traverse the network and meet demand? | Link usage, inflow, outflow, and net outflow |

Bandwidth may depend on “node hosts a service” published by Route, but it should not read Route's internal array indices or solver handles. A later active-active, security, or reliability requirement can be added as another context while reusing the two existing interfaces.

A useful context boundary normally has all of these properties:

- names come from the ubiquitous business language rather than technical names such as `constraint_1` or `x_group`;
- ownership of a set of decision variables and derived values is explicit;
- its inputs, outputs, objectives, and constraints can be listed;
- external dependencies are expressed through domain objects or named intermediate values;
- it can be initialized, registered, and tested independently.

## 2. Layers and responsibilities

A typical application has three layers:

| Layer | Main responsibility | Must not own |
|---|---|---|
| Domain | Entities, value objects, aggregates, variables, intermediate values, rules, and pipelines | Solver selection and transport protocols |
| Application | Select a scenario and orchestrate `init → register → construct → solve → analyze` | Reimplement domain formulas |
| Infrastructure | Solver adapters, persistence, DTOs, remote calls, and runtime concerns | Domain boundaries |

Common building blocks in the domain layer are:

| Building block | Meaning |
|---|---|
| Context | Public entry point and lifecycle of a bounded context |
| Aggregation | Owns variables, intermediate values, and shared domain objects |
| Model | Represents entities, value objects, and local mathematical structures |
| Service | Holds domain rules or algorithms that do not belong to one entity |
| Pipeline | A named, composable, delayed unit of constraint registration |
| PipelineListGenerator | Assembles pipelines for a business mode or solution path |

## 3. Ownership of variables and intermediate values

In conventional solver code, a variable is usually created and owned by one solver model. That couples domain knowledge to one solve. Reusing it across contexts, constructing multiple models, or changing the solution path then requires new mappings.

The OSPF DDD pattern promotes variables and intermediate values to properties of domain objects and registers them with one or more meta-models later:

~~~kotlin
class RouteAggregation(
    val nodes: List<Node>,
    val services: List<Service>
) {
    lateinit var assignment: BinVariable2
    lateinit var nodeAssigned: LinearIntermediateSymbols1<Flt64>
    lateinit var serviceAssigned: LinearIntermediateSymbols1<Flt64>

    fun register(model: AbstractLinearMetaModel<Flt64>): Try {
        // Create or reuse domain symbols, then register them for this solve.
        return model.add(assignment)
            .andThen { model.add(nodeAssigned) }
            .andThen { model.add(serviceAssigned) }
    }
}
~~~

The important part is the ownership direction, not the exact type names:

$$
\text{Domain object} \longrightarrow
\{\text{variable},\ \text{intermediate value}\}
\longrightarrow \text{meta model}.
$$

The meta-model is the compilation and solving vehicle, not the sole owner of domain semantics.

## 4. Intermediate values are context interfaces

An intermediate value is a named, reusable symbolic expression. It should:

1. be semantically equivalent to its expanded anonymous polynomial or functional expression;
2. be usable like a variable in the expression grammar;
3. have a stable name and model-wide visibility during a domain-model lifecycle;
4. hide whether its implementation is a constant, variable, or composite expression.

For example, Route can publish:

$$
\operatorname{NodeAssigned}_i
= \sum_{s \in S} x_{is},
\qquad
\operatorname{ServiceAssigned}_s
= \sum_{i \in N} x_{is}.
$$

Bandwidth uses only the meaning of `NodeAssigned` when constraining flow:

$$
\operatorname{OutFlow}_i
\le
\operatorname{MaxOutBandwidth}_i
\operatorname{NodeAssigned}_i.
$$

The dependency is therefore:

$$
\text{Route context}
\xrightarrow{\text{NodeAssigned}}
\text{Bandwidth context}.
$$

The same intermediate value may also be polymorphic. For example, “estimated payload” can be derived from assignment decisions in full-load mode, a plan or estimate in predistribution mode, and recommendation variables in weight-recommendation mode. Downstream airworthiness and balance contexts depend only on the “estimated payload” semantics.

## 5. Expressing constraints as pipelines

Calling `addConstraint` directly creates side effects that are difficult to discover and compose. A pipeline makes the constraint a testable first-class object:

~~~kotlin
class NodeAssignmentLimit(
    private val aggregation: RouteAggregation
) : LinearPipeline<Flt64> {
    override fun invoke(model: AbstractLinearMetaModel<Flt64>): Try {
        for (node in aggregation.nodes.indices) {
            model.addConstraint(
                aggregation.nodeAssigned[node] leq 1,
                name = "node_assignment_$node"
            )
        }
        return ok
    }
}
~~~

A `PipelineListGenerator` chooses the rules for a scenario, the Context registers them, and the Application only selects the scenario. A new constraint can therefore remain inside its owning context instead of spreading into the main solve procedure.

## 6. Application lifecycle

Make six stages of a solve explicit:

| Stage | Input and output | Failure information |
|---|---|---|
| `init` | DTO → domain objects and contexts | Data, unit, index, or precondition error |
| `register` | Contexts → variables, values, and pipelines | Duplicate registration, missing dependency, or incompatible mode |
| `construct` | Symbolic model → solver-executable model | Unsupported expression or backend capability |
| `solve` | Executable model → status, objective, and values | Infeasible, unbounded, timeout, or solver failure |
| `analyze` | Raw values → domain solution | Missing value, tolerance, or mapping error |
| `diagnose` | Logs and metrics → explainable diagnosis | Rule name, context, and solve trajectory |

~~~kotlin
suspend fun invoke(request: RequestDTO): Ret<ResponseDTO> {
    routeContext.init(request)
    bandwidthContext.init(request, routeContext.aggregation)

    routeContext.register(metaModel)
    bandwidthContext.register(metaModel)

    metaModel.construct()
    val solution = solver.solve(metaModel)

    return analyze(solution)
}
~~~

The Application may compose contexts, but it should not bypass a Context to mutate its internal variables or recreate its constraints.

## 7. Documentation contract for a context

Every context's mathematical-model document should contain at least these sections. The complex-example subpages follow the same contract:

1. **Purpose and boundary**: what the context solves and deliberately excludes;
2. **Inputs and assumptions**: sets, indices, constants, units, and preconditions;
3. **Decision variables**: symbol, domain, shape, meaning, and owner;
4. **Intermediate values**: formula, meaning, publisher, and consumers;
5. **Objectives**: priority, direction, dimension, and business interpretation;
6. **Constraints**: name, formula, applicability, and boundary behavior;
7. **Context collaboration**: input/output protocols and dependency direction;
8. **Build and solve**: registration order, solution path, and fallback;
9. **Solution analysis**: reconstruction of domain results;
10. **Verification**: unit tests, integration tests, benchmark instances, and traceability.

Every documented variable, intermediate value, objective, and constraint should trace to an implementation element, and every public mathematical element in the implementation should have a corresponding document entry.

## 8. Test boundaries

DDD does not automatically make a model correct. Verify it at several levels:

- **Model/value-object tests**: unit conversion, indices, and derived properties;
- **Intermediate-value tests**: equivalence between the named expression and its manual expansion;
- **Pipeline tests**: constraint count, coefficients, names, boundaries, and activation conditions;
- **Context tests**: initialization, registration idempotency, and missing dependencies;
- **Application tests**: context composition, solution paths, and solution analysis;
- **Benchmark tests**: compare with a known optimum, bound, or manually verifiable instance;
- **Backend contract tests**: consistent statuses, tolerances, and result mappings across solvers.

## 9. Common mistakes

- **Contexts named after variables, constraints, and objectives**: that is a technical classification, not a domain boundary.
- **Reading internal arrays across contexts**: the consumer becomes coupled to the implementation; publish a named intermediate value or domain protocol instead.
- **Selecting a solver inside a Context**: infrastructure policy leaks into the domain.
- **Rewriting formulas in the Application**: domain rules acquire two sources of truth.
- **Splitting directories without splitting ownership**: a global object still controls every variable and rule, preventing independent tests and reuse.
- **Treating an intermediate value as a cache**: it is first a mathematical semantic interface; performance work must preserve its definition.

## 10. Further reading

- [DDD optimization components versus conventional development](https://github.com/fuookami/ospf-kotlin/blob/main/docs/ddd.md)
- [Complex Example 1: Service Placement](/examples/framework-example1)
- [DDD architecture with column generation](/guide/use-ddd-architecture-with-column-generation)
- [DDD architecture with Benders decomposition](/guide/use-ddd-architecture-with-benders)
- [Deductive logic expression in mathematical models](/guide/deductive-logic-expression)
