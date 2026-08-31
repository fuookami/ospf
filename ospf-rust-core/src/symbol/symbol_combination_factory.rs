//! 符号组合工厂
//! Symbol Combination Factory
//!
//! 提供从领域对象列表派生符号组合的工厂函数。
//! Provides factory functions for deriving symbol combinations from domain object lists.
//!
//! # 工厂模式 / Factory Patterns
//!
//! - `flat_map`: 构造函数返回 `Linear<V>` 多项式
//!   Constructor returns `Linear<V>` polynomial
//! - `map`: 构造函数返回单个值 `V`，包装为常数项
//!   Constructor returns a single value `V`, wrapped as constant term

use std::fmt::Debug;
use std::ops::{Add, Mul};

use ospf_rust_multiarray::shape::Shape;

use crate::symbol::flatten::Linear;
use crate::symbol::intermediate_symbol::next_auto_intermediate_symbol_id;
use crate::symbol::{LinearExpressionSymbol, SymbolCombination};

// ============================================================================
// 一维工厂 / 1D Factories
// ============================================================================

/// 一维工厂：从领域对象列表派生一维符号组合（多项式版本）。
/// 1D factory: derive 1D symbol combination from domain object list (polynomial version).
///
/// # 参数 / Parameters
///
/// - `name`: 符号组合名称前缀 / Symbol combination name prefix
/// - `objs`: 领域对象切片 / Domain object slice
/// - `ctor`: 构造函数，接收领域对象引用，返回 `Linear<V>` / Constructor returning `Linear<V>`
/// - `suffix`: 名称后缀函数，接收索引和对象引用 / Name suffix function
pub fn flat_map1<T, V>(
    name: &str,
    objs: &[T],
    ctor: impl Fn(&T) -> Linear<V>,
    suffix: impl Fn(usize, &T) -> String,
) -> SymbolCombination<V, LinearExpressionSymbol<V>, Shape<1>>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    SymbolCombination::new(Shape::new([objs.len()]), name, |index, _vec| {
        let obj = &objs[index];
        let linear = ctor(obj);
        let id = next_auto_intermediate_symbol_id();
        let sym_name = format!("{}_{}", name, suffix(index, obj));
        LinearExpressionSymbol::new(
            id,
            &sym_name,
            linear.monomials().to_vec(),
            linear.constant_term().clone(),
        )
    })
}

/// 一维工厂：从领域对象列表派生一维符号组合（标量版本）。
/// 1D factory: derive 1D symbol combination from domain object list (scalar version).
///
/// 构造函数返回单个值 `V`，包装为仅含常数项的符号。
/// Constructor returns a single value `V`, wrapped as constant-only symbol.
pub fn map1<T, V>(
    name: &str,
    objs: &[T],
    ctor: impl Fn(&T) -> V,
    suffix: impl Fn(usize, &T) -> String,
) -> SymbolCombination<V, LinearExpressionSymbol<V>, Shape<1>>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    SymbolCombination::new(Shape::new([objs.len()]), name, |index, _vec| {
        let obj = &objs[index];
        let value = ctor(obj);
        let id = next_auto_intermediate_symbol_id();
        let sym_name = format!("{}_{}", name, suffix(index, obj));
        LinearExpressionSymbol::new(id, &sym_name, vec![], value)
    })
}

// ============================================================================
// 二维工厂 / 2D Factories
// ============================================================================

/// 二维工厂：从两组领域对象派生二维符号组合（多项式版本）。
/// 2D factory: derive 2D symbol combination from two domain object lists (polynomial version).
pub fn flat_map2<T1, T2, V>(
    name: &str,
    objs1: &[T1],
    objs2: &[T2],
    ctor: impl Fn(&T1, &T2) -> Linear<V>,
    suffix: impl Fn(usize, &T1, usize, &T2) -> String,
) -> SymbolCombination<V, LinearExpressionSymbol<V>, Shape<2>>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    let d1 = objs1.len();
    let d2 = objs2.len();
    SymbolCombination::new(Shape::new([d1, d2]), name, |_index, vec| {
        let i = vec[0];
        let j = vec[1];
        let linear = ctor(&objs1[i], &objs2[j]);
        let id = next_auto_intermediate_symbol_id();
        let sym_name = format!("{}_{}", name, suffix(i, &objs1[i], j, &objs2[j]));
        LinearExpressionSymbol::new(
            id,
            &sym_name,
            linear.monomials().to_vec(),
            linear.constant_term().clone(),
        )
    })
}

