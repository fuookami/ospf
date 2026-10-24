//! 算法组合层 / Algorithm composition layer
//!
//! 本层把骨架与算子装配成具体算法：每个工厂只描述「用哪个骨架、装哪些算子」，不写任何循环。
//! This layer assembles skeletons and operators into concrete algorithms: each factory only describes "which skeleton and
//! which operators", never a loop.

pub mod config;
pub mod memetic;
pub mod multi_objective;
pub mod population;
pub mod q_learning;
pub mod trajectory;

pub use config::SearchConfig;
pub use memetic::{MemeticConfig, MemeticSolver, memetic};
pub use multi_objective::{nsga2, nsga2_minimization, nsga3, nsga3_minimization};
pub use q_learning::q_learning;
pub use population::{
    IslandConfig, IslandSolver, ParticleSwarmSolver, PopulationConfig, PopulationSolver,
    SwarmConfig, genetic, island, particle_swarm, population, position_update,
};
pub use trajectory::{
    TrajectorySolver, hill_climbing, neighborhood_search, random_walk, simulated_annealing,
};
