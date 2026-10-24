//! 算法入口 / Algorithm entries
//!
//! 每个算法一个模块，只做一件事：把默认算子填进组合层工厂。用户既可以调用这里的便捷入口，也可以直接使用
//! `composition` 自己装配算子。
//! One module per algorithm doing exactly one thing: filling default operators into a composition factory. Users may call
//! these convenience entries or assemble operators directly through `composition`.

pub mod apso;
pub mod clpso;
pub mod cmaes;
pub mod de;
pub mod ga;
pub mod hca;
pub mod island_ga;
pub mod ns;
pub mod position_update;
pub mod pso;
pub mod qlearning;
pub mod shade;

pub use apso::{AdaptiveParticleSwarmSolver, adaptive_particle_swarm};
pub use clpso::{ClpsoSolver, comprehensive_learning_particle_swarm};
pub use cmaes::{CmaEsSolver, cma_es};
pub use de::differential_evolution;
pub use ga::genetic;
pub use hca::hill_climbing;
pub use island_ga::{IslandSpec, island_genetic};
pub use ns::neighborhood_search;
pub use position_update::{
    arithmetic_optimization, bat_algorithm, cuckoo_search, energy_valley, firefly,
    germinal_center, gravitational_search, grasshopper, grey_wolf, harmony_search,
    harris_hawk, moth_flame, multi_verse, salp_swarm, seagull, sine_cosine,
    teaching_learning, war_strategy, water_cycle, whale,
};
pub use pso::particle_swarm;
pub use qlearning::q_learning;
pub use shade::{ShadeSolver, l_shade, shade};
