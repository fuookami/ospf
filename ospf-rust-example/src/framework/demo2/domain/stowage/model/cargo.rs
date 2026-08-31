//! 货物模型 / Cargo model
/// 货物代码 / Cargo code (对齐 Kotlin CargoCode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CargoCode {
    BAL, ELD, FKI, CCD, ICE, AVI, AOG, ELI, ELM, MAG, HUB, PER, YYI, MAT, RRM,
    BIG, OHG, RFG, RFL, RFS, ROX, YYE, HWJ, RRY, RRW, CVV,
    Crush, Stiff, Empty, Virtual,
}

impl CargoCode {
    /// 是否为特殊货物代码 / Whether this is a special cargo code
    pub fn is_special(&self) -> bool {
        matches!(self, CargoCode::AOG | CargoCode::MAT | CargoCode::Virtual)
    }
}

/// 货物类型 / Cargo type (对齐 Kotlin CargoType)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CargoType {
    /// 货物代码 / Cargo code
    pub code: Option<CargoCode>,
    /// 类型名称 / Type name
    pub type_name: String,
}

impl CargoType {
    /// 从货物代码创建 / Create from cargo code
    pub fn from_code(code: CargoCode) -> Self {
        Self { code: Some(code), type_name: format!("{:?}", code) }
    }

    /// 从名称创建 / Create from name
    pub fn from_name(name: &str) -> Self {
        Self { code: None, type_name: name.to_string() }
    }
}

/// 货物 / Cargo (对齐 Kotlin Cargo)
#[derive(Debug, Clone)]
pub struct Cargo {
    /// 货物代码 / Cargo code
    pub code: CargoCode,
    /// 货物类型列表 / Cargo type list
    pub types: Vec<CargoType>,
}

impl Cargo {
    /// 是否包含指定货物代码 / Whether this cargo contains the specified cargo code
    pub fn contains(&self, code: &CargoCode) -> bool {
        self.code == *code || self.types.iter().any(|t| t.code.as_ref() == Some(code))
    }
}
