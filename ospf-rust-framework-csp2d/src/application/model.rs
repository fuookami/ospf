//! 二维套裁问题定义 / 2D cutting stock problem definition
//!
//! 问题由「物料 + 产品需求 + 工艺 + 刀缝 + 配置」组成，是应用层的唯一输入。
//! A problem is "materials + product demands + process + kerf + config", the only input of the application
//! layer.

use ospf_rust_base::error::Ret;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

use crate::domain::material::{
    Costar2, Machine2, Material2, Product2, ProductDemand2, product_id2,
};
use crate::infrastructure::cutting_process::CuttingProcess2;
use crate::infrastructure::error::validation_error;

// ============================================================================
// Csp2dConfiguration - 求解配置 / Solve configuration
// ============================================================================

/// 求解配置 / Solve configuration
#[derive(Debug, Clone)]
pub struct Csp2dConfiguration {
    /// 每轮最大候选图案数 / Maximum candidate patterns per iteration
    pub max_candidates_per_iteration: usize,
    /// 最大迭代次数 / Maximum iteration count
    pub iteration_limit: usize,
    /// 余料权重 / Waste weight
    pub waste_weight: f64,
    /// 是否启用最终整数求解 / Whether the final integer solve is enabled
    pub final_integer_enabled: bool,
}

impl Default for Csp2dConfiguration {
    fn default() -> Self {
        Self {
            max_candidates_per_iteration: 32,
            iteration_limit: 20,
            waste_weight: 1.0,
            final_integer_enabled: true,
        }
    }
}

// ============================================================================
// Csp2dProblem - 二维套裁问题 / 2D cutting stock problem
// ============================================================================

/// 二维套裁问题 / 2D cutting stock problem
#[derive(Debug, Clone)]
pub struct Csp2dProblem {
    /// 物料 / Materials
    pub materials: Vec<Material2<f64, Meter>>,
    /// 产品 / Products
    pub products: Vec<Product2<f64, Meter>>,
    /// 产品需求 / Product demands
    pub demands: Vec<ProductDemand2<f64, Meter>>,
    /// 设备 / Machines
    pub machines: Vec<Machine2<f64>>,
    /// 副产物 / By-products
    pub costars: Vec<Costar2<f64, Meter>>,
    /// 刀缝宽度 / Kerf width
    pub kerf: Quantity<f64, Meter>,
    /// 裁切工艺 / Cutting process
    pub process: CuttingProcess2,
    /// 求解配置 / Solve configuration
    pub configuration: Csp2dConfiguration,
}

impl Csp2dProblem {
    /// 创建问题 / Create a problem
    pub fn new(
        materials: Vec<Material2<f64, Meter>>,
        products: Vec<Product2<f64, Meter>>,
        demands: Vec<ProductDemand2<f64, Meter>>,
        kerf: Quantity<f64, Meter>,
    ) -> Self {
        Self {
            materials,
            products,
            demands,
            machines: Vec::new(),
            costars: Vec::new(),
            kerf,
            process: CuttingProcess2::Freeform,
            configuration: Csp2dConfiguration::default(),
        }
    }

    /// 设置工艺 / Set the process
    pub fn with_process(mut self, process: CuttingProcess2) -> Self {
        self.process = process;
        self
    }

    /// 设置副产物 / Set the by-products
    pub fn with_costars(mut self, costars: Vec<Costar2<f64, Meter>>) -> Self {
        self.costars = costars;
        self
    }

    /// 设置配置 / Set the configuration
    pub fn with_configuration(mut self, configuration: Csp2dConfiguration) -> Self {
        self.configuration = configuration;
        self
    }

    /// 校验问题 / Validate the problem
    pub fn validate(&self) -> Ret<()> {
        if self.materials.is_empty() {
            return Err(validation_error(
                "问题没有物料 / The problem has no material",
            ));
        }
        if self.products.is_empty() || self.demands.is_empty() {
            return Err(validation_error(
                "问题没有产品或需求 / The problem has no product or demand",
            ));
        }
        if self.kerf.value < 0.0 {
            return Err(validation_error(
                "刀缝不能为负 / The kerf must not be negative",
            ));
        }
        for material in &self.materials {
            material.validate()?;
        }
        for product in &self.products {
            product.validate()?;
        }
        Ok(())
    }

    /// 产品编号到产品的映射 / Product identifier to product map
    pub fn products_by_id(
        &self,
    ) -> std::collections::BTreeMap<
        crate::domain::material::ProductId2,
        &Product2<f64, Meter>,
    > {
        self.products
            .iter()
            .map(|product| (product.id.clone(), product))
            .collect()
    }

    /// 需求的产品与件数列表 / Demand products with their piece counts
    ///
    /// 面积需求按单件面积折算为件数并向上取整。
    /// Area demands are converted to piece counts by dividing by the unit area, rounded up.
    pub fn demand_products(&self) -> Vec<(Product2<f64, Meter>, u64)> {
        self.demands
            .iter()
            .map(|demand| {
                let amount = match demand.demand {
                    crate::domain::material::DemandValue2::Amount(amount) => amount,
                    crate::domain::material::DemandValue2::Area(area) => {
                        let unit = demand.product.area().value;
                        if unit <= 0.0 {
                            0
                        } else {
                            (area / unit).ceil().max(0.0) as u64
                        }
                    }
                };
                (demand.product.clone(), amount)
            })
            .filter(|(_, amount)| *amount > 0)
            .collect()
    }

    /// 需求项：供主问题使用 / Demand entries for the master problem
    pub fn master_demands(&self) -> Vec<crate::domain::produce::Csp2dMasterDemand> {
        self.demands
            .iter()
            .map(|demand| {
                crate::domain::produce::Csp2dMasterDemand::new(
                    demand.product.id.clone(),
                    demand.demand.clone(),
                )
            })
            .collect()
    }

    /// 需求件数上界：全部需求件数之和 / Piece-count upper bound: the sum of every demanded piece
    pub fn demand_piece_bound(&self) -> u64 {
        self.demand_products()
            .iter()
            .map(|(_, amount)| *amount)
            .sum()
    }

    /// 便捷构造：单物料 + 单产品需求 / Convenience: one material with one product demand
    pub fn single(
        material: Material2<f64, Meter>,
        product: Product2<f64, Meter>,
        amount: u64,
        kerf: Quantity<f64, Meter>,
    ) -> Self {
        Self::new(
            vec![material],
            vec![product.clone()],
            vec![ProductDemand2::amount(product, amount)],
            kerf,
        )
    }
}

/// 由产品编号与件数构造需求 / Build a demand from a product identifier and a piece count
pub fn demand_of(
    product: Product2<f64, Meter>,
    amount: u64,
) -> ProductDemand2<f64, Meter> {
    ProductDemand2::amount(product, amount)
}

/// 构造一个占位产品编号 / Build a placeholder product identifier
pub fn placeholder_product_id(value: impl Into<String>) -> crate::domain::material::ProductId2 {
    product_id2(value)
}
