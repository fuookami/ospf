//! 甲板定义 / Deck definitions
use super::position::Position;
/// 甲板位置 / Deck location (对齐 Kotlin DeckLocation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckLocation {
    /// 主甲板 / Main deck
    Main,
    /// 下前舱 / Lower forward compartment
    LowForward,
    /// 下后舱 / Lower aft compartment
    LowAft,
}

/// 甲板 / Deck (对齐 Kotlin Deck)
#[derive(Debug, Clone)]
pub struct Deck {
    /// 甲板名称 / Deck name
    pub name: String,
    /// 甲板位置 / Deck location
    pub location: DeckLocation,
    /// 舱位列表 / Position list
    pub positions: Vec<Position>,
}
