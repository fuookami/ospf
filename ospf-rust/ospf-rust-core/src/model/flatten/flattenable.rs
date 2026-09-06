//! 可平展 trait
//! Flattenable Trait

use super::{FlattenContextTrait, FlattenedPolynomial};

// ============================================================================
// Identified Trait
// ============================================================================

/// 可标识的 trait / Identified Trait
///
/// 拥有唯一标识符的对象可实现此 trait。
/// Objects with unique identifiers can implement this trait.
pub trait Identified {
    /// 获取唯一标识符 / Get unique identifier
    fn identifier(&self) -> u64;
}

// ============================================================================
// Flattenable Trait
// ============================================================================

/// 可平展的 trait / Flattenable Trait
///
/// 平展操作将中间符号展开为纯变量表达式。
/// Flatten operation expands intermediate symbols into pure variable expressions.
///
/// # 设计说明 / Design Notes
///
/// 注意：函数中间符号的辅助变量和约束注册由 `FunctionSymbol` trait 处理，
/// 不在 `Flattenable` 中进行回调。
///
/// Note: Auxiliary variable and constraint registration for function symbols
/// is handled by `FunctionSymbol` trait, not via callbacks in `Flattenable`.
///
/// # 类型参数 / Type Parameters
/// - `C`: 平展上下文类型 / Flatten context type
/// - `V`: 值类型 / Value type
pub trait Flattenable<C: FlattenContextTrait<V>, V>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
    C::Polynomial: Clone,
    C::Monomial: Clone,
{
    /// 平展为纯变量表达式 / Flatten to pure variable expression
    fn flatten(&self, ctx: &mut C) -> C::Polynomial;

    /// 带缓存的平展 / Flatten with cache
    ///
    /// 如果已缓存则直接返回，否则执行平展并缓存结果。
    /// Returns cached result if available, otherwise flattens and caches result.
    fn flatten_cached(&self, ctx: &mut C) -> C::Polynomial
    where
        Self: Identified,
    {
        let id = self.identifier();
        if let Some(cached) = ctx.polynomial_cache().get(&id) {
            return cached.polynomial.clone();
        }
        let result = self.flatten(ctx);
        ctx.polynomial_cache_mut().insert(
            id,
            FlattenedPolynomial {
                polynomial: result.clone(),
            },
        );
        result
    }

    /// 强制重新平展并更新缓存 / Force re-flatten and update cache
    fn flatten_force(&self, ctx: &mut C) -> C::Polynomial
    where
        Self: Identified,
    {
        let id = self.identifier();
        let result = self.flatten(ctx);
        ctx.polynomial_cache_mut().insert(
            id,
            FlattenedPolynomial {
                polynomial: result.clone(),
            },
        );
        result
    }
}

// ============================================================================
// FlattenableWithDependencies Trait
// ============================================================================

/// 带依赖的可平展 trait / Flattenable with Dependencies Trait
///
/// 支持依赖感知的平展操作。
/// Supports dependency-aware flattening operations.
pub trait FlattenableWithDependencies<C: FlattenContextTrait<V>, V>: Flattenable<C, V>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
    C::Polynomial: Clone,
    C::Monomial: Clone,
{
    /// 获取依赖的其他符号标识符 / Get identifiers of dependent symbols
    fn dependencies(&self) -> Vec<u64>;

    /// 检查依赖是否都已缓存 / Check if all dependencies are cached
    fn are_dependencies_cached(&self, ctx: &C) -> bool {
        self.dependencies()
            .iter()
            .all(|id| ctx.polynomial_cache().contains_key(id))
    }

    /// 清除依赖缓存 / Clear dependency caches
    fn clear_dependency_caches(&self, ctx: &mut C) {
        for id in self.dependencies() {
            ctx.clear_polynomial(id);
        }
    }
}

// ============================================================================
// FlattenableMonomial Trait
// ============================================================================

/// 可平展单项式 trait / Flattenable Monomial Trait
///
/// 单项式平展操作。
/// Monomial flattening operations.
pub trait FlattenableMonomial<C: FlattenContextTrait<V>, V>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
    C::Monomial: Clone,
{
    /// 平展为单项式 / Flatten to monomial
    fn flatten_to_monomial(&self, ctx: &mut C) -> C::Monomial;

    /// 带缓存的平展单项式 / Flatten monomial with cache
    fn flatten_to_monomial_cached(&self, ctx: &mut C) -> C::Monomial
    where
        Self: Identified,
    {
        let id = self.identifier();
        if let Some(cached) = ctx.monomial_cache().get(&id) {
            return cached.monomial.clone();
        }
        let result = self.flatten_to_monomial(ctx);
        ctx.monomial_cache_mut().insert(
            id,
            super::FlattenedMonomial {
                monomial: result.clone(),
            },
        );
        result
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    // 单元测试将在实现 lazy_context 后添加
}