/// 二维工厂：从两组领域对象派生二维符号组合（标量版本）。
/// 2D factory: derive 2D symbol combination from two domain object lists (scalar version).
pub fn map2<T1, T2, V>(
    name: &str,
    objs1: &[T1],
    objs2: &[T2],
    ctor: impl Fn(&T1, &T2) -> V,
    suffix: impl Fn(usize, &T1, usize, &T2) -> String,
) -> SymbolCombination<V, LinearExpressionSymbol<V>, Shape<2>>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    let d1 = objs1.len();
    let d2 = objs2.len();
    SymbolCombination::new(Shape::new([d1, d2]), name, |_index, vec| {
        let i = vec[0];
        let j = vec[1];
        let value = ctor(&objs1[i], &objs2[j]);
        let id = next_auto_intermediate_symbol_id();
        let sym_name = format!("{}_{}", name, suffix(i, &objs1[i], j, &objs2[j]));
        LinearExpressionSymbol::new(id, &sym_name, vec![], value)
    })
}

// ============================================================================
// 三维工厂 / 3D Factories
// ============================================================================

/// 三维工厂：从三组领域对象派生三维符号组合（多项式版本）。
/// 3D factory: derive 3D symbol combination from three domain object lists (polynomial version).
pub fn flat_map3<T1, T2, T3, V>(
    name: &str,
    objs1: &[T1],
    objs2: &[T2],
    objs3: &[T3],
    ctor: impl Fn(&T1, &T2, &T3) -> Linear<V>,
    suffix: impl Fn(usize, &T1, usize, &T2, usize, &T3) -> String,
) -> SymbolCombination<V, LinearExpressionSymbol<V>, Shape<3>>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    let d1 = objs1.len();
    let d2 = objs2.len();
    let d3 = objs3.len();
    SymbolCombination::new(Shape::new([d1, d2, d3]), name, |_index, vec| {
        let i = vec[0];
        let j = vec[1];
        let k = vec[2];
        let linear = ctor(&objs1[i], &objs2[j], &objs3[k]);
        let id = next_auto_intermediate_symbol_id();
        let sym_name = format!(
            "{}_{}",
            name,
            suffix(i, &objs1[i], j, &objs2[j], k, &objs3[k])
        );
        LinearExpressionSymbol::new(
            id,
            &sym_name,
            linear.monomials().to_vec(),
            linear.constant_term().clone(),
        )
    })
}

/// 三维工厂：从三组领域对象派生三维符号组合（标量版本）。
/// 3D factory: derive 3D symbol combination from three domain object lists (scalar version).
pub fn map3<T1, T2, T3, V>(
    name: &str,
    objs1: &[T1],
    objs2: &[T2],
    objs3: &[T3],
    ctor: impl Fn(&T1, &T2, &T3) -> V,
    suffix: impl Fn(usize, &T1, usize, &T2, usize, &T3) -> String,
) -> SymbolCombination<V, LinearExpressionSymbol<V>, Shape<3>>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    let d1 = objs1.len();
    let d2 = objs2.len();
    let d3 = objs3.len();
    SymbolCombination::new(Shape::new([d1, d2, d3]), name, |_index, vec| {
        let i = vec[0];
        let j = vec[1];
        let k = vec[2];
        let value = ctor(&objs1[i], &objs2[j], &objs3[k]);
        let id = next_auto_intermediate_symbol_id();
        let sym_name = format!(
            "{}_{}",
            name,
            suffix(i, &objs1[i], j, &objs2[j], k, &objs3[k])
        );
        LinearExpressionSymbol::new(id, &sym_name, vec![], value)
    })
}

// ============================================================================
// 索引版一维工厂 / Indexed 1D Factories
// ============================================================================

