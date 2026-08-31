//! 资源 trait 定义 / Resource trait definitions
//!
//! 定义三类资源的核心接口：执行资源、存储资源、连接资源。
//! Defines core interfaces for three resource types: execution, storage, and connection.

use std::fmt::Debug;
use super::capacity::ResourceCapacity;
use crate::infrastructure::TimeRange;

/// 资源 trait / Resource trait
///
/// 所有资源的基础接口，提供 ID、名称、容量列表和初始量。
/// Base interface for all resources, providing ID, name, capacity list, and initial quantity.
pub trait ResourceTrait: Send + Sync + Debug + 'static {
    /// 资源 ID / Resource ID
    fn id(&self) -> &str;
    /// 资源名称 / Resource name
    fn name(&self) -> &str;
    /// 容量列表 / Capacity list
    fn capacities(&self) -> &[ResourceCapacity];
    /// 初始量（solver 值域）/ Initial quantity in solver value domain
    fn initial_quantity(&self) -> f64;
}

/// 执行资源 trait / Execution resource trait
///
/// 任务级消耗资源。每个任务在该资源上的消耗量通过 `used_by` 计算。
/// Task-level consumption resource. Each task's consumption is computed via `used_by`.
pub trait ExecutionResourceTrait: ResourceTrait {
    /// 计算任务在时间范围内的消耗量 / Compute task consumption in time range
    ///
    /// 返回值在 solver 值域。
    /// Returns value in solver value domain.
    fn used_by(&self, task_index: usize, time_range: &TimeRange) -> f64;
}

/// 存储资源 trait / Storage resource trait
///
/// 库存型资源，支持固定和任务级的消耗/供给。
/// Inventory resource, supporting fixed and task-level cost/supply.
pub trait StorageResourceTrait: ResourceTrait {
    /// 计算任务在给定持续时间的消耗量 / Compute task consumption for given duration
    fn cost_by(&self, task_index: usize, duration: f64) -> f64;
    /// 计算任务在给定持续时间的供给量 / Compute task supply for given duration
    fn supply_by(&self, task_index: usize, duration: f64) -> f64;
    /// 固定消耗量（solver 值域）/ Fixed consumption in solver value domain
    fn fixed_cost_in(&self, duration: f64) -> f64;
    /// 固定供给量（solver 值域）/ Fixed supply in solver value domain
    fn fixed_supply_in(&self, duration: f64) -> f64;
}

/// 连接资源 trait / Connection resource trait
///
/// 任务间连接消耗资源。消耗量取决于前后两个任务的连接。
/// Inter-task connection resource. Consumption depends on the connection between two tasks.
pub trait ConnectionResourceTrait: ResourceTrait {
    /// 计算前后任务间连接消耗量 / Compute connection consumption between tasks
    fn used_by(&self, prev_task_index: Option<usize>, task_index: Option<usize>, time_range: &TimeRange) -> f64;
}

/// 基础执行资源 / Basic execution resource
///
/// 提供最简的执行资源实现，使用回调函数计算任务消耗。
/// Provides a minimal execution resource implementation using a callback for task consumption.
pub struct BasicExecutionResource {
    /// 资源 ID / Resource ID
    pub id: String,
    /// 资源名称 / Resource name
    pub name: String,
    /// 容量列表 / Capacity list
    pub capacities: Vec<ResourceCapacity>,
    /// 初始量 / Initial quantity
    pub initial_quantity: f64,
    /// 任务消耗计算回调 / Task consumption callback
    pub used_by_fn: Arc<dyn Fn(usize, &TimeRange) -> f64 + Send + Sync>,
}

impl std::fmt::Debug for BasicExecutionResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicExecutionResource")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("capacities", &self.capacities.len())
            .finish()
    }
}

use std::sync::Arc;

impl ResourceTrait for BasicExecutionResource {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }
    fn capacities(&self) -> &[ResourceCapacity] { &self.capacities }
    fn initial_quantity(&self) -> f64 { self.initial_quantity }
}

impl ExecutionResourceTrait for BasicExecutionResource {
    fn used_by(&self, task_index: usize, time_range: &TimeRange) -> f64 {
        (self.used_by_fn)(task_index, time_range)
    }
}

/// 基础存储资源 / Basic storage resource
///
/// 提供最简的存储资源实现，使用回调函数计算任务消耗/供给。
/// Provides a minimal storage resource implementation using callbacks for task cost/supply.
pub struct BasicStorageResource {
    /// 资源 ID / Resource ID
    pub id: String,
    /// 资源名称 / Resource name
    pub name: String,
    /// 容量列表 / Capacity list
    pub capacities: Vec<ResourceCapacity>,
    /// 初始量 / Initial quantity
    pub initial_quantity: f64,
    /// 任务消耗计算回调 / Task cost callback
    pub cost_by_fn: Arc<dyn Fn(usize, f64) -> f64 + Send + Sync>,
    /// 任务供给计算回调 / Task supply callback
    pub supply_by_fn: Arc<dyn Fn(usize, f64) -> f64 + Send + Sync>,
    /// 固定消耗率（每单位时间）/ Fixed cost rate per unit time
    pub fixed_cost_rate: f64,
    /// 固定供给率（每单位时间）/ Fixed supply rate per unit time
    pub fixed_supply_rate: f64,
}

