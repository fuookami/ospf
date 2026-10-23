//! 二元列变量组件及受管行边界管线。 / Binary column-variable component and managed row-bound pipelines.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use ospf_rust_core::model::{ConstraintGroup, ConstraintRelation, MetaModel};
use ospf_rust_core::variable::{Binary, VariableRange};
use crate::error::{NetworkSchedulingError, Result};

/// 稳定列 ID 与模型变量的索引，删除后保留固定为零的退休 token。 / Stable column-to-model indices; removed tokens are retained fixed at zero.
#[derive(Debug, Clone, Default)]
pub struct PathCompilation {
    pub(super) variables: BTreeMap<String, usize>,
    pub(super) retired: BTreeSet<usize>,
    serial: usize,
}

impl PathCompilation {
    /// 活动列变量 / Active column variables.
    pub fn variables(&self) -> &BTreeMap<String, usize> { &self.variables }

    /// 已删除且必须保持零值的旧变量 / Removed variables that must remain zero.
    pub fn retired_variables(&self) -> &BTreeSet<usize> { &self.retired }

    pub(super) fn add(&mut self, model: &mut MetaModel<f64>, id: &str, name: &str) -> Result<()> {
        let next = self.serial.checked_add(1).ok_or_else(|| NetworkSchedulingError::contract("路径变量序号溢出 / path-variable serial overflow"))?;
        let index = model.register_auto_variable_with_range::<Binary>(
            &format!("{name}_column_{}", self.serial), VariableRange::bounded(0.0, 1.0),
        ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        self.variables.insert(id.to_owned(), index);
        self.serial = next;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(super) struct PathCompilationRows {
    pub row: String,
    pub name: String,
    pub terms: Vec<(usize, f64)>,
    pub relation: ConstraintRelation,
    pub rhs: f64,
}

impl PathCompilationRows {
    pub fn register(&self, model: &mut MetaModel<f64>, group: &Arc<ConstraintGroup>) -> Result<()> {
        model.add_linear_constraint_with_metadata(&self.terms, self.relation, self.rhs, &self.name,
            Some(group.clone()), false, 0, None).map_err(|error| NetworkSchedulingError::model(error.to_string()))
    }
}
