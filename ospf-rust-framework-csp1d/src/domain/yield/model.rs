//! Yield domain model types
//!
//! Contains [`YieldSlackVariables`] which wraps [`OptionalIndexedVariableArray`] for
//! managing under-production and over-production slack variable indices.

use std::fmt;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::variable::{UContinuous, VariableRange};
use ospf_rust_framework::model::OptionalIndexedVariableArray;

/// Yield slack variable tracking using [`OptionalIndexedVariableArray`].
///
/// Manages under-production and over-production slack variable indices,
/// replacing raw `Vec<Option<usize>>` with typed, indexed access backed by
/// [`OptionalIndexedVariableArray<usize, UContinuous>`].
///
/// A parallel `Vec<Option<usize>>` cache is maintained for backward-compatible
/// positional access and to support `Clone` semantics.
pub struct YieldSlackVariables {
    inner_under: OptionalIndexedVariableArray<usize, UContinuous>,
    inner_over: OptionalIndexedVariableArray<usize, UContinuous>,
    under_cache: Vec<Option<usize>>,
    over_cache: Vec<Option<usize>>,
    prefix_under: String,
    prefix_over: String,
}

impl YieldSlackVariables {
    /// Create a new yield slack variables tracker.
    pub fn new() -> Self {
        Self {
            inner_under: OptionalIndexedVariableArray::new("under_production"),
            inner_over: OptionalIndexedVariableArray::new("over_production"),
            under_cache: Vec::new(),
            over_cache: Vec::new(),
            prefix_under: "under_production".to_string(),
            prefix_over: "over_production".to_string(),
        }
    }

    /// Clear all registered variables for re-registration.
    pub fn clear(&mut self) {
        self.inner_under = OptionalIndexedVariableArray::new(&self.prefix_under);
        self.inner_over = OptionalIndexedVariableArray::new(&self.prefix_over);
        self.under_cache.clear();
        self.over_cache.clear();
    }

    /// Record that no under-production variable exists for this demand index.
    pub fn push_under_none(&mut self) {
        self.under_cache.push(None);
    }

    /// Record that no over-production variable exists for this demand index.
    pub fn push_over_none(&mut self) {
        self.over_cache.push(None);
    }

    /// Register an under-production variable for the given demand index.
    ///
    /// Registers in the model via the inner [`OptionalIndexedVariableArray`] and
    /// records the index in the positional cache.
    pub fn register_under(
        &mut self,
        demand_index: usize,
        model: &mut MetaModel<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while self.under_cache.len() <= demand_index {
            self.under_cache.push(None);
        }
        let idx = self.inner_under.register_if_needed(
            demand_index,
            model,
            |key| format!("under_production_{key}"),
            |_| VariableRange::new(Some(0.0), None),
        )?;
        self.under_cache[demand_index] = Some(idx);
        Ok(())
    }

    /// Register an over-production variable for the given demand index.
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
            |key| format!("over_production_{key}"),
            |_| VariableRange::new(Some(0.0), None),
        )?;
        self.over_cache[demand_index] = Some(idx);
        Ok(())
    }

    /// Get the under-production variable index for a demand.
    pub fn under_index(&self, demand_index: usize) -> Option<usize> {
        self.under_cache.get(demand_index).copied().flatten()
    }

    /// Get the over-production variable index for a demand.
    pub fn over_index(&self, demand_index: usize) -> Option<usize> {
        self.over_cache.get(demand_index).copied().flatten()
    }

    /// Backward-compatible positional access to under-production indices.
    pub fn under_production(&self) -> &[Option<usize>] {
        &self.under_cache
    }

    /// Backward-compatible positional access to over-production indices.
    pub fn over_production(&self) -> &[Option<usize>] {
        &self.over_cache
    }

    /// Whether any slack variables have been registered.
    pub fn has_any(&self) -> bool {
        !self.inner_under.is_empty() || !self.inner_over.is_empty()
    }
}

impl Default for YieldSlackVariables {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for YieldSlackVariables {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("YieldSlackVariables")
            .field("under_count", &self.inner_under.len())
            .field("over_count", &self.inner_over.len())
            .finish()
    }
}

impl Clone for YieldSlackVariables {
    fn clone(&self) -> Self {
        Self {
            inner_under: OptionalIndexedVariableArray::new(&self.prefix_under),
            inner_over: OptionalIndexedVariableArray::new(&self.prefix_over),
            under_cache: self.under_cache.clone(),
            over_cache: self.over_cache.clone(),
            prefix_under: self.prefix_under.clone(),
            prefix_over: self.prefix_over.clone(),
        }
    }
}
