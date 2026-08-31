//! 范围缓存上下文
//! Range Cache Context

use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::{Arc, OnceLock};
use crate::symbol::flatten::Cacheable;
use crate::token::TokenList;
use crate::variable::VariableRange;

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
