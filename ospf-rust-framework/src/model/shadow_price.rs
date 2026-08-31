//! Shadow Price 定义
//! Shadow Price Definitions
//!
//! 本模块提供 Shadow Price 管理相关的数据结构和 trait。
//! This module provides data structures and traits for shadow price management.
//!
//! # 核心概念 / Core Concepts
//!
//! - [`ShadowPriceKey`] - Shadow Price 键，用于标识 Shadow Price / Shadow Price key for identifying shadow prices
//! - [`ShadowPrice`] - Shadow Price 数据结构 / Shadow Price data structure
//! - [`ShadowPriceMap`] - Shadow Price 映射表 trait / Shadow Price map trait

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use ospf_rust_core::error::Result;
use super::CGPipeline;

/// Shadow Price 键 / Shadow Price Key
///
/// 用于标识 Shadow Price 的键，包含类型限制。
/// Key for identifying shadow price, containing type constraints.
#[derive(Debug, Clone)]
pub struct ShadowPriceKey {
    /// 类型标识 / Type identifier
    pub limit: TypeId,
    /// 可选名称 / Optional name
    pub name: Option<String>,
}

impl ShadowPriceKey {
    /// 创建新的 Shadow Price 键 / Create new shadow price key
    pub fn new<T: 'static>() -> Self {
        Self {
            limit: TypeId::of::<T>(),
            name: None,
        }
    }

    /// 创建带名称的 Shadow Price 键 / Create named shadow price key
    pub fn named<T: 'static>(name: impl Into<String>) -> Self {
        Self {
            limit: TypeId::of::<T>(),
            name: Some(name.into()),
        }
    }

    /// 检查类型是否匹配 / Check if type matches
    pub fn is_type<T: 'static>(&self) -> bool {
        self.limit == TypeId::of::<T>()
    }
}

impl PartialEq for ShadowPriceKey {
    fn eq(&self, other: &Self) -> bool {
        self.limit == other.limit && self.name == other.name
    }
}

impl Eq for ShadowPriceKey {}

impl std::hash::Hash for ShadowPriceKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.limit.hash(state);
        self.name.hash(state);
    }
}

impl std::fmt::Display for ShadowPriceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(name) => write!(f, "{}", name),
            None => write!(f, "ShadowPriceKey({:?})", self.limit),
        }
    }
}

/// Shadow Price 数据结构 / Shadow Price Data Structure
///
/// 表示一个 Shadow Price，包含键和价格值。
/// Represents a shadow price, containing key and price value.
#[derive(Debug, Clone)]
pub struct ShadowPrice {
    /// 键 / Key
    pub key: ShadowPriceKey,
    /// 价格值 / Price value
    pub price: f64,
}

impl ShadowPrice {
    /// 创建新的 Shadow Price / Create new shadow price
    pub fn new(key: ShadowPriceKey, price: f64) -> Self {
        Self { key, price }
    }

    /// 从类型创建 Shadow Price / Create shadow price from type
    pub fn from_type<T: 'static>(price: f64) -> Self {
        Self {
            key: ShadowPriceKey::new::<T>(),
            price,
        }
    }

    /// 从类型和名称创建 Shadow Price / Create shadow price from type and name
    pub fn from_type_named<T: 'static>(name: impl Into<String>, price: f64) -> Self {
        Self {
            key: ShadowPriceKey::named::<T>(name),
            price,
        }
    }
}

impl std::fmt::Display for ShadowPrice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.key, self.price)
    }
}

/// Shadow Price 提取器类型 / Shadow Price Extractor Type
pub type ShadowPriceExtractor<Args, Map> = Arc<dyn Fn(&Map, &Args) -> f64 + Send + Sync>;

/// Shadow Price 映射表 trait / Shadow Price Map Trait
///
/// 用于管理 Shadow Price 的映射表。
/// For managing shadow price mappings.
///
/// # 类型参数 / Type Parameters
///
/// - `Args` - 参数类型，用于计算 Shadow Price / Argument type for calculating shadow price
pub trait ShadowPriceMap<Args>: Send + Sync {
    /// 获取 Shadow Price / Get shadow price
    fn get(&self, key: &ShadowPriceKey) -> Option<ShadowPrice>;

    /// 设置 Shadow Price / Set shadow price
    fn set(&mut self, key: ShadowPriceKey, value: ShadowPrice);

    /// 添加 Shadow Price / Put shadow price
    fn put(&mut self, price: ShadowPrice);

    /// 添加或累加 Shadow Price / Put or add shadow price
    fn put_or_add(&mut self, price: ShadowPrice);

    /// 移除 Shadow Price / Remove shadow price
    fn remove(&mut self, key: &ShadowPriceKey);

    /// 收缩（移除零值）/ Shrink (remove zero values)
    fn shrink(&mut self);

    /// 计算 Shadow Price / Calculate shadow price
    fn invoke(&self, arg: &Args) -> f64;

    /// 添加提取器 / Add extractor
    fn add_extractor(&mut self, extractor: ShadowPriceExtractor<Args, Self>)
    where
        Self: Sized;
}

