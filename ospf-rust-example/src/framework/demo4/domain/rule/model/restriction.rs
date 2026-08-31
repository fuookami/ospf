/// 限制类型 / Restriction type (对齐 Kotlin RestrictionType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RestrictionType {
    General,
    Relation,
}

/// 限制检查结果 / Restriction checking result (对齐 Kotlin RestrictionCheckingResult)
#[derive(Debug, Clone)]
pub enum RestrictionCheckingResult {
    NotMatter,
    Violate { reason: String },
    NotViolate,
    ViolableViolate { reason: String },
}

/// 限制 / Restriction (对齐 Kotlin Restriction sealed interface)
#[derive(Debug, Clone)]
pub struct Restriction {
    pub id: String,
    pub restriction_type: RestrictionType,
    pub description: String,
}

impl Restriction {
    /// 检查限制 / Check restriction
    /// 对齐 Kotlin Restriction.check
    pub fn check(&self) -> RestrictionCheckingResult {
        RestrictionCheckingResult::NotMatter
    }
}
