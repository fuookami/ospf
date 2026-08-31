/// 重量属性 / Weight attribute
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightAttribute {
    /// 最大层数 / Maximum layer count
    pub max_layer: Option<u64>,
}

impl Default for WeightAttribute {
    fn default() -> Self {
        Self { max_layer: None }
    }
}

/// 变形属性 / Deformation attribute
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeformationAttribute {
    /// 线性体积变形 / Linear volume deformation
    Linear {
        /// 变形系数 / Deformation coefficient
        coefficient: f64,
    },
}

impl Default for DeformationAttribute {
    fn default() -> Self {
        Self::Linear { coefficient: 0.0 }
    }
}

impl DeformationAttribute {
    /// 计算变形量 / Calculate deformation quantity
    pub fn deformation_quantity(self, volume: f64) -> [f64; 3] {
        match self {
            Self::Linear { coefficient } => {
                let value = volume * coefficient;
                [value, value, value]
            }
        }
    }
}

/// 悬空策略 / Hanging policy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HangingPolicy {
    /// 绝对悬空距离策略 / Absolute hanging-difference policy
    Absolute {
        /// 最大差值 / Maximum difference
        max_difference: f64,
        /// 是否检查重量 / Whether to check weight
        with_weight: bool,
    },
    /// 相对悬空面积比例策略 / Relative hanging-area policy
    Relative {
        /// 允许悬空比例 / Allowed hanging percentage
        hanging_percentage: f64,
        /// 是否检查重量 / Whether to check weight
        with_weight: bool,
    },
}

impl Default for HangingPolicy {
    fn default() -> Self {
        Self::Absolute {
            max_difference: 0.0,
            with_weight: true,
        }
    }
}

impl HangingPolicy {
    /// 判断底部支撑是否允许堆叠 / Check whether bottom support allows stacking
    pub fn enabled_stacking_on_support(
        self,
        item_weight: f64,
        footprint_area: f64,
        footprint_min_span: f64,
        bottom_support_area: f64,
        bottom_support_weight: f64,
    ) -> bool {
        let hanging_area = (footprint_area - bottom_support_area).max(0.0);
        match self {
            Self::Absolute {
                max_difference,
                with_weight,
            } => {
                if with_weight && bottom_support_weight < item_weight {
                    return false;
                }
                hanging_area <= max_difference * footprint_min_span
            }
            Self::Relative {
                hanging_percentage,
                with_weight,
            } => {
                if with_weight && bottom_support_weight < item_weight {
                    return false;
                }
                hanging_area <= footprint_area * hanging_percentage
            }
        }
    }
}

/// 堆叠策略 / Stacking-on policy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StackingOnPolicy {
    /// 硬箱堆叠策略 / Box stacking policy
    Box {
        /// 最大尺寸差 / Maximum size difference
        max_difference: f64,
        /// 最大超重 / Maximum overweight
        max_over_weight: f64,
    },
    /// 纸箱堆叠策略 / Carton-container stacking policy
    CartonContainer {
        /// 最大尺寸差 / Maximum size difference
        max_difference: f64,
        /// 最大超重 / Maximum overweight
        max_over_weight: f64,
    },
    /// 过滤式堆叠策略 / Filter stacking policy
    Filter {
        /// 最大超重 / Maximum overweight
        max_over_weight: f64,
    },
}

impl Default for StackingOnPolicy {
    fn default() -> Self {
        Self::Filter {
            max_over_weight: 10.0,
        }
    }
}