/// 基本 Shadow Price 映射表实现 / Basic Shadow Price Map Implementation
pub struct BasicShadowPriceMap<Args> {
    /// Shadow Price 映射 / Shadow price mapping
    map: HashMap<ShadowPriceKey, ShadowPrice>,
    /// 提取器列表 / Extractor list
    extractors: Vec<ShadowPriceExtractor<Args, BasicShadowPriceMap<Args>>>,
}

impl<Args> std::fmt::Debug for BasicShadowPriceMap<Args> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicShadowPriceMap")
            .field("map", &self.map)
            .field(
                "extractors",
                &format!("{} extractors", self.extractors.len()),
            )
            .finish()
    }
}

impl<Args> Default for BasicShadowPriceMap<Args> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Args> BasicShadowPriceMap<Args> {
    /// 创建新的映射表 / Create new map
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            extractors: Vec::new(),
        }
    }

    /// 获取 Shadow Price 引用 / Get shadow price by reference
    pub fn get_ref(&self, key: &ShadowPriceKey) -> Option<&ShadowPrice> {
        self.map.get(key)
    }
}

impl<Args> ShadowPriceMap<Args> for BasicShadowPriceMap<Args> {
    fn get(&self, key: &ShadowPriceKey) -> Option<ShadowPrice> {
        self.map.get(key).cloned()
    }

    fn set(&mut self, key: ShadowPriceKey, value: ShadowPrice) {
        self.map.insert(key, value);
    }

    fn put(&mut self, price: ShadowPrice) {
        self.map.insert(price.key.clone(), price);
    }

    fn put_or_add(&mut self, price: ShadowPrice) {
        self.map
            .entry(price.key.clone())
            .and_modify(|existing| existing.price += price.price)
            .or_insert(price);
    }

    fn remove(&mut self, key: &ShadowPriceKey) {
        self.map.remove(key);
    }

    fn shrink(&mut self) {
        self.map.retain(|_, v| v.price != 0.0);
    }

    fn invoke(&self, arg: &Args) -> f64 {
        self.extractors.iter().map(|e| e(self, arg)).sum()
    }

    fn add_extractor(&mut self, extractor: ShadowPriceExtractor<Args, Self>) {
        self.extractors.push(extractor);
    }
}

impl<Args> std::ops::Index<&ShadowPriceKey> for BasicShadowPriceMap<Args> {
    type Output = ShadowPrice;

    fn index(&self, key: &ShadowPriceKey) -> &Self::Output {
        self.get_ref(key).expect("ShadowPriceKey not found")
    }
}

impl<Args> std::ops::IndexMut<&ShadowPriceKey> for BasicShadowPriceMap<Args> {
    fn index_mut(&mut self, key: &ShadowPriceKey) -> &mut Self::Output {
        self.map.get_mut(key).expect("ShadowPriceKey not found")
    }
}

/// 线程安全的 Shadow Price 映射表 / Thread-safe Shadow Price Map
pub struct ConcurrentShadowPriceMap<Args> {
    /// 内部映射表 / Inner map
    inner: RwLock<BasicShadowPriceMap<Args>>,
    /// 并发提取器列表 / Concurrent extractor list
    extractors: Vec<ShadowPriceExtractor<Args, ConcurrentShadowPriceMap<Args>>>,
}

impl<Args> std::fmt::Debug for ConcurrentShadowPriceMap<Args> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConcurrentShadowPriceMap")
            .field("inner", &self.inner.read())
            .field(
                "extractors",
                &format!("{} extractors", self.extractors.len()),
            )
            .finish()
    }
}

impl<Args> Default for ConcurrentShadowPriceMap<Args> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Args> ConcurrentShadowPriceMap<Args> {
    /// 创建新的映射表 / Create new map
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(BasicShadowPriceMap::new()),
            extractors: Vec::new(),
        }
    }
}

impl<Args> ShadowPriceMap<Args> for ConcurrentShadowPriceMap<Args> {
    fn get(&self, key: &ShadowPriceKey) -> Option<ShadowPrice> {
        self.inner.read().get_ref(key).cloned()
    }

    fn set(&mut self, key: ShadowPriceKey, value: ShadowPrice) {
        self.inner.write().set(key, value);
    }

    fn put(&mut self, price: ShadowPrice) {
        self.inner.write().put(price);
    }

    fn put_or_add(&mut self, price: ShadowPrice) {
        self.inner.write().put_or_add(price);
    }

    fn remove(&mut self, key: &ShadowPriceKey) {
        self.inner.write().remove(key);
    }

    fn shrink(&mut self) {
        self.inner.write().shrink();
    }

    fn invoke(&self, arg: &Args) -> f64 {
        let base = self.inner.read().invoke(arg);
        let extra: f64 = self
            .extractors
            .iter()
            .map(|extractor| extractor(self, arg))
            .sum();
        base + extra
    }

    fn add_extractor(&mut self, extractor: ShadowPriceExtractor<Args, Self>) {
        self.extractors.push(extractor);
    }
}

