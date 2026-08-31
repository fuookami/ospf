//! 限制规则模型 / Restriction rule model

/// 限制类型 / Restriction type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RestrictionType {
    /// 通用限制 / General restriction
    General,
    /// 关联限制 / Relation restriction
    Relation,
}

/// 限制检查结果 / Restriction checking result (对齐 Kotlin RestrictionCheckingResult)
#[derive(Debug, Clone)]
pub enum RestrictionCheckingResult {
    /// 无关 / Not applicable
    NotMatter,
    /// 违规，包含原因 / Violation with reason
    Violate { reason: String },
    /// 未违规 / No violation
    NotViolate,
    /// 可豁免违规，包含原因 / Waivable violation with reason
    ViolableViolate { reason: String },
}

/// 限制 / Restriction (对齐 Kotlin Restriction sealed interface)
#[derive(Debug, Clone)]
pub struct Restriction {
    /// 限制标识 / Restriction identifier
    pub id: String,
    /// 限制类型 / Restriction type
    pub restriction_type: RestrictionType,
    /// 限制描述 / Restriction description
    pub description: String,
}

impl Restriction {
    /// 检查限制 / Check restriction
    /// 对齐 Kotlin Restriction.check
    pub fn check(&self) -> RestrictionCheckingResult {
        RestrictionCheckingResult::NotMatter
    }
}
