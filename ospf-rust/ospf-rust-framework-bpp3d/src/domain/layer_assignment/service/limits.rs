//! 层分配约束和目标 Pipeline / Layer assignment constraint and objective pipelines
//!
//! 实现层分配 RMP 和 Final MILP 阶段的约束族和目标族。
//! Implements constraint and objective families for layer assignment
//! RMP and Final MILP phases.

use ospf_rust_core::error::Result;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::symbol::LinearIntermediateSymbol;
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;
use ospf_rust_framework::model::pipeline::CGPipeline;
use ospf_rust_framework::model::pipeline::Pipeline;
use ospf_rust_framework::model::shadow_price::{
    BasicShadowPriceMap, ShadowPrice, ShadowPriceKey, ShadowPriceMap,
};
use std::fmt::Debug;
use std::sync::Arc;

use super::{
    Bpp3dDemandEntry, Bpp3dSolverValueAdapter, Bpp3dSolverValueAdapterKind, DemandShadowPriceKey,
    ImpreciseAssignment, PreciseAssignment,
};
use crate::domain::item::BinType;

include!("limits/demand_constraint.rs");
include!("limits/precise_assignment_activation_constraint.rs");
include!("limits/bin_capacity_constraint.rs");
include!("limits/bin_depth_constraint.rs");
include!("limits/bin_amount_minimization.rs");
include!("limits/volume_minimization.rs");
include!("limits/better_layer_maximization.rs");
include!("limits/tail_bin_assignment_constraint.rs");
include!("limits/deferred.rs");
include!("limits/tests.rs");
