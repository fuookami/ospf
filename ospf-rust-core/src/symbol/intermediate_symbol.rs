//! 中间符号特征定义 / Intermediate symbol trait definitions.

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use ospf_rust_math::symbol::Symbol;

use crate::error::Result;
use crate::model::{
    LinearConstraint, QuadraticConstraint, RangeCacheContextTrait, RangeCacheKey,
    ValueCacheContextTrait, ValueCacheKey,
};
use crate::token::{Token, TokenList};
use crate::variable::VariableRange;

/// 自动中间符号 ID 的起始值，使用较高命名空间以降低与显式 ID 冲突的概率。
/// The starting value for auto intermediate symbol IDs, using a high namespace
/// to reduce collision risk with explicit IDs.
const AUTO_INTERMEDIATE_SYMBOL_ID_START: u64 = 1_000_000_000;

/// 下一个自动中间符号 ID 的原子计数器。
/// Atomic counter for the next auto intermediate symbol ID.
static NEXT_AUTO_INTERMEDIATE_SYMBOL_ID: AtomicU64 =
    AtomicU64::new(AUTO_INTERMEDIATE_SYMBOL_ID_START);

/// 生成自动中间符号 ID，使用较高命名空间以降低与显式 ID 冲突的概率。
/// Generate an auto intermediate symbol id from a high namespace to reduce collision risk with explicit ids.
pub fn next_auto_intermediate_symbol_id() -> u64 {
    NEXT_AUTO_INTERMEDIATE_SYMBOL_ID.fetch_add(1, Ordering::Relaxed)
}

/// 生成自动中间符号名称，保持名称可读并包含唯一 ID。
/// Generate a readable auto intermediate symbol name that includes the unique id.
pub(crate) fn auto_intermediate_symbol_name(prefix: &str, id: u64) -> String {
    format!("{}_{}", prefix, id)
}

/// 中间符号唯一标识符 / Unique identifier for intermediate symbols.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IntermediateSymbolId {
    /// 符号的数值 ID / Numeric ID of the symbol.
    pub id: u64,
    /// 符号的名称 / Name of the symbol.
    pub name: String,
}

impl IntermediateSymbolId {
    /// 使用指定的 ID 和名称创建标识符 / Create an identifier with the given ID and name.
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }

    /// 仅使用数值 ID 创建标识符，名称自动生成为 `sym_{id}` / Create an identifier from a numeric ID only; the name defaults to `sym_{id}`.
    pub fn from_id(id: u64) -> Self {
        Self {
            id,
            name: format!("sym_{}", id),
        }
    }
}

/// 符号类别 / Symbol category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// 线性符号 / Linear symbol.
    Linear,
    /// 二次符号 / Quadratic symbol.
    Quadratic,
    /// 多项式符号 / Polynomial symbol.
    Polynomial,
    /// 非线性符号 / Nonlinear symbol.
    Nonlinear,
}

/// 中间符号的值求值上下文 / Value-evaluation context for intermediate symbols.
pub struct IntermediateSymbolEvalContext<'a, V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 变量值映射（变量索引 → 值）/ Variable value mapping (variable index → value).
    pub values: &'a HashMap<usize, V>,
    /// 值缓存上下文 / Value cache context.
    pub cache: &'a mut dyn ValueCacheContextTrait<V>,
}

impl<'a, V> IntermediateSymbolEvalContext<'a, V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的求值上下文 / Create a new evaluation context.
    pub fn new(
        values: &'a HashMap<usize, V>,
        cache: &'a mut dyn ValueCacheContextTrait<V>,
    ) -> Self {
        Self { values, cache }
    }
}

/// 中间符号的范围推断上下文 / Range-inference context for intermediate symbols.
pub struct IntermediateSymbolRangeContext<'a, V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 范围缓存上下文 / Range cache context.
    pub cache: &'a mut dyn RangeCacheContextTrait<V>,
}

impl<'a, V> IntermediateSymbolRangeContext<'a, V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的范围推断上下文 / Create a new range-inference context.
    pub fn new(cache: &'a mut dyn RangeCacheContextTrait<V>) -> Self {
        Self { cache }
    }
}

