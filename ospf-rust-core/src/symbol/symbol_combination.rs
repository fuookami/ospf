//! 符号组合模块
//! Symbol Combination Module
//!
//! 提供多维符号组合类型，用于管理一组中间符号。
//! Provides multi-dimensional symbol combination types for managing a group of intermediate symbols.
//!
//! # 核心概念 / Core Concepts
//!
//! - `SymbolCombination`: 多维符号组合，在创建时自动分配唯一组 ID
//!   Multi-dimensional symbol combination, automatically assigned unique group ID on creation
//! - 每个组合内的符号共享相同的组 ID，通过索引区分
//!   Symbols within a combination share the same group ID, distinguished by index

use std::fmt::Debug;
use std::ops::Index;
use std::sync::Arc;

use ospf_rust_multiarray::{
    MultiArray, MultiArrayBuilder,
    shape::{AbstractShape, Shape},
};

use crate::symbol::IntermediateSymbol;
use crate::variable::new_group_id;

// ============================================================================
// SymbolCombination - 多维符号组合
// ============================================================================

/// 多维符号组合 / Multi-dimensional Symbol Combination
///
/// 在创建时自动分配唯一组 ID 的多维符号数组。
/// Multi-dimensional symbol array with automatically assigned unique group ID on creation.
///
/// # 类型参数 / Type Parameters
///
/// - `V`: 值类型 / Value type
/// - `Sym`: 符号类型（实现 `IntermediateSymbol<V>`）/ Symbol type (implements `IntermediateSymbol<V>`)
/// - `S`: 形状类型（实现 `AbstractShape`）/ Shape type (implements `AbstractShape`)
pub struct SymbolCombination<V, Sym, S>
where
    V: Clone + Debug + Send + Sync + 'static,
    Sym: IntermediateSymbol<V>,
    S: AbstractShape,
{
    /// 符号数组 / Symbol array
    symbols: MultiArray<Arc<Sym>, S>,
    /// 组 ID / Group ID
    group_id: usize,
    /// 名称前缀 / Name prefix
    name_prefix: String,
    _marker: std::marker::PhantomData<V>,
}

