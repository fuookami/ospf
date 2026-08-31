//! Length assignment domain model types
//!
//! Contains [`LengthSlackVariables`] which wraps [`OptionalIndexedVariableArray`] for
//! managing assigned-length and over-length slack variable indices.

use std::fmt;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::variable::{UContinuous, VariableRange};
use ospf_rust_framework::model::OptionalIndexedVariableArray;

/// Length slack variable tracking using [`OptionalIndexedVariableArray`].
///
/// Manages assigned-length and over-length slack variable indices,
/// replacing raw `Vec<Option<usize>>` with typed, indexed access backed by
/// [`OptionalIndexedVariableArray<usize, UContinuous>`].
///
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
    /// Create a new length slack variables tracker.
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

    /// Clear all registered variables for re-registration.
    pub fn clear(&mut self) {
        self.inner_assigned = OptionalIndexedVariableArray::new(&self.prefix_assigned);
        self.inner_over = OptionalIndexedVariableArray::new(&self.prefix_over);
        self.assigned_cache.clear();
        self.over_cache.clear();
    }

    /// Record that no assigned-length variable exists for this demand index.
    pub fn push_assigned_none(&mut self) {
        self.assigned_cache.push(None);
    }

    /// Record that no over-length variable exists for this demand index.
    pub fn push_over_none(&mut self) {
        self.over_cache.push(None);
    }

    /// Register an assigned-length variable for the given demand index.
    ///
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

    /// Register an over-length variable for the given demand index.
    ///
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

    /// Get the assigned-length variable index for a demand.
    pub fn assigned_index(&self, demand_index: usize) -> Option<usize> {
        self.assigned_cache.get(demand_index).copied().flatten()
    }

    /// Get the over-length variable index for a demand.
    pub fn over_index(&self, demand_index: usize) -> Option<usize> {
        self.over_cache.get(demand_index).copied().flatten()
    }

    /// Backward-compatible positional access to assigned-length indices.
    pub fn assigned_length(&self) -> &[Option<usize>] {
        &self.assigned_cache
    }

    /// Backward-compatible positional access to over-length indices.
    pub fn over_length(&self) -> &[Option<usize>] {
        &self.over_cache
    }

    /// Whether any slack variables have been registered.
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