/// 一维工厂（索引版）：构造函数接收 `(index, &T)` 而非 `&T`。
/// 1D factory (indexed): ctor receives `(index, &T)` instead of `&T`.
///
/// 消除 `flat_map1` 中常见的 `iter().position()` 反模式。
/// Eliminates the common `iter().position()` anti-pattern inside `flat_map1`.
pub fn flat_map1_indexed<T, V>(
    name: &str,
    objs: &[T],
    ctor: impl Fn(usize, &T) -> Linear<V>,
    suffix: impl Fn(usize, &T) -> String,
) -> SymbolCombination<V, LinearExpressionSymbol<V>, Shape<1>>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    SymbolCombination::new(Shape::new([objs.len()]), name, |index, _vec| {
        let obj = &objs[index];
        let linear = ctor(index, obj);
        let id = next_auto_intermediate_symbol_id();
        let sym_name = format!("{}_{}", name, suffix(index, obj));
        LinearExpressionSymbol::new(
            id,
            &sym_name,
            linear.monomials().to_vec(),
            linear.constant_term().clone(),
        )
    })
}

/// 一维工厂（索引版，标量）：构造函数接收 `(index, &T)` 而非 `&T`。
/// 1D factory (indexed, scalar): ctor receives `(index, &T)` instead of `&T`.
pub fn map1_indexed<T, V>(
    name: &str,
    objs: &[T],
    ctor: impl Fn(usize, &T) -> V,
    suffix: impl Fn(usize, &T) -> String,
) -> SymbolCombination<V, LinearExpressionSymbol<V>, Shape<1>>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    SymbolCombination::new(Shape::new([objs.len()]), name, |index, _vec| {
        let obj = &objs[index];
        let value = ctor(index, obj);
        let id = next_auto_intermediate_symbol_id();
        let sym_name = format!("{}_{}", name, suffix(index, obj));
        LinearExpressionSymbol::new(id, &sym_name, vec![], value)
    })
}

// ============================================================================
// 索引版二维工厂 / Indexed 2D Factories
// ============================================================================

/// 二维工厂（索引版）：构造函数接收 `(i, &T1, j, &T2)` 而非 `(&T1, &T2)`。
/// 2D factory (indexed): ctor receives `(i, &T1, j, &T2)` instead of `(&T1, &T2)`.
pub fn flat_map2_indexed<T1, T2, V>(
    name: &str,
    objs1: &[T1],
    objs2: &[T2],
    ctor: impl Fn(usize, &T1, usize, &T2) -> Linear<V>,
    suffix: impl Fn(usize, &T1, usize, &T2) -> String,
) -> SymbolCombination<V, LinearExpressionSymbol<V>, Shape<2>>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    let d1 = objs1.len();
    let d2 = objs2.len();
    SymbolCombination::new(Shape::new([d1, d2]), name, |_index, vec| {
        let i = vec[0];
        let j = vec[1];
        let linear = ctor(i, &objs1[i], j, &objs2[j]);
        let id = next_auto_intermediate_symbol_id();
        let sym_name = format!("{}_{}", name, suffix(i, &objs1[i], j, &objs2[j]));
        LinearExpressionSymbol::new(
            id,
            &sym_name,
            linear.monomials().to_vec(),
            linear.constant_term().clone(),
        )
    })
}

/// 二维工厂（索引版，标量）：构造函数接收 `(i, &T1, j, &T2)` 而非 `(&T1, &T2)`。
/// 2D factory (indexed, scalar): ctor receives `(i, &T1, j, &T2)` instead of `(&T1, &T2)`.
pub fn map2_indexed<T1, T2, V>(
    name: &str,
    objs1: &[T1],
    objs2: &[T2],
    ctor: impl Fn(usize, &T1, usize, &T2) -> V,
    suffix: impl Fn(usize, &T1, usize, &T2) -> String,
) -> SymbolCombination<V, LinearExpressionSymbol<V>, Shape<2>>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    let d1 = objs1.len();
    let d2 = objs2.len();
    SymbolCombination::new(Shape::new([d1, d2]), name, |_index, vec| {
        let i = vec[0];
        let j = vec[1];
        let value = ctor(i, &objs1[i], j, &objs2[j]);
        let id = next_auto_intermediate_symbol_id();
        let sym_name = format!("{}_{}", name, suffix(i, &objs1[i], j, &objs2[j]));
        LinearExpressionSymbol::new(id, &sym_name, vec![], value)
    })
}

