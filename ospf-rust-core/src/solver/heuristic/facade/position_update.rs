//! 位置更新类算法入口 / Position-update algorithm entries
//!
//! 这些算法的共同结构是「一次迭代 = 一次被群体动力学牵引的位置更新」：各自只提供 [`SolutionUpdate`] 的具体实现。
//! Their common structure is "one iteration = one position update driven by population dynamics": each supplies only a
//! concrete [`SolutionUpdate`].

use crate::solver::heuristic::composition::population::position_update;
use crate::solver::heuristic::composition::PopulationSolver;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::stopping::StoppingCriterion;
use crate::solver::heuristic::operator::update::{
    ArithmeticOptimizationUpdate, BatAlgorithmUpdate, CuckooSearchUpdate, EnergyValleyUpdate,
    FireflyUpdate, GerminalCenterUpdate, GrasshopperUpdate, GravitationalSearchUpdate,
    GreyWolfUpdate, HarmonySearchUpdate, HarrisHawkUpdate, MothFlameUpdate, MultiVerseUpdate,
    SalpSwarmUpdate, SeagullUpdate, SineCosineUpdate, TeachingLearningUpdate,
    WarStrategyUpdate, WaterCycleUpdate, WhaleOptimizationUpdate,
};

/// 灰狼优化 / Grey wolf optimizer
pub fn grey_wolf<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "gwo",
        size,
        stopping,
        entropy,
        Box::new(GreyWolfUpdate::default()),
    )
}

/// 正弦余弦算法 / Sine cosine algorithm
pub fn sine_cosine<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "sca",
        size,
        stopping,
        entropy,
        Box::new(SineCosineUpdate::default()),
    )
}

/// 多元宇宙优化 / Multi-verse optimizer
pub fn multi_verse<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "mvo",
        size,
        stopping,
        entropy,
        Box::new(MultiVerseUpdate::default()),
    )
}

/// 鲸鱼优化 / Whale optimization algorithm
pub fn whale<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "woa",
        size,
        stopping,
        entropy,
        Box::new(WhaleOptimizationUpdate::default()),
    )
}

/// 哈里斯鹰优化 / Harris hawks optimization
pub fn harris_hawk<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "hho",
        size,
        stopping,
        entropy,
        Box::new(HarrisHawkUpdate::default()),
    )
}

/// 算术优化算法 / Arithmetic optimization algorithm
pub fn arithmetic_optimization<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "aoa",
        size,
        stopping,
        entropy,
        Box::new(ArithmeticOptimizationUpdate::default()),
    )
}

/// 和声搜索 / Harmony search
pub fn harmony_search<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "hs",
        size,
        stopping,
        entropy,
        Box::new(HarmonySearchUpdate::default()),
    )
}

/// 水循環算法 / Water cycle algorithm
pub fn water_cycle<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "wca",
        size,
        stopping,
        entropy,
        Box::new(WaterCycleUpdate::default()),
    )
}

/// 海鸥优化 / Seagull optimization algorithm
pub fn seagull<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "soa",
        size,
        stopping,
        entropy,
        Box::new(SeagullUpdate::default()),
    )
}

/// 教与学优化 / Teaching-learning-based optimization
pub fn teaching_learning<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "tlbo",
        size,
        stopping,
        entropy,
        Box::new(TeachingLearningUpdate::default()),
    )
}

/// 樽海鞘群算法 / Salp swarm algorithm
pub fn salp_swarm<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "ssa",
        size,
        stopping,
        entropy,
        Box::new(SalpSwarmUpdate::default()),
    )
}

/// 引力搜索算法 / Gravitational search algorithm
pub fn gravitational_search<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "gsa",
        size,
        stopping,
        entropy,
        Box::new(GravitationalSearchUpdate::default()),
    )
}

/// 萤火虫算法 / Firefly algorithm
pub fn firefly<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "fa",
        size,
        stopping,
        entropy,
        Box::new(FireflyUpdate::default()),
    )
}

/// 布谷鸟搜索 / Cuckoo search
pub fn cuckoo_search<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "cs",
        size,
        stopping,
        entropy,
        Box::new(CuckooSearchUpdate::default()),
    )
}

/// 蝙蝠算法 / Bat algorithm
pub fn bat_algorithm<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "ba",
        size,
        stopping,
        entropy,
        Box::new(BatAlgorithmUpdate::default()),
    )
}

/// 飞蛾扑火优化 / Moth-flame optimization
pub fn moth_flame<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "mfo",
        size,
        stopping,
        entropy,
        Box::new(MothFlameUpdate::default()),
    )
}

/// 蚱蜢优化算法 / Grasshopper optimization algorithm
pub fn grasshopper<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "goa",
        size,
        stopping,
        entropy,
        Box::new(GrasshopperUpdate::default()),
    )
}

/// 战争策略优化 / War strategy optimization
pub fn war_strategy<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "warso",
        size,
        stopping,
        entropy,
        Box::new(WarStrategyUpdate::default()),
    )
}

/// 能量谷优化 / Energy valley optimizer
pub fn energy_valley<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "evo",
        size,
        stopping,
        entropy,
        Box::new(EnergyValleyUpdate::default()),
    )
}

/// 生发中心优化 / Germinal center optimization
pub fn germinal_center<V>(
    size: usize,
    stopping: Box<dyn StoppingCriterion>,
    entropy: Box<dyn Entropy>,
) -> PopulationSolver<V>
where
    V: HeuristicValue,
{
    position_update(
        "gco",
        size,
        stopping,
        entropy,
        Box::new(GerminalCenterUpdate::default()),
    )
}
