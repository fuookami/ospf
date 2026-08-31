// ============================================================================
// ColumnGenerationState - 列生成状态 / Column generation state
// ============================================================================

/// 列生成状态枚举 / Column generation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnGenerationStatus {
    /// 未开始 / Not started
    NotStarted,
    /// 运行中 / Running
    Running,
    /// 收敛 / Converged
    Converged,
    /// 达到迭代限制 / Iteration limit reached
    IterationLimit,
    /// 达到列数量限制 / Column limit reached
    ColumnLimit,
    /// 达到时间限制 / Time limit reached
    TimeLimit,
    /// 失败 / Failed
    Failed,
}

/// 列生成状态 / Column generation state
#[derive(Debug, Clone)]
pub struct ColumnGenerationState {
    /// 当前迭代 / Current iteration
    pub iteration: usize,
    /// 总列数 / Total column count
    pub total_columns: usize,
    /// 未改进迭代次数 / Non-improving iteration count
    pub not_better_iterations: usize,
    /// 最佳目标值 / Best objective
    pub best_objective: Option<f64>,
    /// 状态 / Status
    pub status: ColumnGenerationStatus,
    /// 开始时间 / Start time
    pub started_at: Instant,
}

impl Default for ColumnGenerationState {
    fn default() -> Self {
        Self::new()
    }
}

impl ColumnGenerationState {
    /// 创建初始状态 / Create initial state
    pub fn new() -> Self {
        Self {
            iteration: 0,
            total_columns: 0,
            not_better_iterations: 0,
            best_objective: None,
            status: ColumnGenerationStatus::NotStarted,
            started_at: Instant::now(),
        }
    }

    /// 标记开始 / Mark as started
    pub fn start(&mut self) {
        self.status = ColumnGenerationStatus::Running;
        self.started_at = Instant::now();
    }

    /// 推进迭代 / Advance iteration
    pub fn advance_iteration(&mut self) {
        self.iteration += 1;
    }

    /// 登记新增列 / Register generated columns
    pub fn register_columns(&mut self, amount: usize) {
        self.total_columns += amount;
    }

    /// 观察目标值 / Observe objective value
    pub fn observe_objective(
        &mut self,
        objective: f64,
        sense: ObjectiveSense,
        tolerance: f64,
    ) -> bool {
        let improved = match self.best_objective {
            None => true,
            Some(best) => match sense {
                ObjectiveSense::Minimize => objective < best - tolerance.abs(),
                ObjectiveSense::Maximize => objective > best + tolerance.abs(),
            },
        };

        if improved {
            self.best_objective = Some(objective);
            self.not_better_iterations = 0;
        } else {
            self.not_better_iterations += 1;
        }
        improved
    }

    /// 判断是否应继续 / Check whether generation should continue
    pub fn should_continue(&mut self, config: &ColumnGenerationConfig) -> bool {
        if self.iteration >= config.max_iterations {
            self.status = ColumnGenerationStatus::IterationLimit;
            return false;
        }
        if self.total_columns >= config.max_column_amount {
            self.status = ColumnGenerationStatus::ColumnLimit;
            return false;
        }
        if self.not_better_iterations >= config.max_not_better_iterations {
            self.status = ColumnGenerationStatus::Converged;
            return false;
        }
        if self.started_at.elapsed() >= config.time_limit {
            self.status = ColumnGenerationStatus::TimeLimit;
            return false;
        }
        self.status = ColumnGenerationStatus::Running;
        true
    }
}