impl<V, Sym, S> SymbolCombination<V, Sym, S>
where
    V: Clone + Debug + Send + Sync + 'static,
    Sym: IntermediateSymbol<V>,
    S: AbstractShape,
{
    /// 创建新的符号组合 / Create new symbol combination
    ///
    /// # 参数 / Parameters
    ///
    /// - `shape`: 数组形状 / Array shape
    /// - `name_prefix`: 名称前缀 / Name prefix
    /// - `ctor`: 构造函数，接收线性索引和向量坐标，返回符号
    ///   Constructor, receives linear index and vector coordinates, returns symbol
    pub fn new<F>(shape: S, name_prefix: &str, ctor: F) -> Self
    where
        F: Fn(usize, &S::VectorType) -> Sym,
    {
        let group_id = new_group_id();
        let symbols = MultiArrayBuilder::new_by(shape, |index, vector| {
            Arc::new(ctor(index, vector))
        });
        Self {
            symbols,
            group_id,
            name_prefix: name_prefix.to_string(),
            _marker: std::marker::PhantomData,
        }
    }

    /// 使用名称生成器创建符号组合 / Create symbol combination with name generator
    ///
    /// # 参数 / Parameters
    ///
    /// - `shape`: 数组形状 / Array shape
    /// - `name_prefix`: 名称前缀 / Name prefix
    /// - `name_gen`: 名称后缀生成器，接收索引和向量坐标 / Name suffix generator
    /// - `ctor`: 构造函数，接收线性索引、向量坐标和完整名称 / Constructor, receives linear index, vector coordinates, and full name
    pub fn with_name_generator<F, N>(shape: S, name_prefix: &str, name_gen: N, ctor: F) -> Self
    where
        F: Fn(usize, &S::VectorType, String) -> Sym,
        N: Fn(usize, &S::VectorType) -> String,
    {
        let group_id = new_group_id();
        let prefix = name_prefix.to_string();
        let symbols = MultiArrayBuilder::new_by(shape, |index, vector| {
            let suffix = name_gen(index, vector);
            let full_name = format!("{}_{}", prefix, suffix);
            Arc::new(ctor(index, vector, full_name))
        });
        Self {
            symbols,
            group_id,
            name_prefix: prefix,
            _marker: std::marker::PhantomData,
        }
    }

    /// 获取组 ID / Get group ID
    pub fn group_id(&self) -> usize {
        self.group_id
    }

    /// 获取符号数量 / Get symbol count
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// 检查是否为空 / Check if empty
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// 获取名称前缀 / Get name prefix
    pub fn name_prefix(&self) -> &str {
        &self.name_prefix
    }

    /// 获取形状引用 / Get shape reference
    pub fn shape(&self) -> &S {
        &self.symbols.shape
    }

    /// 获取符号数组引用 / Get symbol array reference
    pub fn as_array(&self) -> &MultiArray<Arc<Sym>, S> {
        &self.symbols
    }

    /// 消耗 self，返回符号数组 / Consume self, return symbol array
    pub fn into_array(self) -> MultiArray<Arc<Sym>, S> {
        self.symbols
    }

    /// 获取符号引用迭代器 / Get symbol reference iterator
    pub fn iter(&self) -> impl Iterator<Item = &Arc<Sym>> {
        self.symbols.iter()
    }

    /// 获取 `Arc<dyn IntermediateSymbol<V>>` 迭代器，用于批量注册。
    /// Returns iterator of `Arc<dyn IntermediateSymbol<V>>` for batch registration.
    pub fn iter_arc(&self) -> impl Iterator<Item = Arc<dyn IntermediateSymbol<V>>> + '_
    where
        Sym: 'static,
    {
        self.symbols
            .iter()
            .cloned()
            .map(|arc| arc as Arc<dyn IntermediateSymbol<V>>)
    }

    /// 按索引获取符号的多项式表示。
    /// Get the polynomial representation of the symbol at the given index.
    ///
    /// 自动解引用 `Arc<Sym>` 并调用 `to_linear_polynomial()`。
    /// Automatically dereferences `Arc<Sym>` and calls `to_linear_polynomial()`.
    pub fn symbol_polynomial(&self, index: usize) -> crate::symbol::flatten::Linear<V>
    where
        Sym: crate::symbol::LinearIntermediateSymbol<V>,
    {
        self.symbols[index].to_linear_polynomial()
    }

    /// 按向量坐标获取符号的多项式表示。
    /// Get the polynomial representation of the symbol at the given vector coordinates.
    pub fn symbol_polynomial_at(&self, vector: &S::VectorType) -> crate::symbol::flatten::Linear<V>
    where
        Sym: crate::symbol::LinearIntermediateSymbol<V>,
    {
        self.symbols[vector].to_linear_polynomial()
    }

    /// 按索引获取符号名称。
    /// Get the symbol name at the given index.
    pub fn symbol_name(&self, index: usize) -> &str
    where
        Sym: ospf_rust_math::symbol::DynSymbol,
    {
        self.symbols[index].name()
    }

    /// 按索引获取符号 ID。
    /// Get the symbol ID at the given index.
    pub fn symbol_id(&self, index: usize) -> u64
    where
        Sym: crate::symbol::IntermediateSymbol<V>,
    {
        self.symbols[index].id().id
    }

    /// 对固定前缀索引的某维度求和，返回 `Linear<V>` 多项式。
    /// Sum across a dimension with fixed prefix indices, returning `Linear<V>` polynomial.
    ///
    /// 遍历符号组合中所有在 `dim` 维度以外的坐标与 `fixed_indices` 匹配的元素，
    /// 通过 `to_linear_polynomial()` 提取每个符号的多项式，合并所有单项式。
    ///
    /// Iterates over all elements whose coordinates (except dimension `dim`)
    /// match `fixed_indices`, extracts each symbol's polynomial via `to_linear_polynomial()`,
    /// and merges all monomials.
    ///
    /// # 参数 / Parameters
    ///
    /// - `fixed_indices`: 固定维度的索引值，长度必须等于 `S::DIMENSION - 1`
    ///   Fixed dimension index values, length must equal `S::DIMENSION - 1`
    /// - `dim`: 要求和的维度（0-based）
    ///   Dimension to sum across (0-based)
    /// - `zero`: 常数项零值，通常为 `0.0`
    ///   Zero value for constant term, typically `0.0`
    ///
    /// # Panics
    ///
    /// - `fixed_indices` 长度不等于 `S::DIMENSION - 1`
    /// - `dim` 超出维度范围
    pub fn sum_along_dimension(
        &self,
        fixed_indices: &[usize],
        dim: usize,
        zero: V,
    ) -> crate::symbol::flatten::Linear<V>
    where
        V: std::ops::Add<Output = V>,
        Sym: crate::symbol::LinearIntermediateSymbol<V> + 'static,
    {
        let ndim = self.symbols.shape.dimension();
        assert_eq!(
            fixed_indices.len(),
            ndim - 1,
            "fixed_indices length must be S::DIMENSION - 1 (expected {}, got {})",
            ndim - 1,
            fixed_indices.len()
        );
        assert!(
            dim < ndim,
            "dim {} out of range (ndim = {})",
            dim,
            ndim
        );

        use crate::symbol::flatten::Linear as ModelLinear;

        let mut all_monomials = Vec::new();
        let mut constant = zero;

        for (_linear, vector, sym) in self.symbols.enumerate() {
            // 检查除 dim 维度外的所有坐标是否与 fixed_indices 匹配
            let mut fixed_idx = 0;
            let matches = (0..ndim).all(|d| {
                if d == dim {
                    true
                } else {
                    let expected = fixed_indices[fixed_idx];
                    fixed_idx += 1;
                    vector[d] == expected
                }
            });

            if matches {
                let poly = sym.to_linear_polynomial();
                all_monomials.extend_from_slice(poly.monomials());
                constant = constant + poly.constant_term().clone();
            }
        }

        ModelLinear::new(all_monomials, constant)
    }
}

