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
    use super::*;
    use crate::model::flatten::LazyLinearFlattenContext;
    use crate::symbol::flatten::{Linear, LinearMonomial};
    use crate::token::VecTokenList;
    use std::sync::Arc;

    /// 被测上下文类型别名 / Context type alias under test.
    type Context = LazyLinearFlattenContext<f64, VecTokenList<f64>>;

    /// 构建已初始化的线性平展上下文 / Build an initialized linear flatten context.
    fn context() -> Context {
        let context = Context::new();
        context.init(Arc::new(VecTokenList::new()));
        context
    }

    /// 线性多项式辅助构造 / Helper to build a linear polynomial.
    fn linear(coefficient: f64, var_index: usize, constant: f64) -> Linear<f64> {
        Linear::new(vec![LinearMonomial::new(coefficient, var_index)], constant)
    }

    /// 结构化比较线性多项式。
    ///
    /// `Linear` 以堆地址作为身份（`Cacheable`），因此不实现 `PartialEq`；
    /// 这里按单项式与常数项逐项比较，才能断言"数值相等"。
    ///
    /// Structurally compare linear polynomials. `Linear` carries address identity
    /// (`Cacheable`) and therefore does not implement `PartialEq`; comparing terms and
    /// the constant is what actually asserts numeric equality.
    fn assert_linear_eq(actual: &Linear<f64>, expected: &Linear<f64>) {
        assert_eq!(
            actual.constant_term(),
            expected.constant_term(),
            "constant term differs"
        );
        let actual_terms: Vec<(usize, f64)> = actual
            .monomials()
            .iter()
            .map(|m| (m.var_index(), *m.coefficient()))
            .collect();
        let expected_terms: Vec<(usize, f64)> = expected
            .monomials()
            .iter()
            .map(|m| (m.var_index(), *m.coefficient()))
            .collect();
        assert_eq!(actual_terms, expected_terms, "monomial terms differ");
    }

    /// 可平展的测试替身：带稳定标识、可计数、可声明依赖。
    /// Flattenable stand-in: stable identifier, call counter, and declared dependencies.
    struct Probe {
        id: u64,
        dependencies: Vec<u64>,
        calls: std::cell::Cell<usize>,
    }

    impl Probe {
        fn new(id: u64) -> Self {
            Self {
                id,
                dependencies: Vec::new(),
                calls: std::cell::Cell::new(0),
            }
        }

        fn with_dependencies(id: u64, dependencies: Vec<u64>) -> Self {
            Self {
                id,
                dependencies,
                calls: std::cell::Cell::new(0),
            }
        }

        fn calls(&self) -> usize {
            self.calls.get()
        }
    }

    impl Identified for Probe {
        fn identifier(&self) -> u64 {
            self.id
        }
    }

    impl Flattenable<Context, f64> for Probe {
        fn flatten(&self, _ctx: &mut Context) -> Linear<f64> {
            self.calls.set(self.calls.get() + 1);
            linear(1.0, self.id as usize, 0.0)
        }
    }

    impl FlattenableWithDependencies<Context, f64> for Probe {
        fn dependencies(&self) -> Vec<u64> {
            self.dependencies.clone()
        }
    }

    #[test]
    fn cached_flatten_evaluates_only_once() {
        // 缓存命中后不得重复平展：平展是热路径，重复执行会破坏缓存的意义。
        // A cache hit must not re-flatten; flattening is a hot path and repeating it
        // defeats the purpose of the cache.
        let mut ctx = context();
        let probe = Probe::new(101);

        let first = probe.flatten_cached(&mut ctx);
        let second = probe.flatten_cached(&mut ctx);

        assert_eq!(probe.calls(), 1, "第二次调用必须命中缓存");
        assert_linear_eq(&first, &second);
        assert!(ctx.polynomial_cache().contains_key(&101));
    }

    #[test]
    fn force_flatten_bypasses_the_cache_and_refreshes_it() {
        // flatten_force 必须无视既有缓存重新计算，并覆盖缓存内容。
        // flatten_force must recompute regardless of the cache and overwrite it.
        let mut ctx = context();
        let probe = Probe::new(102);

        probe.flatten_cached(&mut ctx);
        assert_eq!(probe.calls(), 1);

        let forced = probe.flatten_force(&mut ctx);

        assert_eq!(probe.calls(), 2, "force 必须真正重新平展");
        assert_linear_eq(&forced, &linear(1.0, 102, 0.0));
        assert!(ctx.polynomial_cache().contains_key(&102));
    }

    #[test]
    fn distinct_identifiers_do_not_alias() {
        // 不同标识必须占用独立缓存槽，否则会静默复用错误的平展结果。
        // Distinct identifiers must occupy separate cache slots; otherwise a wrong
        // flatten result is silently reused.
        let mut ctx = context();
        let first = Probe::new(201);
        let second = Probe::new(202);

        first.flatten_cached(&mut ctx);
        second.flatten_cached(&mut ctx);
        first.flatten_cached(&mut ctx);
        second.flatten_cached(&mut ctx);

        assert_eq!(first.calls(), 1);
        assert_eq!(second.calls(), 1);
        assert_eq!(ctx.polynomial_cache().len(), 2);
        assert_linear_eq(&ctx.polynomial_cache().get(&201).expect("cached").polynomial, &linear(1.0, 201, 0.0));
        assert_linear_eq(&ctx.polynomial_cache().get(&202).expect("cached").polynomial, &linear(1.0, 202, 0.0));
    }

    #[test]
    fn dependency_readiness_requires_every_dependency() {
        // 只要缺一个依赖就不得视为"依赖齐备"，否则会基于不完整的缓存平展。
        // A single missing dependency must make readiness false; otherwise flattening
        // proceeds on an incomplete cache.
        let mut ctx = context();
        let empty = Probe::with_dependencies(301, Vec::new());
        let two = Probe::with_dependencies(302, vec![401, 402]);

        assert!(empty.are_dependencies_cached(&ctx), "无依赖视为恒齐备");

        assert!(!two.are_dependencies_cached(&ctx));
        ctx.polynomial_cache_mut().insert(
            401,
            FlattenedPolynomial {
                polynomial: linear(1.0, 401, 0.0),
            },
        );
        assert!(!two.are_dependencies_cached(&ctx), "仅一半依赖不得视为齐备");

        ctx.polynomial_cache_mut().insert(
            402,
            FlattenedPolynomial {
                polynomial: linear(1.0, 402, 0.0),
            },
        );
        assert!(two.are_dependencies_cached(&ctx), "依赖齐备后才可平展");
    }

    #[test]
    fn clearing_dependencies_drops_every_dependency_entry() {
        // 依赖失效必须一次性清掉全部依赖条目，且不得误伤无关条目。
        // Dependency invalidation must drop every dependency entry and leave
        // unrelated entries untouched.
        let mut ctx = context();
        let probe = Probe::with_dependencies(501, vec![601, 602]);

        for id in [601u64, 602u64, 999u64] {
            ctx.polynomial_cache_mut().insert(
                id,
                FlattenedPolynomial {
                    polynomial: linear(1.0, id as usize, 0.0),
                },
            );
        }
        assert_eq!(ctx.polynomial_cache().len(), 3);

        probe.clear_dependency_caches(&mut ctx);

        assert_eq!(ctx.polynomial_cache().len(), 1, "只应保留无关条目 999");
        assert!(!ctx.polynomial_cache().contains_key(&601));
        assert!(!ctx.polynomial_cache().contains_key(&602));
        assert!(ctx.polynomial_cache().contains_key(&999));
        assert!(!probe.are_dependencies_cached(&ctx), "清理后依赖必须变为未就绪");
    }

    #[test]
    fn clearing_dependencies_is_a_no_op_without_dependencies() {
        // 无依赖对象清理不得影响任何缓存条目。
        // Clearing a dependency-free object must not disturb any cache entry.
        let mut ctx = context();
        let probe = Probe::with_dependencies(701, Vec::new());
        ctx.polynomial_cache_mut().insert(
            801,
            FlattenedPolynomial {
                polynomial: linear(1.0, 801, 0.0),
            },
        );

        probe.clear_dependency_caches(&mut ctx);

        assert_eq!(ctx.polynomial_cache().len(), 1);
    }
}
