//! 生产任务 trait / Production task trait
//!
//! 扩展 TaskTrait 以支持产出和消耗数量查询。
//! Extends TaskTrait to support produce and consumption quantity queries.

use crate::domain::task::{ExecutorTrait, AssignmentPolicyTrait, TaskTrait};

/// 生产任务 trait / Production task trait
///
/// 扩展 `TaskTrait`，增加按物料 ID 查询产出和消耗量的能力。
/// Extends `TaskTrait` with the ability to query produce and consumption quantities by material ID.
pub trait ProductionTaskTrait<E, A>: TaskTrait<E, A>
where
    E: ExecutorTrait,
    A: AssignmentPolicyTrait<E>,
{
    /// 获取任务的产出量 / Get task's produce quantity
    ///
    /// 返回指定产品 ID 的产出量（solver 值域）。
    /// Returns the produce quantity for the specified product ID (solver value domain).
    fn produce_quantity(&self, product_id: &str) -> f64;

    /// 获取任务的消耗量 / Get task's consumption quantity
    ///
    /// 返回指定物料 ID 的消耗量（solver 值域）。
    /// Returns the consumption quantity for the specified material ID (solver value domain).
    fn consumption_quantity(&self, material_id: &str) -> f64;
}
