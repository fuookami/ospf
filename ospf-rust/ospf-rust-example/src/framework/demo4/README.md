# Framework Demo4 - Airline Crew Scheduling

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

demo4 demonstrates **airline crew scheduling** using the gantt_scheduling framework. It models flight recovery scenarios where aircraft must be assigned to flight tasks while respecting duty time limits, connection rules, and fleet balance constraints. The demo also includes a generic quantity sample showing how to use framework types for various scheduling dimensions.

## Scope

- Model aircraft with types, capacities, and cost rates.
- Define flight tasks, legs, and recovery scenarios.
- Generate flight task bunches (feasible duty sequences).
- Compile bunches into flight-linked schedules.
- Apply fleet balance and capacity constraints.
- Use generic quantities for time, cost, resource capacity, and switches.

## Domain Model

### task/ - Flight Tasks

| Struct | Description |
| --- | --- |
| FlightTaskImpl | Implements TaskTrait, contains time window, executor, status |
| Aircraft | Implements ExecutorTrait, contains type, capacity, cost rate |
| FlightLeg | Concrete flight task with departure/arrival airports |
| FlightTaskBunch | Task bunch (column in column generation) |
| Airport | Airport with ICAO code, type, transfer times |
| FlightLegPlan | Flight plan with scheduled/estimated/actual times |

### crew/ - Crew

| Struct | Description |
| --- | --- |
| Crew | Crew member composition |
| Pilot | Pilot with rank |
| CrewMan | Crew member with rank |

### passenger/ - Passengers

| Struct | Description |
| --- | --- |
| Passenger | Passenger and route |
| PassengerAmount | Passenger count symbol registration |
| PassengerCancel | Passenger cancellation symbol registration |
| PassengerChange | Passenger change symbol registration (class_change + flight_change) |

### rule/ - Rules

| Struct | Description |
| --- | --- |
| FlowControl | Flow control restrictions |
| Link | Flight link (connection time) |
| Restriction | Business restrictions |

### bunch_generation/ - Bunch Generation (Core Algorithm)

| Struct | Description |
| --- | --- |
| Graph | Route graph (Root -> Task -> End) |
| Node | Graph node: Root, Task{task_id, time, index}, End |
| Edge | Directed edge between nodes |
| FlightTaskReverse | Reversible task pair management |
| RouteGraphGenerator | BFS route graph construction |
| FlightTaskBunchGenerator | Label Setting pricing algorithm |
| InitialFlightTaskBunchGenerator | Initial bunch generation |
| AggregationInitializer | Initialization orchestration |

### bunch_compilation/ - Bunch Compilation

| Struct | Description |
| --- | --- |
| Compilation | Compilation result (bunch_id, flights, aircraft_type, cost) |
| FleetBalance | Fleet balance with checkpoints and slack variables |
| FlightCapacity | Flight passenger/cargo capacity symbols |
| FlightLink | Flight link with connection time and slack variables |

### bunch_selection/ - Bunch Selection

| Struct | Description |
| --- | --- |
| BranchAndPriceAlgorithm | Branch-and-price solver with iteration limit and tolerance |

## Column Generation Flow

`
                    +---------------------------+
                    |    AggregationInitializer  |
                    |  1. Build FlightTaskReverse|
                    |  2. Build RouteGraph (BFS) |
                    |  3. Generate initial bunches|
                    +---------------------------+
                                |
                                v
+----------+    +-----------------------+    +------------------+
|  Master  |    |     Pricing           |    |  Compilation     |
|  Problem |<---|  (Label Setting)      |--->|  (Constraints)   |
|  (RMP)   |    |                       |    |                  |
+----------+    +-----------------------+    +------------------+
     |                   ^                         |
     | Shadow Prices     |                         |
     +-------------------+                         |
     |                                             |
     | Updated columns                              |
     +---------------------------------------------+
`

1. **Initialization** (AggregationInitializer):
   - Build FlightTaskReverse from reversible task pairs.
   - For each aircraft, generate RouteGraph via BFS from aircraft location.
   - Generate initial bunches via InitialFlightTaskBunchGenerator, ensuring locked tasks are covered.

