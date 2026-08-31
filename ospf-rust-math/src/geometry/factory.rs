//! 几何便捷构造函数
//! Geometry convenience factory functions

use num_traits::Float;
use crate::algebra::Field;
use super::{Point2, Point3, Point4, Vector2, Vector3};

/// 创建 2D 点。
/// Create a 2D point.
pub fn point2<S: Field + Float>(x: S, y: S) -> Point2<S> {
    Point2::new(x, y)
}

/// 创建 3D 点。
/// Create a 3D point.
pub fn point3<S: Field + Float>(x: S, y: S, z: S) -> Point3<S> {
    Point3::new(x, y, z)
}

/// 创建 2D 向量。
/// Create a 2D vector.
pub fn vector2<S: Field + Float>(x: S, y: S) -> Vector2<S> {
    Vector2::new(x, y)
}

/// 创建 4D 点。
pub fn point4<S: Field + Float>(x: S, y: S, z: S, w: S) -> Point4<S> {
    Point4::new(x, y, z, w)
}

/// 创建 3D 向量。
/// Create a 3D vector.
pub fn vector3<S: Field + Float>(x: S, y: S, z: S) -> Vector3<S> {
    Vector3::new(x, y, z)
}

#[cfg(test)]
mod tests {
    use super::{point2, point3, vector2, vector3};

    #[test]
    fn point_factories_create_points() {
        let p2 = point2(1.0, 2.0);
        assert_eq!(p2.x(), 1.0);
        assert_eq!(p2.y(), 2.0);

        let p3 = point3(1.0, 2.0, 3.0);
        assert_eq!(p3.x(), 1.0);
        assert_eq!(p3.y(), 2.0);
        assert_eq!(p3.z(), 3.0);
    }

    #[test]
    fn vector_factories_create_vectors() {
        let v2 = vector2(1.0, 2.0);
        assert_eq!(v2.x(), 1.0);
        assert_eq!(v2.y(), 2.0);

        let v3 = vector3(1.0, 2.0, 3.0);
        assert_eq!(v3.x(), 1.0);
        assert_eq!(v3.y(), 2.0);
        assert_eq!(v3.z(), 3.0);
    }
}
