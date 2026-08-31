//! 包络线模型 / Envelope model
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{Point2, UnivariateLinearPiecewiseFunction};

/// 包络线 / Envelope (对齐 Kotlin airworthiness_security Envelope)
///
/// 使用 UnivariateLinearPiecewiseFunction 替代原线性近似。
/// Uses UnivariateLinearPiecewiseFunction to replace former linear approximation.
///
/// Kotlin-Rust 映射 / Kotlin-Rust Mapping:
/// - Kotlin `envelope[point]` -> Rust `UnivariateLinearPiecewiseFunction(totalWeight)`
/// - 输入: totalWeight (连续变量)
/// - 输出: index (通过分段线性插值计算)
#[derive(Debug, Clone)]
pub struct Envelope {
    /// 包络线数据点 (总重量, 索引) / Envelope data points (total weight, index)
    pub points: Vec<(f64, f64)>, // (totalWeight, index)
}

impl Envelope {
    /// 注册包络线中间符号到模型
    ///
    /// 对齐 Kotlin Envelope.register:
    /// 使用 UnivariateLinearPiecewiseFunction 将总重量映射到包络线索引。
    /// 替代原线性近似，提供精确的分段线性插值。
    ///
    /// Kotlin-Rust 映射 / Kotlin-Rust Mapping:
    /// - `envelope[point]` -> `UnivariateLinearPiecewiseFunction(totalWeight)`
    pub fn register(
        &self,
        id: u64,
        name: &str,
        model: &mut MetaModel<f64>,
        total_weight_idx: usize,
    ) -> Result<usize, Box<dyn Error>> {
        let points: Vec<Point2<f64>> = self
            .points
            .iter()
            .map(|(w, idx)| Point2::new(*w, *idx))
            .collect();
        let input = Linear::new(vec![LinearMonomial::new(1.0, total_weight_idx)], 0.0);
        let ulp_fn = UnivariateLinearPiecewiseFunction::new(
            id,
            &format!("envelope_{}", name),
            input,
            points,
        );
        let result_idx = ulp_fn.result_variable().index();
        model.add_symbol(Arc::new(ulp_fn))?;
        Ok(result_idx)
    }
}
