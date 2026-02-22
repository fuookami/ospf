# Complex Example 4: Flight Recovery Problem

## Problem Description

## Business Architecture

```mermaid
C4Context
  System_Boundary(application_layer, "Application Layer") {
    System(passenger_flight_application, "Passenger Flight Application")
    System(cargo_flight_application, "Cargo Flight Application")
  }
  System_Boundary(domain_layer, "Domain Layer") {
    System(passenger_context, "Passenger Context")
    System(scheduling_context, "Scheduling Context")
    System(cargo_context, "Cargo Context")
    System(task_context, "Task Context")
    System(rule_context, "Rule Context")
  }

  Rel(passenger_flight_application, passenger_context, "", "")
  Rel(cargo_flight_application, cargo_context, "", "")
  Rel(passenger_context, scheduling_context, "", "")
  Rel(cargo_context, scheduling_context, "", "")
  Rel(scheduling_context, task_context, "", "")
  Rel(scheduling_context, rule_context, "", "")

  UpdateLayoutConfig($c4ShapeInRow="5", $c4BoundaryInRow="1")
```

## Mathematical Model

### RMP (Restricted Master Problem)

#### Scheduling Context

#### Passenger Context

#### Cargo Context

### SP (Subproblem)

## Code Implementation

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo4)