2. **Pricing** (FlightTaskBunchGenerator):
   - Use Label Setting algorithm to traverse the route graph.
   - Extend labels along outgoing edges, accumulating task costs and shadow price deductions.
   - Apply dominance pruning: at the same end node, keep only labels that are not dominated in both time and reduced cost.
   - Output bunches with negative reduced cost.

3. **Master Problem** (RMP):
   - Solve the Restricted Master Problem with current columns.
   - Extract shadow prices from fleet balance and flight link constraints.

4. **Compilation** (BunchCompilationContext):
   - Register fleet balance constraints: at each airport, arrivals - departures = expected balance.
   - Register flight link constraints: connection time between consecutive flights >= minimum.
   - Register flight capacity constraints: passenger/cargo limits per flight.

5. **Iteration**:
   - Feed shadow prices back to pricing step.
   - Add new columns (bunches) with negative reduced cost.
   - Repeat until no improving columns remain, then perform final MILP solve.

### Boundaries Between Modules

| Module | Owns | Does Not Own |
| --- | --- | --- |
| unch_generation | Route graph, initial bunches, pricing | Master constraints, fleet balance, solution extraction |
| unch_compilation | Master constraint registration, fleet balance, flight links | Label Setting, route graph, reduced cost |
| unch_selection | Branch-and-price orchestration, shadow price extraction, add columns | Specific pricing logic |

## Shadow Price and Reduced Cost

### Shadow Price

The **shadow price** (dual value) of a master problem constraint measures how much the objective would improve if the constraint were relaxed by one unit. In the column generation context:

- Each **fleet balance constraint** has a shadow price reflecting the marginal cost of requiring one more or one fewer aircraft at an airport.
- Each **flight link constraint** has a shadow price reflecting the marginal cost of the connection requirement.

Shadow prices are extracted from the RMP solution after each iteration and passed to the pricing sub-problem.

### Reduced Cost

The **reduced cost** of a candidate column (bunch) measures its potential to improve the current RMP solution:

`
reduced_cost = original_cost - sum(shadow_price_i * contribution_i)
`

Where:
- original_cost is the total operating cost of the bunch (fuel, crew, delays, etc.).
- contribution_i is how much this bunch contributes to constraint i (e.g., covers flight i, uses aircraft type i at airport j).
- shadow_price_i is the dual value of constraint i.

**Decision rule**:
- If 
educed_cost < 0, adding this column can improve the objective. The FlightTaskBunchGenerator outputs it.
- If 
educed_cost >= 0 for all candidate columns, the current solution is optimal and iteration stops.

### Dominance Pruning

During Label Setting, two labels ending at the same node are compared. Label A **dominates** label B if:
- A has visited at least as many tasks as B.
- A s reduced cost is less than or equal to Bs.
- Every task in Bs visited set is also in As.

Dominated labels are pruned to avoid redundant exploration.

## Generic Quantity Sample

The pp.rs entry point demonstrates how to use framework generic types:

| Type | Description |
| --- | --- |
| TimeRange | Time interval with start/end instants |
| Cost | Cost quantity |
| TimeWindow | Time window with duration and instant value conversions |
| WorkingCalendar | Working calendar with actual time range queries |
| FlightHour / FlightCycle | Flight hours and flight cycles |

## Usage

`powershell
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo4
`

## Testing

`powershell
cargo test -p ospf-rust-example --features backend-gurobi -- bunch_generation
`

Covered scenarios:
- Graph: node/edge operations, path queries
- FlightTaskReverse: reversible pairs, symmetry detection
- RouteGraphGenerator: route graph construction, feasibility checks, order-change edges
- FlightTaskBunchGenerator: negative reduced cost column generation, no feasible columns, dominance pruning
- InitialFlightTaskBunchGenerator: locked task coverage, empty bunch

## Related Modules

- [OSPF Rust Example README](../../README.md)
- [OSPF Rust Root README](../../../README.md)
