use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;

/// 力矩 / Torque (对齐 Kotlin Torque)
#[derive(Debug, Clone)]
pub struct Torque {
    pub estimate_longitudinal: f64,
    pub actual_longitudinal: f64,
    pub lateral: f64,
}

impl Torque {
    /// 注册力矩中间符号到模型
    /// 对齐 Kotlin Torque.register
    pub fn register(
        &self,
        model: &mut MetaModel<f64>,
        position_longitudinal_arms: &[f64],
        position_lateral_arms: &[f64],
        cargo_weights: &[f64],
        x_idx: &[Vec<usize>],
    ) -> Result<(), Box<dyn Error>> {
        if cargo_weights.is_empty() || position_longitudinal_arms.is_empty() {
            return Ok(());
        }

        let mut next_id = 10000u64;

        // 估算纵向力矩: sum(weight * longArm * x[c][p])
        let mut est_long_monomials = Vec::new();
        for (c, weight) in cargo_weights.iter().enumerate() {
            for (p, arm) in position_longitudinal_arms.iter().enumerate() {
                if c < x_idx.len() && p < x_idx[c].len() {
                    est_long_monomials.push(LinearMonomial::new(weight * arm, x_idx[c][p]));
                }
            }
        }
        if !est_long_monomials.is_empty() {
            let est_long_symbol = LinearExpressionSymbol::new(
                next_id,
                "estimate_longitudinal_torque",
                est_long_monomials,
                0.0,
            );
            model.add_symbol(Arc::new(est_long_symbol))?;
            next_id += 1;
        }

        // 实际纵向力矩
        let mut actual_long_monomials = Vec::new();
        for (c, weight) in cargo_weights.iter().enumerate() {
            for (p, arm) in position_longitudinal_arms.iter().enumerate() {
                if c < x_idx.len() && p < x_idx[c].len() {
                    actual_long_monomials.push(LinearMonomial::new(weight * arm, x_idx[c][p]));
                }
            }
        }
        if !actual_long_monomials.is_empty() {
            let actual_long_symbol = LinearExpressionSymbol::new(
                next_id,
                "actual_longitudinal_torque",
                actual_long_monomials,
                0.0,
            );
            model.add_symbol(Arc::new(actual_long_symbol))?;
            next_id += 1;
        }

        // 横向力矩: sum(weight * latArm * x[c][p])
        let mut lat_monomials = Vec::new();
        for (c, weight) in cargo_weights.iter().enumerate() {
            for (p, arm) in position_lateral_arms.iter().enumerate() {
                if c < x_idx.len() && p < x_idx[c].len() {
                    lat_monomials.push(LinearMonomial::new(weight * arm, x_idx[c][p]));
                }
            }
        }
        if !lat_monomials.is_empty() {
            let lat_symbol = LinearExpressionSymbol::new(
                next_id,
                "lateral_torque",
                lat_monomials,
                0.0,
            );
            model.add_symbol(Arc::new(lat_symbol))?;
        }

        Ok(())
    }
}

