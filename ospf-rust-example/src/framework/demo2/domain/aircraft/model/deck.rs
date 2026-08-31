use super::position::Position;
/// 甲板位置 / Deck location (对齐 Kotlin DeckLocation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckLocation {
    Main,
    LowForward,
    LowAft,
}

/// 甲板 / Deck (对齐 Kotlin Deck)
#[derive(Debug, Clone)]
pub struct Deck {
    pub name: String,
    pub location: DeckLocation,
    pub positions: Vec<Position>,
}
