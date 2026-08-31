//! 平展上下文 trait
//! Flatten Context Trait

use std::collections::HashMap;

use super::{Canonical, CanonicalMonomial, Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use crate::token::TokenList;
use crate::variable::VariableId;

// ============================================================================
// 平展结果类型
// Flatten Result Types
// ============================================================================

/// 平展后的单项式 / Flattened Monomial
#[derive(Debug, Clone)]
pub struct FlattenedMonomial<M> {
    /// 单项式 / Monomial
    pub monomial: M,
}

/// 平展后的多项式 / Flattened Polynomial
#[derive(Debug, Clone)]
pub struct FlattenedPolynomial<P> {
    /// 多项式 / Polynomial
    pub polynomial: P,
}

/// 平展后的符号 / Flattened Symbol
#[derive(Debug, Clone)]
pub struct FlattenedSymbol<P> {
    /// 多项式 / Polynomial
    pub polynomial: P,
}

// ============================================================================
// 平展上下文 trait
// Flatten Context Trait
// ============================================================================

/// 平展上下文 trait / Flatten Context Trait
///
/// 根据多项式类型区分不同的上下文，携带 TokenList 引用用于查询注册的变量。
/// Distinguishes different contexts by polynomial type, carries TokenList reference for querying registered variables.
///
/// # 类型参数 / Type Parameters
/// - `V`: 统一值类型 / Unified value type
pub trait FlattenContextTrait<V>: Send + Sync
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    /// 多项式类型 / Polynomial type
    type Polynomial;
    /// 单项式类型 / Monomial type
    type Monomial;

    /// 获取 TokenList 引用 / Get TokenList reference
    fn token_list(&self) -> &dyn TokenList<V>;

    /// 单项式缓存 / Monomial cache
    fn monomial_cache(&self) -> &HashMap<u64, FlattenedMonomial<Self::Monomial>>;
    fn monomial_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedMonomial<Self::Monomial>>;

    /// 多项式缓存 / Polynomial cache
    fn polynomial_cache(&self) -> &HashMap<u64, FlattenedPolynomial<Self::Polynomial>>;
    fn polynomial_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedPolynomial<Self::Polynomial>>;

    /// 中间符号缓存 / Intermediate symbol cache
    fn symbol_cache(&self) -> &HashMap<u64, FlattenedSymbol<Self::Polynomial>>;
    fn symbol_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedSymbol<Self::Polynomial>>;

    /// 检查变量是否已注册 / Check if variable is registered
    fn is_registered(&self, var_id: VariableId) -> bool {
        self.token_list().find_by_id(var_id).is_some()
    }

    /// 通过 ID 查找 Token 索引 / Find token index by ID
    fn find_token_index(&self, var_id: VariableId) -> Option<usize> {
        self.token_list().find_by_id(var_id).map(|t| t.solver_index)
    }

    /// 清除所有缓存 / Clear all caches
    fn clear(&mut self) {
        self.monomial_cache_mut().clear();
        self.polynomial_cache_mut().clear();
        self.symbol_cache_mut().clear();
    }

    /// 清除指定单项式的缓存 / Clear cache for specific monomial
    fn clear_monomial(&mut self, id: u64) -> bool {
        self.monomial_cache_mut().remove(&id).is_some()
    }

    /// 清除指定多项式的缓存 / Clear cache for specific polynomial
    fn clear_polynomial(&mut self, id: u64) -> bool {
        self.polynomial_cache_mut().remove(&id).is_some()
    }

    /// 清除指定中间符号的缓存 / Clear cache for specific intermediate symbol
    fn clear_symbol(&mut self, id: u64) -> bool {
        self.symbol_cache_mut().remove(&id).is_some()
    }

    /// 批量清除指定项的缓存 / Clear cache for multiple items
    fn clear_items(&mut self, ids: &[u64]) {
        for id in ids {
            self.monomial_cache_mut().remove(id);
            self.polynomial_cache_mut().remove(id);
            self.symbol_cache_mut().remove(id);
        }
    }

    /// 缓存大小 / Cache size
    fn cache_size(&self) -> usize {
        self.monomial_cache().len() + self.polynomial_cache().len() + self.symbol_cache().len()
    }

    /// 检查缓存是否为空 / Check if cache is empty
    fn is_cache_empty(&self) -> bool {
        self.monomial_cache().is_empty()
            && self.polynomial_cache().is_empty()
            && self.symbol_cache().is_empty()
    }
}

// ============================================================================
// 线性平展上下文 trait
// Linear Flatten Context Trait
// ============================================================================

/// 线性平展上下文 trait / Linear Flatten Context Trait
pub trait LinearFlattenContext<V>:
    FlattenContextTrait<V, Polynomial = Linear<V>, Monomial = LinearMonomial<V>>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
}

// ============================================================================
// 二次平展上下文 trait
// Quadratic Flatten Context Trait
// ============================================================================

/// 二次平展上下文 trait / Quadratic Flatten Context Trait
pub trait QuadraticFlattenContext<V>:
    FlattenContextTrait<V, Polynomial = Quadratic<V>, Monomial = QuadraticMonomial<V>>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
}

// ============================================================================
// 标准平展上下文 trait
// Canonical Flatten Context Trait
// ============================================================================

/// 标准平展上下文 trait / Canonical Flatten Context Trait
pub trait CanonicalFlattenContext<V>:
    FlattenContextTrait<V, Polynomial = Canonical<V>, Monomial = CanonicalMonomial<V>>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    // 单元测试将在实现 lazy_context 后添加
}
