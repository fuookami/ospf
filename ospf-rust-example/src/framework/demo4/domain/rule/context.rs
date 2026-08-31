/// 规则上下文 / Rule context
/// 对齐 Kotlin RuleContext
#[derive(Debug)]
pub struct RuleContext {
    pub locks: Vec<super::model::Lock>,
    pub links: Vec<super::model::Link>,
    pub restrictions: Vec<super::model::Restriction>,
}

impl RuleContext {
    pub fn new() -> Self {
        Self {
            locks: Vec::new(),
            links: Vec::new(),
            restrictions: Vec::new(),
        }
    }
}