// ============================================================================
// 索引版三维工厂 / Indexed 3D Factories
// ============================================================================

/// 三维工厂（索引版）：构造函数接收 `(i, &T1, j, &T2, k, &T3)` 而非 `(&T1, &T2, &T3)`。
/// 3D factory (indexed): ctor receives `(i, &T1, j, &T2, k, &T3)` instead of `(&T1, &T2, &T3)`.
pub fn flat_map3_indexed<T1, T2, T3, V>(
    name: &str,
    objs1: &[T1],
    objs2: &[T2],
    objs3: &[T3],
    ctor: impl Fn(usize, &T1, usize, &T2, usize, &T3) -> Linear<V>,
    suffix: impl Fn(usize, &T1, usize, &T2, usize, &T3) -> String,
) -> SymbolCombination<V, LinearExpressionSymbol<V>, Shape<3>>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    let d1 = objs1.len();
    let d2 = objs2.len();
    let d3 = objs3.len();
    SymbolCombination::new(Shape::new([d1, d2, d3]), name, |_index, vec| {
        let i = vec[0];
        let j = vec[1];
        let k = vec[2];
        let linear = ctor(i, &objs1[i], j, &objs2[j], k, &objs3[k]);
        let id = next_auto_intermediate_symbol_id();
        let sym_name = format!(
            "{}_{}",
            name,
            suffix(i, &objs1[i], j, &objs2[j], k, &objs3[k])
        );
        LinearExpressionSymbol::new(
            id,
            &sym_name,
            linear.monomials().to_vec(),
            linear.constant_term().clone(),
        )
    })
}

