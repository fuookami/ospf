# Example 10: Traveling Salesman Problem with MTZ ordering

## 1. Overview

This bounded context selects one closed tour through five cities, anchored at Beijing, minimizing directed travel distance while using Miller–Tucker–Zemlin (MTZ) order variables to eliminate subtours. The Kotlin and Rust pages describe the same data and mathematics but use independent model APIs.

The source city list is Shanghai, Hefei, Guangzhou, Chengdu, and Beijing. Its directed distance table is:

| From | Shanghai | Hefei | Guangzhou | Chengdu | Beijing |
| :---: | ---: | ---: | ---: | ---: | ---: |
| Shanghai | -- | 472 | 1520 | 2095 | 1244 |
| Hefei | 472 | -- | 1257 | 1615 | 1044 |
| Guangzhou | 1529 | 1257 | -- | 1954 | 2174 |
| Chengdu | 2095 | 1615 | 1954 | -- | 1854 |
| Beijing | 1244 | 1044 | 2174 | 1854 | -- |

The matrix is directed: Guangzhou-to-Shanghai is 1529, while the reverse entry is 1520.

### 1. Dependent Contexts

1. None. The route model is self-contained.

---

## 2. Concepts / Entities

### 1. City

A city is a node that must have one selected outgoing and one selected incoming tour arc.

**$C$**: the city category; the sample contains Shanghai, Hefei, Guangzhou, Chengdu, and Beijing.

**$b$**: the anchored beginning city, Beijing.

### 2. Directed Arc

A directed arc connects two distinct cities and has a source distance.

**$d_{ij}$**: distance from city $i$ to city $j$; the source table is directed, so $d_{ij}$ and $d_{ji}$ may differ.

---

## 3. Variables

### 1. Decision Variables

**$x_{ij}$**: binary arc-selection variable, dimensionless, domain ${0,1}$, indicating whether the tour uses $i\to j$, $\forall(i,j)\in A$ where $A=\{(i,j)\in C^2\mid i\ne j\}$.

**$u_i$**: integer MTZ order/potential variable, dimensionless, domain $[-|C|,|C|]\cap\mathbb{Z}$ for $i\in C\setminus\{b\}$; the source fixes $u_b=0$. It is not an isolated-subcycle indicator.

### 2. Auxiliary Variables

`Depart_i`, `Reached_i`, and `Distance` are registered intermediate/objective symbols derived from $x$. No separate binary subtour indicator is created.

---

## 4. Predicates

### 1. Arc Predicates

**`DistinctArc(i,j)`**: true when $i\ne j$; only these arcs can be selected.

**`NonAnchor(i)`**: true when $i\in C\setminus\{b\}$; MTZ pair constraints are generated only for non-anchor cities.

---

## 5. Sets

### 1. City Categories

**$C$**: universal set of cities.

**$C^N$**: non-anchor cities, $C^N=C\setminus\{b\}$.

**$A$**: directed non-self arcs, $A=\{(i,j)\in C\times C\mid i\ne j\}$.

### 2. Entity Pairs / Relations

**$TourArc$**: relation $A$ linking a departure city to an arrival city.

**$MTZPair$**: ordered pairs $(i,j)\in C^N\times C^N$ with $i\ne j$.

---

## 6. Intermediate Values

### 1. Departures per City

**Description**: Number of selected arcs leaving city $i$.

$$
Depart_i=\sum_{j:(i,j)\in A}x_{ij},\qquad \forall i\in C.
$$

### 2. Arrivals per City

**Description**: Number of selected arcs entering city $i$.

$$
Reached_i=\sum_{j:(j,i)\in A}x_{ji},\qquad \forall i\in C.
$$

### 3. Total Distance

**Description**: Directed distance of all selected arcs; diagonal entries are excluded in Kotlin and fixed to zero in Rust.

$$
Distance=\sum_{(i,j)\in A}d_{ij}x_{ij}.
$$

### 4. MTZ Left-Hand Side

**Description**: The expression used by each non-anchor city pair to prevent a selected cycle that does not contain Beijing.

$$
MTZ_{ij}=u_i-u_j+|C|x_{ij},\qquad \forall(i,j)\in MTZPair.
$$

---

## 7. Assertions

### 1. One-In/One-Out Tour Degree

**Description**: In every feasible solution, every city has exactly one selected departure and one selected arrival.

$$
\forall i\in C\;(Depart_i=1\wedge Reached_i=1).
$$

### 2. Selected-Arc Order Increase

**Description**: For a selected non-anchor arc, the MTZ inequality forces the destination's order to be at least one greater than the source's order.

$$
\forall(i,j)\in MTZPair\;(x_{ij}=1\Longrightarrow u_j\ge u_i+1).
$$

### 3. Single Anchored Tour

**Description**: Degree equations, the fixed anchor $u_b=0$, and MTZ pair inequalities exclude cycles that omit Beijing, leaving one Hamiltonian cycle.

