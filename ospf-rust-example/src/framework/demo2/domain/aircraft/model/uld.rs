//! ULD（集装器）模型定义 / ULD (Unit Load Device) model definitions
/// ULD 分类 / ULD category (对齐 Kotlin ULDCategory)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UldCategory {
    /// 集装板 / Pallet
    Pallet,
    /// 集装箱 / Container
    Container,
}

/// ULD 代码 / ULD code (对齐 Kotlin ULDCode enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UldCode {
    PAG, PAJ, PMC, PMD, PLA, PLB, PLP, PGE, PGA, PQP, PQF, FQA,
    AKE, DPE, P6P, LAY, ALF, AMA, AMD,
}

impl UldCode {
    /// 获取 ULD 分类 / Get ULD category
    pub fn category(&self) -> UldCategory {
        match self {
            UldCode::PAG | UldCode::PAJ | UldCode::PMC | UldCode::PMD
            | UldCode::PLA | UldCode::PLB | UldCode::PLP | UldCode::PGE
            | UldCode::PGA | UldCode::PQP | UldCode::PQF | UldCode::FQA => UldCategory::Pallet,
            UldCode::AKE | UldCode::DPE | UldCode::P6P | UldCode::LAY
            | UldCode::ALF | UldCode::AMA | UldCode::AMD => UldCategory::Container,
        }
    }
}
