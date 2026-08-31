//! 索引变量组合与符号组合适配器
//! Indexed Variable and Symbol Combination Adapters
//!
//! 将领域键映射到 `VariableCombination` / `SymbolCombination` 的线性索引，
//! 避免 BPP3D、CSP1D、Gantt 各自维护不兼容的变量/符号数组。
//!
//! Maps domain keys to linear indices of `VariableCombination` / `SymbolCombination`,
//! avoiding incompatible variable/symbol arrays across BPP3D, CSP1D, and Gantt.

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::{SymbolCombination, LinearExpressionSymbol};
use ospf_rust_core::variable::{VariableCombination, VariableTypeTrait, VariableRange};
use ospf_rust_core::token::IntoValue;
use ospf_rust_multiarray::{MultiArray, Shape};

// ============================================================================
// IndexedVariableCombination1 - 一维索引变量组合
// ============================================================================

/// 一维索引变量组合 / 1D Indexed Variable Combination
///
/// 将领域键 `K` 映射到一维 `VariableCombination` 的线性索引。
/// Maps domain key `K` to linear index of a 1D `VariableCombination`.
#[derive(Clone)]
pub struct IndexedVariableCombination1<K, VT>
where
    K: Debug + Clone + Eq + Hash,
    VT: VariableTypeTrait,
{
    combination: VariableCombination<VT, Shape<1>>,
    model_indices: MultiArray<usize, Shape<1>>,
    key_map: HashMap<K, usize>,
    name_prefix: String,
}

impl<K, VT> IndexedVariableCombination1<K, VT>
where
    K: Debug + Clone + Eq + Hash,
    VT: VariableTypeTrait + Clone,
    VT::Value: IntoValue<f64>,
{
    /// 创建并注册一维索引变量组合
    /// Create and register a 1D indexed variable combination
    ///
    /// # 参数 / Parameters
    /// - `prefix`: 变量名前缀 / Variable name prefix
    /// - `keys`: 领域键列表 / Domain key list
    /// - `model`: 元模型 / Meta model
    /// - `name_gen`: 名称生成器，接收键引用 / Name generator, receives key reference
    /// - `range_gen`: 范围生成器，接收键引用 / Range generator, receives key reference
    pub fn new(
        prefix: &str,
        keys: &[K],
        model: &mut MetaModel<f64>,
        name_gen: impl Fn(&K) -> String,
        range_gen: impl Fn(&K) -> VariableRange<VT::Value>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let key_map: HashMap<K, usize> = keys
            .iter()
            .enumerate()
            .map(|(i, k)| (k.clone(), i))
            .collect();

        let keys_ref = keys;
        let _key_map_ref = &key_map;
        let combination = VariableCombination::with_name_and_range_generator(
            Shape::new([keys.len()]),
            prefix,
            |_index, vector| {
                let key = &keys_ref[vector[0]];
                name_gen(key)
            },
            |_index, vector| {
                let key = &keys_ref[vector[0]];
                range_gen(key)
            },
        );

        let model_indices = model.register_combination(&combination)?;

        Ok(Self {
            combination,
            model_indices,
            key_map,
            name_prefix: prefix.to_string(),
        })
    }

    /// 获取领域键对应的模型索引
    /// Get model index for domain key
    pub fn model_index(&self, key: &K) -> Option<usize> {
        self.key_map.get(key).map(|&i| self.model_indices[i])
    }

    /// 获取领域键对应的模型索引（无检查）
    /// Get model index for domain key (unchecked)
    pub fn model_index_unchecked(&self, key: &K) -> usize {
        self.model_indices[self.key_map[key]]
    }

    /// 获取底层变量组合引用
    /// Get reference to underlying variable combination
    pub fn combination(&self) -> &VariableCombination<VT, Shape<1>> {
        &self.combination
    }

    /// 获取模型索引数组引用
    /// Get reference to model indices array
    pub fn model_indices(&self) -> &MultiArray<usize, Shape<1>> {
        &self.model_indices
    }

    /// 获取键映射引用
    /// Get reference to key mapping
    pub fn key_map(&self) -> &HashMap<K, usize> {
        &self.key_map
    }

    /// 获取名称前缀
    /// Get name prefix
    pub fn name_prefix(&self) -> &str {
        &self.name_prefix
    }

    /// 获取变量数量
    /// Get number of variables
    pub fn len(&self) -> usize {
        self.combination.len()
    }

    /// 是否为空
    /// Whether empty
    pub fn is_empty(&self) -> bool {
        self.combination.is_empty()
    }
}

impl<K, VT> std::fmt::Debug for IndexedVariableCombination1<K, VT>
where
    K: Debug + Clone + Eq + Hash,
    VT: VariableTypeTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexedVariableCombination1")
            .field("name_prefix", &self.name_prefix)
            .field("key_map", &self.key_map)
            .field("len", &self.combination.len())
            .finish()
    }
}

