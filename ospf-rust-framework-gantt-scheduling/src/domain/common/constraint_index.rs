//! 约束索引映射 / Constraint index mapping
//!
//! 将业务 key、约束名称和求解器对偶向量索引稳定关联起来。
//! Stably associates business keys, constraint names, and solver dual-vector indices.

use std::collections::HashMap;

/// 约束索引键 / Constraint index key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstraintIndexKey {
    /// 任务编译约束 / Task compilation constraint
    TaskCompilation {
        /// 任务索引 / Task index
        task_index: usize,
    },
    /// 执行器编译约束 / Executor compilation constraint
    ExecutorCompilation {
        /// 执行器 ID / Executor id
        executor_id: String,
    },
    /// 执行器-时隙编译约束 / Executor-slot compilation constraint
    ExecutorSlotCompilation {
        /// 执行器 ID / Executor id
        executor_id: String,
        /// 时隙索引 / Slot index
        slot_index: usize,
    },
    /// 产能列选择约束 / Capacity-column selection constraint
    CapacityColumnSelection {
        /// 执行器 ID / Executor id
        executor_id: String,
        /// 时隙索引 / Slot index
        slot_index: usize,
    },
    /// 任务束约束 / Bunch constraint
    Bunch {
        /// 束索引 / Bunch index
        bunch_index: usize,
    },
    /// 自定义约束 / Custom constraint
    Custom {
        /// 约束族 / Constraint family
        family: String,
        /// 业务 key / Business key
        key: String,
    },
}

impl ConstraintIndexKey {
    /// 创建任务编译 key / Create task-compilation key
    pub fn task_compilation(task_index: usize) -> Self {
        Self::TaskCompilation { task_index }
    }

    /// 创建执行器编译 key / Create executor-compilation key
    pub fn executor_compilation(executor_id: impl Into<String>) -> Self {
        Self::ExecutorCompilation {
            executor_id: executor_id.into(),
        }
    }

    /// 创建执行器-时隙编译 key / Create executor-slot compilation key
    pub fn executor_slot_compilation(
        executor_id: impl Into<String>,
        slot_index: usize,
    ) -> Self {
        Self::ExecutorSlotCompilation {
            executor_id: executor_id.into(),
            slot_index,
        }
    }

    /// 创建产能列选择 key / Create capacity-column selection key
    pub fn capacity_column_selection(
        executor_id: impl Into<String>,
        slot_index: usize,
    ) -> Self {
        Self::CapacityColumnSelection {
            executor_id: executor_id.into(),
            slot_index,
        }
    }

    /// 创建自定义 key / Create custom key
    pub fn custom(family: impl Into<String>, key: impl Into<String>) -> Self {
        Self::Custom {
            family: family.into(),
            key: key.into(),
        }
    }
}

/// 约束索引条目 / Constraint index entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintIndexEntry {
    /// 业务 key / Business key
    pub key: ConstraintIndexKey,
    /// 约束名称 / Constraint name
    pub constraint_name: String,
    /// 对偶向量索引 / Dual vector index
    pub dual_index: usize,
}

/// 约束索引映射 / Constraint index map
#[derive(Debug, Clone, Default)]
pub struct ConstraintIndexMap {
    by_key: HashMap<ConstraintIndexKey, ConstraintIndexEntry>,
    by_name: HashMap<String, ConstraintIndexKey>,
}

impl ConstraintIndexMap {
    /// 创建空映射 / Create empty map
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册约束映射 / Register constraint mapping
    pub fn register(
        &mut self,
        key: ConstraintIndexKey,
        constraint_name: impl Into<String>,
        dual_index: usize,
    ) {
        let constraint_name = constraint_name.into();
        let entry = ConstraintIndexEntry {
            key: key.clone(),
            constraint_name: constraint_name.clone(),
            dual_index,
        };
        self.by_name.insert(constraint_name, key.clone());
        self.by_key.insert(key, entry);
    }

    /// 注册任务编译约束映射 / Register task-compilation constraint mappings
    pub fn register_task_compilation_constraints(
        &mut self,
        n_tasks: usize,
        constraint_name_to_index: &HashMap<String, usize>,
    ) {
        for task_index in 0..n_tasks {
            let constraint_name = format!("task_compilation_{}", task_index);
            if let Some(&dual_index) = constraint_name_to_index.get(&constraint_name) {
                self.register(
                    ConstraintIndexKey::task_compilation(task_index),
                    constraint_name,
                    dual_index,
                );
            }
        }
    }

