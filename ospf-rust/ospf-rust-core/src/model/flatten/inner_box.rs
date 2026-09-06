//! Inner Box 设计 - 多项式内部类型
//! Inner Box Design - Polynomial Internal Types

use super::{CacheKey, Cacheable};
use std::collections::HashMap;

// ============================================================================
// 线性单项式 Inner Box 设计
// Linear Monomial Inner Box Design
// ============================================================================

/// 线性单项式内部数据 / Linear Monomial Inner Data
///
/// 实际的单项式数据，存储在 Box 中以获得稳定地址，Clone 为深拷贝。
/// Actual monomial data, stored in Box for stable address, Clone performs deep copy.
#[derive(Debug, Clone)]
pub struct LinearMonomialInner<V> {
    /// 系数 / Coefficient
    pub coefficient: V,
    /// 变量索引 / Variable index
    pub var_index: usize,
}

/// 线性单项式 / Linear Monomial
///
/// 包装 Inner Box 的单项式，拥有稳定内存地址，Clone 为深拷贝。
/// Monomial wrapping Inner Box, having stable memory address, Clone performs deep copy.
///
/// # 设计说明 / Design Notes
///
/// - 使用 `Box<Inner>` 而非 `Arc<Inner>`，确保深拷贝语义
/// - 每个实例拥有唯一的堆地址，可作为缓存 Key
/// - 无引用计数开销，更轻量
///
/// - Uses `Box<Inner>` instead of `Arc<Inner>`, ensuring deep copy semantics
/// - Each instance has a unique heap address, can be used as cache key
/// - No reference counting overhead, more lightweight
#[derive(Debug, Clone)]
pub struct LinearMonomial<V> {
    inner: Box<LinearMonomialInner<V>>,
}

impl<V> LinearMonomial<V> {
    /// 创建新单项式 / Create new monomial
    pub fn new(coefficient: V, var_index: usize) -> Self {
        Self {
            inner: Box::new(LinearMonomialInner {
                coefficient,
                var_index,
            }),
        }
    }

    /// 获取系数 / Get coefficient
    pub fn coefficient(&self) -> &V {
        &self.inner.coefficient
    }

    /// 获取变量索引 / Get variable index
    pub fn var_index(&self) -> usize {
        self.inner.var_index
    }

    /// 获取内部引用 / Get inner reference
    pub fn inner(&self) -> &LinearMonomialInner<V> {
        &self.inner
    }

    /// 设置系数 / Set coefficient
    pub fn set_coefficient(&mut self, coefficient: V) {
        self.inner.coefficient = coefficient;
    }
}

