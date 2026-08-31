// ============================================================================
// Bpp3dSuiteSummary - 套件汇总 / Suite summary
// ============================================================================

/// 套件汇总 / Suite summary
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bpp3dSuiteSummary {
    /// fixture 总数 / Total fixture count
    pub total: usize,
    /// 成功数 / Success count
    pub succeeded: usize,
    /// 失败数 / Failure count
    pub failed: usize,
    /// 跳过数 / Skipped count
    pub skipped: usize,
    /// 总渲染方案数 / Total render plan count
    pub total_render_plans: usize,
    /// 总装箱数 / Total packed bin count
    pub total_packed_bins: usize,
    /// 总选中层数 / Total selected layer count
    pub total_selected_layers: usize,
}

impl Bpp3dSuiteSummary {
    /// 从报告列表计算汇总 / Compute summary from fixture reports
    pub fn from_reports(reports: &[Bpp3dFixtureReport]) -> Self {
        let total = reports.len();
        let succeeded = reports.iter().filter(|r| r.status.is_success()).count();
        let failed = reports.iter().filter(|r| r.status.is_failed()).count();
        let skipped = reports.len() - succeeded - failed;
        let total_render_plans = reports.iter().map(|r| r.render_plan_count).sum();
        let total_packed_bins = reports.iter().map(|r| r.packed_bin_count).sum();
        let total_selected_layers = reports.iter().map(|r| r.selected_layer_count).sum();
        Self {
            total,
            succeeded,
            failed,
            skipped,
            total_render_plans,
            total_packed_bins,
            total_selected_layers,
        }
    }

    /// 是否全部通过 / Whether all fixtures passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// 摘要字符串 / Summary string
    pub fn summary_string(&self) -> String {
        format!(
            "total={}, succeeded={}, failed={}, skipped={}, render_plans={}, packed_bins={}, selected_layers={}",
            self.total,
            self.succeeded,
            self.failed,
            self.skipped,
            self.total_render_plans,
            self.total_packed_bins,
            self.total_selected_layers,
        )
    }
}

