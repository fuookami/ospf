//! 规则上下文 / Rule context
/// 对齐 Kotlin RuleContext
#[derive(Debug)]
pub struct RuleContext {
    /// 锁定列表 / Lock list
    pub locks: Vec<super::model::Lock>,
    /// 链接列表 / Link list
    pub links: Vec<super::model::Link>,
    /// 限制列表 / Restriction list
    pub restrictions: Vec<super::model::Restriction>,
}

impl RuleContext {
    /// 创建空的规则上下文 / Create an empty rule context
    pub fn new() -> Self {
        Self {
            locks: Vec::new(),
            links: Vec::new(),
            restrictions: Vec::new(),
        }
    }
}
