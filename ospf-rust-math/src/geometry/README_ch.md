# 几何模块

:us: English | :cn: [简体中文](README_ch.md)

本模块提供几何实体和计算几何算法。

## 特性

- **泛型类型**：使用 `const generics` 支持编译期维度，确保类型安全和性能优化
- **多种距离度量**：欧几里得、曼哈顿、切比雪夫、闵可夫斯基距离
- **Delaunay 三角剖分**：Bowyer-Watson 算法实现

## 模块结构

```
geometry/
├── mod.rs           # 模块导出和文档
├── distance.rs      # 距离度量 trait 和实现
├── point.rs         # 泛型点实体
├── vector.rs        # 泛型向量实体
├── edge.rs          # 边（线段）实体
├── triangle.rs      # 三角形实体
├── quadrilateral.rs # 四边形实体
├── circle.rs        # 圆/球体实体
└── delaunay.rs      # Delaunay 三角剖分算法
```

## 核心类型

### Point

泛型点实体，支持任意维度：

```rust
use ospf_rust_math::geometry::{Point, Point2, Point3};

// 创建 2D 点
let p2 = Point2::new(1.0, 2.0);
assert_eq!(p2.dim(), 2);

// 创建 3D 点
let p3 = Point3::new(1.0, 2.0, 3.0);
assert_eq!(p3.dim(), 3);

// 距离计算
let p1 = Point2::new(0.0, 0.0);
let p2 = Point2::new(3.0, 4.0);
assert!((p1.distance(&p2) - 5.0).abs() < 1e-10);
```

### Vector

泛型向量实体，支持任意维度：

```rust
use ospf_rust_math::geometry::{Vector2, Vector3};

let v = Vector2::new(3.0, 4.0);
assert!((v.norm() - 5.0).abs() < 1e-10);

// 归一化
let unit = v.normalize().unwrap();
assert!((unit.norm() - 1.0).abs() < 1e-10);

// 2D 叉积（返回标量）
let v1 = Vector2::new(1.0, 0.0);
let v2 = Vector2::new(0.0, 1.0);
assert!((v1.cross_2d(&v2) - 1.0).abs() < 1e-10);

// 3D 叉积
let v3d1 = Vector3::new(1.0, 0.0, 0.0);
let v3d2 = Vector3::new(0.0, 1.0, 0.0);
let cross = v3d1.cross(&v3d2);
assert!((cross.z() - 1.0).abs() < 1e-10);
```

### Edge

边（线段）实体，连接两个点：

```rust
use ospf_rust_math::geometry::{Point2, Edge2};

let p1 = Point2::new(0.0, 0.0);
let p2 = Point2::new(3.0, 4.0);
let edge = Edge2::new(p1, p2);

assert!((edge.length() - 5.0).abs() < 1e-10);

// 中点
let mid = edge.midpoint();
assert!((mid.x() - 1.5).abs() < 1e-10);
```

### Triangle

三角形实体：

```rust
use ospf_rust_math::geometry::{Point2, Triangle2};

let t = Triangle2::new(
    Point2::new(0.0, 0.0),
    Point2::new(3.0, 0.0),
    Point2::new(1.5, 2.0),
);

// 面积（Heron 公式）
assert!((t.area() - 3.0).abs() < 1e-10);

// 外接圆
let circle = t.circumcircle();
```

### Quadrilateral

四边形实体：

```rust
use ospf_rust_math::geometry::{Point2, Quadrilateral2};

let rect = Quadrilateral2::new(
    Point2::new(0.0, 0.0),
    Point2::new(4.0, 0.0),
    Point2::new(4.0, 3.0),
    Point2::new(0.0, 3.0),
);

// 面积（鞋带公式）
assert!((rect.area() - 12.0).abs() < 1e-10);

// 检查属性
assert!(rect.is_convex());
assert!(rect.is_rectangle(1e-10));
```

### Circle

圆（2D）/ 球体（3D）实体：

```rust
use ospf_rust_math::geometry::{Point2, Circle2};

let center = Point2::new(1.0, 2.0);
let circle = Circle2::new(center, 3.0);

// 面积
assert!((circle.area() - std::f64::consts::PI * 9.0).abs() < 1e-10);

// 包含点
let inside = Point2::new(2.0, 2.0);
assert!(circle.contains_point(&inside));
```

### Delaunay 三角剖分

使用 Bowyer-Watson 算法进行二维 Delaunay 三角剖分：

```rust
use ospf_rust_math::geometry::{Point2, delaunay_triangulate};

let points = vec![
    Point2::new(0.0, 0.0),
    Point2::new(1.0, 0.0),
    Point2::new(0.5, 1.0),
    Point2::new(1.5, 0.5),
];

let result = delaunay_triangulate(&points).unwrap();
println!("生成了 {} 个三角形", result.triangles().len());

// 获取所有唯一的边
let edges = result.edges();
```

## 距离度量

```rust
use ospf_rust_math::geometry::distance::{Distance, Euclidean, Manhattan, Chebyshev, Minkowski};

let p = [0.0_f64, 0.0];
let q = [3.0, 4.0];

// 欧几里得距离
assert!((Euclidean.distance(&p, &q) - 5.0).abs() < 1e-10);

// 曼哈顿距离
assert!((Manhattan.distance(&p, &q) - 7.0).abs() < 1e-10);

// 切比雪夫距离
assert!((Chebyshev.distance(&p, &q) - 4.0).abs() < 1e-10);

// 闵可夫斯基距离（p=2 时为欧几里得距离）
let minkowski = Minkowski::new(2.0);
assert!((minkowski.distance(&p, &q) - 5.0).abs() < 1e-10);
```

## 设计原则

1. **泛型优先**：使用 `const generics` 支持编译期维度，避免运行时开销
2. **类型安全**：不同维度的点/向量类型互不兼容