impl<V, Sym, S> Clone for SymbolCombination<V, Sym, S>
where
    V: Clone + Debug + Send + Sync + 'static,
    Sym: IntermediateSymbol<V>,
    S: AbstractShape + Clone,
{
    fn clone(&self) -> Self {
        Self {
            symbols: MultiArray::clone(&self.symbols),
            group_id: self.group_id,
            name_prefix: self.name_prefix.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<V, Sym, S> Debug for SymbolCombination<V, Sym, S>
where
    V: Clone + Debug + Send + Sync + 'static,
    Sym: IntermediateSymbol<V>,
    S: AbstractShape,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SymbolCombination")
            .field("group_id", &self.group_id)
            .field("name_prefix", &self.name_prefix)
            .field("len", &self.symbols.len())
            .finish()
    }
}

impl<V, Sym, S> std::ops::Deref for SymbolCombination<V, Sym, S>
where
    V: Clone + Debug + Send + Sync + 'static,
    Sym: IntermediateSymbol<V>,
    S: AbstractShape,
{
    type Target = MultiArray<Arc<Sym>, S>;

    fn deref(&self) -> &Self::Target {
        &self.symbols
    }
}

// ============================================================================
// Index 实现 / Index Implementations
// ============================================================================

impl<V, Sym, S> Index<usize> for SymbolCombination<V, Sym, S>
where
    V: Clone + Debug + Send + Sync + 'static,
    Sym: IntermediateSymbol<V>,
    S: AbstractShape,
{
    type Output = Arc<Sym>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.symbols[index]
    }
}

impl<V, Sym, S> Index<&S::VectorType> for SymbolCombination<V, Sym, S>
where
    V: Clone + Debug + Send + Sync + 'static,
    Sym: IntermediateSymbol<V>,
    S: AbstractShape,
{
    type Output = Arc<Sym>;

    fn index(&self, vector: &S::VectorType) -> &Self::Output {
        &self.symbols[vector]
    }
}

// ============================================================================
// 便捷类型别名 / Convenience Type Aliases
// ============================================================================

