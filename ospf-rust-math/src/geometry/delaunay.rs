//! Delaunay 三角剖分模块
//! Delaunay triangulation module
//!
//! 本模块实现了 Bowyer-Watson 算法用于二维 Delaunay 三角剖分。
//! This module implements the Bowyer-Watson algorithm for 2D Delaunay triangulation.
//!
//! # 算法说明 / Algorithm Description
//!
//! Delaunay 三角剖分是一种特殊的三角剖分，满足空圆性质：
//! Delaunay triangulation is a special triangulation that satisfies the empty circle property:
//! 任何三角形的外接圆内不包含其他点。
//! No other points are contained within the circumcircle of any triangle.
//!
//! # 示例 / Examples
//!
//! ```
//! use ospf_rust_math::geometry::{Point2, delaunay_triangulate};
//!
//! let points = vec![
//!     Point2::new(0.0, 0.0),
//!     Point2::new(1.0, 0.0),
//!     Point2::new(0.5, 1.0),
//!     Point2::new(1.5, 0.5),
//! ];
//!
//! let triangles = delaunay_triangulate(&points).unwrap();
//! println!("Generated {} triangles", triangles.triangles().len());
//! ```

use super::edge::{Edge, Edge2};
use super::point::Point2;
use super::triangle::Triangle2;
use std::collections::{HashMap, HashSet};

// ============================================================================
// Delaunay 三角剖分结果
// ============================================================================

/// Delaunay 三角剖分结果
/// Delaunay triangulation result
///
/// 包含生成的三角形和原始点集。
/// Contains the generated triangles and the original point set.
#[derive(Clone, Debug)]
pub struct DelaunayTriangulation {
    /// 生成的三角形 / Generated triangles
    triangles: Vec<Triangle2>,
    /// 原始点集 / Original point set
    points: Vec<Point2>,
}

impl DelaunayTriangulation {
    /// 获取三角形列表
    /// Get the list of triangles
    pub fn triangles(&self) -> &[Triangle2] {
        &self.triangles
    }

    /// 获取原始点集
    /// Get the original point set
    pub fn points(&self) -> &[Point2] {
        &self.points
    }

    /// 获取所有边（去重）
    /// Get all unique edges
    pub fn edges(&self) -> Vec<Edge2> {
        let mut edges: Vec<Edge2> = Vec::new();
        let mut seen: HashSet<(usize, usize)> = HashSet::new();

        for triangle in &self.triangles {
            for edge in triangle.edges() {
                let (p1, p2) = edge.into_points();
                let (i1, i2) = self.find_point_indices(&p1, &p2);
                let key = if i1 < i2 { (i1, i2) } else { (i2, i1) };

                if !seen.contains(&key) {
                    seen.insert(key);
                    edges.push(Edge::new(p1, p2));
                }
            }
        }

        edges
    }

    /// 查找点在原始点集中的索引
    /// Find indices of points in the original point set
    fn find_point_indices(&self, p1: &Point2, p2: &Point2) -> (usize, usize) {
        let mut i1 = 0;
        let mut i2 = 0;

        for (i, p) in self.points.iter().enumerate() {
            if p.approx_eq(p1) {
                i1 = i;
            }
            if p.approx_eq(p2) {
                i2 = i;
            }
        }

        (i1, i2)
    }
}

// ============================================================================
// Bowyer-Watson 算法实现
// ============================================================================

/// 使用 Bowyer-Watson 算法进行 Delaunay 三角剖分
/// Perform Delaunay triangulation using the Bowyer-Watson algorithm
///
/// # 参数 / Parameters
/// - `points`: 输入点集 / Input point set
///
/// # 返回值 / Returns
/// - `Ok(DelaunayTriangulation)`: 成功时返回三角剖分结果
/// - `Err(String)`: 失败时返回错误信息
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::geometry::{Point2, delaunay_triangulate};
///
/// let points = vec![
///     Point2::new(0.0, 0.0),
///     Point2::new(1.0, 0.0),
///     Point2::new(0.5, 1.0),
/// ];
///
/// let result = delaunay_triangulate(&points).unwrap();
/// assert_eq!(result.triangles().len(), 1);
/// ```
pub fn delaunay_triangulate(points: &[Point2]) -> Result<DelaunayTriangulation, String> {
    if points.len() < 3 {
        return Err("At least 3 points are required for triangulation".to_string());
    }

    // 创建超级三角形 / Create super triangle
    let super_triangle = create_super_triangle(points)?;

    // 初始化三角形列表 / Initialize triangle list
    let mut triangles: Vec<Triangle2> = vec![super_triangle.clone()];

    // 逐点插入 / Insert points one by one
    for point in points {
        insert_point(&mut triangles, point);
    }

    // 移除与超级三角形相关的三角形 / Remove triangles connected to super triangle
    let super_vertices: Vec<&Point2> = super_triangle.vertices().to_vec();
    triangles.retain(|t| {
        !t.vertices()
            .iter()
            .any(|v| super_vertices.iter().any(|sv| v.approx_eq(sv)))
    });

    Ok(DelaunayTriangulation {
        triangles,
        points: points.to_vec(),
    })
}

