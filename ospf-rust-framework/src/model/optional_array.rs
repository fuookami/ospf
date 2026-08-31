//! 稀疏索引数组适配器
//! Optional Indexed Array Adapters
//!
//! 用于"部分领域键不建变量/符号"的场景：
//! For scenarios where some domain keys don't have variables/symbols:
//! - CSP1D under/over slack
//! - CSP1D assigned/over length
//! - Gantt switch time symbols
//! - Gantt masking/front/between sparse symbols

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::variable::{VariableTypeTrait, VariableRange};
use ospf_rust_core::token::IntoValue;

// ============================================================================
// OptionalIndexedVariableArray - 稀疏索引变量数组
// ============================================================================

/// 稀疏索引变量数组 / Optional Indexed Variable Array
///
/// 部分领域键有变量，部分没有。用于 slack、optional 等场景。
/// Some domain keys have variables, some don't. Used for slack, optional, etc.
pub struct OptionalIndexedVariableArray<K, VT>
where
    K: Debug + Clone + Eq + Hash,
    VT: VariableTypeTrait,
{
    /// 已注册变量的键到模型索引的映射
    /// Mapping from registered variable keys to model indices
    indices: HashMap<K, usize>,
    /// 变量名前缀
    /// Variable name prefix
    name_prefix: String,
    _phantom: std::marker::PhantomData<VT>,
}

impl<K, VT> OptionalIndexedVariableArray<K, VT>
where
    K: Debug + Clone + Eq + Hash,
    VT: VariableTypeTrait + Clone,
    VT::Value: IntoValue<f64>,
{
    /// 创建稀疏变量数组
    /// Create optional variable array
    pub fn new(prefix: &str) -> Self {
        Self {
            indices: HashMap::new(),
            name_prefix: prefix.to_string(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// 按需注册单个变量
    /// Register a single variable on demand
    pub fn register_if_needed(
        &mut self,
        key: K,
        model: &mut MetaModel<f64>,
        name_gen: impl Fn(&K) -> String,
        range_gen: impl Fn(&K) -> VariableRange<VT::Value>,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        if let Some(&idx) = self.indices.get(&key) {
            return Ok(idx);
        }

        let name = name_gen(&key);
        let range = range_gen(&key);
        let var = ospf_rust_core::variable::VariableItem::<VT>::auto_with_range(&name, range);
        let idx = model.register_variable(var)?;
        self.indices.insert(key, idx);
        Ok(idx)
    }

    /// 获取键对应的模型索引（可能不存在）
    /// Get model index for key (may not exist)
    pub fn model_index(&self, key: &K) -> Option<usize> {
        self.indices.get(key).copied()
    }

    /// 获取键对应的模型索引（无检查）
    /// Get model index for key (unchecked, panics if not registered)
    pub fn model_index_unchecked(&self, key: &K) -> usize {
        self.indices[key]
    }

    /// 检查键是否已注册
    /// Check if key is registered
    pub fn contains_key(&self, key: &K) -> bool {
        self.indices.contains_key(key)
    }

    /// 获取已注册变量数量
    /// Get number of registered variables
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// 是否为空
    /// Whether empty
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// 获取名称前缀
    /// Get name prefix
    pub fn name_prefix(&self) -> &str {
        &self.name_prefix
    }

    /// 遍历所有已注册的键和索引
    /// Iterate over all registered keys and indices
    pub fn iter(&self) -> impl Iterator<Item = (&K, &usize)> {
        self.indices.iter()
    }
}

// ============================================================================
// OptionalIndexedLinearExpressionSymbols - 稀疏索引符号组合
// ============================================================================

/// 稀疏索引线性表达式符号组合 / Optional Indexed Linear Expression Symbols
///
/// 部分领域键有符号，部分没有。
/// Some domain keys have symbols, some don't.
pub struct OptionalIndexedLinearExpressionSymbols<K>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 已注册符号的键到 (线性索引, 符号组合) 的映射
    /// Mapping from registered symbol keys to (linear index, symbol combination)
    symbols: HashMap<K, usize>,
    /// 符号名前缀
    /// Symbol name prefix
    name_prefix: String,
}

impl<K> OptionalIndexedLinearExpressionSymbols<K>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 创建稀疏符号组合
    /// Create optional symbol combination
    pub fn new(prefix: &str) -> Self {
        Self {
            symbols: HashMap::new(),
            name_prefix: prefix.to_string(),
        }
    }

    /// 插入符号（从已有 LinearExpressionSymbol）
    /// Insert symbol (from existing LinearExpressionSymbol)
    pub fn insert(&mut self, key: K, index: usize) {
        self.symbols.insert(key, index);
    }

    /// 获取键对应的符号索引
    /// Get symbol index for key
    pub fn symbol_index(&self, key: &K) -> Option<usize> {
        self.symbols.get(key).copied()
    }

    /// 检查键是否有符号
    /// Check if key has a symbol
    pub fn contains_key(&self, key: &K) -> bool {
        self.symbols.contains_key(key)
    }

    /// 获取已注册符号数量
    /// Get number of registered symbols
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// 是否为空
    /// Whether empty
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// 获取名称前缀
    /// Get name prefix
    pub fn name_prefix(&self) -> &str {
        &self.name_prefix
    }
}
