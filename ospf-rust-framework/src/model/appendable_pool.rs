//! 列生成适配器
//! Column Generation Pool Adapters
//!
//! 支持 `register_initial` / `add_columns` / `remove_columns` 生命周期。
//! Supports `register_initial` / `add_columns` / `remove_columns` lifecycle.
//!
//! 用于 BPP3D、CSP1D、Gantt 的列生成场景。
//! Used for column generation scenarios in BPP3D, CSP1D, and Gantt.

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::token::IntoValue;
use ospf_rust_core::variable::{VariableRange, VariableTypeTrait};

// ============================================================================
// AppendableVariablePool - 可追加变量池
// ============================================================================

/// 可追加变量池 / Appendable Variable Pool
///
/// 支持初始注册、追加列、移除列的变量管理。
/// Supports initial registration, column addition, and column removal.
pub struct AppendableVariablePool<K, VT>
where
    K: Debug + Clone + Eq + Hash,
    VT: VariableTypeTrait,
{
    /// 所有键（包括已移除的）
    /// All keys (including removed ones)
    all_keys: Vec<K>,
    /// 活跃键到模型索引的映射
    /// Active key to model index mapping
    active_indices: HashMap<K, usize>,
    /// 已移除的键
    /// Removed keys
    removed_keys: Vec<K>,
    /// 变量名前缀
    /// Variable name prefix
    name_prefix: String,
    /// 当前迭代
    /// Current iteration
    iteration: usize,
    _phantom: std::marker::PhantomData<VT>,
}

