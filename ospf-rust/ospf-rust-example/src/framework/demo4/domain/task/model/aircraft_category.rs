//! 飞机类别模块 / Aircraft category module

/// 飞机类别 / Aircraft category (对齐 Kotlin AircraftCategory)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AircraftCategory {
    /// 窄体机 / Narrow-body aircraft
    NarrowBody,
    /// 宽体机 / Wide-body aircraft
    WideBody,
    /// 货机 / Freighter aircraft
    Freighter,
}

impl AircraftCategory {
    /// 是否为宽体机 / Check if wide-body aircraft
    pub fn is_wide_body(&self) -> bool {
        matches!(self, AircraftCategory::WideBody)
    }
}