// ============================================================================
// IndexedVariableCombination2 - 二维索引变量组合
// ============================================================================

/// 二维索引变量组合 / 2D Indexed Variable Combination
///
/// 将领域键对 `(K1, K2)` 映射到二维 `VariableCombination` 的线性索引。
/// Maps domain key pair `(K1, K2)` to linear index of a 2D `VariableCombination`.
#[derive(Clone)]
pub struct IndexedVariableCombination2<K1, K2, VT>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
    VT: VariableTypeTrait,
{
    combination: VariableCombination<VT, Shape<2>>,
    model_indices: MultiArray<usize, Shape<2>>,
    key_map: HashMap<(K1, K2), usize>,
    name_prefix: String,
}

impl<K1, K2, VT> IndexedVariableCombination2<K1, K2, VT>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
    VT: VariableTypeTrait + Clone,
    VT::Value: IntoValue<f64>,
{
    /// 创建并注册二维索引变量组合
    /// Create and register a 2D indexed variable combination
    pub fn new(
        prefix: &str,
        keys1: &[K1],
        keys2: &[K2],
        model: &mut MetaModel<f64>,
        name_gen: impl Fn(&K1, &K2) -> String,
        range_gen: impl Fn(&K1, &K2) -> VariableRange<VT::Value>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut key_map = HashMap::new();
        for (i, k1) in keys1.iter().enumerate() {
            for (j, k2) in keys2.iter().enumerate() {
                key_map.insert((k1.clone(), k2.clone()), i * keys2.len() + j);
            }
        }

        let keys1_ref = keys1;
        let keys2_ref = keys2;
        let combination = VariableCombination::with_name_and_range_generator(
            Shape::new([keys1.len(), keys2.len()]),
            prefix,
            |_index, vector| {
                name_gen(&keys1_ref[vector[0]], &keys2_ref[vector[1]])
            },
            |_index, vector| {
                range_gen(&keys1_ref[vector[0]], &keys2_ref[vector[1]])
            },
        );

        let model_indices = model.register_combination(&combination)?;

        Ok(Self {
            combination,
            model_indices,
            key_map,
            name_prefix: prefix.to_string(),
        })
    }

    /// 获取领域键对对应的模型索引
    /// Get model index for domain key pair
    pub fn model_index(&self, k1: &K1, k2: &K2) -> Option<usize> {
        self.key_map.get(&(k1.clone(), k2.clone())).map(|&i| {
            let row = i / self.model_indices.shape[1];
            let col = i % self.model_indices.shape[1];
            self.model_indices[&[row, col]]
        })
    }

    /// 获取领域键对对应的模型索引（无检查）
    /// Get model index for domain key pair (unchecked)
    pub fn model_index_unchecked(&self, k1: &K1, k2: &K2) -> usize {
        let i = self.key_map[&(k1.clone(), k2.clone())];
        let row = i / self.model_indices.shape[1];
        let col = i % self.model_indices.shape[1];
        self.model_indices[&[row, col]]
    }

    /// 获取底层变量组合引用
    /// Get reference to underlying variable combination
    pub fn combination(&self) -> &VariableCombination<VT, Shape<2>> {
        &self.combination
    }

    /// 获取模型索引数组引用
    /// Get reference to model indices array
    pub fn model_indices(&self) -> &MultiArray<usize, Shape<2>> {
        &self.model_indices
    }

    /// 获取键映射引用
    /// Get reference to key mapping
    pub fn key_map(&self) -> &HashMap<(K1, K2), usize> {
        &self.key_map
    }

    /// 获取名称前缀
    /// Get name prefix
    pub fn name_prefix(&self) -> &str {
        &self.name_prefix
    }

    /// 获取变量数量
    /// Get number of variables
    pub fn len(&self) -> usize {
        self.combination.len()
    }
}

impl<K1, K2, VT> std::fmt::Debug for IndexedVariableCombination2<K1, K2, VT>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
    VT: VariableTypeTrait,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexedVariableCombination2")
            .field("name_prefix", &self.name_prefix)
            .field("key_map", &self.key_map)
            .field("len", &self.combination.len())
            .finish()
    }
}

// ============================================================================
// IndexedLinearExpressionSymbols1 - 一维索引符号组合
// ============================================================================

/// 一维索引线性表达式符号组合 / 1D Indexed Linear Expression Symbol Combination
///
/// 将领域键 `K` 映射到一维 `SymbolCombination` 的线性索引。
/// Maps domain key `K` to linear index of a 1D `SymbolCombination`.
#[derive(Clone)]
pub struct IndexedLinearExpressionSymbols1<K>
where
    K: Debug + Clone + Eq + Hash,
{
    combination: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    key_map: HashMap<K, usize>,
    name_prefix: String,
}