impl std::fmt::Debug for BasicStorageResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicStorageResource")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("capacities", &self.capacities.len())
            .finish()
    }
}

impl ResourceTrait for BasicStorageResource {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }
    fn capacities(&self) -> &[ResourceCapacity] { &self.capacities }
    fn initial_quantity(&self) -> f64 { self.initial_quantity }
}

impl StorageResourceTrait for BasicStorageResource {
    fn cost_by(&self, task_index: usize, duration: f64) -> f64 {
        (self.cost_by_fn)(task_index, duration)
    }

    fn supply_by(&self, task_index: usize, duration: f64) -> f64 {
        (self.supply_by_fn)(task_index, duration)
    }

    fn fixed_cost_in(&self, duration: f64) -> f64 {
        self.fixed_cost_rate * duration
    }

    fn fixed_supply_in(&self, duration: f64) -> f64 {
        self.fixed_supply_rate * duration
    }
}

/// 基础连接资源 / Basic connection resource
///
/// 提供最简的连接资源实现，使用回调函数计算连接消耗。
/// Provides a minimal connection resource implementation using callback for connection consumption.
pub struct BasicConnectionResource {
    /// 资源 ID / Resource ID
    pub id: String,
    /// 资源名称 / Resource name
    pub name: String,
    /// 容量列表 / Capacity list
    pub capacities: Vec<ResourceCapacity>,
    /// 初始量 / Initial quantity
    pub initial_quantity: f64,
    /// 连接消耗计算回调 / Connection consumption callback
    pub used_by_fn: Arc<dyn Fn(Option<usize>, Option<usize>, &TimeRange) -> f64 + Send + Sync>,
}

impl std::fmt::Debug for BasicConnectionResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicConnectionResource")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("capacities", &self.capacities.len())
            .finish()
    }
}

impl ResourceTrait for BasicConnectionResource {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }
    fn capacities(&self) -> &[ResourceCapacity] { &self.capacities }
    fn initial_quantity(&self) -> f64 { self.initial_quantity }
}

impl ConnectionResourceTrait for BasicConnectionResource {
    fn used_by(&self, prev_task_index: Option<usize>, task_index: Option<usize>, time_range: &TimeRange) -> f64 {
        (self.used_by_fn)(prev_task_index, task_index, time_range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::TimeRange;
    use time::OffsetDateTime;
    use time::ext::NumericalDuration;

    fn test_time_range() -> TimeRange {
        TimeRange::new(
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH + 1.hours(),
        )
    }

    #[test]
    fn test_basic_execution_resource() {
        let resource = BasicExecutionResource {
            id: "machine_1".to_string(),
            name: "Machine 1".to_string(),
            capacities: vec![ResourceCapacity::new(test_time_range(), 0.0, 100.0)],
            initial_quantity: 0.0,
            used_by_fn: Arc::new(|_task, _time| 1.0),
        };
        assert_eq!(resource.id(), "machine_1");
        assert_eq!(resource.capacities().len(), 1);
        assert_eq!(resource.used_by(0, &test_time_range()), 1.0);
    }

    #[test]
    fn test_basic_storage_resource() {
        let resource = BasicStorageResource {
            id: "warehouse_1".to_string(),
            name: "Warehouse 1".to_string(),
            capacities: vec![ResourceCapacity::new(test_time_range(), 0.0, 1000.0)],
            initial_quantity: 500.0,
            cost_by_fn: Arc::new(|_task, duration| duration * 2.0),
            supply_by_fn: Arc::new(|_task, duration| duration * 1.0),
            fixed_cost_rate: 0.5,
            fixed_supply_rate: 0.0,
        };
        assert_eq!(resource.id(), "warehouse_1");
        assert_eq!(resource.initial_quantity(), 500.0);
        assert_eq!(resource.cost_by(0, 10.0), 20.0);
        assert_eq!(resource.supply_by(0, 10.0), 10.0);
        assert_eq!(resource.fixed_cost_in(10.0), 5.0);
        assert_eq!(resource.fixed_supply_in(10.0), 0.0);
    }

    #[test]
    fn test_basic_connection_resource() {
        let resource = BasicConnectionResource {
            id: "transport_1".to_string(),
            name: "Transport 1".to_string(),
            capacities: vec![ResourceCapacity::new(test_time_range(), 0.0, 100.0)],
            initial_quantity: 0.0,
            used_by_fn: Arc::new(|prev, next, _time| {
                match (prev, next) {
                    (Some(_), Some(_)) => 1.0,
                    _ => 0.0,
                }
            }),
        };
        assert_eq!(resource.id(), "transport_1");
        assert_eq!(resource.used_by(Some(0), Some(1), &test_time_range()), 1.0);
        assert_eq!(resource.used_by(None, Some(0), &test_time_range()), 0.0);
    }
}
