# Choose a Framework by Business Scenario

Business frameworks combine reusable domain models, constraints, candidate generation and solving workflows. You supply business data, choose objectives and connect a solver instead of rebuilding every decision variable and constraint.

This path runs alongside [modeling with Core](/guide/getting-started): reuse a suitable framework first; compose contexts or build a model when your requirements exceed its scope. DDD and column-generation theory are not prerequisites for the first lesson.

## Which Framework Fits My Problem?

| Your decision | Framework | First lesson |
| --- | --- | --- |
| Pack existing long items into length-limited containers | [1D packing · BPP1D](./bpp1d/) | [Items, spacing and container inventory](./bpp1d/first-packing) |
| Place existing 2D items on finite sheets | [2D packing · BPP2D](./bpp2d/) | [Rectangle layouts and validation](./bpp2d/first-layout) |
| Load cargo into boxes or shipping containers | [3D packing · BPP3D](./bpp3d/) | [Start with cuboid cargo](./bpp3d/first-loading) |
| Slit material to satisfy orders of different widths | [1D cutting · CSP1D](./csp1d/) | [Product demand and cutting plans](./csp1d/first-slitting) |
| Cut rectangular products from sheets or coils | [2D cutting · CSP2D](./csp2d/) | [Materials, patterns and production counts](./csp2d/first-cutting) |
| Schedule activities jointly using time and resources | [Time and resources · Gantt](./gantt/) | [Finite-candidate activity scheduling](./gantt/first-schedule) |
| Configure supply, flow, paths and network construction | [Network optimization](./network/) | [Start with a small flow problem](./network/first-flow) |

## Two Important Distinctions

### Packing Versus Cutting: Business Semantics, Not Just Dimensions

BPP emphasizes the placement of existing items; CSP emphasizes material processing, product demand and production contributions. BPP2D also has spacing, kerf and remnants, while fixed-sheet CSP2D also has a packing structure. The presence of kerf alone does not decide the framework.

Prefer CSP when you need to explain how often a pattern runs, how much it produces, whether overproduction is allowed and whether the cutting process is manufacturable. Prefer BPP when your main question is which sheet or box receives each item, at what position and orientation. Then check the detailed capability boundaries.

### Gantt Versus Network: Compose When Necessary

Gantt owns global activities, resources, dependencies and calendars. Network owns topology, flows, resource paths and structural decisions. A maintenance schedule may combine Gantt with a Network routing subproblem; a basic activity schedule needs no Network dependency. Vehicle routing is an advanced Network entry point, not a prerequisite for learning network flow.

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
| BPP1D | [Container types, stock and cost](./bpp1d/business-scenario) | Why do minimum count and minimum cost choose different types? |
| BPP2D | [Sheet types, rotation and spacing](./bpp2d/business-scenario) | Why can adequate area still fail to fit? |
| BPP3D | [Bin types and objectives](./bpp3d/business-scenario) | How do volume defaults interact with business cost? |
| CSP1D | [Demand units and business rules](./csp1d/business-scenario) | How do rolls, mass and plan usage relate? |
| CSP2D | [Kerf, cutting process and coils](./csp2d/business-scenario) | Why is geometric fit insufficient for manufacturing? |
| Gantt | [Calendars and rolling recovery](./gantt/business-scenario) | How do facts, frozen choices and adjustable baselines differ? |
| Network | [Shared resources, shortage and embedding](./network/business-scenario) | Why can adequate arc capacity still fail to deliver? |

Second lessons distinguish existing source contracts from exercises you implement. Hand calculations are not native execution logs: use them to design acceptance, then inspect actual solver scope.

## What Counts as Understanding the Result?

Success is not just a successful function return. Explain the quantities and units, hard rules, objective, demand satisfaction and scope of the evidence.

| Observation | Supported conclusion | Unsupported shortcut |
| --- | --- | --- |
| Input validation or model registration succeeds | Those data/assembly checks passed | The problem was solved |
| A mock returns a configured vector | Orchestration and extraction work for that fixture | A native solver found the vector |
| An independent enumeration checks a tiny instance | The enumerated finite instance is checked | All sizes and continuous models are covered |
| Final MILP returns an integer solution | Inspect business validation and the report | Global optimality is automatically proved |
| A restricted candidate model is optimal | That candidate model is optimal | Missing candidates cannot improve it |

Kotlin and Rust share the business explanation, not an assumption of identical APIs or capabilities. Each lesson identifies actual entry points and verification boundaries; numerical assertions belong to the specific language fixture.