impl<Args> ConcurrentShadowPriceMap<Args> {
    /// 获取 Shadow Price 克隆 / Get cloned shadow price
    pub fn get_cloned(&self, key: &ShadowPriceKey) -> Option<ShadowPrice> {
        self.get(key)
    }

    /// 添加提取器（使用闭包）/ Add extractor with closure
    pub fn add_extractor_fn<F>(&mut self, extractor: F)
    where
        F: Fn(&BasicShadowPriceMap<Args>, &Args) -> f64 + Send + Sync + 'static,
    {
        self.inner.write().add_extractor(Arc::new(extractor));
    }
}

/// 提取 Shadow Price / Extract Shadow Price
///
/// 从对偶解中提取 Shadow Price 并更新映射表。
/// Extracts shadow prices from dual solution and updates the map.
pub fn extract_shadow_price<Args, M, Map, Pipeline>(
    shadow_price_map: &mut Map,
    pipeline_list: &[Pipeline],
    model: &M,
    shadow_prices: &[f64],
) -> Result<()>
where
    Args: Send + Sync + 'static,
    Map: ShadowPriceMap<Args>,
    Pipeline: CGPipeline<Args, M, Map>,
{
    for pipeline in pipeline_list {
        pipeline.refresh(shadow_price_map, model, shadow_prices)?;
        if let Some(extractor) = pipeline.extractor() {
            shadow_price_map.add_extractor(Arc::new(extractor));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Pipeline;

    #[test]
    fn test_shadow_price_key() {
        let key1 = ShadowPriceKey::new::<i32>();
        let key2 = ShadowPriceKey::new::<i32>();
        let key3 = ShadowPriceKey::new::<String>();

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert!(key1.is_type::<i32>());
        assert!(!key1.is_type::<String>());
    }

    #[test]
    fn test_shadow_price() {
        let price = ShadowPrice::from_type::<i32>(1.5);
        assert_eq!(price.price, 1.5);
        assert!(price.key.is_type::<i32>());
    }

    #[test]
    fn test_basic_shadow_price_map() {
        let mut map: BasicShadowPriceMap<i32> = BasicShadowPriceMap::new();
        let key = ShadowPriceKey::new::<i32>();
        let price = ShadowPrice::new(key.clone(), 1.0);

        map.put(price);
        assert!(map.get(&key).is_some());
        assert_eq!(map.get(&key).unwrap().price, 1.0);

        map.put_or_add(ShadowPrice::new(key.clone(), 0.5));
        assert_eq!(map.get(&key).unwrap().price, 1.5);

        map.shrink();
        assert!(map.get(&key).is_some()); // price != 0

        map.put_or_add(ShadowPrice::new(key.clone(), -1.5));
        map.shrink();
        assert!(map.get(&key).is_none()); // price == 0
    }

    #[test]
    fn test_concurrent_shadow_price_map() {
        let mut map: ConcurrentShadowPriceMap<i32> = ConcurrentShadowPriceMap::new();
        let key = ShadowPriceKey::new::<i32>();
        map.put(ShadowPrice::new(key.clone(), 2.0));

        let extractor_key = key.clone();
        map.add_extractor(Arc::new(move |shadow_map, arg| {
            shadow_map
                .get(&extractor_key)
                .map(|value| value.price * (*arg as f64))
                .unwrap_or(0.0)
        }));

        assert_eq!(map.get_cloned(&key).unwrap().price, 2.0);
        assert_eq!(map.invoke(&3), 6.0);
    }

    struct DummyModel;

    struct DummyPipeline;

    impl Pipeline<DummyModel> for DummyPipeline {
        fn name(&self) -> &str {
            "dummy"
        }

        fn constraint_group(&self) -> Option<&ospf_rust_core::model::mechanism::ConstraintGroup> {
            None
        }

        fn invoke(&self, _model: &DummyModel) -> Result<()> {
            Ok(())
        }
    }

    impl CGPipeline<i32, DummyModel, BasicShadowPriceMap<i32>> for DummyPipeline {
        type Extractor = fn(&BasicShadowPriceMap<i32>, &i32) -> f64;

        fn extractor(&self) -> Option<Self::Extractor> {
            Some(|map, arg| {
                map.get(&ShadowPriceKey::new::<i32>())
                    .map(|shadow_price| shadow_price.price * (*arg as f64))
                    .unwrap_or(0.0)
            })
        }

        fn refresh(
            &self,
            shadow_price_map: &mut BasicShadowPriceMap<i32>,
            _model: &DummyModel,
            shadow_prices: &[f64],
        ) -> Result<()> {
            let value = shadow_prices.first().copied().unwrap_or(0.0);
            shadow_price_map.put(ShadowPrice::from_type::<i32>(value));
            Ok(())
        }
    }

    #[test]
    fn test_extract_shadow_price_registers_extractor() {
        let mut map = BasicShadowPriceMap::<i32>::new();
        let model = DummyModel;
        let pipelines = vec![DummyPipeline];

        let result = extract_shadow_price::<i32, _, _, _>(&mut map, &pipelines, &model, &[1.5]);
        assert!(result.is_ok());
        assert_eq!(map.invoke(&2), 3.0);
    }
}
