//! 求值缓存上下文
//! Value Cache Context

use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::{Arc, OnceLock};
use crate::symbol::flatten::Cacheable;
use crate::token::{Token, TokenList};
use crate::variable::VariableId;

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
