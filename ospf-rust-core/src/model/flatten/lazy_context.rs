//! 懒加载平展上下文
//! Lazy Flatten Context

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, OnceLock};
use super::{

    Canonical, CanonicalFlattenContext, CanonicalMonomial, FlattenContextTrait, FlattenedMonomial,
    FlattenedPolynomial, FlattenedSymbol, Linear, LinearFlattenContext, LinearMonomial, Quadratic,
    QuadraticFlattenContext, QuadraticMonomial,
};
use crate::token::TokenList;

// ============================================================================
// 懒加载线性平展上下文
// Lazy Linear Flatten Context
// ============================================================================

/// 懒加载的线性平展上下文 / Lazy Linear Flatten Context
///
/// 生命周期与 MetaModel 一致，但延迟初始化直到 token_list 可用。
/// Lifecycle matches MetaModel, but lazy initialization until token_list is available.
///
/// # 设计说明 / Design Notes
///
/// - 使用 `OnceLock` 延迟初始化 TokenList 引用
/// - 支持缓存单项式、多项式和中间符号的平展结果
/// - 可按类型或对象清除缓存
#[derive(Debug)]
pub struct LazyLinearFlattenContext<V, T: TokenList<V>>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    /// TokenList 引用（延迟初始化）/ TokenList reference (lazy initialization)
    token_list: OnceLock<Arc<T>>,
    /// 单项式缓存 / Monomial cache
    monomial_cache: HashMap<u64, FlattenedMonomial<LinearMonomial<V>>>,
    /// 多项式缓存 / Polynomial cache
    polynomial_cache: HashMap<u64, FlattenedPolynomial<Linear<V>>>,
    /// 中间符号缓存 / Intermediate symbol cache
    symbol_cache: HashMap<u64, FlattenedSymbol<Linear<V>>>,
    /// 值类型标记 / Value type marker
    _value: PhantomData<V>,
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static, T: TokenList<V>>
    LazyLinearFlattenContext<V, T>
{
    /// 创建未初始化的上下文 / Create uninitialized context
    pub fn new() -> Self {
        Self {
            token_list: OnceLock::new(),
            monomial_cache: HashMap::new(),
            polynomial_cache: HashMap::new(),
            symbol_cache: HashMap::new(),
            _value: PhantomData,
        }
    }

    /// 初始化（设置 token_list 引用）/ Initialize (set token_list reference)
    ///
    /// 只能调用一次，后续调用将被忽略。
    /// Can only be called once, subsequent calls will be ignored.
    pub fn init(&self, token_list: Arc<T>) {
        let _ = self.token_list.set(token_list);
    }

    /// 检查是否已初始化 / Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.token_list.get().is_some()
    }

    /// 获取 token_list（需确保已初始化）/ Get token_list (must be initialized)
    pub fn token_list_ref(&self) -> Option<&T> {
        self.token_list.get().map(|arc| arc.as_ref())
    }
}

impl<V, T: TokenList<V>> FlattenContextTrait<V> for LazyLinearFlattenContext<V, T>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    type Polynomial = Linear<V>;
    type Monomial = LinearMonomial<V>;

    fn token_list(&self) -> &dyn TokenList<V> {
        self.token_list
            .get()
            .expect("FlattenContext not initialized. Call init() first.")
            .as_ref()
    }

    fn monomial_cache(&self) -> &HashMap<u64, FlattenedMonomial<Self::Monomial>> {
        &self.monomial_cache
    }

    fn monomial_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedMonomial<Self::Monomial>> {
        &mut self.monomial_cache
    }

    fn polynomial_cache(&self) -> &HashMap<u64, FlattenedPolynomial<Self::Polynomial>> {
        &self.polynomial_cache
    }

    fn polynomial_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedPolynomial<Self::Polynomial>> {
        &mut self.polynomial_cache
    }

    fn symbol_cache(&self) -> &HashMap<u64, FlattenedSymbol<Self::Polynomial>> {
        &self.symbol_cache
    }

    fn symbol_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedSymbol<Self::Polynomial>> {
        &mut self.symbol_cache
    }
}

