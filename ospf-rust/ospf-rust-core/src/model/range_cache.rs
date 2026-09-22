//! 范围缓存上下文
//! Range Cache Context

use crate::symbol::flatten::Cacheable;
use crate::token::TokenList;
use crate::variable::VariableRange;
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::{Arc, OnceLock};

/// 范围缓存 Key / Range cache key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RangeCacheKey {
    /// 对象 ID（多项式地址或符号 ID）/ Object ID (polynomial address or symbol id)
    pub object_id: u64,
}

impl RangeCacheKey {
    /// 从可缓存对象创建 / Create from cacheable object
    pub fn from_cacheable<T: Cacheable>(object: &T) -> Self {
        Self {
            object_id: object.cache_key().value,
        }
    }

    /// 从中间符号标识创建 / Create from intermediate symbol id
    pub fn from_symbol(identifier: u64) -> Self {
        Self {
            object_id: identifier,
        }
    }
}

/// 范围缓存上下文 trait / Range cache context trait
pub trait RangeCacheContextTrait<V>: Send + Sync
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 获取 token 列表引用 / Get token list reference
    fn token_list(&self) -> &dyn TokenList<V>;

    /// 获取缓存映射 / Get cache map
    fn cache(&self) -> &HashMap<RangeCacheKey, VariableRange<V>>;
    /// 获取可变缓存映射 / Get the mutable cache map.
    fn cache_mut(&mut self) -> &mut HashMap<RangeCacheKey, VariableRange<V>>;

    /// 获取缓存范围 / Get cached range
    fn get(&self, key: RangeCacheKey) -> Option<&VariableRange<V>> {
        self.cache().get(&key)
    }

    /// 写入缓存范围 / Set cached range
    fn set(&mut self, key: RangeCacheKey, value: VariableRange<V>) {
        self.cache_mut().insert(key, value);
    }

    /// 获取或计算缓存范围 / Get or compute cached range
    fn get_or_compute<F>(&mut self, key: RangeCacheKey, f: F) -> &VariableRange<V>
    where
        F: FnOnce() -> VariableRange<V>,
        Self: Sized,
    {
        self.cache_mut().entry(key).or_insert_with(f)
    }

    /// 清空缓存 / Clear all cache
    fn clear(&mut self) {
        self.cache_mut().clear();
    }

    /// 清除对象缓存 / Clear cache by object
    fn clear_object(&mut self, key: RangeCacheKey) -> bool {
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
}

/// 懒加载范围缓存上下文 / Lazy range cache context
pub struct LazyRangeCacheContext<V, T: TokenList<V>>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    token_list: OnceLock<Arc<T>>,
    cache: HashMap<RangeCacheKey, VariableRange<V>>,
    _value: PhantomData<V>,
}

impl<V, T: TokenList<V>> LazyRangeCacheContext<V, T>
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

impl<V, T: TokenList<V>> RangeCacheContextTrait<V> for LazyRangeCacheContext<V, T>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn token_list(&self) -> &dyn TokenList<V> {
        self.token_list
            .get()
            .expect("RangeCacheContext not initialized. Call init() first.")
            .as_ref()
    }

    fn cache(&self) -> &HashMap<RangeCacheKey, VariableRange<V>> {
        &self.cache
    }

    fn cache_mut(&mut self) -> &mut HashMap<RangeCacheKey, VariableRange<V>> {
        &mut self.cache
    }
}

