/// 货物代码 / Cargo code (对齐 Kotlin CargoCode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CargoCode {
    BAL, ELD, FKI, CCD, ICE, AVI, AOG, ELI, ELM, MAG, HUB, PER, YYI, MAT, RRM,
    BIG, OHG, RFG, RFL, RFS, ROX, YYE, HWJ, RRY, RRW, CVV,
    Crush, Stiff, Empty, Virtual,
}

impl CargoCode {
    pub fn is_special(&self) -> bool {
        matches!(self, CargoCode::AOG | CargoCode::MAT | CargoCode::Virtual)
    }
}

/// 货物类型 / Cargo type (对齐 Kotlin CargoType)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CargoType {
    pub code: Option<CargoCode>,
    pub type_name: String,
}

impl CargoType {
    pub fn from_code(code: CargoCode) -> Self {
        Self { code: Some(code), type_name: format!("{:?}", code) }
    }

    pub fn from_name(name: &str) -> Self {
        Self { code: None, type_name: name.to_string() }
    }
}

/// 货物 / Cargo (对齐 Kotlin Cargo)
#[derive(Debug, Clone)]
pub struct Cargo {
    pub code: CargoCode,
    pub types: Vec<CargoType>,
}

impl Cargo {
    pub fn contains(&self, code: &CargoCode) -> bool {
        self.code == *code || self.types.iter().any(|t| t.code.as_ref() == Some(code))
    }
}