impl<V> Cacheable for LinearMonomial<V> {
    fn cache_key(&self) -> CacheKey {
        // 使用 Box 内部指针地址作为 Key
        // Use the address of the inner Box pointer as key
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

// ============================================================================
// 线性多项式 Inner Box 设计
// Linear Polynomial Inner Box Design
// ============================================================================

/// 线性多项式内部数据 / Linear Polynomial Inner Data
///
/// 实际的多项式数据，存储在 Box 中以获得稳定地址，Clone 为深拷贝。
/// Actual polynomial data, stored in Box for stable address, Clone performs deep copy.
#[derive(Debug, Clone)]
pub struct LinearInner<V> {
    /// 单项式列表 / Monomial list
    pub monomials: Vec<LinearMonomial<V>>,
    /// 常数项 / Constant term
    pub constant_term: V,
}

/// 线性多项式 / Linear Polynomial
///
/// 包装 Inner Box 的多项式，拥有稳定内存地址，Clone 为深拷贝。
/// Polynomial wrapping Inner Box, having stable memory address, Clone performs deep copy.
///
/// # 设计优势 / Design Benefits
///
/// 1. **稳定地址**: 每个 `Linear<V>` 实例都有唯一的内存地址
/// 2. **高效缓存**: 缓存 Key 直接使用地址，无需计算哈希
/// 3. **深拷贝语义**: 每次克隆都是独立的新实例
/// 4. **比较高效**: 地址比较比内容比较更快
#[derive(Debug, Clone)]
pub struct Linear<V> {
    inner: Box<LinearInner<V>>,
}

impl<V> Linear<V> {
    /// 创建新多项式 / Create new polynomial
    pub fn new(monomials: Vec<LinearMonomial<V>>, constant_term: V) -> Self {
        Self {
            inner: Box::new(LinearInner {
                monomials,
                constant_term,
            }),
        }
    }

    /// 创建常数多项式 / Create constant polynomial
    pub fn constant(constant_term: V) -> Self {
        Self {
            inner: Box::new(LinearInner {
                monomials: Vec::new(),
                constant_term,
            }),
        }
    }

    /// 创建空多项式（零）/ Create empty polynomial (zero)
    pub fn zero(zero: V) -> Self {
        Self {
            inner: Box::new(LinearInner {
                monomials: Vec::new(),
                constant_term: zero,
            }),
        }
    }

    /// 获取单项式列表 / Get monomial list
    pub fn monomials(&self) -> &[LinearMonomial<V>] {
        &self.inner.monomials
    }

    /// 获取常数项 / Get constant term
    pub fn constant_term(&self) -> &V {
        &self.inner.constant_term
    }

    /// 获取内部引用 / Get inner reference
    pub fn inner(&self) -> &LinearInner<V> {
        &self.inner
    }

    /// 添加单项式 / Add monomial
    pub fn add_monomial(&mut self, monomial: LinearMonomial<V>) {
        self.inner.monomials.push(monomial);
    }

    /// 设置常数项 / Set constant term
    pub fn set_constant_term(&mut self, constant_term: V) {
        self.inner.constant_term = constant_term;
    }

    /// 检查是否为空（只有常数项）/ Check if empty (only constant)
    pub fn is_constant_only(&self) -> bool {
        self.inner.monomials.is_empty()
    }

    /// 获取单项式数量 / Get monomial count
    pub fn len(&self) -> usize {
        self.inner.monomials.len()
    }

    /// 检查是否为空 / Check if empty
    pub fn is_empty(&self) -> bool {
        self.inner.monomials.is_empty()
    }
}

impl<V> Cacheable for Linear<V> {
    fn cache_key(&self) -> CacheKey {
        // 使用 Box 内部指针地址作为 Key
        // Use the address of the inner Box pointer as key
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

// ============================================================================
// 二次单项式 Inner Box 设计
// Quadratic Monomial Inner Box Design
// ============================================================================

/// 二次单项式类型 / Quadratic Monomial Kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuadraticMonomialKind {
    /// 二次项 (x1 * x2) / Quadratic term (x1 * x2)
    Quadratic,
    /// 线性项 (x) / Linear term (x)
    Linear,
}

/// 二次单项式内部数据 / Quadratic Monomial Inner Data
#[derive(Debug, Clone)]
pub struct QuadraticMonomialInner<V> {
    /// 系数 / Coefficient
    pub coefficient: V,
    /// 第一个变量索引 / First variable index
    pub var_index1: usize,
    /// 第二个变量索引（仅二次项有效）/ Second variable index (only for quadratic terms)
    pub var_index2: Option<usize>,
}

/// 二次单项式 / Quadratic Monomial
///
/// 包装 Inner Box，Clone 为深拷贝。
/// Wrapping Inner Box, Clone performs deep copy.
#[derive(Debug, Clone)]
pub struct QuadraticMonomial<V> {
    inner: Box<QuadraticMonomialInner<V>>,
}

impl<V> QuadraticMonomial<V> {
    /// 创建二次项 / Create quadratic term
    pub fn new_quadratic(coefficient: V, var_index1: usize, var_index2: usize) -> Self {
        Self {
            inner: Box::new(QuadraticMonomialInner {
                coefficient,
                var_index1,
                var_index2: Some(var_index2),
            }),
        }
    }

    /// 创建线性项 / Create linear term
    pub fn new_linear(coefficient: V, var_index: usize) -> Self {
        Self {
            inner: Box::new(QuadraticMonomialInner {
                coefficient,
                var_index1: var_index,
                var_index2: None,
            }),
        }
    }

    /// 获取系数 / Get coefficient
    pub fn coefficient(&self) -> &V {
        &self.inner.coefficient
    }

    /// 获取第一个变量索引 / Get first variable index
    pub fn var_index1(&self) -> usize {
        self.inner.var_index1
    }

    /// 获取第二个变量索引 / Get second variable index
    pub fn var_index2(&self) -> Option<usize> {
        self.inner.var_index2
    }

    /// 获取单项式类型 / Get monomial kind
    pub fn kind(&self) -> QuadraticMonomialKind {
        if self.inner.var_index2.is_some() {
            QuadraticMonomialKind::Quadratic
        } else {
            QuadraticMonomialKind::Linear
        }
    }

    /// 缓存键 / Cache key
    pub fn cache_key(&self) -> CacheKey {
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

impl<V> Cacheable for QuadraticMonomial<V> {
    fn cache_key(&self) -> CacheKey {
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

// ============================================================================
// 二次多项式 Inner Box 设计
// Quadratic Polynomial Inner Box Design
// ============================================================================

/// 二次多项式内部数据 / Quadratic Polynomial Inner Data
#[derive(Debug, Clone)]
pub struct QuadraticInner<V> {
    /// 单项式列表 / Monomial list
    pub monomials: Vec<QuadraticMonomial<V>>,
    /// 常数项 / Constant term
    pub constant: V,
}

/// 二次多项式 / Quadratic Polynomial
///
/// 包装 Inner Box，Clone 为深拷贝。
/// Wrapping Inner Box, Clone performs deep copy.
#[derive(Debug, Clone)]
pub struct Quadratic<V> {
    inner: Box<QuadraticInner<V>>,
}

impl<V> Quadratic<V> {
    /// 创建新多项式 / Create new polynomial
    pub fn new(monomials: Vec<QuadraticMonomial<V>>, constant: V) -> Self {
        Self {
            inner: Box::new(QuadraticInner {
                monomials,
                constant,
            }),
        }
    }

    /// 创建常数多项式 / Create constant polynomial
    pub fn from_constant(constant: V) -> Self {
        Self {
            inner: Box::new(QuadraticInner {
                monomials: Vec::new(),
                constant,
            }),
        }
    }

    /// 创建空多项式（零）/ Create empty polynomial (zero)
    pub fn zero(zero: V) -> Self {
        Self {
            inner: Box::new(QuadraticInner {
                monomials: Vec::new(),
                constant: zero,
            }),
        }
    }

    /// 从线性多项式创建 / Create from linear polynomial
    pub fn from_linear(linear: &Linear<V>) -> Self
    where
        V: Clone,
    {
        let monomials = linear
            .monomials()
            .iter()
            .map(|m| QuadraticMonomial::new_linear(m.coefficient().clone(), m.var_index()))
            .collect();
        Self {
            inner: Box::new(QuadraticInner {
                monomials,
                constant: linear.constant_term().clone(),
            }),
        }
    }

    /// 获取单项式列表 / Get monomial list
    pub fn monomials(&self) -> &[QuadraticMonomial<V>] {
        &self.inner.monomials
    }

    /// 获取常数项 / Get constant
    pub fn constant(&self) -> &V {
        &self.inner.constant
    }

    /// 获取内部引用 / Get inner reference
    pub fn inner(&self) -> &QuadraticInner<V> {
        &self.inner
    }

    /// 添加单项式 / Add monomial
    pub fn add_monomial(&mut self, monomial: QuadraticMonomial<V>) {
        self.inner.monomials.push(monomial);
    }

    /// 设置常数项 / Set constant
    pub fn set_constant(&mut self, constant: V) {
        self.inner.constant = constant;
    }

    /// 缓存键 / Cache key
    pub fn cache_key(&self) -> CacheKey {
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }

    /// 获取单项式数量 / Get monomial count
    pub fn len(&self) -> usize {
        self.inner.monomials.len()
    }

    /// 检查是否为空 / Check if empty
    pub fn is_empty(&self) -> bool {
        self.inner.monomials.is_empty()
    }
}

impl<V> Cacheable for Quadratic<V> {
    fn cache_key(&self) -> CacheKey {
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

// ============================================================================
// 标准多项式 Inner Box 设计
// Canonical Polynomial Inner Box Design
// ============================================================================

/// 标准单项式内部数据 / Canonical Monomial Inner Data
#[derive(Debug, Clone)]
pub struct CanonicalMonomialInner<V> {
    /// 系数 / Coefficient
    pub coefficient: V,
    /// 变量索引到幂次的映射 / Variable index to power mapping
    pub powers: HashMap<usize, i32>,
}

/// 标准单项式 / Canonical Monomial
///
/// 包装 Inner Box，Clone 为深拷贝。
/// Wrapping Inner Box, Clone performs deep copy.
#[derive(Debug, Clone)]
pub struct CanonicalMonomial<V> {
    inner: Box<CanonicalMonomialInner<V>>,
}

impl<V> CanonicalMonomial<V> {
    /// 创建新单项式 / Create new monomial
    pub fn new(coefficient: V, powers: HashMap<usize, i32>) -> Self {
        Self {
            inner: Box::new(CanonicalMonomialInner {
                coefficient,
                powers,
            }),
        }
    }

    /// 创建单项式变量项 / Create monomial variable term
    pub fn new_variable(coefficient: V, var_index: usize) -> Self {
        let mut powers = HashMap::new();
        powers.insert(var_index, 1);
        Self {
            inner: Box::new(CanonicalMonomialInner {
                coefficient,
                powers,
            }),
        }
    }

    /// 获取系数 / Get coefficient
    pub fn coefficient(&self) -> &V {
        &self.inner.coefficient
    }

    /// 获取幂次映射 / Get powers mapping
    pub fn powers(&self) -> &HashMap<usize, i32> {
        &self.inner.powers
    }

    /// 获取变量的幂次 / Get power of variable
    pub fn get_power(&self, var_index: usize) -> i32 {
        self.inner.powers.get(&var_index).copied().unwrap_or(0)
    }

    /// 缓存键 / Cache key
    pub fn cache_key(&self) -> CacheKey {
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }

    /// 总幂次 / Total power
    pub fn total_power(&self) -> i32 {
        self.inner.powers.values().sum()
    }
}

impl<V> Cacheable for CanonicalMonomial<V> {
    fn cache_key(&self) -> CacheKey {
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

/// 标准多项式内部数据 / Canonical Polynomial Inner Data
#[derive(Debug, Clone)]
pub struct CanonicalInner<V> {
    /// 单项式列表 / Monomial list
    pub monomials: Vec<CanonicalMonomial<V>>,
    /// 常数项 / Constant term
    pub constant: V,
}

/// 标准多项式 / Canonical Polynomial
///
/// 包装 Inner Box，Clone 为深拷贝。
/// Wrapping Inner Box, Clone performs deep copy.
#[derive(Debug, Clone)]
pub struct Canonical<V> {
    inner: Box<CanonicalInner<V>>,
}

impl<V> Canonical<V> {
    /// 创建新多项式 / Create new polynomial
    pub fn new(monomials: Vec<CanonicalMonomial<V>>, constant: V) -> Self {
        Self {
            inner: Box::new(CanonicalInner {
                monomials,
                constant,
            }),
        }
    }

    /// 创建常数多项式 / Create constant polynomial
    pub fn from_constant(constant: V) -> Self {
        Self {
            inner: Box::new(CanonicalInner {
                monomials: Vec::new(),
                constant,
            }),
        }
    }

    /// 创建空多项式（零）/ Create empty polynomial (zero)
    pub fn zero(zero: V) -> Self {
        Self {
            inner: Box::new(CanonicalInner {
                monomials: Vec::new(),
                constant: zero,
            }),
        }
    }

    /// 获取单项式列表 / Get monomial list
    pub fn monomials(&self) -> &[CanonicalMonomial<V>] {
        &self.inner.monomials
    }

    /// 获取常数项 / Get constant
    pub fn constant(&self) -> &V {
        &self.inner.constant
    }

    /// 获取内部引用 / Get inner reference
    pub fn inner(&self) -> &CanonicalInner<V> {
        &self.inner
    }

    /// 添加单项式 / Add monomial
    pub fn add_monomial(&mut self, monomial: CanonicalMonomial<V>) {
        self.inner.monomials.push(monomial);
    }

    /// 设置常数项 / Set constant
    pub fn set_constant(&mut self, constant: V) {
        self.inner.constant = constant;
    }

    /// 缓存键 / Cache key
    pub fn cache_key(&self) -> CacheKey {
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }

    /// 获取单项式数量 / Get monomial count
    pub fn len(&self) -> usize {
        self.inner.monomials.len()
    }

    /// 检查是否为空 / Check if empty
    pub fn is_empty(&self) -> bool {
        self.inner.monomials.is_empty()
    }
}

impl<V> Cacheable for Canonical<V> {
    fn cache_key(&self) -> CacheKey {
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_monomial() {
        let m = LinearMonomial::new(2.5, 3);
        assert_eq!(*m.coefficient(), 2.5);
        assert_eq!(m.var_index(), 3);
    }

    #[test]
    fn test_linear_polynomial() {
        let mut p = Linear::zero(0.0);
        p.add_monomial(LinearMonomial::new(1.0, 0));
        p.add_monomial(LinearMonomial::new(2.0, 1));
        p.set_constant_term(3.0);

        assert_eq!(p.len(), 2);
        assert_eq!(*p.constant_term(), 3.0);
    }

    #[test]
    fn test_quadratic_monomial() {
        let q = QuadraticMonomial::new_quadratic(3.0, 1, 2);
        assert_eq!(*q.coefficient(), 3.0);
        assert_eq!(q.var_index1(), 1);
        assert_eq!(q.var_index2(), Some(2));
        assert_eq!(q.kind(), QuadraticMonomialKind::Quadratic);

        let l = QuadraticMonomial::new_linear(2.0, 3);
        assert_eq!(l.kind(), QuadraticMonomialKind::Linear);
    }

    #[test]
    fn test_canonical_monomial() {
        let mut powers = HashMap::new();
        powers.insert(0, 2);
        powers.insert(1, 1);

        let m = CanonicalMonomial::new(5.0, powers);
        assert_eq!(*m.coefficient(), 5.0);
        assert_eq!(m.get_power(0), 2);
        assert_eq!(m.get_power(1), 1);
        assert_eq!(m.total_power(), 3);
    }
}
