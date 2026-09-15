# Framework Example 2: Context Model Index

[中文](/zh-cn/examples/framework-example2/domain-models)

This index is the documentation boundary for the eleven bounded contexts under
`framework_demo/demo2`. Each linked page describes one context's domain-model
contract in the order defined by the domain-model template. The index is a
navigation aid; the context pages remain the place to review variables,
intermediates, assertions, constraints, and objectives.

## 1. Context map and navigation

The dependency direction is:

```text
aircraft → stowage → {mac, airworthiness_security, soft_security,
                      mac_optimization, express_effectiveness,
                      loading_effectiveness, redundancy,
                      recommended_weight_equalization, payload_maximization}
```

`aircraft` supplies aircraft, deck, fuel, position, and adjacency data.
`stowage` consumes that configuration and owns item-to-position assignment and
load expressions. The remaining contexts consume the shared stowage values for
their own balance, safety, effectiveness, or payload responsibilities.

## 2. Context model pages

| Context | Responsibility | Local model |
| --- | --- | --- |
| `aircraft` | Aircraft, deck, position, fuel, ULD, and adjacency data | [Aircraft](domain-aircraft/domain-model) |
| `stowage` | Item-position assignment, adjustment, load amount, weight, and core loading limits | [Stowage](domain-stowage/domain-model) |
| `mac` | Torque, CLIM, index, and MAC intermediate values | [Mean Aerodynamic Chord (MAC)](domain-mac/domain-model) |
| `airworthiness_security` | Density, cumulative/zone load, payload, total-weight, envelope, trim, and CLIM limits | [Airworthiness Security](domain-airworthiness_security/domain-model) |
| `soft_security` | Soft empty-position, door, divide-empty, and ballast preferences | [Soft Security](domain-soft_security/domain-model) |
| `mac_optimization` | Longitudinal/lateral balance and horizontal-stabilizer limits and objective | [MAC Optimization](domain-mac_optimization/domain-model) |
| `express_effectiveness` | Must-ship and item-priority ordering | [Express Effectiveness](domain-express_effectiveness/domain-model) |
| `loading_effectiveness` | Source/destination adjacency, loading order, reweigh, trailer, and sequence policies | [Loading Effectiveness](domain-loading_effectiveness/domain-model) |
| `redundancy` | Redundancy and experimental longitudinal-balance limits | [Redundancy](domain-redundancy/domain-model) |
| `recommended_weight_equalization` | Item order, priority appointments, and recommended-weight deviation | [Recommended Weight Equalization](domain-recommended_weight_equalization/domain-model) |
| `payload_maximization` | Maximum payload limit and payload objective | [Payload Maximization](domain-payload_maximization/domain-model) |

Every page provides a link to its Chinese or English mirror. The parent page
documents the exact mode-specific registration boundary.

## 3. Mode registration notes

| Mode | Contexts in the ordinary model | Boundary |
| --- | --- | --- |
| `LoadingOrder` | `aircraft` | Export-only configuration; no meta-model solve |
| `FullLoad` | `stowage`, `mac`, `airworthiness_security`, `soft_security`, `mac_optimization`, `express_effectiveness`, `loading_effectiveness` | Airworthiness may be moved to the Benders subproblem |
| `Predistribution` | FullLoad contexts plus `redundancy` | Loading-order pipeline is mode-gated |
| `WeightRecommendation` | `stowage`, `mac`, `airworthiness_security`, `express_effectiveness`, `recommended_weight_equalization`, `payload_maximization` | Recommendation deviation and payload objective are mode-specific |

The presence of a class or model file in a context does not by itself make a
constraint active. Only a pipeline returned by the selected mode's generator
enters that mode's model.