impl<V, T: TokenList<V>> Default for LazyRangeCacheContext<V, T>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// f64 范围缓存上下文 / f64 range cache context
pub type F64RangeCacheContext<T> = LazyRangeCacheContext<f64, T>;

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{MutableTokenList, Token, TokenList, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableId, VariableRange};

    /// 构建已初始化的范围缓存上下文 / Build an initialized range cache context.
    fn context() -> LazyRangeCacheContext<f64, VecTokenList<f64>> {
        let context = LazyRangeCacheContext::<f64, VecTokenList<f64>>::new();
        context.init(Arc::new(VecTokenList::new()));
        context
    }

    #[test]
    fn symbolic_key_preserves_the_identifier() {
        // 符号 key 必须原样保留标识，不能做任何偏移或编码。
        // A symbolic key must preserve the identifier verbatim, with no offset or encoding.
        assert_eq!(RangeCacheKey::from_symbol(0).object_id, 0);
        assert_eq!(RangeCacheKey::from_symbol(7).object_id, 7);
        assert_eq!(RangeCacheKey::from_symbol(u64::MAX).object_id, u64::MAX);
    }

    #[test]
    fn symbolic_keys_are_distinct_per_identifier() {
        // 不同符号必须映射到不同 key，否则会串缓存并静默返回错误范围。
        // Distinct symbols must map to distinct keys, otherwise caches alias and
        // silently return the wrong range.
        assert_ne!(RangeCacheKey::from_symbol(1), RangeCacheKey::from_symbol(2));
        assert_eq!(RangeCacheKey::from_symbol(1), RangeCacheKey::from_symbol(1));
    }

    #[test]
    fn get_or_compute_evaluates_only_once() {
        // 命中缓存后不得重复计算：这是缓存存在的全部意义。
        // A cache hit must not recompute; that is the entire point of the cache.
        let mut context = context();
        let key = RangeCacheKey::from_symbol(11);
        let mut calls = 0usize;

        let first = context
            .get_or_compute(key, || {
                calls += 1;
                VariableRange::bounded(0.0, 1.0)
            })
            .clone();
        let second = context
            .get_or_compute(key, || {
                calls += 1;
                VariableRange::bounded(9.0, 9.0)
            })
            .clone();

        assert_eq!(calls, 1);
        assert_eq!(first, VariableRange::bounded(0.0, 1.0));
        assert_eq!(second, first, "缓存命中必须返回首次计算的结果");
    }

    #[test]
    fn distinct_keys_stay_independent() {
        // 相邻 key 不能互相覆盖 / Adjacent keys must not overwrite each other.
        let mut context = context();
        let lower = RangeCacheKey::from_symbol(1);
        let upper = RangeCacheKey::from_symbol(2);

        context.set(lower, VariableRange::bounded(0.0, 1.0));
        context.set(upper, VariableRange::bounded(2.0, 3.0));

        assert_eq!(context.len(), 2);
        assert_eq!(context.get(lower), Some(&VariableRange::bounded(0.0, 1.0)));
        assert_eq!(context.get(upper), Some(&VariableRange::bounded(2.0, 3.0)));
    }

    #[test]
    fn set_overwrites_the_same_key_without_growing() {
        // 同一 key 覆盖写不得增长缓存 / Overwriting one key must not grow the cache.
        let mut context = context();
        let key = RangeCacheKey::from_symbol(3);

        context.set(key, VariableRange::bounded(0.0, 1.0));
        context.set(key, VariableRange::unbounded());

        assert_eq!(context.len(), 1);
        assert_eq!(context.get(key), Some(&VariableRange::<f64>::unbounded()));
    }

    #[test]
    fn clear_object_removes_only_the_requested_entry() {
        // 单对象清理不得误伤其它条目 / Per-object clearing must not disturb siblings.
        let mut context = context();
        let kept = RangeCacheKey::from_symbol(10);
        let removed = RangeCacheKey::from_symbol(20);
        context.set(kept, VariableRange::bounded(0.0, 1.0));
        context.set(removed, VariableRange::bounded(2.0, 3.0));

        assert!(context.clear_object(removed));
        assert_eq!(context.len(), 1);
        assert!(context.get(kept).is_some());
        assert!(context.get(removed).is_none());

        // 二次清理同一 key 必须报告"未命中"，调用方据此可检测重复失效。
        // Re-clearing the same key must report a miss so callers can detect double invalidation.
        assert!(!context.clear_object(removed));
    }

    #[test]
    fn clear_drops_every_entry() {
        // 全量清理后必须回到空状态 / A full clear must return to the empty state.
        let mut context = context();
        context.set(RangeCacheKey::from_symbol(1), VariableRange::bounded(0.0, 1.0));
        context.set(RangeCacheKey::from_symbol(2), VariableRange::bounded(2.0, 3.0));
        assert!(!context.is_empty());

        context.clear();

        assert!(context.is_empty());
        assert_eq!(context.len(), 0);
        assert!(context.get(RangeCacheKey::from_symbol(1)).is_none());
    }

    #[test]
    fn a_fresh_context_is_empty_and_uninitialized() {
        // 新建上下文必须既空又未初始化 / A fresh context is both empty and uninitialized.
        let context = LazyRangeCacheContext::<f64, VecTokenList<f64>>::new();

        assert!(context.is_empty());
        assert_eq!(context.len(), 0);
        assert!(!context.is_initialized());
        assert!(context.token_list_ref().is_none());
    }

    #[test]
    fn init_is_idempotent_and_keeps_the_first_token_list() {
        // OnceLock 语义：重复 init 不得替换首个 token 列表，否则已缓存的索引会失配。
        // OnceLock semantics: a repeated init must not replace the first token list,
        // otherwise already-cached indices desynchronize.
        let context = LazyRangeCacheContext::<f64, VecTokenList<f64>>::new();
        let mut first_list = VecTokenList::new();
        first_list.add_token(Token::from_generic(
            ContinuousVariableItem::create(VariableId::standalone(1), "first"),
            0,
        ));
        let first = Arc::new(first_list);
        let second = Arc::new(VecTokenList::new());

        context.init(first);
        context.init(second);

        assert!(context.is_initialized());
        assert_eq!(context.token_list_ref().expect("initialized").len(), 1);
    }

    #[test]
    #[should_panic(expected = "RangeCacheContext not initialized")]
    fn uninitialized_context_panics_instead_of_returning_empty() {
        // 未初始化必须显式 panic：静默返回空 token 列表会掩盖注册顺序错误。
        // An uninitialized context must panic loudly; silently returning an empty token
        // list would mask a registration-order bug.
        let context = LazyRangeCacheContext::<f64, VecTokenList<f64>>::new();
        let _ = context.token_list();
    }

    #[test]
    fn default_matches_new() {
        // Default 必须等价于 new / Default must be equivalent to new.
        let context = LazyRangeCacheContext::<f64, VecTokenList<f64>>::default();

        assert!(context.is_empty());
        assert!(!context.is_initialized());
    }
}
