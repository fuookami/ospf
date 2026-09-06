//! 长度分配领域模型类型 / Length assignment domain model types
//!
//! 包含 [`LengthSlackVariables`]，封装 [`OptionalIndexedVariableArray`] 用于
//! 管理已分配长度和超长松弛变量索引。
//! Contains [`LengthSlackVariables`] which wraps [`OptionalIndexedVariableArray`] for
//! managing assigned-length and over-length slack variable indices.

use std::fmt;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::variable::{UContinuous, VariableRange};
use ospf_rust_framework::model::OptionalIndexedVariableArray;

/// 使用 [`OptionalIndexedVariableArray`] 的长度松弛变量跟踪。 / Length slack variable tracking using [`OptionalIndexedVariableArray`].
///
/// 管理已分配长度和超长松弛变量索引，
/// 用基于 [`OptionalIndexedVariableArray<usize, UContinuous>`] 的类型化索引访问
/// 替代原始 `Vec<Option<usize>>`。
/// Manages assigned-length and over-length slack variable indices,
/// replacing raw `Vec<Option<usize>>` with typed, indexed access backed by
/// [`OptionalIndexedVariableArray<usize, UContinuous>`].
///
/// 维护并行的 `Vec<Option<usize>>` 缓存以提供向后兼容的位置访问
/// 并支持 `Clone` 语义。
/// A parallel `Vec<Option<usize>>` cache is maintained for backward-compatible
/// positional access and to support `Clone` semantics.
pub struct LengthSlackVariables {
    inner_assigned: OptionalIndexedVariableArray<usize, UContinuous>,
    inner_over: OptionalIndexedVariableArray<usize, UContinuous>,
    assigned_cache: Vec<Option<usize>>,
    over_cache: Vec<Option<usize>>,
    prefix_assigned: String,
    prefix_over: String,
}

impl LengthSlackVariables {
    /// 创建新的长度松弛变量跟踪器。 / Create a new length slack variables tracker.
    pub fn new() -> Self {
        Self {
            inner_assigned: OptionalIndexedVariableArray::new("assigned_length"),
            inner_over: OptionalIndexedVariableArray::new("over_length"),
            assigned_cache: Vec::new(),
            over_cache: Vec::new(),
            prefix_assigned: "assigned_length".to_string(),
            prefix_over: "over_length".to_string(),
        }
    }

    /// 清除所有已注册变量以便重新注册。 / Clear all registered variables for re-registration.
    pub fn clear(&mut self) {
        self.inner_assigned = OptionalIndexedVariableArray::new(&self.prefix_assigned);
        self.inner_over = OptionalIndexedVariableArray::new(&self.prefix_over);
        self.assigned_cache.clear();
        self.over_cache.clear();
    }

    /// 记录此需求索引不存在已分配长度变量。 / Record that no assigned-length variable exists for this demand index.
    pub fn push_assigned_none(&mut self) {
        self.assigned_cache.push(None);
    }

    /// 记录此需求索引不存在超长变量。 / Record that no over-length variable exists for this demand index.
    pub fn push_over_none(&mut self) {
        self.over_cache.push(None);
    }

    /// 为给定需求索引注册已分配长度变量。
    /// Register an assigned-length variable for the given demand index.
    ///
    /// 通过内部 [`OptionalIndexedVariableArray`] 在模型中注册，
    /// 并将索引记录到位置缓存中。
    /// Registers in the model via the inner [`OptionalIndexedVariableArray`] and
    /// records the index in the positional cache.
    pub fn register_assigned(
        &mut self,
        demand_index: usize,
        model: &mut MetaModel<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while self.assigned_cache.len() <= demand_index {
            self.assigned_cache.push(None);
        }
        let idx = self.inner_assigned.register_if_needed(
            demand_index,
            model,
            |key| format!("assigned_length_{key}"),
            |_| VariableRange::new(Some(0.0), None),
        )?;
        self.assigned_cache[demand_index] = Some(idx);
        Ok(())
    }

    /// 为给定需求索引注册超长变量。
    /// Register an over-length variable for the given demand index.
    ///
    /// 通过内部 [`OptionalIndexedVariableArray`] 在模型中注册，
    /// 并将索引记录到位置缓存中。
    /// Registers in the model via the inner [`OptionalIndexedVariableArray`] and
    /// records the index in the positional cache.
    pub fn register_over(
        &mut self,
        demand_index: usize,
        model: &mut MetaModel<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while self.over_cache.len() <= demand_index {
            self.over_cache.push(None);
        }
        let idx = self.inner_over.register_if_needed(
            demand_index,
            model,
            |key| format!("over_length_{key}"),
            |_| VariableRange::new(Some(0.0), None),
        )?;
        self.over_cache[demand_index] = Some(idx);
        Ok(())
    }

    /// 获取需求的已分配长度变量索引。 / Get the assigned-length variable index for a demand.
    pub fn assigned_index(&self, demand_index: usize) -> Option<usize> {
        self.assigned_cache.get(demand_index).copied().flatten()
    }

    /// 获取需求的超长变量索引。 / Get the over-length variable index for a demand.
    pub fn over_index(&self, demand_index: usize) -> Option<usize> {
        self.over_cache.get(demand_index).copied().flatten()
    }

    /// 向后兼容的已分配长度索引位置访问。 / Backward-compatible positional access to assigned-length indices.
    pub fn assigned_length(&self) -> &[Option<usize>] {
        &self.assigned_cache
    }

    /// 向后兼容的超长索引位置访问。 / Backward-compatible positional access to over-length indices.
    pub fn over_length(&self) -> &[Option<usize>] {
        &self.over_cache
    }

    /// 是否已注册任何松弛变量。 / Whether any slack variables have been registered.
    pub fn has_any(&self) -> bool {
        !self.inner_assigned.is_empty() || !self.inner_over.is_empty()
    }
}

impl Default for LengthSlackVariables {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for LengthSlackVariables {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LengthSlackVariables")
            .field("assigned_count", &self.inner_assigned.len())
            .field("over_count", &self.inner_over.len())
            .finish()
    }
}

impl Clone for LengthSlackVariables {
    fn clone(&self) -> Self {
        Self {
            inner_assigned: OptionalIndexedVariableArray::new(&self.prefix_assigned),
            inner_over: OptionalIndexedVariableArray::new(&self.prefix_over),
            assigned_cache: self.assigned_cache.clone(),
            over_cache: self.over_cache.clone(),
            prefix_assigned: self.prefix_assigned.clone(),
            prefix_over: self.prefix_over.clone(),
        }
    }
}
