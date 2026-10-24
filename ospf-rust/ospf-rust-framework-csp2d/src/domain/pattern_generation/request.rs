//! 图案生成请求与接口 / Pattern generation request and interface
//!
//! 图案生成的输入是「一块料 + 待排的成品件」，输出是一组候选图案。请求本身承担校验职责：
//! 刀缝非负、物料合法、至少一个候选产品、剩余件数为正。
//! The input of pattern generation is "one piece of material plus the parts to lay out" and the output is a set
//! of candidate patterns. The request itself validates: non-negative kerf, legal material, at least one
//! candidate product, and positive remaining amounts.

use std::collections::BTreeMap;

use ospf_rust_base::error::Ret;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::material::{Costar2, Material2, Pattern2, Product2};
use crate::infrastructure::cutting_process::CuttingProcess2;
use crate::infrastructure::error::validation_error;

// ============================================================================
// PatternGenerationRequest2 - 图案生成请求 / Pattern generation request
// ============================================================================

/// 图案生成请求 / Pattern generation request
#[derive(Debug, Clone)]
pub struct PatternGenerationRequest2<V, U: UnitTrait> {
    /// 目标物料 / Target material
    pub material: Material2<V, U>,
    /// 可选产品及其剩余件数 / Candidate products and their remaining piece counts
    pub products: Vec<(Product2<V, U>, u64)>,
    /// 刀缝宽度 / Kerf width
    pub kerf: Quantity<V, U>,
    /// 裁切工艺 / Cutting process
    pub process: CuttingProcess2,
    /// 可填充的副产物 / By-products available for filling
    pub costars: Vec<Costar2<V, U>>,
    /// 卷材模式的最大段高 / Maximum segment height in coil mode
    pub max_segment_height: Option<Quantity<V, U>>,
    /// 最大候选数 / Maximum candidate count
    pub max_candidates: usize,
    /// 迭代序号 / Iteration index
    pub iteration: i64,
    /// 历史图案，用于回放 / Historical patterns for replay
    pub existing_patterns: Vec<Pattern2<V, U>>,
    /// 产品编号到影子价格的映射（按面积计价）/ Product identifier to shadow price, priced by area
    pub shadow_prices: BTreeMap<String, f64>,
    /// 余料权重 / Waste weight
    pub waste_weight: Option<f64>,
}

impl<V, U> PatternGenerationRequest2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    /// 创建请求 / Create a request
    pub fn new(
        material: Material2<V, U>,
        products: Vec<(Product2<V, U>, u64)>,
        kerf: Quantity<V, U>,
    ) -> Self {
        Self {
            material,
            products,
            kerf,
            process: CuttingProcess2::Freeform,
            costars: Vec::new(),
            max_segment_height: None,
            max_candidates: 32,
            iteration: 0,
            existing_patterns: Vec::new(),
            shadow_prices: BTreeMap::new(),
            waste_weight: None,
        }
    }

    /// 设置工艺 / Set the process
    pub fn with_process(mut self, process: CuttingProcess2) -> Self {
        self.process = process;
        self
    }

    /// 设置副产物 / Set the by-products
    pub fn with_costars(mut self, costars: Vec<Costar2<V, U>>) -> Self {
        self.costars = costars;
        self
    }

    /// 设置最大段高 / Set the maximum segment height
    pub fn with_max_segment_height(mut self, height: Quantity<V, U>) -> Self {
        self.max_segment_height = Some(height);
        self
    }

    /// 设置最大候选数 / Set the maximum candidate count
    pub fn with_max_candidates(mut self, max_candidates: usize) -> Self {
        self.max_candidates = max_candidates;
        self
    }

    /// 设置历史图案 / Set the historical patterns
    pub fn with_existing_patterns(mut self, patterns: Vec<Pattern2<V, U>>) -> Self {
        self.existing_patterns = patterns;
        self
    }

    /// 设置影子价格 / Set the shadow prices
    pub fn with_shadow_prices(mut self, prices: BTreeMap<String, f64>) -> Self {
        self.shadow_prices = prices;
        self
    }

    /// 校验请求 / Validate the request
    pub fn validate(&self) -> Ret<()> {
        if self.kerf.value < V::zero() {
            return Err(validation_error(format!(
                "刀缝不能为负 / The kerf must not be negative: {}",
                self.kerf.value.to_f64().unwrap_or(f64::NAN)
            )));
        }
        self.material.validate()?;
        if self.products.is_empty() {
            return Err(validation_error(
                "图案生成请求没有可选产品 / The request has no candidate product",
            ));
        }
        for (product, amount) in &self.products {
            product.validate()?;
            if *amount == 0 {
                return Err(validation_error(format!(
                    "产品 {} 的剩余件数为零 / Product {} has a zero remaining amount",
                    product.id, product.id
                )));
            }
        }
        Ok(())
    }

    /// 料域高度上界 / The region height upper bound
    ///
    /// 定尺模式直接取板高；卷材模式最坏情况是每件各占一行，故取「全部件高之和 + 行间刀缝」，
    /// 再由 `max_segment_height` 截断。
    /// Sheet mode uses the sheet height directly. In coil mode the worst case puts every piece in its own row, so
    /// the bound is the sum of all piece heights plus the row kerfs, truncated by `max_segment_height`.
    pub fn region_height_bound(&self) -> Quantity<V, U> {
        if let Some(height) = &self.material.height {
            return height.clone();
        }
        let mut total = Quantity::new(V::zero(), self.material.width.unit.clone());
        for (product, amount) in &self.products {
            // 上界必须计入件数：把每个件的「高度 + 刀缝」累加，因此最坏情况（每件单独一行）一定装得下。
            // 同时按 4096 截断，避免病态输入造成巨大循环。
            // The bound must account for the amounts: accumulate "height plus kerf" per piece so the worst case
            // (one piece per row) always fits. It is also capped at 4096 to avoid a pathological loop.
            let count = (*amount as usize).min(4096);
            for _ in 0..count {
                total = Quantity::new(
                    total.value.clone() + product.height.value.clone() + self.kerf.value.clone(),
                    total.unit.clone(),
                );
            }
        }
        match &self.max_segment_height {
            Some(limit) if limit.value < total.value => limit.clone(),
            _ => total,
        }
    }

    /// 料域宽度上界 / The region width upper bound
    ///
    /// 卷材与定尺都取物料幅宽。
    /// Both coil and sheet use the material width.
    pub fn region_width_bound(&self) -> Quantity<V, U> {
        self.material.width.clone()
    }
}

// ============================================================================
// 图案生成器接口 / Pattern generator trait
// ============================================================================

/// 图案生成器 / Pattern generator
pub trait PatternGenerator2<V, U: UnitTrait>: std::fmt::Debug + Send + Sync {
    /// 生成器名称 / Generator name
    fn name(&self) -> &str;

    /// 生成候选图案 / Generate candidate patterns
    fn generate(&self, request: &PatternGenerationRequest2<V, U>) -> Vec<Pattern2<V, U>>;
}