impl<K> IndexedLinearExpressionSymbols1<K>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 创建一维索引符号组合（从已有组合）
    /// Create a 1D indexed symbol combination (from existing combination)
    pub fn new(
        prefix: &str,
        keys: &[K],
        combination: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    ) -> Self {
        let key_map: HashMap<K, usize> = keys
            .iter()
            .enumerate()
            .map(|(i, k)| (k.clone(), i))
            .collect();

        Self {
            combination,
            key_map,
            name_prefix: prefix.to_string(),
        }
    }

    /// 获取领域键对应的符号多项式
    /// Get symbol polynomial for domain key
    pub fn symbol_polynomial(&self, key: &K) -> Option<ospf_rust_core::symbol::flatten::Linear<f64>> {
        let &i = self.key_map.get(key)?;
        Some(self.combination.symbol_polynomial(i))
    }

    /// 获取领域键对应的符号多项式（无检查）
    /// Get symbol polynomial for domain key (unchecked)
    pub fn symbol_polynomial_unchecked(&self, key: &K) -> ospf_rust_core::symbol::flatten::Linear<f64> {
        let i = self.key_map[key];
        self.combination.symbol_polynomial(i)
    }

    /// 注册符号组合到模型
    /// Register symbol combination to model
    pub fn register(&self, model: &mut MetaModel<f64>) -> Result<(), Box<dyn std::error::Error>>
    where
        ospf_rust_core::symbol::LinearExpressionSymbol<f64>: 'static,
    {
        model.add_symbol_combination(&self.combination)?;
        Ok(())
    }

    /// 获取底层符号组合引用
    /// Get reference to underlying symbol combination
    pub fn combination(&self) -> &SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>> {
        &self.combination
    }

    /// 获取键映射引用
    /// Get reference to key mapping
    pub fn key_map(&self) -> &HashMap<K, usize> {
        &self.key_map
    }

    /// 获取名称前缀
    /// Get name prefix
    pub fn name_prefix(&self) -> &str {
        &self.name_prefix
    }

    /// 获取符号数量
    /// Get number of symbols
    pub fn len(&self) -> usize {
        self.combination.len()
    }
}

impl<K> std::fmt::Debug for IndexedLinearExpressionSymbols1<K>
where
    K: Debug + Clone + Eq + Hash,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexedLinearExpressionSymbols1")
            .field("name_prefix", &self.name_prefix)
            .field("key_map", &self.key_map)
            .field("len", &self.combination.len())
            .finish()
    }
}

// ============================================================================
// IndexedLinearExpressionSymbols2 - 二维索引符号组合
// ============================================================================

/// 二维索引线性表达式符号组合 / 2D Indexed Linear Expression Symbol Combination
pub struct IndexedLinearExpressionSymbols2<K1, K2>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
{
    combination: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<2>>,
    key_map: HashMap<(K1, K2), usize>,
    name_prefix: String,
}

impl<K1, K2> IndexedLinearExpressionSymbols2<K1, K2>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
{
    /// 创建二维索引符号组合（从已有组合）
    /// Create a 2D indexed symbol combination (from existing combination)
    pub fn new(
        prefix: &str,
        keys1: &[K1],
        keys2: &[K2],
        combination: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<2>>,
    ) -> Self {
        let mut key_map = HashMap::new();
        for (i, k1) in keys1.iter().enumerate() {
            for (j, k2) in keys2.iter().enumerate() {
                key_map.insert((k1.clone(), k2.clone()), i * keys2.len() + j);
            }
        }

        Self {
            combination,
            key_map,
            name_prefix: prefix.to_string(),
        }
    }

    /// 获取领域键对对应的符号多项式
    /// Get symbol polynomial for domain key pair
    pub fn symbol_polynomial(&self, k1: &K1, k2: &K2) -> Option<ospf_rust_core::symbol::flatten::Linear<f64>> {
        let &i = self.key_map.get(&(k1.clone(), k2.clone()))?;
        let row = i / self.combination.as_array().shape[1];
        let col = i % self.combination.as_array().shape[1];
        Some(self.combination.symbol_polynomial_at(&[row, col]))
    }

    /// 注册符号组合到模型
    /// Register symbol combination to model
    pub fn register(&self, model: &mut MetaModel<f64>) -> Result<(), Box<dyn std::error::Error>>
    where
        LinearExpressionSymbol<f64>: 'static,
    {
        model.add_symbol_combination(&self.combination)?;
        Ok(())
    }

    /// 获取底层符号组合引用
    /// Get reference to underlying symbol combination
    pub fn combination(&self) -> &SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<2>> {
        &self.combination
    }

    /// 获取键映射引用
    /// Get reference to key mapping
    pub fn key_map(&self) -> &HashMap<(K1, K2), usize> {
        &self.key_map
    }

    /// 获取名称前缀
    /// Get name prefix
    pub fn name_prefix(&self) -> &str {
        &self.name_prefix
    }

    /// 获取符号数量
    /// Get number of symbols
    pub fn len(&self) -> usize {
        self.combination.len()
    }
}