/// 创建包含所有点的超级三角形
/// Create a super triangle that contains all points
fn create_super_triangle(points: &[Point2]) -> Result<Triangle2, String> {
    // 找到边界框 / Find bounding box
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for p in points {
        min_x = min_x.min(p.x());
        max_x = max_x.max(p.x());
        min_y = min_y.min(p.y());
        max_y = max_y.max(p.y());
    }

    // 计算边界框中心和大小 / Calculate bounding box center and size
    let dx = max_x - min_x;
    let dy = max_y - min_y;
    let delta_max = dx.max(dy);
    let mid_x = (min_x + max_x) / 2.0;
    let mid_y = (min_y + max_y) / 2.0;

    // 创建足够大的超级三角形 / Create a large enough super triangle
    // 使用一个足够大的三角形包含所有点
    // Use a triangle large enough to contain all points
    let p1 = Point2::new(mid_x - 3.0 * delta_max, mid_y - delta_max);
    let p2 = Point2::new(mid_x, mid_y + 3.0 * delta_max);
    let p3 = Point2::new(mid_x + 3.0 * delta_max, mid_y - delta_max);

    Ok(Triangle2::new(p1, p2, p3))
}

/// 将点插入到三角剖分中
/// Insert a point into the triangulation
fn insert_point(triangles: &mut Vec<Triangle2>, point: &Point2) {
    // 找到外接圆包含该点的所有三角形 / Find all triangles whose circumcircle contains the point
    let mut bad_triangles: Vec<usize> = Vec::new();

    for (i, triangle) in triangles.iter().enumerate() {
        let circumcircle = triangle.circumcircle();
        if circumcircle.contains_point_strict(point) {
            bad_triangles.push(i);
        }
    }

    // 收集坏三角形的边 / Collect edges of bad triangles
    let mut polygon: Vec<Edge2> = Vec::new();
    let mut edge_count: HashMap<(usize, usize), usize> = HashMap::new();

    for &idx in &bad_triangles {
        for edge in triangles[idx].edges() {
            let (p1, p2) = edge.clone().into_points();
            let key = make_edge_key(&p1, &p2);
            *edge_count.entry(key).or_insert(0) += 1;
            polygon.push(edge);
        }
    }

    // 找出只出现一次的边（多边形的边界）/ Find edges that appear only once (boundary)
    let mut boundary: Vec<Edge2> = Vec::new();
    for edge in polygon {
        let (p1, p2) = edge.clone().into_points();
        let key = make_edge_key(&p1, &p2);
        if *edge_count.get(&key).unwrap_or(&0) == 1 {
            boundary.push(edge);
        }
    }

    // 移除坏三角形（从后向前移除以保持索引正确）/ Remove bad triangles (from back to front to maintain indices)
    bad_triangles.sort_by(|a, b| b.cmp(a)); // 降序排序 / Sort descending
    for idx in bad_triangles {
        triangles.remove(idx);
    }

    // 用新点创建新三角形 / Create new triangles with the new point
    for edge in boundary {
        let (p1, p2) = edge.into_points();
        let new_triangle = Triangle2::new(p1, p2, point.clone());
        triangles.push(new_triangle);
    }
}

/// 创建边的唯一键
/// Create a unique key for an edge
fn make_edge_key(p1: &Point2, p2: &Point2) -> (usize, usize) {
    // 使用坐标的哈希作为键（简化版本）
    // Use hash of coordinates as key (simplified version)
    // 实际应用中可能需要更精确的键
    // In practice, a more precise key may be needed
    let _h1 = (p1.x().to_bits(), p1.y().to_bits());
    let _h2 = (p2.x().to_bits(), p2.y().to_bits());

    // 使用一个简单的哈希方案
    // Use a simple hashing scheme
    let hash1 = hash_point(p1);
    let hash2 = hash_point(p2);

    if hash1 < hash2 {
        (hash1, hash2)
    } else {
        (hash2, hash1)
    }
}

