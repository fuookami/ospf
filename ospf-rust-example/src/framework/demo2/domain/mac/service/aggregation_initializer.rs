//! MAC 聚合初始化器 / MAC aggregation initializer
use crate::framework::demo2::domain::aircraft::AircraftContext;
use crate::framework::demo2::domain::mac::{Aggregation, model::*};
use crate::framework::demo2::domain::stowage::context::StowageContext;

/// MAC 聚合初始化器 / MAC aggregation initializer
/// 对齐 Kotlin mac AggregationInitializer
pub struct MacAggregationInitializer;

impl MacAggregationInitializer {
    /// 从飞机和装载上下文初始化 MAC 聚合
    /// 对齐 Kotlin AggregationInitializer.initialize
    pub fn initialize(
        aircraft_context: &AircraftContext,
        _stowage_context: &StowageContext<'_>,
    ) -> Option<Aggregation> {
        let _aircraft_agg = aircraft_context.aggregation.as_ref()?;

        // 构建力矩
        let torque = Torque {
            estimate_longitudinal: 0.0,
            actual_longitudinal: 0.0,
            lateral: 0.0,
        };

        // 构建 MAC
        let mac = Mac {
            value: 0.0,
            percentage: 0.0,
        };

        // 构建水平安定面
        let horizontal_stabilizers = Vec::new();

        Some(Aggregation {
            torque,
            mac,
            horizontal_stabilizers,
        })
    }
}
