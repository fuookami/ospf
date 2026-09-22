//! 求值缓存上下文
//! Value Cache Context

use crate::symbol::flatten::Cacheable;
use crate::token::{Token, TokenList};
use crate::variable::VariableId;
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::{Arc, OnceLock};

/// 计算值缓存 Key / Computed value cache key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueCacheKey {
    /// 缓存类型 / Cache kind
    pub kind: ValueCacheKind,
    /// 对象 ID（多项式地址或符号 ID）/ Object ID (polynomial address or symbol id)
    pub object_id: u64,
}

/// 计算值缓存类型 / Computed value cache kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueCacheKind {
    /// 多项式求值 / Polynomial evaluation
    Polynomial,
    /// 中间符号求值 / Intermediate symbol evaluation
    IntermediateSymbol,
    /// 目标函数求值 / Objective evaluation
    Objective,
}

impl ValueCacheKey {
    /// 从多项式对象创建 / Create from polynomial object
    pub fn from_polynomial<P: Cacheable>(poly: &P) -> Self {
        Self {
            kind: ValueCacheKind::Polynomial,
            object_id: poly.cache_key().value,
        }
    }

    /// 从中间符号标识创建 / Create from intermediate symbol id
    pub fn from_symbol(identifier: u64) -> Self {
        Self {
            kind: ValueCacheKind::IntermediateSymbol,
            object_id: identifier,
        }
    }

    /// 从目标函数标识创建 / Create from objective id
    pub fn from_objective(id: u64) -> Self {
        Self {
            kind: ValueCacheKind::Objective,
            object_id: id,
        }
    }
}

/// 求值缓存上下文 trait / Value cache context trait
pub trait ValueCacheContextTrait<V>: Send + Sync
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 获取 token 列表引用 / Get token list reference
    fn token_list(&self) -> &dyn TokenList<V>;

    /// 获取缓存映射 / Get cache map
    fn cache(&self) -> &HashMap<ValueCacheKey, V>;
    /// 获取可变缓存映射 / Get the mutable cache map.
    fn cache_mut(&mut self) -> &mut HashMap<ValueCacheKey, V>;

    /// 获取缓存值 / Get cached value
    fn get(&self, key: ValueCacheKey) -> Option<&V> {
        self.cache().get(&key)
    }

    /// 写入缓存值 / Set cached value
    fn set(&mut self, key: ValueCacheKey, value: V) {
        self.cache_mut().insert(key, value);
    }

    /// 获取或计算缓存值 / Get or compute cached value
    fn get_or_compute<F>(&mut self, key: ValueCacheKey, f: F) -> &V
    where
        F: FnOnce() -> V,
        Self: Sized,
    {
        self.cache_mut().entry(key).or_insert_with(f)
    }

    /// 清空缓存 / Clear all cache
    fn clear(&mut self) {
        self.cache_mut().clear();
    }

    /// 清除某一类缓存 / Clear cache by kind
    fn clear_kind(&mut self, kind: ValueCacheKind) {
        self.cache_mut().retain(|k, _| k.kind != kind);
    }

    /// 清除某个对象缓存 / Clear cache by object
    fn clear_object(&mut self, key: ValueCacheKey) -> bool {
        self.cache_mut().remove(&key).is_some()
    }

    /// 缓存大小 / Cache size
    fn len(&self) -> usize {
        self.cache().len()
    }

    /// 是否为空 / Whether cache is empty
    fn is_empty(&self) -> bool {
        self.cache().is_empty()
    }

    /// 检查变量是否已注册 / Check whether variable is registered
    fn is_registered(&self, var_id: VariableId) -> bool {
        self.token_list().find_by_id(var_id).is_some()
    }

    /// 通过 ID 查找 token / Find token by id
    fn find_token(&self, var_id: VariableId) -> Option<&Token<V>> {
        self.token_list().find_by_id(var_id)
    }
}

/// 懒加载求值缓存上下文 / Lazy value cache context
pub struct LazyValueCacheContext<V, T: TokenList<V>>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    token_list: OnceLock<Arc<T>>,
    cache: HashMap<ValueCacheKey, V>,
    _value: PhantomData<V>,
}

impl<V, T: TokenList<V>> LazyValueCacheContext<V, T>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建未初始化上下文 / Create uninitialized context
    pub fn new() -> Self {
        Self {
            token_list: OnceLock::new(),
            cache: HashMap::new(),
            _value: PhantomData,
        }
    }

    /// 初始化 token 列表引用 / Initialize token list reference
    pub fn init(&self, token_list: Arc<T>) {
        let _ = self.token_list.set(token_list);
    }

    /// 是否已初始化 / Whether context has been initialized
    pub fn is_initialized(&self) -> bool {
        self.token_list.get().is_some()
    }

    /// 获取 token 列表引用（可选）/ Get optional token list reference
    pub fn token_list_ref(&self) -> Option<&T> {
        self.token_list.get().map(|arc| arc.as_ref())
    }
}

