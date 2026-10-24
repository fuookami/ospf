# Choose a Framework by Business Scenario

Business frameworks combine reusable domain models, constraints, candidate generation and solving workflows. You supply business data, choose objectives and connect a solver instead of rebuilding every decision variable and constraint.

This path runs alongside [modeling with Core](/guide/getting-started): reuse a suitable framework first; combine business frameworks or build a custom model with Core when your requirements exceed its scope. DDD and column-generation theory are not prerequisites for the first lesson.

## Which Framework Fits My Problem?

| Your decision | Framework | First lesson |
| --- | --- | --- |
| Pack existing long items into length-limited containers (cut-to-length profile loading) | [1D packing · BPP1D](./bpp1d/) | [Your First 1D Packing: Items, spacing and container inventory](./bpp1d/first-packing) |
| Place existing 2D items on finite sheets (pallet loading) | [2D packing · BPP2D](./bpp2d/) | [Your First 2D Layout: Rectangle layouts and validation](./bpp2d/first-layout) |
| Load cargo into boxes or shipping containers (container consolidation) | [3D packing · BPP3D](./bpp3d/) | [Your First 3D Loading: Start with cuboid cargo](./bpp3d/first-loading) |
| Slit material to satisfy orders of different widths (steel/film slitting) | [1D cutting · CSP1D](./csp1d/) | [Your First Slitting Plan: Product demand and cutting plans](./csp1d/first-slitting) |
| Cut rectangular products from sheets or coils (sheet/glass cutting) | [2D cutting · CSP2D](./csp2d/) | [Your First Cutting Plan: Materials, patterns and production counts](./csp2d/first-cutting) |
| Schedule activities jointly using time and resources (APS machine scheduling) | [Time and resources · Gantt](./gantt/) | [Your First Activity Schedule: Finite-candidate activity scheduling](./gantt/first-schedule) |
| Configure supply, flow, paths and network construction (supply-chain allocation) | [Network optimization](./network/) | [Your First Network Flow: Start with a small flow problem](./network/first-flow) |

## Two Important Distinctions

### Packing Versus Cutting: Business Semantics, Not Just Dimensions

- **Choose CSP for pattern reuse, production quantities and overproduction rules:** focus on material processing, demand and manufacturing constraints.
- **Choose BPP for container assignment, coordinates and orientation:** focus on placing existing items.
- Check business semantics: BPP2D includes spacing, kerf and remnants; fixed-sheet CSP2D also has a packing structure. Kerf alone does not determine the choice.

### Gantt Versus Network: Compose When Necessary

- **Choose Gantt for activity timing and resource occupancy:** activities, dependencies and calendars.
- **Choose Network for flow, paths and network construction:** topology, resource paths and structural decisions.
- Combine when needed: a maintenance schedule can use a Network routing subproblem. Vehicle routing is an advanced Network topic.

## Recommended Learning Path

1. Read [environment and running examples](./setup). Add domain dependencies and choose a real backend or the documented default heuristic.
2. Select a framework and inspect its inputs, outputs and exclusions.
3. Use the first-lesson example function and launcher in a small application to construct input, call the framework and read results.
4. Make the business change at the end and update the corresponding acceptance checks.
5. Read [Integrate Your Business](./integration), then follow the framework's second lesson to replace demand, objectives and rules with your own data.
6. Continue to [integrated examples](/examples/framework-examples) or [DDD architecture](/guide/use-ddd-architecture) for cross-domain composition.

## What Comes After the First Run?

| Framework | Second lesson | Question to answer |
| --- | --- | --- |
| 1D packing · BPP1D | [Container types, stock and cost](./bpp1d/business-scenario) | Why do minimum count and minimum cost choose different types? |
| 2D packing · BPP2D | [Sheet types, rotation and spacing](./bpp2d/business-scenario) | Why can adequate area still fail to fit? |
| 3D packing · BPP3D | [Bin types and objectives](./bpp3d/business-scenario) | How do volume defaults interact with business cost? |
| 1D cutting · CSP1D | [Demand units and business rules](./csp1d/business-scenario) | How do rolls, mass and plan usage relate? |
| 2D cutting · CSP2D | [Kerf, cutting process and coils](./csp2d/business-scenario) | Why is geometric fit insufficient for manufacturing? |
| Time and resources · Gantt | [Calendars and rolling recovery](./gantt/business-scenario) | How do facts, frozen choices and adjustable baselines differ? |
| Network optimization | [Shared resources, shortage and embedding](./network/business-scenario) | Why can adequate arc capacity still fail to deliver? |

Second lessons provide scenario inputs and business-rule examples, and identify the parts your application must implement. Use hand calculations as business references and interpret the solving scope from the returned reports.

## What Counts as Understanding the Result?

Success is not just a successful function return. Explain the quantities and units, hard rules, objective, demand satisfaction and scope of the evidence.

| Observation | Supported conclusion | Unsupported shortcut |
| --- | --- | --- |
| Input validation or model registration succeeds | Those data/assembly checks passed | The problem was solved |
| A mock returns a configured vector | The solving workflow and result extraction work for this test data | A native solver found the vector |
| An independent enumeration checks a tiny instance | The enumerated finite instance is checked | All sizes and continuous models are covered |
| Final MILP returns an integer solution | Inspect business validation and the report | Global optimality is automatically proved |
| The current candidate pool (restricted column pool) is optimal | That candidate model is optimal | Missing candidates cannot improve it |

::: info Kotlin and Rust API Differences
Kotlin and Rust share business explanations, while APIs and implemented capabilities can differ. Each lesson identifies its entry points and result semantics; numerical checks apply to that language’s example.
:::

## Terminology and Reading Conventions {#terminology}

BPP1D uses **container/container type**, BPP2D **sheet/sheet type**, and BPP3D **bin/bin type**. A CSP1D **cutting plan** includes output per execution; a CSP2D **cutting pattern** describes the layout for one execution. API class, field and parameter names remain unchanged.

| Term | Meaning in these lessons |
| --- | --- |
| Candidate and column | A candidate is a selectable loading or cutting combination; a column typically represents one candidate through a variable and its coefficients |
| Master problem and restricted master problem (RMP) | The master chooses candidates and their usages; the restricted master considers only generated candidates, usually starting with a linear relaxation during column generation |
| Linear programming (LP) and mixed-integer linear programming (MILP) | LP allows continuous variables; MILP requires some or all variables to be integer. A linear relaxation temporarily allows fractional values to obtain bounds and guide candidate search |
| Column generation and pricing | Column generation adds candidates in rounds; pricing uses feedback from the master to search for candidates that improve the objective, not to set product prices. Algorithm guarantees depend on the entry point |
| Solver backend and adapter | The backend performs optimization, for example SCIP or Gurobi; an adapter translates framework models, settings and results to the backend interface |
| Model registration | Adding variables, constraints and objectives derived from domain objects to the optimization model |
| Heuristic solving | A rule-based search that generally does not prove full-problem optimality; business validation still applies |
| Current feasible solution (incumbent) | A feasible solution found so far, not necessarily optimal; validate it before business use |
| Candidate completeness (`complete`) | All candidates under the current rules have been enumerated; this alone does not prove integer optimality |
| Objective bound and optimality gap | Evidence of how far a solution is from the known objective bound; preserve unavailable values as unavailable |
| Result metrics (KPI) | Business statistics such as bin count, utilization, cost and unmet demand; fields vary by framework and language |
