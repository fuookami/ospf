# Geometry Module

This module provides geometric entities and algorithms for computational geometry.

[中文文档](./README_ch.md)

## Features

- **Generic Types**: Uses `const generics` for compile-time dimension support, ensuring type safety and performance optimization
- **Multiple Distance Metrics**: Euclidean, Manhattan, Chebyshev, and Minkowski distances
- **Delaunay Triangulation**: Bowyer-Watson algorithm implementation

## Module Structure

```
geometry/
├── mod.rs           # Module exports and documentation
├── distance.rs      # Distance metrics trait and implementations
├── point.rs         # Generic point entity
├── vector.rs        # Generic vector entity
├── edge.rs          # Edge (line segment) entity
├── triangle.rs      # Triangle entity
├── quadrilateral.rs # Quadrilateral entity
├── circle.rs        # Circle/Sphere entity
└── delaunay.rs      # Delaunay triangulation algorithm
```

## Core Types

### Point

Generic point entity supporting arbitrary dimensions:

```rust
use ospf_rust_math::geometry::{Point, Point2, Point3};

// Create 2D point
let p2 = Point2::new(1.0, 2.0);
assert_eq!(p2.dim(), 2);

// Create 3D point
let p3 = Point3::new(1.0, 2.0, 3.0);
assert_eq!(p3.dim(), 3);

// Distance calculation
let p1 = Point2::new(0.0, 0.0);
let p2 = Point2::new(3.0, 4.0);
assert!((p1.distance(&p2) - 5.0).abs() < 1e-10);
```

### Vector

Generic vector entity supporting arbitrary dimensions:

```rust
use ospf_rust_math::geometry::{Vector2, Vector3};

let v = Vector2::new(3.0, 4.0);
assert!((v.norm() - 5.0).abs() < 1e-10);

// Normalize
let unit = v.normalize().unwrap();
assert!((unit.norm() - 1.0).abs() < 1e-10);

// 2D cross product (returns scalar)
let v1 = Vector2::new(1.0, 0.0);
let v2 = Vector2::new(0.0, 1.0);
assert!((v1.cross_2d(&v2) - 1.0).abs() < 1e-10);

// 3D cross product
let v3d1 = Vector3::new(1.0, 0.0, 0.0);
let v3d2 = Vector3::new(0.0, 1.0, 0.0);
let cross = v3d1.cross(&v3d2);
assert!((cross.z() - 1.0).abs() < 1e-10);
```

### Edge

Edge (line segment) entity connecting two points:

```rust
use ospf_rust_math::geometry::{Point2, Edge2};

let p1 = Point2::new(0.0, 0.0);
let p2 = Point2::new(3.0, 4.0);
let edge = Edge2::new(p1, p2);

assert!((edge.length() - 5.0).abs() < 1e-10);

// Midpoint
let mid = edge.midpoint();
assert!((mid.x() - 1.5).abs() < 1e-10);
```

### Triangle

Triangle entity with geometric operations:

```rust
use ospf_rust_math::geometry::{Point2, Triangle2};

let t = Triangle2::new(
    Point2::new(0.0, 0.0),
    Point2::new(3.0, 0.0),
    Point2::new(1.5, 2.0),
);

// Area (Heron's formula)
assert!((t.area() - 3.0).abs() < 1e-10);

// Circumcircle
let circle = t.circumcircle();
```

### Quadrilateral

Quadrilateral entity:

```rust
use ospf_rust_math::geometry::{Point2, Quadrilateral2};

let rect = Quadrilateral2::new(
    Point2::new(0.0, 0.0),
    Point2::new(4.0, 0.0),
    Point2::new(4.0, 3.0),
    Point2::new(0.0, 3.0),
);

// Area (Shoelace formula)
assert!((rect.area() - 12.0).abs() < 1e-10);

// Check properties
assert!(rect.is_convex());
assert!(rect.is_rectangle(1e-10));
```

### Circle

Circle (2D) / Sphere (3D) entity:

```rust
use ospf_rust_math::geometry::{Point2, Circle2};

let center = Point2::new(1.0, 2.0);
let circle = Circle2::new(center, 3.0);

// Area
assert!((circle.area() - std::f64::consts::PI * 9.0).abs() < 1e-10);

// Contains point
let inside = Point2::new(2.0, 2.0);
assert!(circle.contains_point(&inside));
```

### Delaunay Triangulation

Bowyer-Watson algorithm for 2D Delaunay triangulation:

```rust
use ospf_rust_math::geometry::{Point2, delaunay_triangulate};

let points = vec![
    Point2::new(0.0, 0.0),
    Point2::new(1.0, 0.0),
    Point2::new(0.5, 1.0),
    Point2::new(1.5, 0.5),
];

let result = delaunay_triangulate(&points).unwrap();
println!("Generated {} triangles", result.triangles().len());

// Get all unique edges
let edges = result.edges();
```

## Distance Metrics

```rust
use ospf_rust_math::geometry::distance::{Distance, Euclidean, Manhattan, Chebyshev, Minkowski};

let p = [0.0_f64, 0.0];
let q = [3.0, 4.0];

// Euclidean distance
assert!((Euclidean.distance(&p, &q) - 5.0).abs() < 1e-10);

// Manhattan distance
assert!((Manhattan.distance(&p, &q) - 7.0).abs() < 1e-10);

// Chebyshev distance
assert!((Chebyshev.distance(&p, &q) - 4.0).abs() < 1e-10);

// Minkowski distance (p=2 is Euclidean)
let minkowski = Minkowski::new(2.0);
assert!((minkowski.distance(&p, &q) - 5.0).abs() < 1e-10);
```

## Design Principles

1. **Generic First**: Uses `const generics` for compile-time dimension support, avoiding runtime overhead
2. **Type Safety**: Points/vectors of different dimensions are incompatible types