impl<V, T: TokenList<V>> ValueCacheContextTrait<V> for LazyValueCacheContext<V, T>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn token_list(&self) -> &dyn TokenList<V> {
        self.token_list
            .get()
            .expect("ValueCacheContext not initialized. Call init() first.")
            .as_ref()
    }

    fn cache(&self) -> &HashMap<ValueCacheKey, V> {
        &self.cache
    }

    fn cache_mut(&mut self) -> &mut HashMap<ValueCacheKey, V> {
        &mut self.cache
    }
}

impl<V, T: TokenList<V>> Default for LazyValueCacheContext<V, T>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// f64 求值缓存上下文 / f64 value cache context
pub type F64ValueCacheContext<T> = LazyValueCacheContext<f64, T>;

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{MutableTokenList, VecTokenList};
    use crate::variable::ContinuousVariableItem;

    /// 构建已初始化的求值缓存上下文 / Build an initialized value cache context.
    fn context() -> LazyValueCacheContext<f64, VecTokenList<f64>> {
        let context = LazyValueCacheContext::<f64, VecTokenList<f64>>::new();
        context.init(Arc::new(VecTokenList::new()));
        context
    }

    /// 构建上下文本并预置一个已注册变量 / Build a context pre-seeded with one registered variable.
    fn context_with_token(id: usize) -> LazyValueCacheContext<f64, VecTokenList<f64>> {
        let mut list = VecTokenList::new();
        list.add_token(Token::from_generic(
            ContinuousVariableItem::create(VariableId::standalone(id), "seeded"),
            0,
        ));
        let context = LazyValueCacheContext::<f64, VecTokenList<f64>>::new();
        context.init(Arc::new(list));
        context
    }

    #[test]
    fn keys_of_different_kinds_never_collide() {
        // 这是本模块最关键的不变量：Polynomial / IntermediateSymbol / Objective
        // 共用同一个 object_id 数值空间，只有 kind 判别符能阻止串缓存。
        // 若 kind 被移除或比较时被忽略，这里会静默返回另一个对象的数值。
        //
        // This is the module's central invariant: the three kinds share one object_id
        // numeric space, so only the kind discriminator prevents aliasing. If the
        // discriminator were dropped or ignored in comparison, this would silently
        // return another object's value.
        let id = 42u64;
        let polynomial = ValueCacheKey::from_polynomial(&RawCacheable(id));
        let symbol = ValueCacheKey::from_symbol(id);
        let objective = ValueCacheKey::from_objective(id);

        assert_eq!(polynomial.object_id, id);
        assert_eq!(symbol.object_id, id);
        assert_eq!(objective.object_id, id);
        assert_ne!(polynomial, symbol);
        assert_ne!(symbol, objective);
        assert_ne!(polynomial, objective);

        let mut context = context();
        context.set(polynomial, 1.0);
        context.set(symbol, 2.0);
        context.set(objective, 3.0);

        assert_eq!(context.len(), 3, "不同 kind 必须占用独立槽位");
        assert_eq!(context.get(polynomial), Some(&1.0));
        assert_eq!(context.get(symbol), Some(&2.0));
        assert_eq!(context.get(objective), Some(&3.0));
    }

    #[test]
    fn clear_kind_removes_only_that_kind() {
        // 按类清理是失效某一层缓存的手段，绝不能波及其它 kind。
        // Kind-scoped clearing invalidates one cache layer and must not touch others.
        let mut context = context();
        context.set(ValueCacheKey::from_symbol(1), 1.0);
        context.set(ValueCacheKey::from_symbol(2), 2.0);
        context.set(ValueCacheKey::from_objective(1), 3.0);

        context.clear_kind(ValueCacheKind::IntermediateSymbol);

        assert_eq!(context.len(), 1, "只应保留 Objective 条目");
        assert!(context.get(ValueCacheKey::from_symbol(1)).is_none());
        assert!(context.get(ValueCacheKey::from_symbol(2)).is_none());
        assert_eq!(context.get(ValueCacheKey::from_objective(1)), Some(&3.0));
    }

    #[test]
    fn get_or_compute_evaluates_only_once() {
        // 命中缓存后不得重复求值：重复求值会让 O(1) 退化为 O(n)。
        // A cache hit must not re-evaluate; otherwise O(1) degrades to O(n).
        let mut context = context();
        let key = ValueCacheKey::from_symbol(7);
        let mut calls = 0usize;

        let first = *context.get_or_compute(key, || {
            calls += 1;
            1.5
        });
        let second = *context.get_or_compute(key, || {
            calls += 1;
            9.9
        });

        assert_eq!(calls, 1);
        assert_eq!(first, 1.5);
        assert_eq!(second, 1.5, "缓存命中必须返回首次计算的结果");
    }

    #[test]
    fn clear_object_removes_only_the_requested_entry() {
        // 单对象清理不得误伤同 kind 的兄弟条目。
        // Per-object clearing must not disturb siblings of the same kind.
        let mut context = context();
        let kept = ValueCacheKey::from_symbol(10);
        let removed = ValueCacheKey::from_symbol(20);
        context.set(kept, 1.0);
        context.set(removed, 2.0);

        assert!(context.clear_object(removed));
        assert_eq!(context.len(), 1);
        assert_eq!(context.get(kept), Some(&1.0));
        assert!(!context.clear_object(removed), "重复清理必须报告未命中");
    }

    #[test]
    fn clear_drops_every_kind() {
        // 全量清理必须跨 kind 生效 / A full clear must span every kind.
        let mut context = context();
        context.set(ValueCacheKey::from_symbol(1), 1.0);
        context.set(ValueCacheKey::from_objective(1), 2.0);
        assert!(!context.is_empty());

        context.clear();

        assert!(context.is_empty());
        assert_eq!(context.len(), 0);
    }

    #[test]
    fn is_registered_and_find_token_follow_the_token_list() {
        // 变量注册查询必须代理到 token 列表，不能读缓存或凭空返回。
        // Variable-registration queries must delegate to the token list rather than
        // reading the cache or inventing a result.
        let context = context_with_token(31);
        let present = VariableId::standalone(31);
        let absent = VariableId::standalone(32);

        assert!(context.is_registered(present));
        assert!(context.find_token(present).is_some());
        assert!(!context.is_registered(absent));
        assert!(context.find_token(absent).is_none());
    }

    #[test]
    fn a_fresh_context_is_empty_and_uninitialized() {
        // 新建上下文必须既空又未初始化 / A fresh context is both empty and uninitialized.
        let context = LazyValueCacheContext::<f64, VecTokenList<f64>>::new();

        assert!(context.is_empty());
        assert_eq!(context.len(), 0);
        assert!(!context.is_initialized());
        assert!(context.token_list_ref().is_none());
    }

    #[test]
    fn init_is_idempotent_and_keeps_the_first_token_list() {
        // OnceLock 语义：重复 init 不得替换首个 token 列表，否则已缓存的结果会失配。
        // OnceLock semantics: a repeated init must not replace the first token list,
        // otherwise already-cached results desynchronize.
        let mut first_list = VecTokenList::new();
        first_list.add_token(Token::from_generic(
            ContinuousVariableItem::create(VariableId::standalone(41), "first"),
            0,
        ));
        let context = LazyValueCacheContext::<f64, VecTokenList<f64>>::new();
        context.init(Arc::new(first_list));
        context.init(Arc::new(VecTokenList::new()));

        assert!(context.is_initialized());
        assert!(context.is_registered(VariableId::standalone(41)));
    }

    #[test]
    #[should_panic(expected = "ValueCacheContext not initialized")]
    fn uninitialized_context_panics_instead_of_returning_empty() {
        // 未初始化必须显式 panic：静默返回空 token 列表会掩盖注册顺序错误。
        // An uninitialized context must panic loudly; silently returning an empty token
        // list would mask a registration-order bug.
        let context = LazyValueCacheContext::<f64, VecTokenList<f64>>::new();
        let _ = context.token_list();
    }

    #[test]
    fn default_matches_new() {
        // Default 必须等价于 new / Default must be equivalent to new.
        let context = LazyValueCacheContext::<f64, VecTokenList<f64>>::default();

        assert!(context.is_empty());
        assert!(!context.is_initialized());
    }

    /// 以固定地址充当 poly 缓存键，使 `from_polynomial` 的 object_id 可预测。
    /// Stand-in cacheable whose address yields a predictable object id via `from_polynomial`.
    struct RawCacheable(u64);

    impl Cacheable for RawCacheable {
        fn cache_key(&self) -> crate::symbol::flatten::CacheKey {
            crate::symbol::flatten::CacheKey::from_raw(self.0)
        }
    }
}
