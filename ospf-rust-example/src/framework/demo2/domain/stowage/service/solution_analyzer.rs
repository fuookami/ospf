use crate::framework::demo2::domain::stowage::context::StowageContext;
use crate::framework::demo2::infrastructure::dto::Demo2Response;

/// 装载方案分析器 / Stowage solution analyzer
/// 对齐 Kotlin stowage SolutionAnalyzer
pub struct StowageSolutionAnalyzer;

impl StowageSolutionAnalyzer {
    pub fn analyze(
        context: &StowageContext<'_>,
        solution: &[f64],
    ) -> Vec<String> {
        let mut assignments = Vec::new();
        for c in 0..context.request.cargos.len() {
            for p in 0..context.request.positions.len() {
                let value = solution.get(context.x_idx[c][p]).copied().unwrap_or(0.0);
                if value > 0.5 {
                    assignments.push(format!(
                        "{} -> {}",
                        context.request.cargos[c].name,
                        context.request.positions[p].name
                    ));
                }
            }
        }
        assignments
    }
}