$$
\text{Feasible tour}\Longrightarrow\text{one closed cycle containing every city and }b.
$$

### 4. Numeric Result Scope

**Description**: The source extracts selected arcs as a city-to-city map, while the current structure test checks construction only. This page therefore does not claim a particular tour orientation or distance value.

---

## 8. Constraints

### 1. No Self-Arc (禁止自环约束)

**Description**: A city cannot travel to itself. Kotlin fixes diagonal items to false without registering them; Rust registers the diagonal with a fixed-zero range.

$$
s.t. \quad x_{ii}=0,\qquad \forall i\in C.
$$

### 2. One Departure per City (每城一条出弧约束)

**Description**: Exactly one selected arc leaves every city.

$$
s.t. \quad Depart_i=1,\qquad \forall i\in C.
$$

### 3. One Arrival per City (每城一条入弧约束)

**Description**: Exactly one selected arc enters every city.

$$
s.t. \quad Reached_i=1,\qquad \forall i\in C.
$$

### 4. MTZ Subtour Elimination (MTZ 子环消除约束)

**Description**: A selected arc between two non-anchor cities must increase the order variable; an omitted arc activates the relaxed inequality.

$$
s.t. \quad u_i-u_j+|C|x_{ij}\le |C|-1,\qquad \forall(i,j)\in MTZPair.
$$

### 5. Order Domain and Anchor (顺序变量定义域与锚点约束)

**Description**: Non-anchor order variables use the source's explicit range and Beijing is fixed to zero.

$$
s.t. \quad -|C|\le u_i\le |C|,\quad u_i\in\mathbb{Z},\ \forall i\in C^N;\qquad u_b=0.
$$

---

## 9. Objective Function (if applicable)

**Description**: Minimize the directed distance of the selected closed tour.

$$
\min Distance.
$$

---

## 10. Algorithm References

MTZ is a standard subtour-elimination formulation referenced inline; no separate algorithm file is required in this example context.

| Algorithm Name | File Path | Referenced In | Brief Description |
|----------------|-----------|---------------|-------------------|
| Miller–Tucker–Zemlin (MTZ) | Inline Section 8.4 | Section 8.4 | Uses order potentials to exclude non-anchor subtours. |

---

## 11. Ubiquitous Language

| Term | Symbol | Definition |
|------|--------|------------|
| City | $i,j\in C$ | Tour node. |
| Tour arc | $x_{ij}$ | Binary decision to travel from $i$ to $j$. |
| Departure | $Depart_i$ | Number of selected outgoing arcs at city $i$. |
| Arrival | $Reached_i$ | Number of selected incoming arcs at city $i$. |
| Order potential | $u_i$ | MTZ variable used to eliminate subtours. |
| Anchor | $b$ | Beijing, fixed as the route's reference city. |

---

## 12. Design Decisions

| Decision | Alternatives | Rationale | Date |
|----------|--------------|-----------|------|
| Use directed distances | Symmetrize the table | The source has $d_{Guangzhou,Shanghai}=1529$ and the reverse value 1520 | 2026-09-08 |
| Use Beijing as an order anchor | Let all order variables float | The current source fixes the Beijing item to zero | 2026-09-08 |
| Keep the implementation difference visible | Claim identical registration details | Kotlin omits diagonal/free-anchor registrations; Rust registers fixed ranges | 2026-09-08 |

### Minimal current model-building snippets

The city list and distance table are taken from `Demo10.kt`/`demo10.rs`; these blocks show the variables, intermediate expressions, objective, and constraints, while source-owned data setup is omitted.

::: code-group
```kotlin [Kotlin]
// `cities`, `beginCity`, `distances`, and `flt64Converter` come from Demo10.kt.
val metaModel = LinearMetaModel<Flt64>("demo10", converter = flt64Converter)
val x = BinVariable2("x", Shape2(cities.size, cities.size))
for (city1 in cities) for (city2 in cities) {
    if (city1 != city2) metaModel.add(x[city1, city2])
    else x[city1, city2].range.eq(false)
}
val u = IntVariable1("u", Shape1(cities.size))
for (city in cities) {
    if (city.name == beginCity) {
        u[city].range.eq(Int64.zero)
    } else {
        u[city].range.set(ValueRange(
            Int64(-cities.size.toLong()), Int64(cities.size.toLong())
        ).value!!)
        metaModel.add(u[city])
    }
}
val distance = LinearExpressionSymbol(
    sum(cities.flatMap { i -> cities.mapNotNull { j ->
        if (i == j) null else distances[i to j]?.let { it * x[i, j] }
    }}), name = "distance"
)
val depart = LinearIntermediateSymbols1<Flt64>("depart", Shape1(cities.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[cities[i], _a]), name = "depart_$i")
}
val reached = LinearIntermediateSymbols1<Flt64>("reached", Shape1(cities.size)) { i, _ ->
    LinearExpressionSymbol(sum(x[_a, cities[i]]), name = "reached_$i")
}
metaModel.add(distance); metaModel.add(depart); metaModel.add(reached)
metaModel.minimize(distance, "distance")
for (city in cities) {
    metaModel.addConstraint(depart[city] eq Flt64.one)
    metaModel.addConstraint(reached[city] eq Flt64.one)
}
for (i in cities.filter { it.name != beginCity }) for (j in cities.filter { it.name != beginCity }) {
    if (i != j) metaModel.addConstraint(
        u[i] - u[j] + Flt64(cities.size.toDouble()) * x[i, j]
            leq Flt64((cities.size - 1).toDouble())
    )
}
```