impl<V, T: TokenList<V>> LinearFlattenContext<V> for LazyLinearFlattenContext<V, T> where
    V: Clone + std::fmt::Debug + Send + Sync + 'static
{
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static, T: TokenList<V>> Default
    for LazyLinearFlattenContext<V, T>
{
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 懒加载二次平展上下文
// Lazy Quadratic Flatten Context
// ============================================================================

/// 懒加载的二次平展上下文 / Lazy Quadratic Flatten Context
#[derive(Debug)]
pub struct LazyQuadraticFlattenContext<V, T: TokenList<V>>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    /// TokenList 引用（延迟初始化）/ TokenList reference (lazy initialization)
    token_list: OnceLock<Arc<T>>,
    /// 单项式缓存 / Monomial cache
    monomial_cache: HashMap<u64, FlattenedMonomial<QuadraticMonomial<V>>>,
    /// 多项式缓存 / Polynomial cache
    polynomial_cache: HashMap<u64, FlattenedPolynomial<Quadratic<V>>>,
    /// 中间符号缓存 / Intermediate symbol cache
    symbol_cache: HashMap<u64, FlattenedSymbol<Quadratic<V>>>,
    /// 值类型标记 / Value type marker
    _value: PhantomData<V>,
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static, T: TokenList<V>>
    LazyQuadraticFlattenContext<V, T>
{
    /// 创建未初始化的上下文 / Create uninitialized context
    pub fn new() -> Self {
        Self {
            token_list: OnceLock::new(),
            monomial_cache: HashMap::new(),
            polynomial_cache: HashMap::new(),
            symbol_cache: HashMap::new(),
            _value: PhantomData,
        }
    }

    /// 初始化（设置 token_list 引用）/ Initialize (set token_list reference)
    pub fn init(&self, token_list: Arc<T>) {
        let _ = self.token_list.set(token_list);
    }

    /// 检查是否已初始化 / Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.token_list.get().is_some()
    }
}

impl<V, T: TokenList<V>> FlattenContextTrait<V> for LazyQuadraticFlattenContext<V, T>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    type Polynomial = Quadratic<V>;
    type Monomial = QuadraticMonomial<V>;

    fn token_list(&self) -> &dyn TokenList<V> {
        self.token_list
            .get()
            .expect("FlattenContext not initialized. Call init() first.")
            .as_ref()
    }

    fn monomial_cache(&self) -> &HashMap<u64, FlattenedMonomial<Self::Monomial>> {
        &self.monomial_cache
    }

    fn monomial_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedMonomial<Self::Monomial>> {
        &mut self.monomial_cache
    }

    fn polynomial_cache(&self) -> &HashMap<u64, FlattenedPolynomial<Self::Polynomial>> {
        &self.polynomial_cache
    }

    fn polynomial_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedPolynomial<Self::Polynomial>> {
        &mut self.polynomial_cache
    }

    fn symbol_cache(&self) -> &HashMap<u64, FlattenedSymbol<Self::Polynomial>> {
        &self.symbol_cache
    }

    fn symbol_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedSymbol<Self::Polynomial>> {
        &mut self.symbol_cache
    }
}