use crate::symbol::LinearExpressionSymbol;

/// 一维线性表达式符号组合 / 1D Linear Expression Symbol Combination
pub type LinearExpressionSymbols1<V> =
    SymbolCombination<V, LinearExpressionSymbol<V>, Shape<1>>;

/// 二维线性表达式符号组合 / 2D Linear Expression Symbol Combination
pub type LinearExpressionSymbols2<V> =
    SymbolCombination<V, LinearExpressionSymbol<V>, Shape<2>>;

/// 三维线性表达式符号组合 / 3D Linear Expression Symbol Combination
pub type LinearExpressionSymbols3<V> =
    SymbolCombination<V, LinearExpressionSymbol<V>, Shape<3>>;

/// 四维线性表达式符号组合 / 4D Linear Expression Symbol Combination
pub type LinearExpressionSymbols4<V> =
    SymbolCombination<V, LinearExpressionSymbol<V>, Shape<4>>;

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::flatten::LinearMonomial;
    use ospf_rust_math::symbol::Symbol;

    #[test]
    fn test_symbol_combination_creation() {
        let combo: LinearExpressionSymbols1<f64> =
            SymbolCombination::new(Shape::new([3]), "bandwidth", |index, _vec| {
                LinearExpressionSymbol::new(
                    index as u64 + 1,
                    &format!("bandwidth_{}", index),
                    vec![],
                    0.0,
                )
            });

        assert_eq!(combo.len(), 3);
        assert!(!combo.is_empty());
        assert_eq!(combo.name_prefix(), "bandwidth");
    }

    #[test]
    fn test_index_access() {
        let combo: LinearExpressionSymbols1<f64> =
            SymbolCombination::new(Shape::new([3]), "sym", |index, _vec| {
                LinearExpressionSymbol::new(
                    index as u64 + 100,
                    &format!("sym_{}", index),
                    vec![LinearMonomial::new(1.0, index)],
                    0.0,
                )
            });

        // 线性索引 / Linear indexing
        let sym0 = &combo[0];
        assert_eq!(sym0.id().id, 100);

        let sym2 = &combo[2];
        assert_eq!(sym2.id().id, 102);
    }

    #[test]
    fn test_2d_vector_index() {
        let combo: LinearExpressionSymbols2<f64> =
            SymbolCombination::new(Shape::new([2, 3]), "matrix", |index, _vec| {
                LinearExpressionSymbol::new(
                    index as u64 + 1,
                    &format!("m_{}", index),
                    vec![],
                    0.0,
                )
            });

        assert_eq!(combo.len(), 6);
        // 向量坐标索引 / Vector coordinate indexing
        let sym = &combo[&[1, 2]];
        assert_eq!(sym.id().id, 6); // row-major: 1*3+2 = 5, id = 5+1 = 6
    }

    #[test]
    fn test_group_id_unique() {
        let combo1: LinearExpressionSymbols1<f64> =
            SymbolCombination::new(Shape::new([2]), "a", |index, _| {
                LinearExpressionSymbol::new(index as u64 + 1, "a", vec![], 0.0)
            });
        let combo2: LinearExpressionSymbols1<f64> =
            SymbolCombination::new(Shape::new([2]), "b", |index, _| {
                LinearExpressionSymbol::new(index as u64 + 100, "b", vec![], 0.0)
            });

        assert_ne!(combo1.group_id(), combo2.group_id());
    }

    #[test]
    fn test_iter_arc() {
        let combo: LinearExpressionSymbols1<f64> =
            SymbolCombination::new(Shape::new([3]), "sym", |index, _vec| {
                LinearExpressionSymbol::new(
                    index as u64 + 1,
                    &format!("sym_{}", index),
                    vec![],
                    0.0,
                )
            });

        let arc_syms: Vec<Arc<dyn IntermediateSymbol<f64>>> =
            combo.iter_arc().collect();
        assert_eq!(arc_syms.len(), 3);
        assert_eq!(arc_syms[0].id().id, 1);
        assert_eq!(arc_syms[2].id().id, 3);
    }
}
