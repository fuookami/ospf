# Demand Units and Business Rules

After [your first slitting plan](./first-slitting), specify what the customer orders before adding machines and compatibility rules. Demand labels, physical units, per-use contributions and plan usage must form one consistent chain.

## 1. Start with an Algorithm-Independent Acceptance Case

Suppose validated plan P produces two rolls of A per use, each weighing 10 kg. This is a business calculation, not an input protocol where the framework guesses the unit of a bare number:

| Order | P contribution per use | Required uses | Output |
| --- | --- | ---: | --- |
| A, six rolls | Two rolls | 3 | Six rolls |
| A, 60 kg | 20 kg | 3 | 60 kg |
| A, 30 kg | 20 kg | Not exactly satisfiable | Explicit overproduction or another plan required |

Do not silently use an indivisible plan 1.5 times. With modeled overproduction, two uses can deliver 40 kg; with exact production and no other plan, the problem is infeasible. The allowance belongs in the model, not just a changed order label.

## 2. Express Orders with Actual Demand Types

| Demand | Kotlin entry | Rust entry |
| --- | --- | --- |
| Legacy roll count | `ProductDemand.legacyRoll` | `ProductDemand::legacy_roll` |
| Legacy sheet count | `ProductDemand.legacySheet` | `ProductDemand::legacy_sheet` |
| Legacy weight | `ProductDemand.legacyWeight` | `ProductDemand::legacy_weight` |

These helpers express demand conventions; they do not invent product weight, length or unit-weight data. For quantity constructors, discrete/continuous behavior also follows the actual unit, not only `mode`.

Use the first-lesson input and solve entry and read contributions in the actual demand unit. Legacy mode labels do not override quantities or production-density data.

## 3. Establish Contributions Before Solving

1. Identify demands by stable product IDs, retaining width, optional length and weight sources.
2. Check `CuttingPlanDemandContribution` quantities against order units. A roll order still counts rolls when the product also has weight.
3. Use the language-specific contribution construction/calculation path rather than treating slice counts as kilograms. Verify the dimensions of width, length, areal density or per-piece mass.
4. Recompute each product's output as `Σ contribution × usage`, with separate surplus, shortage and allowed bounds.

Whether two measurement requirements for the same product must both hold is a business decision. An adapter must not overwrite an order merely because the product ID matches.

## 4. Put Machine Rules in the Right Layer

“Material M cannot run on machine X” can be checked per plan. “Machine X can process at most 100 m this shift” needs a sum across plans; checking each plan separately against 100 is insufficient.

Cross-plan rules belong in the relevant context/modeling pipelines. With column generation, inspect LP, dual/pricing and final participation too. Removing offending plans after solving does not preserve demand coverage.

## 5. Retain Three Assertion Groups

- Changing demand mode while retaining the numeric amount changes units and contribution calculations, not just a label on the same output.
- A prohibited material-machine pair cannot enter selection. Prohibiting all feasible pairs leaves no deliverable output.
- Tightened aggregate capacity is checked across all plan usages, not independently per selected plan.

Use the [integration guide](../integration) to distinguish normal no-solution outcomes, technical failure and native reports. Feasibility recovery or heuristic fallback retains its real status, not an automatic upgrade to optimality.

Repository regression commands live in [Verification](../verification). Complete the business input, call and result checks before using them.