    /// 注册执行器-时隙编译约束映射 / Register executor-slot compilation constraint mappings
    pub fn register_executor_slot_compilation_constraints<I>(
        &mut self,
        executor_ids: I,
        slot_count: usize,
        constraint_name_to_index: &HashMap<String, usize>,
    )
    where
        I: IntoIterator,
        I::Item: std::fmt::Display,
    {
        for executor_id in executor_ids {
            let executor_id = executor_id.to_string();
            for slot_index in 0..slot_count {
                let constraint_name = format!(
                    "executor_slot_compilation_{}_{}",
                    executor_id,
                    slot_index,
                );
                if let Some(&dual_index) = constraint_name_to_index.get(&constraint_name) {
                    self.register(
                        ConstraintIndexKey::executor_slot_compilation(
                            executor_id.clone(),
                            slot_index,
                        ),
                        constraint_name,
                        dual_index,
                    );
                }
            }
        }
    }

    /// 通过业务 key 查找条目 / Find entry by business key
    pub fn get(&self, key: &ConstraintIndexKey) -> Option<&ConstraintIndexEntry> {
        self.by_key.get(key)
    }

    /// 通过约束名称查找条目 / Find entry by constraint name
    pub fn get_by_name(&self, name: &str) -> Option<&ConstraintIndexEntry> {
        self.by_name
            .get(name)
            .and_then(|key| self.by_key.get(key))
    }

    /// 通过业务 key 提取对偶值 / Extract dual value by business key
    pub fn dual_value(&self, key: &ConstraintIndexKey, dual_values: &[f64]) -> Option<f64> {
        let entry = self.get(key)?;
        dual_values.get(entry.dual_index).copied()
    }

    /// 提取任务编译影子价格 / Extract task-compilation shadow prices
    pub fn extract_task_shadow_prices(
        &self,
        n_tasks: usize,
        dual_values: &[f64],
    ) -> HashMap<usize, f64> {
        let mut prices = HashMap::new();
        for task_index in 0..n_tasks {
            let key = ConstraintIndexKey::task_compilation(task_index);
            if let Some(price) = self.dual_value(&key, dual_values) {
                if price.abs() > f64::EPSILON {
                    prices.insert(task_index, price);
                }
            }
        }
        prices
    }

    /// 条目数量 / Entry count
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_index_map_register_and_lookup() {
        let mut map = ConstraintIndexMap::new();
        let key = ConstraintIndexKey::task_compilation(2);

        map.register(key.clone(), "task_compilation_2", 5);

        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&key).unwrap().dual_index, 5);
        assert_eq!(map.get_by_name("task_compilation_2").unwrap().key, key);
        assert_eq!(map.dual_value(&key, &[0.0, 1.0, 2.0, 3.0, 4.0, 9.0]), Some(9.0));
    }

    #[test]
    fn test_constraint_index_map_extract_task_shadow_prices() {
        let mut map = ConstraintIndexMap::new();
        map.register(ConstraintIndexKey::task_compilation(0), "task_compilation_0", 2);
        map.register(ConstraintIndexKey::task_compilation(1), "task_compilation_1", 3);

        let prices = map.extract_task_shadow_prices(3, &[0.0, 0.0, 4.0, 0.0]);

        assert_eq!(prices.get(&0), Some(&4.0));
        assert!(!prices.contains_key(&1));
        assert!(!prices.contains_key(&2));
    }

    #[test]
    fn test_constraint_index_map_register_task_compilation_constraints() {
        let mut name_to_index = HashMap::new();
        name_to_index.insert("task_compilation_0".to_string(), 4);
        name_to_index.insert("task_compilation_2".to_string(), 6);
        name_to_index.insert("unrelated".to_string(), 10);

        let mut map = ConstraintIndexMap::new();
        map.register_task_compilation_constraints(3, &name_to_index);

        assert_eq!(map.len(), 2);
        assert_eq!(
            map.dual_value(
                &ConstraintIndexKey::task_compilation(0),
                &[0.0, 0.0, 0.0, 0.0, 1.5, 0.0, 2.5],
            ),
            Some(1.5),
        );
        assert!(map.get(&ConstraintIndexKey::task_compilation(1)).is_none());
        assert_eq!(
            map.get_by_name("task_compilation_2").unwrap().dual_index,
            6,
        );
    }

    #[test]
    fn test_constraint_index_map_missing_dual_returns_none() {
        let mut map = ConstraintIndexMap::new();
        let key = ConstraintIndexKey::executor_compilation("exec_1");
        map.register(key.clone(), "executor_compilation_exec_1", 10);

        assert_eq!(map.dual_value(&key, &[1.0]), None);
        assert!(map.get_by_name("missing").is_none());
    }
}
