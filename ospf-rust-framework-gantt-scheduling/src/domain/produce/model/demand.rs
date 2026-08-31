//! 物料需求与储备 / Material demand and reserves
//!
//! 定义产出需求和消耗储备的数量规格。
//! Defines quantity specifications for production demand and consumption reserves.

/// 物料需求（产出侧）/ Material demand (production side)
///
/// 描述产品的需求量范围和松弛限制。
/// Describes product demand quantity range and slack limits.
#[derive(Debug, Clone)]
pub struct MaterialDemand {
    /// 物料 ID / Material ID
    pub material_id: String,
    /// 需求下界（solver 值域）/ Lower bound in solver value domain
    pub lower_bound: f64,
    /// 需求上界（solver 值域）/ Upper bound in solver value domain
    pub upper_bound: f64,
    /// 允许不足量上限 / Allowed less quantity limit
    pub less_slack_limit: Option<f64>,
    /// 允许过量上限 / Allowed over quantity limit
    pub over_slack_limit: Option<f64>,
}

impl MaterialDemand {
    /// 创建新的物料需求 / Create new material demand
    pub fn new(material_id: impl Into<String>, lower_bound: f64, upper_bound: f64) -> Self {
        Self {
            material_id: material_id.into(),
            lower_bound,
            upper_bound,
            less_slack_limit: None,
            over_slack_limit: None,
        }
    }

    /// 创建带松弛限制的物料需求 / Create material demand with slack limits
    pub fn with_slack(
        material_id: impl Into<String>,
        lower_bound: f64,
        upper_bound: f64,
        less_slack_limit: Option<f64>,
        over_slack_limit: Option<f64>,
    ) -> Self {
        Self {
            material_id: material_id.into(),
            lower_bound,
            upper_bound,
            less_slack_limit,
            over_slack_limit,
        }
    }

    /// 是否允许不足量 / Whether less slack is enabled
    pub fn less_enabled(&self) -> bool {
        self.less_slack_limit.map_or(false, |v| v > 0.0)
    }

    /// 是否允许过量 / Whether over slack is enabled
    pub fn over_enabled(&self) -> bool {
        self.over_slack_limit.map_or(false, |v| v > 0.0)
    }
}

/// 物料储备（消耗侧）/ Material reserves (consumption side)
///
/// 描述原料的储备量范围和松弛限制。
/// Describes material reserve quantity range and slack limits.
#[derive(Debug, Clone)]
pub struct MaterialReserves {
    /// 物料 ID / Material ID
    pub material_id: String,
    /// 储备下界（solver 值域）/ Lower bound in solver value domain
    pub lower_bound: f64,
    /// 储备上界（solver 值域）/ Upper bound in solver value domain
    pub upper_bound: f64,
    /// 允许不足量上限 / Allowed less quantity limit
    pub less_slack_limit: Option<f64>,
    /// 允许过量上限 / Allowed over quantity limit
    pub over_slack_limit: Option<f64>,
}

impl MaterialReserves {
    /// 创建新的物料储备 / Create new material reserves
    pub fn new(material_id: impl Into<String>, lower_bound: f64, upper_bound: f64) -> Self {
        Self {
            material_id: material_id.into(),
            lower_bound,
            upper_bound,
            less_slack_limit: None,
            over_slack_limit: None,
        }
    }

    /// 创建带松弛限制的物料储备 / Create material reserves with slack limits
    pub fn with_slack(
        material_id: impl Into<String>,
        lower_bound: f64,
        upper_bound: f64,
        less_slack_limit: Option<f64>,
        over_slack_limit: Option<f64>,
    ) -> Self {
        Self {
            material_id: material_id.into(),
            lower_bound,
            upper_bound,
            less_slack_limit,
            over_slack_limit,
        }
    }

    /// 是否允许不足量 / Whether less slack is enabled
    pub fn less_enabled(&self) -> bool {
        self.less_slack_limit.map_or(false, |v| v > 0.0)
    }

    /// 是否允许过量 / Whether over slack is enabled
    pub fn over_enabled(&self) -> bool {
        self.over_slack_limit.map_or(false, |v| v > 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_demand_bounds() {
        let demand = MaterialDemand::new("product_a", 10.0, 100.0);
        assert_eq!(demand.material_id, "product_a");
        assert_eq!(demand.lower_bound, 10.0);
        assert_eq!(demand.upper_bound, 100.0);
        assert!(!demand.less_enabled());
        assert!(!demand.over_enabled());
    }

    #[test]
    fn test_material_demand_with_slack() {
        let demand = MaterialDemand::with_slack("product_a", 10.0, 100.0, Some(5.0), Some(20.0));
        assert!(demand.less_enabled());
        assert!(demand.over_enabled());
    }

    #[test]
    fn test_material_reserves_bounds() {
        let reserves = MaterialReserves::new("raw_mat", 0.0, 50.0);
        assert_eq!(reserves.material_id, "raw_mat");
        assert_eq!(reserves.lower_bound, 0.0);
        assert_eq!(reserves.upper_bound, 50.0);
        assert!(!reserves.less_enabled());
        assert!(!reserves.over_enabled());
    }
}
