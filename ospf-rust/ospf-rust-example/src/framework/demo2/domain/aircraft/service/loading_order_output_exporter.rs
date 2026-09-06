//! 装载顺序输出导出器 / Loading order output exporter
use crate::framework::demo2::domain::aircraft::Aggregation;

/// 装载顺序输出导出器 / Loading order output exporter
/// 对齐 Kotlin LoadingOrderOutputExporter
pub struct LoadingOrderOutputExporter;

impl LoadingOrderOutputExporter {
    /// 导出装载顺序 / Export loading orders
    /// 对齐 Kotlin LoadingOrderOutputExporter.export
    pub fn export(aggregation: &Aggregation) -> Vec<String> {
        let mut orders = Vec::new();
        for deck in &aggregation.decks {
            for position in &deck.positions {
                orders.push(format!(
                    "{}: {} (order={})",
                    position.id, position.space_name, position.loading_order
                ));
            }
        }
        orders
    }
}
