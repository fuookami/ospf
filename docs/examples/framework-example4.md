# Framework Example 4: Flight Recovery Branch-and-Price — Overview

[中文](/zh-cn/examples/framework-example4)

## 1. Overview

Demo4 is an architecture sample. `Application` is empty, while the generic quantity sample is executable; context pages describe implemented contracts without claiming that one production model is assembled.

## 2. Contexts and Dependencies

| Context | Responsibility | Dependency |
|---|---|---|
| Task and rule | Tasks, connections, and business rules | Input configuration |
| Crew, cargo, passenger | Resources and transport demand | Tasks and corresponding resource data |
| Bunch generation | Searches policy-compatible flight bunches | Task, rule, and resource contexts |
| Bunch compilation | Compiles columns into coverage, capacity, and objective expressions | Generated bunches and business pipelines |
| Bunch selection | Coordinates column selection and branch-and-price policy | Generation and compilation contracts |

## 3. Concepts, Sets, and Predicates

The vocabulary includes flight tasks, connections, crews, cargo, passengers, and flight bunches. Predicates express feasible transitions, resource compatibility, and column acceptance. Each context defines its own sets rather than merging all resources into an unowned global set.

## 4. Variables and Intermediate Values

Bunch compilation owns column usage and derives task-, aircraft-, and connection-related expressions from column coefficients. Generation and selection describe service and callback contracts without duplicating master variables. The context pages define precise notation and domains.

## 5. Assertions, Constraints, and Objectives

Compilation provides task, connection, aircraft, and related resource contracts. Passenger and other business pipelines add their rows and objectives when enabled. An empty top-level `Application` defines no complete global objective; these contracts must not be presented as an assembled end-to-end model.

## 6. Algorithms and Lifecycle

The architectural sequence is to build business policies, initialize dual information, generate bunches, compile the master, and continue according to reduced costs and branch policies. The generic-quantity example separately demonstrates quantity types and linear-symbol APIs.

## 7. Register → Construct → Solve → Analyze

Each child page explains its registrable symbols, pipelines, and callbacks. Kotlin's top-level `Application` is a placeholder, so this page describes context composition rather than an unavailable end-to-end solver call chain.

## 8. Source Entry Points

- [Kotlin Demo4 source](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo4)
- [Rust Demo4 source](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/framework/demo4)

## 9. Kotlin/Rust Comparison and Design Decisions

Both languages have framework-example directories, but matching names do not establish equal top-level completeness. The pages use Kotlin context contracts as the reference and distinguish architectural composition from an executed global model.

## 10. Context Model Pages

[Open the context index](framework-example4/domain-models) for task, rule, crew, cargo, passenger, bunch generation, bunch compilation, and bunch selection.

## 11. Change Log

| Version | Change | Reason |
|---|---|---|
| 1.1 | Aligned bilingual overviews, notation, and source entry points | Keep the overview consistent with its context models |
