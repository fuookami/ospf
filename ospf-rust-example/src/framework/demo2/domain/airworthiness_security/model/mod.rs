/// 包络线 / Envelope (对齐 Kotlin airworthiness_security Envelope)
#[derive(Debug, Clone)]
pub struct Envelope {
    pub points: Vec<(f64, f64)>, // (totalWeight, index)
}

/// 线密度 / Linear density (对齐 Kotlin LinearDensity)
#[derive(Debug, Clone)]
pub struct LinearDensity {
    pub weight: f64,
    pub length: f64,
}

impl LinearDensity {
    pub fn density(&self) -> f64 {
        if self.length > 0.0 { self.weight / self.length } else { 0.0 }
    }
}

/// 最大 CLIM / Max CLIM (对齐 Kotlin MaxCLIM)
#[derive(Debug, Clone)]
pub struct MaxCLIM {
    pub value: f64,
}

/// 最大累积载荷重量 / Max cumulative load weight (对齐 Kotlin MaxCumulativeLoadWeight)
#[derive(Debug, Clone)]
pub struct MaxCumulativeLoadWeight {
    pub max_weight: f64,
}

/// 最大非对称线密度 / Max unsymmetrical linear density (对齐 Kotlin MaxUnsymmetricalLinearDensity)
#[derive(Debug, Clone)]
pub struct MaxUnsymmetricalLinearDensity {
    pub max_density: f64,
}

/// 最大区域载荷重量 / Max zone load weight (对齐 Kotlin MaxZoneLoadWeight)
#[derive(Debug, Clone)]
pub struct MaxZoneLoadWeight {
    pub zone: String,
    pub max_weight: f64,
}

/// 最小低载荷 / Min low payload (对齐 Kotlin MinLowPayload)
#[derive(Debug, Clone)]
pub struct MinLowPayload {
    pub min_payload: f64,
}

/// 表面密度 / Surface density (对齐 Kotlin SurfaceDensity)
#[derive(Debug, Clone)]
pub struct SurfaceDensity {
    pub weight: f64,
    pub area: f64,
}

impl SurfaceDensity {
    pub fn density(&self) -> f64 {
        if self.area > 0.0 { self.weight / self.area } else { 0.0 }
    }
}
