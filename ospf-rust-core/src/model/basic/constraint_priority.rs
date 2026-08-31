//! 约束优先级语义
//! Constraint priority semantics

use crate::model::intermediate::{
    BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
};

/// 约束优先级 / Constraint priority
///
/// 存储层仍使用 `u32`，该枚举只提供更清晰的语义入口。
/// The storage layer still uses `u32`; this enum only provides clearer semantic entry points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintPriority {
    /// 无优先级 / No priority
    None,
    /// 可选约束 / Nice-to-have constraint
    NiceToHave,
    /// 建议约束 / Suggested constraint
    Suggested,
    /// 强制约束 / Mandatory constraint
    Mandatory,
    /// 自定义优先级 / Custom priority
    Custom(u32),
}

impl ConstraintPriority {
    /// 创建自定义优先级 / Create a custom priority
    pub fn custom(priority: u32) -> Self {
        Self::Custom(priority)
    }

    /// 返回底层数值 / Return storage value
    pub fn value(self) -> u32 {
        match self {
            Self::None => 0,
            Self::NiceToHave => 10,
            Self::Suggested => 50,
            Self::Mandatory => 100,
            Self::Custom(priority) => priority,
        }
    }

    /// 是否为非空优先级 / Whether this is a non-empty priority
    pub fn is_non_null(self) -> bool {
        self.value() != 0
    }
}

impl From<ConstraintPriority> for u32 {
    fn from(priority: ConstraintPriority) -> Self {
        priority.value()
    }
}

impl From<u32> for ConstraintPriority {
    fn from(priority: u32) -> Self {
        match priority {
            0 => Self::None,
            10 => Self::NiceToHave,
            50 => Self::Suggested,
            100 => Self::Mandatory,
            priority => Self::Custom(priority),
        }
    }
}

/// 约束优先级统计 / Constraint priority statistics
pub trait ConstraintPriorityStats {
    /// 非空优先级数量 / Non-empty priority count
    fn non_null_constraint_priority_count(&self) -> usize;

    /// Kotlin 对齐命名别名 / Kotlin-aligned naming alias
    fn non_null_constraint_priority_amount(&self) -> usize {
        self.non_null_constraint_priority_count()
    }
}

impl ConstraintPriorityStats for BasicLinearTriadModel {
    fn non_null_constraint_priority_count(&self) -> usize {
        self.constraint_priorities
            .iter()
            .filter(|priority| **priority != 0)
            .count()
    }
}

impl ConstraintPriorityStats for LinearTriadModel {
    fn non_null_constraint_priority_count(&self) -> usize {
        self.basic.non_null_constraint_priority_count()
    }
}

impl ConstraintPriorityStats for BasicQuadraticTetradModel {
    fn non_null_constraint_priority_count(&self) -> usize {
        self.linear.non_null_constraint_priority_count()
    }
}

impl ConstraintPriorityStats for QuadraticTetradModel {
    fn non_null_constraint_priority_count(&self) -> usize {
        self.basic.non_null_constraint_priority_count()
            + self
                .quadratic_constraint_priorities
                .iter()
                .filter(|priority| **priority != 0)
                .count()
    }
}

#[cfg(test)]
mod tests {
    use super::{ConstraintPriority, ConstraintPriorityStats};
    use crate::model::intermediate::{BasicLinearTriadModel, LinearTriadModel, SparseVector};

    #[test]
    fn constraint_priority_maps_to_storage_values() {
        assert_eq!(u32::from(ConstraintPriority::None), 0);
        assert_eq!(u32::from(ConstraintPriority::NiceToHave), 10);
        assert_eq!(u32::from(ConstraintPriority::Suggested), 50);
        assert_eq!(u32::from(ConstraintPriority::Mandatory), 100);
        assert_eq!(u32::from(ConstraintPriority::custom(7)), 7);
        assert_eq!(ConstraintPriority::from(7), ConstraintPriority::Custom(7));
    }

    #[test]
    fn linear_model_counts_non_null_priorities() {
        let mut basic = BasicLinearTriadModel::new("priority_stats");
        basic.add_constraint_with_metadata(
            SparseVector::new(),
            0.0,
            "c0".to_string(),
            None,
            false,
            0,
            None,
            None,
        );
        basic.add_constraint_with_metadata(
            SparseVector::new(),
            1.0,
            "c1".to_string(),
            None,
            false,
            ConstraintPriority::Mandatory.into(),
            None,
            None,
        );

        let model = LinearTriadModel::from_basic(basic);
        assert_eq!(model.non_null_constraint_priority_count(), 1);
        assert_eq!(model.non_null_constraint_priority_amount(), 1);
    }
}
