//! 平均气动弦模型 / Mean aerodynamic chord model
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use std::error::Error;
use std::sync::Arc;

/// MAC (平均气动力弦) / Mean Aerodynamic Chord (对齐 Kotlin MAC)
#[derive(Debug, Clone)]
pub struct Mac {
    /// MAC 值 / MAC value
    pub value: f64,
    /// MAC 百分比 / MAC percentage
    pub percentage: f64,
}

impl Mac {
    /// 注册 MAC 中间符号到模型
    /// 对齐 Kotlin MAC.register
    pub fn register(
        &self,
        model: &mut MetaModel<f64>,
        dow: f64,
        doi: f64,
        standard_datum: f64,
        chord: f64,
    ) -> Result<(), Box<dyn Error>> {
        let next_id = 10100u64;

        let mac_constant = if chord > 0.0 {
            (dow * standard_datum + doi) / chord
        } else {
            0.0
        };

        let mac_symbol =
            LinearExpressionSymbol::new(next_id, "mac_value", Vec::new(), mac_constant);
        model.add_symbol(Arc::new(mac_symbol))?;

        let mac_pct_symbol =
            LinearExpressionSymbol::new(next_id + 1, "mac_percentage", Vec::new(), mac_constant);
        model.add_symbol(Arc::new(mac_pct_symbol))?;

        Ok(())
    }
}
