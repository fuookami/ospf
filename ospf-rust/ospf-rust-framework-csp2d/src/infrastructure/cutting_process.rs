//! 裁切工艺 / Cutting processes
//!
//! 二维套裁需要同时支持两种工艺：
//! 1. **任意排样**：只给放置坐标，设备（激光/水刀/CNC）按轮廓切。
//! 2. **断头台**：每一刀必须贯穿当前料块，把料切成两块，递归进行；适用于剪板机/开料锯。
//!
//! 断头台模式下，排样必须能由一个**切分树**产生，且切分树本身就是给车间的下刀顺序，因此必须显式建模，
//! 而不是只存放置坐标。
//! 2D cutting stock must support two processes: **arbitrary nesting**, where only placement coordinates are given
//! and the machine cuts along contours; and **guillotine**, where every cut must run through the current piece
//! and split it into two, recursively. In guillotine mode a layout must be produced by a **cut tree**, and that
//! tree is itself the cut sequence handed to the workshop, so it has to be modelled explicitly instead of
//! storing placement coordinates only.

// ============================================================================
// CuttingProcess2 - 裁切工艺 / Cutting process
// ============================================================================

/// 裁切工艺 / Cutting process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CuttingProcess2 {
    /// 任意排样：设备按轮廓切，只需要放置坐标 / Arbitrary nesting: contour cutting, coordinates only
    Freeform,
    /// 断头台 2 级：先切条、再切件 / Guillotine two stages: strips then pieces
    GuillotineTwoStage,
    /// 断头台 3 级：切条 → 切块 → 切件 / Guillotine three stages: strips, blocks, pieces
    GuillotineThreeStage,
}

impl CuttingProcess2 {
    /// 允许的最大工序级数 / The maximum allowed process stage
    ///
    /// 任意排样不受级数约束，返回 `usize::MAX`。
    /// Arbitrary nesting has no stage limit and returns `usize::MAX`.
    pub fn max_stage(&self) -> usize {
        match self {
            Self::Freeform => usize::MAX,
            Self::GuillotineTwoStage => 2,
            Self::GuillotineThreeStage => 3,
        }
    }

    /// 是否要求输出断头台切分树 / Whether a guillotine cut tree is required
    pub fn requires_cut_tree(&self) -> bool {
        !matches!(self, Self::Freeform)
    }

    /// 判断工序级数是否被本工艺允许 / Whether a process stage is allowed
    pub fn allows_stage(&self, stage: usize) -> bool {
        stage >= 1 && stage <= self.max_stage()
    }

    /// 工序级数的双语标签 / Bilingual label of a process stage
    pub fn stage_label(&self, stage: usize) -> String {
        match stage {
            1 => "切条 / strip".to_string(),
            2 => "切块 / block".to_string(),
            3 => "切件 / piece".to_string(),
            other => format!("第 {other} 级 / stage {other}"),
        }
    }
}