impl<K, VT> AppendableVariablePool<K, VT>
where
    K: Debug + Clone + Eq + Hash,
    VT: VariableTypeTrait + Clone,
    VT::Value: IntoValue<f64>,
{
    /// 创建可追加变量池
    /// Create appendable variable pool
    pub fn new(prefix: &str) -> Self {
        Self {
            all_keys: Vec::new(),
            active_indices: HashMap::new(),
            removed_keys: Vec::new(),
            name_prefix: prefix.to_string(),
            iteration: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    /// 注册初始列
    /// Register initial columns
    pub fn register_initial(
        &mut self,
        keys: &[K],
        model: &mut MetaModel<f64>,
        name_gen: impl Fn(&K, usize) -> String,
        range_gen: impl Fn(&K) -> VariableRange<VT::Value>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for key in keys {
            let name = name_gen(key, self.iteration);
            let range = range_gen(key);
            let var = ospf_rust_core::variable::VariableItem::<VT>::auto_with_range(&name, range);
            let idx = model.register_variable(var)?;
            self.active_indices.insert(key.clone(), idx);
            self.all_keys.push(key.clone());
        }
        self.iteration += 1;
        Ok(())
    }

    /// 追加新列
    /// Add new columns
    pub fn add_columns(
        &mut self,
        keys: &[K],
        model: &mut MetaModel<f64>,
        name_gen: impl Fn(&K, usize) -> String,
        range_gen: impl Fn(&K) -> VariableRange<VT::Value>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for key in keys {
            if self.active_indices.contains_key(key) {
                continue; // 已存在，跳过
            }
            let name = name_gen(key, self.iteration);
            let range = range_gen(key);
            let var = ospf_rust_core::variable::VariableItem::<VT>::auto_with_range(&name, range);
            let idx = model.register_variable(var)?;
            self.active_indices.insert(key.clone(), idx);
            self.all_keys.push(key.clone());
        }
        self.iteration += 1;
        Ok(())
    }

    /// 移除列（标记为 retired，不实际删除）
    /// Remove columns (mark as retired, don't actually delete)
    pub fn remove_columns(&mut self, keys: &[K]) {
        for key in keys {
            if self.active_indices.remove(key).is_some() {
                self.removed_keys.push(key.clone());
            }
        }
    }

    /// 获取活跃键的模型索引
    /// Get model index for active key
    pub fn model_index(&self, key: &K) -> Option<usize> {
        self.active_indices.get(key).copied()
    }

    /// 获取活跃键的模型索引（无检查）
    /// Get model index for active key (unchecked)
    pub fn model_index_unchecked(&self, key: &K) -> usize {
        self.active_indices[key]
    }

    /// 检查键是否活跃
    /// Check if key is active
    pub fn is_active(&self, key: &K) -> bool {
        self.active_indices.contains_key(key)
    }

    /// 遍历所有活跃键和索引
    /// Iterate over all active keys and indices
    pub fn active_iter(&self) -> impl Iterator<Item = (&K, &usize)> {
        self.active_indices.iter()
    }

    /// 获取活跃变量数量
    /// Get number of active variables
    pub fn active_len(&self) -> usize {
        self.active_indices.len()
    }

    /// 获取总变量数量（包括已移除）
    /// Get total number of variables (including removed)
    pub fn total_len(&self) -> usize {
        self.all_keys.len()
    }

    /// 获取已移除键列表
    /// Get removed keys
    pub fn removed_keys(&self) -> &[K] {
        &self.removed_keys
    }

    /// 获取当前迭代
    /// Get current iteration
    pub fn iteration(&self) -> usize {
        self.iteration
    }

    /// 获取名称前缀
    /// Get name prefix
    pub fn name_prefix(&self) -> &str {
        &self.name_prefix
    }
}

// ============================================================================
// AppendableSymbolPool - 可追加符号池
// ============================================================================

/// 可追加符号池 / Appendable Symbol Pool
///
/// 支持初始注册、追加列、移除列的符号管理。
/// Supports initial registration, column addition, and column removal.
pub struct AppendableSymbolPool<K>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 活跃键到线性索引的映射
    /// Active key to linear index mapping
    active_indices: HashMap<K, usize>,
    /// 已移除的键
    /// Removed keys
    removed_keys: Vec<K>,
    /// 符号名前缀
    /// Symbol name prefix
    name_prefix: String,
    /// 当前迭代
    /// Current iteration
    iteration: usize,
}

impl<K> AppendableSymbolPool<K>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 创建可追加符号池
    /// Create appendable symbol pool
    pub fn new(prefix: &str) -> Self {
        Self {
            active_indices: HashMap::new(),
            removed_keys: Vec::new(),
            name_prefix: prefix.to_string(),
            iteration: 0,
        }
    }

    /// 注册初始符号
    /// Register initial symbols
    pub fn register_initial(&mut self, keys: &[K], indices: &[usize]) {
        for (key, &idx) in keys.iter().zip(indices.iter()) {
            self.active_indices.insert(key.clone(), idx);
        }
        self.iteration += 1;
    }

    /// 追加新符号
    /// Add new symbols
    pub fn add_symbols(&mut self, keys: &[K], indices: &[usize]) {
        for (key, &idx) in keys.iter().zip(indices.iter()) {
            if !self.active_indices.contains_key(key) {
                self.active_indices.insert(key.clone(), idx);
            }
        }
        self.iteration += 1;
    }

    /// 移除符号
    /// Remove symbols
    pub fn remove_symbols(&mut self, keys: &[K]) {
        for key in keys {
            if self.active_indices.remove(key).is_some() {
                self.removed_keys.push(key.clone());
            }
        }
    }

    /// 获取活跃键的符号索引
    /// Get symbol index for active key
    pub fn symbol_index(&self, key: &K) -> Option<usize> {
        self.active_indices.get(key).copied()
    }

    /// 检查键是否活跃
    /// Check if key is active
    pub fn is_active(&self, key: &K) -> bool {
        self.active_indices.contains_key(key)
    }

    /// 遍历所有活跃键和索引
    /// Iterate over all active keys and indices
    pub fn active_iter(&self) -> impl Iterator<Item = (&K, &usize)> {
        self.active_indices.iter()
    }

    /// 获取活跃符号数量
    /// Get number of active symbols
    pub fn active_len(&self) -> usize {
        self.active_indices.len()
    }

    /// 获取已移除键列表
    /// Get removed keys
    pub fn removed_keys(&self) -> &[K] {
        &self.removed_keys
    }

    /// 获取当前迭代
    /// Get current iteration
    pub fn iteration(&self) -> usize {
        self.iteration
    }

    /// 获取名称前缀
    /// Get name prefix
    pub fn name_prefix(&self) -> &str {
        &self.name_prefix
    }
}