/// 核心中间符号特征 / Core intermediate symbol trait.
pub trait IntermediateSymbol<V = f64>: Symbol<Id = IntermediateSymbolId> + Send + Sync
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 返回符号的类别 / Return the category of the symbol.
    fn category(&self) -> Category;

    /// 返回符号的操作类别，默认与 `category` 相同 / Return the operation category; defaults to `category`.
    fn operation_category(&self) -> Category {
        self.category()
    }

    /// 返回是否已缓存值 / Return whether the symbol has a cached value.
    fn cached(&self) -> bool;

    /// 返回父符号引用，默认为 `None` / Return the parent symbol reference; defaults to `None`.
    fn parent(&self) -> Option<&dyn IntermediateSymbol<V>> {
        None
    }

    /// 返回所有依赖的中间符号集合 / Return the set of all dependent intermediate symbols.
    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>>;

    /// 可选的声明依赖 ID（模型级显式依赖图），默认为空。
    /// Optional declared dependency IDs (model-level explicit dependency graph). Defaults to empty.
    fn declared_dependency_ids(&self) -> Vec<u64> {
        Vec::new()
    }

    /// 刷新缓存，`force` 为 true 时强制清除 / Flush the cache; when `force` is true, clear unconditionally.
    fn flush(&self, force: bool);

    /// 注册辅助令牌（用于函数符号），默认实现不做任何操作。
    /// Register auxiliary tokens (for function symbols). Default implementation does nothing.
    fn register_auxiliary_tokens(&self, _tokens: &mut Vec<Token<V>>) -> Result<()> {
        Ok(())
    }

    /// 构建该符号生成的机制层线性约束，默认实现不生成任何约束。
    /// Build mechanism-layer linear constraints generated by this symbol.
    /// Default implementation emits no constraints.
    fn mechanism_constraints(
        &self,
        _symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        Ok(Vec::new())
    }

    /// 构建带令牌上下文的机制层线性约束，默认委托给 `mechanism_constraints`。
    /// Build mechanism-layer linear constraints with token context.
    /// The default implementation delegates to `mechanism_constraints`.
    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let _ = tokens;
        self.mechanism_constraints(symbol_to_index)
    }

    /// 构建该符号生成的机制层二次约束，默认实现不生成任何约束。
    /// Build mechanism-layer quadratic constraints generated by this symbol.
    /// Default implementation emits no constraints.
    fn quadratic_mechanism_constraints(
        &self,
        _symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        Ok(Vec::new())
    }

    /// 构建带令牌上下文的机制层二次约束，默认委托给 `quadratic_mechanism_constraints`。
    /// Build mechanism-layer quadratic constraints with token context.
    /// The default implementation delegates to `quadratic_mechanism_constraints`.
    fn quadratic_mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let _ = tokens;
        self.quadratic_mechanism_constraints(symbol_to_index)
    }

    /// 从令牌表上下文求值符号（用于函数符号），默认返回 `None` 并回退到 `prepare(values)`。
    /// Evaluate symbol from token-table context (for function symbols).
    /// Default implementation returns `None` and falls back to `prepare(values)`.
    fn evaluate_from_tokens(
        &self,
        _token_table: &dyn TokenList<V>,
        _zero_if_none: bool,
    ) -> Option<V> {
        None
    }

    /// 根据变量值映射预计算符号值 / Pre-compute the symbol value from the variable value mapping.
    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V>;

    /// 返回求值缓存键 / Return the evaluation cache key.
    fn evaluation_cache_key(&self) -> ValueCacheKey {
        ValueCacheKey::from_symbol(self.id().id)
    }

    /// 上下文驱动的求值入口，默认实现桥接到 `evaluate_with_ctx(values, cache)`。
    /// Context-driven evaluation entry.
    /// Default implementation bridges to legacy `evaluate_with_ctx(values, cache)`.
    fn evaluate(&self, ctx: &mut IntermediateSymbolEvalContext<'_, V>) -> Option<V> {
        self.evaluate_with_ctx(ctx.values, ctx.cache)
    }

    /// 使用缓存上下文求值，先检查缓存再预热依赖 / Evaluate with cache context; checks cache first, then preheats dependencies.
    fn evaluate_with_ctx(
        &self,
        values: &HashMap<usize, V>,
        ctx: &mut dyn ValueCacheContextTrait<V>,
    ) -> Option<V> {
        let key = self.evaluation_cache_key();
        if let Some(cached) = ctx.get(key) {
            return Some(cached.clone());
        }

        // 在求值当前符号之前预热依赖值 / Preheat dependency values before evaluating current symbol.
        let mut visited_dependency_ids = HashSet::new();
        let self_id = self.id().id;
        for dependency in self.dependencies() {
            let dependency_id = dependency.id().id;
            if dependency_id == self_id || !visited_dependency_ids.insert(dependency_id) {
                continue;
            }
            let _ = dependency.evaluate_with_ctx(values, ctx);
        }

        let value = self
            .evaluate_from_tokens(ctx.token_list(), false)
            .or_else(|| self.prepare(values));
        if let Some(v) = value.clone() {
            ctx.set(key, v);
        }
        value
    }

    /// 返回符号的值范围，默认为 `None` / Return the value range of the symbol; defaults to `None`.
    fn range(&self) -> Option<VariableRange<V>> {
        None
    }

    /// 返回范围缓存键 / Return the range cache key.
    fn range_cache_key(&self) -> RangeCacheKey {
        RangeCacheKey::from_symbol(self.id().id)
    }

    /// 上下文驱动的范围推断入口，默认实现桥接到 `range_with_ctx(cache)`。
    /// Context-driven range entry.
    /// Default implementation bridges to legacy `range_with_ctx(cache)`.
    fn range_in_context(
        &self,
        ctx: &mut IntermediateSymbolRangeContext<'_, V>,
    ) -> Option<VariableRange<V>> {
        self.range_with_ctx(ctx.cache)
    }

    /// 使用缓存上下文推断范围，先检查缓存再计算 / Infer range with cache context; checks cache first, then computes.
    fn range_with_ctx(&self, ctx: &mut dyn RangeCacheContextTrait<V>) -> Option<VariableRange<V>> {
        let key = self.range_cache_key();
        if let Some(cached) = ctx.get(key) {
            return Some(cached.clone());
        }

        let range = self.range();
        if let Some(r) = range.clone() {
            ctx.set(key, r);
        }
        range
    }

    /// 将符号转换为原始字符串表示，`unfold` 控制展开深度 / Convert the symbol to a raw string representation; `unfold` controls the unfolding depth.
    fn to_raw_string(&self, unfold: u64) -> String;
}

