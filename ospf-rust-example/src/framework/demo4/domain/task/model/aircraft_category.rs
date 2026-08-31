/// 飞机类别 / Aircraft category (对齐 Kotlin AircraftCategory)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AircraftCategory {
    NarrowBody,
    WideBody,
    Freighter,
}

impl AircraftCategory {
    pub fn is_wide_body(&self) -> bool {
        matches!(self, AircraftCategory::WideBody)
    }
}