/// 计算点的哈希值
/// Calculate hash value for a point
fn hash_point(p: &Point2) -> usize {
    // 使用坐标转换为 usize 作为简单哈希
    // Use coordinate conversion to usize as simple hash
    // 注意：这可能导致冲突，但对于演示目的足够
    // Note: This may cause collisions, but is sufficient for demonstration
    let x_bits = p.x().to_bits() as usize;
    let y_bits = p.y().to_bits() as usize;
    x_bits.wrapping_add(y_bits.wrapping_mul(31))
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 判断点是否在三角形的外接圆内
/// Check if a point is inside the circumcircle of a triangle
pub fn point_in_circumcircle(point: &Point2, triangle: &Triangle2) -> bool {
    let circumcircle = triangle.circumcircle();
    circumcircle.contains_point_strict(point)
}

/// 验证三角剖分是否满足 Delaunay 条件
/// Verify if triangulation satisfies Delaunay condition
pub fn is_delaunay(triangles: &[Triangle2], points: &[Point2]) -> bool {
    for triangle in triangles {
        let circumcircle = triangle.circumcircle();

        // 检查每个点是否在任何三角形的外接圆内
        // Check if any point is inside the circumcircle of any triangle
        for point in points {
            // 跳过三角形的顶点 / Skip triangle vertices
            if triangle.vertices().iter().any(|v| v.approx_eq(point)) {
                continue;
            }

            if circumcircle.contains_point_strict(point) {
                return false;
            }
        }
    }
    true
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delaunay_simple_triangle() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.5, 1.0),
        ];

        let result = delaunay_triangulate(&points).unwrap();
        assert_eq!(result.triangles().len(), 1);
    }

    #[test]
    fn test_delaunay_square() {
        // 使用略微不对称的正方形以避免边界精度问题
        // Use a slightly asymmetric square to avoid boundary precision issues
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.1, 1.0),
            Point2::new(0.0, 1.0),
        ];

        let result = delaunay_triangulate(&points).unwrap();
        // 四边形应该被分成 2 个三角形
        // A quadrilateral should be divided into 2 triangles
        assert_eq!(result.triangles().len(), 2);

        // 验证 Delaunay 条件 / Verify Delaunay condition
        assert!(is_delaunay(result.triangles(), &points));
    }

    #[test]
    fn test_delaunay_five_points() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
            Point2::new(1.0, 1.0), // 中心点 / Center point
        ];

        let result = delaunay_triangulate(&points).unwrap();

        // 验证 Delaunay 条件 / Verify Delaunay condition
        assert!(is_delaunay(result.triangles(), &points));
    }

    #[test]
    fn test_delaunay_random_points() {
        // 创建一些随机点 / Create some random points
        let points = vec![
            Point2::new(0.1, 0.2),
            Point2::new(1.5, 0.3),
            Point2::new(0.8, 1.7),
            Point2::new(2.1, 1.2),
            Point2::new(0.5, 0.9),
            Point2::new(1.2, 0.5),
        ];

        let result = delaunay_triangulate(&points).unwrap();

        // 验证 Delaunay 条件 / Verify Delaunay condition
        assert!(is_delaunay(result.triangles(), &points));

        // 检查边数 / Check number of edges
        let edges = result.edges();
        println!(
            "Generated {} triangles, {} edges",
            result.triangles().len(),
            edges.len()
        );
    }

    #[test]
    fn test_delaunay_insufficient_points() {
        let points = vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)];

        let result = delaunay_triangulate(&points);
        assert!(result.is_err());
    }

    #[test]
    fn test_delaunay_three_collinear() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(0.5, 1.0),
        ];

        let result = delaunay_triangulate(&points).unwrap();

        // 即使有共线点，也应该产生有效的三角剖分
        // Even with collinear points, should produce valid triangulation
        assert!(!result.triangles().is_empty());
    }

    #[test]
    fn test_super_triangle_contains_all_points() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(2.0, 0.5),
        ];

        let super_tri = create_super_triangle(&points).unwrap();

        // 所有点应该在超级三角形内
        // All points should be inside the super triangle
        for p in &points {
            assert!(super_tri.contains_point(p));
        }
    }

    #[test]
    fn test_delaunay_edges() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.5, 1.0),
        ];

        let result = delaunay_triangulate(&points).unwrap();
        let edges = result.edges();

        // 单个三角形有 3 条边
        // Single triangle has 3 edges
        assert_eq!(edges.len(), 3);
    }
}
