# Example 2: Product assignment

## 1. Overview

This bounded context assigns each product to exactly one company while allowing each company to produce at most one product, minimising the resulting total cost.

### 1. Dependent Contexts

1. Core linear assignment context (binary variables, linear expressions, constraints, and solver adapter).

The Kotlin and Rust snippets are model-building fragments. The company/product data and cost mappings come from the linked Demo2 implementations.

---

## 2. Concepts / Entities

### 1. Company

A company is a producer that can receive at most one product in the current model.

**$Cost_{cp}$** : cost for company $c$ to produce product $p$, defined on the allowed assignment relation.

The current cost matrix (company rows, product columns) is:

|  | P1 | P2 | P3 | P4 |
| :---: | ---: | ---: | ---: | ---: |
| C1 | 920 | 480 | 650 | 340 |
| C2 | 870 | 510 | 700 | 350 |
| C3 | 880 | 500 | 720 | 400 |
| C4 | 930 | 490 | 680 | 410 |

### 2. Product

A product is a demand item that must be assigned to one company.

**$ProductID_p$** : identity of product $p$; the current data contain P1 through P4.

---

## 3. Variables

### 1. Decision Variables

**$x_{cp}$** : assignment variable, dimensionless binary, domain $\{0,1\}$, equals $1$ when company $c$ produces product $p$, $\forall(c,p)\in A$.

### 2. Auxiliary Variables

None. The company and product assignment counts are registered linear intermediate values.

---

## 4. Predicates

### 1. Assignment Relation

> Predicates classify entity pairs and their assignment state.

**CostDefined**$(c,p)$ : a cost entry exists for the company-product pair $(c,p)$.

**Assigned**$(c,p)$ : product $p$ is assigned to company $c$, equivalently $x_{cp}=1$.

---

## 5. Sets

### 1. Company Category

**$C$** : universal set of companies.

### 2. Product Category

**$P$** : universal set of products.

### 3. Entity Pairs / Relations

**$A=C\times P$** : allowed company-product assignment pairs. For the current instance every pair is allowed.

**$A^{CostDefined}$** : subset satisfying CostDefined, $A^{CostDefined}=\{(c,p)\in A\mid Cost_{cp}\text{ is defined}\}$, the pairs that contribute a cost and a variable in the source model.

For the current matrix, $A^{CostDefined}=A$. If a future instance contains forbidden pairs, variables and sums must be restricted to $A^{CostDefined}$, matching the Kotlin map-filtering and the Rust rectangular-data assumption.

---

## 6. Intermediate Values

### 1. Total Assignment Cost

**Description**: Total assignment cost sums the cost of every selected company-product pair and is the quantity minimised by the model.

$$
Cost(x)=\sum_{(c,p)\in A}Cost_{cp}x_{cp}.
$$

### 2. Company Assignment Count

**Description**: The company assignment count records how many products are assigned to company $c$.

$$
Assignment^{Company}_c(x)=\sum_{p\in P:(c,p)\in A}x_{cp},\qquad \forall c\in C.
$$

### 3. Product Assignment Count

**Description**: The product assignment count records how many companies are assigned to product $p$.

$$
Assignment^{Product}_p(x)=\sum_{c\in C:(c,p)\in A}x_{cp},\qquad \forall p\in P.
$$

---

## 7. Assertions

### 1. Binary Assignment

**Description**: Every allowed pair is either selected or not selected; fractional assignment is not part of this model.

$$
\forall(c,p)\in A\; (x_{cp}=0\vee x_{cp}=1).
$$

### 2. Complete Current Cost Data

**Description**: The current four-by-four data table defines a cost for every allowed pair.

$$
\forall(c,p)\in A\; CostDefined(c,p).
$$

---

## 8. Constraints

> Both constraints are hard assignment rules.

### 1. At Most One Product per Company

**[每家公司至多一个产品]**: a company cannot receive more than one product assignment.

$$
s.t.\quad Assignment^{Company}_c(x)=\sum_{p\in P:(c,p)\in A}x_{cp}\le 1,\qquad \forall c\in C.
$$

### 2. Exactly One Company per Product

**[每个产品恰好一家企业]**: every product must be assigned to exactly one company.

$$
s.t.\quad Assignment^{Product}_p(x)=\sum_{c\in C:(c,p)\in A}x_{cp}=1,\qquad \forall p\in P.
$$

Because the current instance has four companies and four products, these two families force every company to receive exactly one product as well; that is a consequence of the current cardinalities, not a separate constraint.

---

## 9. Objective Function (if applicable)

**Description**: minimise the total cost of all selected company-product assignments.

$$
\min\; Cost(x)=\sum_{(c,p)\in A}Cost_{cp}x_{cp}.
$$

---

## 10. Algorithm References

