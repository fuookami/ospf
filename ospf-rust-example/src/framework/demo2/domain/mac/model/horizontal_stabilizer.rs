use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{AbsFunction, SlackFunction};

/// 水平安定面变量索引 / Horizontal stabilizer variable indices
#[derive(Debug, Clone)]
pub struct HorizontalStabilizerVariables {
    /// hsSlack = |left_expr - right_expr| (SlackFunction result variable solver index)
    pub hs_slack: usize,
    /// hsTrim = |trim_expr| (AbsFunction result variable solver index)
    pub hs_trim: usize,
}

/// 水平安定面 / Horizontal stabilizer (对齐 Kotlin HorizontalStabilizer)
///
/// Kotlin-Rust 映射 / Kotlin-Rust Mapping:
/// - Kotlin `hsSlack` -> Rust `SlackFunction` (松弛函数)
/// - Kotlin `hsTrim`  -> Rust `AbsFunction`  (绝对值函数)
///
/// 原 Rust 实现使用空 LinearExpressionSymbol（无项，仅常量），
/// 现替换为完整的 SlackFunction + AbsFunction 对齐 Kotlin。
#[derive(Debug, Clone)]
pub struct HorizontalStabilizer {
    pub key: String,
    pub points: Vec<(f64, f64)>,
    pub limit: f64,
}

impl HorizontalStabilizer {
    /// 注册水平安定面约束到模型
    ///
    /// 对齐 Kotlin HorizontalStabilizer.register:
    /// 1. 创建 hsSlack = SlackFunction(left_expr, right_expr) 松弛函数
    /// 2. 创建 hsTrim = AbsFunction(trim_expr) 绝对值函数
    ///
    /// 在 WeightRecommendation 模式下跳过注册（与 Kotlin 行为一致）。
    ///
    /// 参数 / Parameters:
    /// - `stowage_mode`: 装载模式 ("WeightRecommendation" 时跳过)
    /// - `model`: 元模型
    /// - `next_id`: 起始符号 ID
    /// - `left_idx`: SlackFunction 左侧表达式的变量索引
    /// - `right_idx`: SlackFunction 右侧表达式的变量索引
    /// - `trim_idx`: AbsFunction 输入表达式的变量索引
    pub fn register(
        &self,
        stowage_mode: &str,
        model: &mut MetaModel<f64>,
        next_id: u64,
        left_idx: usize,
        right_idx: usize,
        trim_idx: usize,
    ) -> Result<HorizontalStabilizerVariables, Box<dyn Error>> {
        if stowage_mode == "WeightRecommendation" {
            // WeightRecommendation 模式下返回零索引（不注册符号）
            return Ok(HorizontalStabilizerVariables {
                hs_slack: 0,
                hs_trim: 0,
            });
        }

        // 1. 创建 hsSlack = SlackFunction(left_expr, right_expr)
        // Kotlin: val hsSlack = SlackFunction(left, right)
        // slack = |left - right|
        let left_linear = Linear::new(vec![LinearMonomial::new(1.0, left_idx)], 0.0);
        let right_linear = Linear::new(vec![LinearMonomial::new(1.0, right_idx)], 0.0);
        let slack_fn = SlackFunction::new(
            next_id,
            &format!("hs_slack_{}", self.key),
            left_linear,
            right_linear,
        );
        let slack_result_idx = slack_fn.result_variable().index();
        model.add_symbol(Arc::new(slack_fn))?;

        // 2. 创建 hsTrim = AbsFunction(trim_expr)
        // Kotlin: val hsTrim = AbsFunction(trim)
        // result = |trim|
        let trim_linear = Linear::new(vec![LinearMonomial::new(1.0, trim_idx)], 0.0);
        let abs_fn = AbsFunction::new(
            next_id + 1,
            &format!("hs_trim_{}", self.key),
            trim_linear,
        );
        let abs_result_idx = abs_fn.result_variable().index();
        model.add_symbol(Arc::new(abs_fn))?;

        Ok(HorizontalStabilizerVariables {
            hs_slack: slack_result_idx,
            hs_trim: abs_result_idx,
        })
    }
}