/// 三维工厂（索引版，标量）：构造函数接收 `(i, &T1, j, &T2, k, &T3)` 而非 `(&T1, &T2, &T3)`。
/// 3D factory (indexed, scalar): ctor receives `(i, &T1, j, &T2, k, &T3)` instead of `(&T1, &T2, &T3)`.
pub fn map3_indexed<T1, T2, T3, V>(
    name: &str,
    objs1: &[T1],
    objs2: &[T2],
    objs3: &[T3],
    ctor: impl Fn(usize, &T1, usize, &T2, usize, &T3) -> V,
    suffix: impl Fn(usize, &T1, usize, &T2, usize, &T3) -> String,
) -> SymbolCombination<V, LinearExpressionSymbol<V>, Shape<3>>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    let d1 = objs1.len();
    let d2 = objs2.len();
    let d3 = objs3.len();
    SymbolCombination::new(Shape::new([d1, d2, d3]), name, |_index, vec| {
        let i = vec[0];
        let j = vec[1];
        let k = vec[2];
        let value = ctor(i, &objs1[i], j, &objs2[j], k, &objs3[k]);
        let id = next_auto_intermediate_symbol_id();
        let sym_name = format!(
            "{}_{}",
            name,
            suffix(i, &objs1[i], j, &objs2[j], k, &objs3[k])
        );
        LinearExpressionSymbol::new(id, &sym_name, vec![], value)
    })
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::flatten::LinearMonomial;
    use crate::symbol::symbol_combination::LinearExpressionSymbols1;
    use ospf_rust_math::symbol::Symbol;

    #[test]
    fn test_flat_map1() {
        let edges = vec!["e0", "e1", "e2"];
        let combo: LinearExpressionSymbols1<f64> = flat_map1(
            "bandwidth",
            &edges,
            |_e| Linear::new(vec![LinearMonomial::new(1.0, 42)], 0.0),
            |i, e| format!("{}_{}", e, i),
        );

        assert_eq!(combo.len(), 3);
        assert_eq!(combo.name_prefix(), "bandwidth");

        // 验证符号名称格式 / Verify symbol name format
        assert_eq!(combo[0].id().name, "bandwidth_e0_0");
        assert_eq!(combo[1].id().name, "bandwidth_e1_1");
        assert_eq!(combo[2].id().name, "bandwidth_e2_2");

        // 验证符号 ID 唯一 / Verify symbol IDs are unique
        let ids: Vec<u64> = (0..3).map(|i| combo[i].id().id).collect();
        assert!(ids.windows(2).all(|w| w[0] != w[1]));

        // 验证多项式内容 / Verify polynomial content
        let poly = combo[0].to_linear_polynomial();
        assert_eq!(poly.monomials().len(), 1);
        assert_eq!(*poly.constant_term(), 0.0);
    }

    #[test]
    fn test_map1() {
        let values = vec![10.0, 20.0, 30.0];
        let combo = map1(
            "capacity",
            &values,
            |v| *v,
            |i, _v| format!("{}", i),
        );

        assert_eq!(combo.len(), 3);
        // map1 创建的符号只有常数项 / map1 symbols have only constant term
        let poly = combo[0].to_linear_polynomial();
        assert!(poly.monomials().is_empty());
        assert_eq!(*poly.constant_term(), 10.0);
    }

    #[test]
    fn test_flat_map2() {
        let nodes = vec!["n0", "n1"];
        let services = vec!["s0", "s1", "s2"];
        let combo = flat_map2(
            "flow",
            &nodes,
            &services,
            |_n, _s| Linear::new(vec![LinearMonomial::new(2.0, 7)], 1.0),
            |_i, n, _j, s| format!("{}_{}", n, s),
        );

        assert_eq!(combo.len(), 6);
        assert_eq!(combo[&[0, 0]].id().name, "flow_n0_s0");
        assert_eq!(combo[&[1, 2]].id().name, "flow_n1_s2");
    }

    #[test]
    fn test_symbol_ids_from_auto_namespace() {
        let objs = vec![1, 2, 3];
        let combo = flat_map1(
            "test",
            &objs,
            |_| Linear::zero(0.0),
            |i, _| format!("{}", i),
        );

        // ID 应来自自动命名空间（>= 1_000_000_000）
        // IDs should come from auto namespace (>= 1_000_000_000)
        for i in 0..3 {
            assert!(combo[i].id().id >= 1_000_000_000);
        }
    }

    #[test]
    fn test_flat_map1_indexed() {
        let edges = vec!["e0", "e1", "e2"];
        let combo: LinearExpressionSymbols1<f64> = flat_map1_indexed(
            "bw_idx",
            &edges,
            |i, _e| Linear::new(vec![LinearMonomial::new(1.0, i + 42)], 0.0),
            |i, e| format!("{}_{}", e, i),
        );

        assert_eq!(combo.len(), 3);
        assert_eq!(combo[0].id().name, "bw_idx_e0_0");
        assert_eq!(combo[1].id().name, "bw_idx_e1_1");
        assert_eq!(combo[2].id().name, "bw_idx_e2_2");

        // Verify index flows through correctly: monomial var_index = i + 42
        let poly = combo[1].to_linear_polynomial();
        assert_eq!(poly.monomials().len(), 1);
        assert_eq!(poly.monomials()[0].var_index(), 43); // index 1 + 42
    }

    #[test]
    fn test_flat_map2_indexed() {
        let nodes = vec!["n0", "n1"];
        let services = vec!["s0", "s1", "s2"];
        let combo = flat_map2_indexed(
            "flow_idx",
            &nodes,
            &services,
            |i, _n, j, _s| Linear::new(vec![LinearMonomial::new((i + j) as f64, 7)], 0.0),
            |i, n, j, s| format!("{}_{}_{}_{}", n, i, s, j),
        );

        assert_eq!(combo.len(), 6);
        assert_eq!(combo[&[0, 0]].id().name, "flow_idx_n0_0_s0_0");
        assert_eq!(combo[&[1, 2]].id().name, "flow_idx_n1_1_s2_2");
    }

    use crate::symbol::LinearIntermediateSymbol;
}