No standalone algorithm document is referenced. This is a direct binary linear assignment model.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| Not applicable | — | — | No domain-specific algorithm is needed. |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| Company | $c\in C$ | Producer that can receive at most one product. |
| Product | $p\in P$ | Item that must be assigned to one company. |
| Assignment | $x_{cp}$ | Binary decision for company $c$ and product $p$. |
| Pair cost | $Cost_{cp}$ | Cost of assigning product $p$ to company $c$. |
| Company count | $Assignment^{Company}_c$ | Number of products assigned to company $c$. |
| Product count | $Assignment^{Product}_p$ | Number of companies assigned to product $p$. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Use a binary pair variable | Quantity allocation variable | The source uses BinVariable2; each product is assigned as a whole to one company. | Current implementation |
| Restrict expressions to defined cost pairs | Treat a missing cost as zero | Kotlin uses mapNotNull/let to omit undefined pairs; the current data define all pairs. | Current implementation |
| Keep the two implementation APIs separate | Copy Kotlin calls into Rust | Kotlin uses BinVariable2 and intermediate symbols; Rust uses VariableCombination2D, SymbolCombination, and MetaModel. | Current implementation |

### Minimal current implementation fragments

The following are non-standalone fragments extracted from Demo2. companies, products, flt64Converter, and model registration are supplied by the linked source; the snippets do not invent a common cross-language API.

::: code-group

```kotlin [Kotlin]
// Fragment from Demo2.initVariable/initSymbol/initConstraint.
// Data source: Demo2's private products, companies, and cost maps.
val x = BinVariable2("x", Shape2(companies.size, products.size))
val cost = LinearExpressionSymbol(
    flatSum(companies) { c ->
        products.mapNotNull { p -> c.cost[p]?.let { it * x[c, p] } }
    },
    name = "cost"
)
val assignmentCompany = LinearIntermediateSymbols(
    "assignment_company",
    Shape1(companies.size),
    Flt64
)
for (c in companies) {
    assignmentCompany[c].asMutable() +=
        sumVars(products) { p -> c.cost[p]?.let { x[c, p] } }
}
metaModel.add(x)
metaModel.add(cost)
metaModel.minimize(cost)
for (c in companies) {
    metaModel.addConstraint(assignmentCompany[c] leq 1)
}
```

```rust [Rust]
// Fragment from demo2.rs::TransportModel::register/add_constraints.
// Data source: build_companies/build_products; helper imports come from demo2.rs.
let x_shape = Shape::new([companies.len(), products.len()]);
let x_vars: VariableCombination2D<Binary> =
    VariableCombination2D::with_name_generator(x_shape.clone(), "x", |_index, vector| {
        format!("{}_{}", vector[0], vector[1])
    });
let x_idx = model.register_combination(&x_vars)?;
let cost = flat_map1_indexed(
    "cost",
    companies,
    |c, company| {
        let monomials: Vec<_> = products
            .iter()
            .enumerate()
            .map(|(p, _)| {
                ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    company.cost_of(p),
                    x_idx[&[c, p]],
                )
            })
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, company| company.name.clone(),
);
model.add_symbol_combination(&cost)?;
let assignment_company = flat_map1_indexed(
    "assign_company",
    companies,
    |c, _company| {
        let monomials: Vec<_> = products
            .iter()
            .enumerate()
            .map(|(p, _)| {
                ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    1.0,
                    x_idx[&[c, p]],
                )
            })
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, company| company.name.clone(),
);
model.add_symbol_combination(&assignment_company)?;
let assignment_product = flat_map1_indexed(
    "assign_product",
    products,
    |p, _product| {
        let monomials: Vec<_> = companies
            .iter()
            .enumerate()
            .map(|(c, _)| {
                ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    1.0,
                    x_idx[&[c, p]],
                )
            })
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    },
    |_, product| product.name.clone(),
);
model.add_symbol_combination(&assignment_product)?;
let mut cost_coeffs = Vec::new();
for c in 0..companies.len() {
    let poly = cost.symbol_polynomial(c);
    for monomial in poly.monomials() {
        cost_coeffs.push((monomial.var_index(), *monomial.coefficient()));
    }
}
let cost_input = LinearObjectiveInput::minimize("cost").terms(cost_coeffs.into_iter());
model.set_linear_objective_input(cost_input);
for c in 0..companies.len() {
    let coeffs = extract_coeffs(&transport.assignment_company[c]);
    model.add_linear_constraint(
        &coeffs,
        ConstraintRelation::LessEqual,
        1.0,
        &format!("company_{}", c),
    )?;
}
for p in 0..products.len() {
    let coeffs = extract_coeffs(&transport.assignment_product[p]);
    model.add_linear_constraint(
        &coeffs,
        ConstraintRelation::Equal,
        1.0,
        &format!("product_{}", p),
    )?;
}
```

:::

#### Source and verification

- [Rust counterpart: src/core/demo2.rs](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo2.rs)

The Rust counterpart uses the same mathematical model and data, but its Rust MetaModel, variable-combination, and symbol-combination APIs are independent of the Kotlin API.

- [Current implementation: Demo2.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo2.kt)
- [Core build-structure test: CoreDemoBuildOnlyStructureTest.kt](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)

The source is a binary assignment model, not a quantity-allocation model. The four products are indexed through Kotlin AutoIndexed; Rust uses explicit vector indices. The snippets are model-building excerpts rather than complete runnable programs.

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| 1.0 | Reorganised the example into the domain-model template and added quantified assignment constraints, assertions, and Kotlin/Rust fragments. | Make pair coverage and API boundaries explicit. |
