//! 奖励算子 / Reward operators

/// 奖励契约：把「这一轮有没有变好」翻译成标量奖励 / Reward contract translating "did this round improve" into a scalar reward
pub trait Reward {
    /// 计算奖励 / Compute the reward
    fn reward(&self, improved: bool) -> f64;
}

/// 改进奖励：变好给正奖励，变差给负奖励 / Improvement reward: positive when improved, negative otherwise
#[derive(Debug, Clone, Copy)]
pub struct ImprovementReward {
    /// 改进时的奖励 / Reward when improved
    pub positive: f64,
    /// 未改进时的奖励 / Reward when not improved
    pub negative: f64,
}

impl ImprovementReward {
    /// 创建改进奖励 / Create an improvement reward
    pub fn new(positive: f64, negative: f64) -> Self {
        Self { positive, negative }
    }
}

impl Default for ImprovementReward {
    fn default() -> Self {
        Self::new(1.0, -0.5)
    }
}

impl Reward for ImprovementReward {
    fn reward(&self, improved: bool) -> f64 {
        if improved {
            self.positive
        } else {
            self.negative
        }
    }
}

/// 对称奖励：改进给 `magnitude`，未改进给它的相反数 / Symmetric reward: `magnitude` when improved, its negation otherwise
#[derive(Debug, Clone, Copy)]
pub struct SymmetricReward {
    /// 奖励幅度 / Reward magnitude
    pub magnitude: f64,
}

impl SymmetricReward {
    /// 创建对称奖励 / Create a symmetric reward
    pub fn new(magnitude: f64) -> Self {
        Self { magnitude }
    }
}

impl Default for SymmetricReward {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl Reward for SymmetricReward {
    fn reward(&self, improved: bool) -> f64 {
        if improved {
            self.magnitude
        } else {
            -self.magnitude
        }
    }
}
