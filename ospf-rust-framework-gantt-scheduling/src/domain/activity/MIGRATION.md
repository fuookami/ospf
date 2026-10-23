# Kotlin 3f15899 Migration

English | [简体中文](MIGRATION_ch.md)

Source: `3f15899e6398c878f641a7f48ccb2e5491bf4c9f`, relative to
`c66cd7fa1779848819ba25bf7fa34ba54733c3c9`. Later Kotlin changes are not included.

- Activity inputs, complete start domains, joint resource phases, four endpoint
  dependencies, presence, baselines, running/completed facts and extension rows live
  in Gantt `domain::activity`; there is no parallel modelling kernel or new crate.
- Signed event inventory supports independent materials, opening/storage bounds,
  future fixed receipts and candidate-relative flows. It is not BOM/MRP or substitution.
- Embedded and standalone applications share transactional registration and complete
  compilation/extraction. No activity column generation or continuous scheduling is added.
- Custom `SchedulingSolverValueAdapter` implementations must now implement fallible
  `try_into_value` and `try_from_value`; legacy methods remain for existing callers,
  but activity never invokes them. `preserves_f64_identity` defaults to false and an
  explicit true declaration is additionally checked against actual prepared coefficients.
- Input quantities and native results retain V. Non-f64 or potentially lossy models
  intentionally do not transfer compiled-model proofs, bounds or no-incumbent conclusions.
  Read the [API guide](README.md) before consuming report objectives or proof scope.
- Native SCIP fixtures cover flexible job-shop, joint calendars, running recovery,
  frozen infeasibility, material waiting and explicit cancellation. They do not skip
  themselves when a native runtime is missing. Verification logs and progress plans
  are local-only and are not shipped as source or historical acceptance numbers.

Out of scope: continuous-time exact optimization, preemption, partial orders,
sequence-dependent setup, automatic material substitution, activity pricing trees
and completion of older resource/produce registration placeholders.