impl<V, T: TokenList<V>> QuadraticFlattenContext<V> for LazyQuadraticFlattenContext<V, T> where
    V: Clone + std::fmt::Debug + Send + Sync + 'static
{
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static, T: TokenList<V>> Default
    for LazyQuadraticFlattenContext<V, T>
{
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 懒加载标准平展上下文
// Lazy Canonical Flatten Context
// ============================================================================

/// 懒加载的标准平展上下文 / Lazy Canonical Flatten Context
#[derive(Debug)]
pub struct LazyCanonicalFlattenContext<V, T: TokenList<V>>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    /// TokenList 引用（延迟初始化）/ TokenList reference (lazy initialization)
    token_list: OnceLock<Arc<T>>,
    /// 单项式缓存 / Monomial cache
    monomial_cache: HashMap<u64, FlattenedMonomial<CanonicalMonomial<V>>>,
    /// 多项式缓存 / Polynomial cache
    polynomial_cache: HashMap<u64, FlattenedPolynomial<Canonical<V>>>,
    /// 中间符号缓存 / Intermediate symbol cache
    symbol_cache: HashMap<u64, FlattenedSymbol<Canonical<V>>>,
    /// 值类型标记 / Value type marker
    _value: PhantomData<V>,
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static, T: TokenList<V>>
    LazyCanonicalFlattenContext<V, T>
{
    /// 创建未初始化的上下文 / Create uninitialized context
    pub fn new() -> Self {
        Self {
            token_list: OnceLock::new(),
            monomial_cache: HashMap::new(),
            polynomial_cache: HashMap::new(),
            symbol_cache: HashMap::new(),
            _value: PhantomData,
        }
    }

    /// 初始化（设置 token_list 引用）/ Initialize (set token_list reference)
    pub fn init(&self, token_list: Arc<T>) {
        let _ = self.token_list.set(token_list);
    }

    /// 检查是否已初始化 / Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.token_list.get().is_some()
    }
}

impl<V, T: TokenList<V>> FlattenContextTrait<V> for LazyCanonicalFlattenContext<V, T>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    type Polynomial = Canonical<V>;
    type Monomial = CanonicalMonomial<V>;

    fn token_list(&self) -> &dyn TokenList<V> {
        self.token_list
            .get()
            .expect("FlattenContext not initialized. Call init() first.")
            .as_ref()
    }

    fn monomial_cache(&self) -> &HashMap<u64, FlattenedMonomial<Self::Monomial>> {
        &self.monomial_cache
    }

    fn monomial_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedMonomial<Self::Monomial>> {
        &mut self.monomial_cache
    }

    fn polynomial_cache(&self) -> &HashMap<u64, FlattenedPolynomial<Self::Polynomial>> {
        &self.polynomial_cache
    }

    fn polynomial_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedPolynomial<Self::Polynomial>> {
        &mut self.polynomial_cache
    }

    fn symbol_cache(&self) -> &HashMap<u64, FlattenedSymbol<Self::Polynomial>> {
        &self.symbol_cache
    }

    fn symbol_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedSymbol<Self::Polynomial>> {
        &mut self.symbol_cache
    }
}

impl<V, T: TokenList<V>> CanonicalFlattenContext<V> for LazyCanonicalFlattenContext<V, T> where
    V: Clone + std::fmt::Debug + Send + Sync + 'static
{
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static, T: TokenList<V>> Default
    for LazyCanonicalFlattenContext<V, T>
{
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 类型别名 / Type Aliases
// ============================================================================

/// f64 精度的线性平展上下文 / Linear flatten context with f64 precision
pub type F64LinearFlattenContext<T> = LazyLinearFlattenContext<f64, T>;

/// f64 精度的二次平展上下文 / Quadratic flatten context with f64 precision
pub type F64QuadraticFlattenContext<T> = LazyQuadraticFlattenContext<f64, T>;

/// f64 精度的标准平展上下文 / Canonical flatten context with f64 precision
pub type F64CanonicalFlattenContext<T> = LazyCanonicalFlattenContext<f64, T>;

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_linear_flatten_context_creation() {
        let ctx: LazyLinearFlattenContext<f64, crate::token::VecTokenList<f64>> =
            LazyLinearFlattenContext::new();
        assert!(!ctx.is_initialized());
    }

    #[test]
    fn test_lazy_quadratic_flatten_context_creation() {
        let ctx: LazyQuadraticFlattenContext<f64, crate::token::VecTokenList<f64>> =
            LazyQuadraticFlattenContext::new();
        assert!(!ctx.is_initialized());
    }

    #[test]
    fn test_lazy_canonical_flatten_context_creation() {
        let ctx: LazyCanonicalFlattenContext<f64, crate::token::VecTokenList<f64>> =
            LazyCanonicalFlattenContext::new();
        assert!(!ctx.is_initialized());
    }
}
