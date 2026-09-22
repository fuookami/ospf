//! 函数符号使用语境摘要 / Function-symbol usage context summary
//!
//! 原生 lowering 不能只依据函数类型和求解器能力决定，还必须知道函数结果与辅助列在最终模型
//! 中的使用方式。摘要只描述已经落入机制模型的引用关系，不重新扫描已经丢失的函数图，因此
//! 求解器适配器与报告都能从同一份数据出发。
//!
//! Native lowering cannot be decided from the function type and solver capability alone; it also
//! needs to know how the function result and helper columns are used in the final model. The
//! summary only describes references that already exist in the mechanism model instead of
//! re-scanning a function graph that is no longer available, so solver adapters and reports can
//! start from the same data.

/// 函数结果与辅助列的使用语境 / Usage context of a function result and its helpers.
///
/// 所有判定都以列引用为准：本函数自身的关系行（`Constraint::from` 指向该符号）不计入外部
/// 引用，其余约束行、其它符号的行以及目标函数都计入。
///
/// Every flag is decided from column references: the function's own relation rows
/// (`Constraint::from` pointing at the symbol) never count as external references, while every
/// other constraint row, another symbol's rows and the objective do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FunctionUsageSummary {
    /// 结果列是否进入目标函数 / Whether the result column enters the objective.
    pub in_objective: bool,
    /// 结果列是否进入非本函数的约束行 / Whether the result column enters a constraint row that is not this function's own.
    pub in_constraint: bool,
    /// 结果列是否被其它中间符号当作输入使用 / Whether another intermediate symbol consumes the result column as input.
    pub nested_as_input: bool,
    /// 辅助列是否被外部引用 / Whether a helper column is referenced externally.
    ///
    /// 结果列不计入本标志；它描述的是除结果列之外的正部、负部、选择器等辅助列。
    /// The result column is not part of this flag; it covers helpers such as positive parts,
    /// negative parts and selectors.
    pub externally_referenced: bool,
    /// 结果列是否已被固定（声明范围退化为单点）。
    ///
    /// 结果被固定意味着求解侧可以用常量代换该列；此时原生写入不再必要，而且替换结果列会改变
    /// 原生结构的含义，因此原生 writer 必须拒绝并回退通用展开。
    ///
    /// Whether the result column is already fixed (its declared range collapsed to a single point).
    ///
    /// A fixed result means the solver can substitute the column by a constant; a native write is
    /// then unnecessary and replacing the result column would change the meaning of the native
    /// structure, so a native writer must reject and fall back to the generic expansion.
    pub result_is_fixed: bool,
}

impl FunctionUsageSummary {
    /// 结果列是否必须保留为可被其它行引用的列。
    ///
    /// 结果进入约束或被其它函数嵌套时，只接受目标项的快捷原生接口不能替代 `y = f(x)` 关系，
    /// 必须保留结果变量。
    ///
    /// Whether the result column must stay referenceable by other rows.
    ///
    /// When the result enters constraints or is nested inside another function, a native
    /// objective-only shortcut cannot replace the `y = f(x)` relation and the result variable
    /// must be kept.
    pub fn requires_referenceable_result(&self) -> bool {
        self.in_constraint || self.nested_as_input
    }

    /// 辅助列是否仅被本函数的关系行独占引用。
    ///
    /// 只有独占的辅助列才可能被原生 writer 省略；结果列不参与该判断。
    ///
    /// Whether the helper columns are referenced exclusively by this function's own relation
    /// rows. Only exclusive helpers may be omitted by a native writer; the result column is not
    /// part of this decision.
    pub fn helpers_are_exclusive(&self) -> bool {
        !self.externally_referenced
    }

    /// 结果列是否被任何下游消费者使用 / Whether any downstream consumer uses the result column.
    pub fn result_is_consumed(&self) -> bool {
        self.in_objective || self.in_constraint || self.nested_as_input
    }

    /// 原生写入是否被禁止。
    ///
    /// 结果被固定时原生写入必须回退：固定列的代换会让原生关系失去意义。该判定与
    /// [`Self::requires_referenceable_result`] 不同，后者说明"结果需要被别的行引用"，本方法
    /// 说明"结果不允许被原生结构继续引用"。
    ///
    /// Whether a native write is forbidden.
    ///
    /// A fixed result forces a fallback because substituting the column would make the native
    /// relation meaningless. This differs from [`Self::requires_referenceable_result`], which says
    /// the result must stay referenceable by other rows; this method says a native structure may no
    /// longer reference it.
    pub fn forbids_native_write(&self) -> bool {
        self.result_is_fixed
    }
}
