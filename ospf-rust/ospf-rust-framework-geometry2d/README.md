# ospf-rust-framework-geometry2d

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

`ospf-rust-framework-geometry2d` is the shared 2D quantity geometry kernel of `ospf-rust`. It is extracted so
that the packing framework `ospf-rust-framework-bpp2d` and the cutting framework
`ospf-rust-framework-csp2d` reuse one quantity-aware 2D geometry instead of each maintaining its own copy.

The crate is solver independent: it depends only on `ospf-rust-base`, `ospf-rust-math` and
`ospf-rust-quantities`.

`Box2Geometry::new` and `of_rectangle` are pure constructors and accept runtime units
and native high-precision values without a `Float` requirement. The separate
`rectangle_overlapped` path uses native ordered addition, requires equal dimensions
and scales, and excludes touching boundaries. It returns `None` for incompatible
units/shapes or unordered values. Callers must validate finite positive extents
before comparison; other shape/containment APIs retain their existing bounds.

## Scope

| Area | Types |
| --- | --- |
| Quantity arithmetic | `quantity_plus_safe`, `quantity_minus_safe`, `quantity_multiply`, `quantity_ord_safe`/`quantity_ord_or_null`, `quantity_greater`/`quantity_less`/`quantity_greater_or_equal`/`quantity_less_or_equal`, `quantity_zero_of`, `quantity_of`, `quantity_scaled_by`, `quantity_sum`, `sum_of_quantity`, `sorted_by_quantity`, `sorted_by_descending_quantity`, `count_fitting`, `scaled_quantity`, `quantity_max_safe`/`quantity_min_safe`, `quantity_max_of`, `is_same_dimension` |
| Points and vectors | `QuantityPoint2`, `QuantityVector2`, `point2_of`, `vector2_of` |
| Shapes | `PackingShape2` (`Rectangle`/`Circle`), `RectanglePackingShape2`, `CirclePackingShape2`, `PackingShape2Type`, `Shape2Candidate`, `candidate_shapes`, `rectangle_shape2`, `circle_shape2` |
| Placement and containment | `Placement2`, `Box2Geometry` |
| Sheet geometry | `Sheet2Geometry`, `Sheet2Shape`, `sheet2_shape`, `origin_box` |
| Kerf | `inflated_by_shape`, `inflated_by_placement`, `pitch_of`, `zero_spacing_of` |
| Area and boundaries | `area_of`, `bounding_width`, `bounding_height`, `bounding_area`, `min_x`, `min_y`, `max_x`, `max_y`, `inside_sheet` |

Non-goals:

1. Business DTOs, request protocols, tenant context or formula languages.
2. Solver backends and their installation or licensing.
3. Formats and readers for external datasets; CSV schemas belong to the consuming framework crate.

## Area convention

`area` is the actually occupied area and `bounding_area` is the enclosing rectangle area:

1. Rectangle: `area = width × height`, `bounding_area = width × height`.
2. Circle: `area = πr²`, `bounding_area = 4r²`.

Objectives and KPI use `area`; grid generation uses the bounding size.

## Overlap semantics

Overlap is always **interior overlap**: tangency does **not** count as overlap. Edge-sharing rectangles,
mutually tangent circles and a circle tangent to a rectangle boundary are all non-overlapping.

This is the precondition for grid layouts: if tangency counted as overlap, no regular grid could ever be
produced. The underlying quantity library treats tangency as overlap, so `Box2Geometry::overlapped`
re-implements the dispatch with strict inequalities.

## Kerf

Kerf is expressed by **inflating the shape that takes part in the overlap test**:

1. `inflated_by_shape`: a rectangle grows by `spacing` in both width and height; a circle's diameter grows by
   `spacing`.
2. `pitch_of` returns the bounding size plus `spacing`, so adjacent pieces end up exactly `spacing` apart.
3. Geometry checks must inflate **both** sides: only when either side is inflated and the shapes still
   intersect is the real gap smaller than the kerf. Inflating one side only misses a narrow gap on the other
   side.

## Generic value boundary

1. Every type is generic over `V` and keeps the compile-time unit `U`.
2. Coordinates, sizes and areas use `Quantity<V, U>`; dimensionless values use a bare `V`.
3. Areas are produced by `quantity_multiply`, which composes dimensions by adding base powers, so a squared
   length is never computed as a cube.
4. Generic code must not call `to_f64()`; counting and scaling go through `count_fitting` and
   `scaled_quantity`.

## Consumers

| Framework | Usage |
| --- | --- |
| `ospf-rust-framework-bpp2d` | 2D packing of rectangular and circular items |
| `ospf-rust-framework-csp2d` | 2D cutting stock of rectangular items |

## Local verification

```powershell
$env:RUSTC_BOOTSTRAP = "1"
cargo test -p ospf-rust-framework-geometry2d
```

## Related documents

- [ospf-rust README](../README.md) lists every crate and its Kotlin boundary.
- [`tests/geometry2d_contract.rs`](tests/geometry2d_contract.rs) pins the area, overlap, kerf and containment
  contracts.