impl<V> PartialEq for dyn IntermediateSymbol<V> + '_
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.id().id == other.id().id
    }
}

impl<V> Eq for dyn IntermediateSymbol<V> + '_ where V: Clone + Debug + Send + Sync + 'static {}

impl<V> Hash for dyn IntermediateSymbol<V> + '_
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().id.hash(state);
    }
}

/// 线性符号特化 / Linear-symbol specialization.
pub trait LinearIntermediateSymbol<V = f64>: IntermediateSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 转换为线性多项式 / Convert to a linear polynomial.
    fn to_linear_polynomial(&self) -> crate::symbol::flatten::Linear<V>;

    /// 转换为二次多项式（线性项作为二次多项式的退化形式）/ Convert to a quadratic polynomial (linear terms as a degenerate quadratic).
    fn to_quadratic_polynomial(&self) -> crate::symbol::flatten::Quadratic<V>;
}

/// 二次符号特化 / Quadratic-symbol specialization.
pub trait QuadraticIntermediateSymbol<V = f64>: IntermediateSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 转换为二次多项式 / Convert to a quadratic polynomial.
    fn to_quadratic_polynomial(&self) -> crate::symbol::flatten::Quadratic<V>;
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::fmt::{Display, Formatter};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use ospf_rust_math::symbol::{DynSymbol, SymbolDynId};

    use crate::model::LazyValueCacheContext;
    use crate::token::VecTokenList;

    use super::*;

    #[derive(Debug, Clone)]
    struct TestSymbol {
        id: IntermediateSymbolId,
        deps: Vec<Arc<dyn IntermediateSymbol>>,
        value: f64,
        eval_calls: Arc<AtomicUsize>,
    }

    impl TestSymbol {
        fn new(
            id: u64,
            name: &str,
            deps: Vec<Arc<dyn IntermediateSymbol>>,
            value: f64,
            eval_calls: Arc<AtomicUsize>,
        ) -> Self {
            Self {
                id: IntermediateSymbolId::new(id, name),
                deps,
                value,
                eval_calls,
            }
        }
    }

    impl Display for TestSymbol {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.id.name)
        }
    }

    impl DynSymbol for TestSymbol {
        fn name(&self) -> &str {
            &self.id.name
        }

        fn display_name(&self) -> &str {
            &self.id.name
        }

        fn dyn_id(&self) -> SymbolDynId<'_> {
            SymbolDynId::standalone(self.id.id as usize)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl Symbol for TestSymbol {
        type Id = IntermediateSymbolId;

        fn id(&self) -> Self::Id {
            self.id.clone()
        }
    }

    impl IntermediateSymbol for TestSymbol {
        fn category(&self) -> Category {
            Category::Linear
        }

        fn cached(&self) -> bool {
            false
        }

        fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol>> {
            self.deps.iter().cloned().collect()
        }

        fn flush(&self, _force: bool) {}

        fn evaluate_from_tokens(
            &self,
            _token_table: &dyn TokenList<f64>,
            _zero_if_none: bool,
        ) -> Option<f64> {
            self.eval_calls.fetch_add(1, Ordering::SeqCst);
            Some(self.value)
        }

        fn prepare(&self, _values: &HashMap<usize, f64>) -> Option<f64> {
            None
        }

        fn to_raw_string(&self, _unfold: u64) -> String {
            self.id.name.clone()
        }
    }

    #[test]
    fn evaluate_with_ctx_preheats_dependencies_once() {
        let dep_calls = Arc::new(AtomicUsize::new(0));
        let root_calls = Arc::new(AtomicUsize::new(0));

        let dependency: Arc<dyn IntermediateSymbol> =
            Arc::new(TestSymbol::new(1, "dep", vec![], 1.0, dep_calls.clone()));
        let root = TestSymbol::new(2, "root", vec![dependency], 2.0, root_calls.clone());

        let token_list = Arc::new(VecTokenList::<f64>::new());
        let mut cache = LazyValueCacheContext::<f64, VecTokenList<f64>>::new();
        cache.init(token_list);

        let values = HashMap::new();
        let mut ctx = IntermediateSymbolEvalContext::new(&values, &mut cache);

        let first = root.evaluate(&mut ctx);
        let second = root.evaluate(&mut ctx);

        assert_eq!(first, Some(2.0));
        assert_eq!(second, Some(2.0));
        assert_eq!(dep_calls.load(Ordering::SeqCst), 1);
        assert_eq!(root_calls.load(Ordering::SeqCst), 1);
    }
}