```rust [Rust]
// `data` comes from TspData::sample() in demo10.rs.
let city_count = data.cities.len();
let mut model = MetaModel::<f64>::new("demo10");
let x_vars: VariableCombination2D<Binary> =
    VariableCombination2D::with_name_and_range_generator(
    Shape::new([city_count, city_count]), "x",
    |_index, vector| format!("{}_{}", vector[0], vector[1]),
    |_index, vector| if vector[0] == vector[1] {
        VariableRange::fixed(0.0)
    } else { VariableRange::bounded(0.0, 1.0) },
);
let x_idx = model.register_combination(&x_vars)?;
let u_vars: VariableCombination1D<Integer> =
    VariableCombination1D::with_name_and_range_generator(
    Shape::new([city_count]), "u", |_index, vector| vector[0].to_string(),
    |_index, vector| if vector[0] == data.begin_idx {
        VariableRange::fixed(0.0)
    } else { VariableRange::bounded(-(city_count as f64), city_count as f64) },
);
let u_idx = model.register_combination(&u_vars)?;
let distance = flat_map1_indexed("distance", &data.cities, |i, _| {
    let monomials = (0..city_count).map(|j|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(
            data.distances.get(i, j), x_idx[&[i, j]],
        )
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |_, city| city.name.clone());
model.add_symbol_combination(&distance)?;
let depart = flat_map1_indexed("depart", &data.cities, |i, _| {
    let monomials = (0..city_count).map(|j|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]])
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |_, city| city.name.clone());
let reached = flat_map1_indexed("reached", &data.cities, |j, _| {
    let monomials = (0..city_count).map(|i|
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]])
    ).collect();
    ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
}, |_, city| city.name.clone());
let mtz = flat_map2_indexed("mtz", &data.cities, &data.cities, |i, _, j, _| {
    ospf_rust_core::symbol::flatten::Linear::new(vec![
        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, u_idx[&[i]]),
        ospf_rust_core::symbol::flatten::LinearMonomial::new(-1.0, u_idx[&[j]]),
        ospf_rust_core::symbol::flatten::LinearMonomial::new(city_count as f64, x_idx[&[i, j]]),
    ], 0.0)
}, |_, city_i, _, city_j| format!("{}_{}", city_i.name, city_j.name));
model.add_symbol_combination(&depart)?;
model.add_symbol_combination(&reached)?;
model.add_symbol_combination(&mtz)?;
let mut dist_coeffs = Vec::new();
for i in 0..city_count {
    for monomial in distance.symbol_polynomial(i).monomials() {
        dist_coeffs.push((monomial.var_index(), *monomial.coefficient()));
    }
}
model.set_linear_objective_input(
    LinearObjectiveInput::minimize("distance").terms(dist_coeffs.into_iter())
);
for i in 0..city_count {
    model.add_linear_constraint(&extract_coeffs(&depart[i]), ConstraintRelation::Equal, 1.0,
        &format!("depart_{}", i))?;
    model.add_linear_constraint(&extract_coeffs(&reached[i]), ConstraintRelation::Equal, 1.0,
        &format!("arrive_{}", i))?;
}
for i in 0..city_count {
    if i == data.begin_idx { continue; }
    for j in 0..city_count {
        if j == data.begin_idx || i == j { continue; }
        model.add_linear_constraint(&extract_coeffs(&mtz[&[i, j]]),
            ConstraintRelation::LessEqual, city_count as f64 - 1.0,
            &format!("mtz_{}_{}", i, j))?;
    }
}
```
:::

---

## 13. Change Log

| Version | Change | Reason |
|---------|--------|--------|
| 2026-09-08 | Reorganized the page into the domain-model template; clarified MTZ semantics, directed data, quantified constraints, and Kotlin/Rust tabs | Match the current Demo10 implementations without asserting an untested numeric tour |

## Source and verification

- [Kotlin implementation: `Demo10.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo10.kt)
- [Rust counterpart: `demo10.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-example/src/core/demo10.rs)
- [Kotlin structure test: `CoreDemoBuildOnlyStructureTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/core_demo/CoreDemoBuildOnlyStructureTest.kt)
