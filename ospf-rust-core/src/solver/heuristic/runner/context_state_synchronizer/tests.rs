//! 原目的状态匹配与真实记忆同步黄金向量 / Golden vectors for original-destination matching and genuine memory synchronization

use crate::solver::heuristic::runner::ShadeMemory;
use crate::variable::{VariableId, new_standalone_id};
use super::*;

struct Problem { order: Vec<VariableId> }

impl SearchProblem<f64, f64> for Problem {
    fn id(&self) -> &str { "bridge" }
    fn fingerprint(&self) -> &str { "bridge:1" }
    fn variable_order(&self) -> Vec<VariableId> { self.order.clone() }
    fn initial_solutions(&self, _: usize) -> Vec<Solution<f64>> { Vec::new() }
    fn objective(&self, _: &Solution<f64>) -> Option<f64> { panic!("imports must already be evaluated"); }
    fn dominates(&self, a: &f64, b: &f64) -> bool { a < b }
    fn valid_objective(&self, value: &f64) -> bool { value.is_finite() }
    fn admissible(&self, solution: &Solution<f64>) -> bool { solution[&self.order[0]] != 999.0 }
}

fn problem() -> Problem { Problem { order: vec![new_standalone_id(), new_standalone_id()] } }
fn point(problem: &Problem, values: [f64; 2], fitness: f64) -> Individual<f64, f64> {
    Individual::new(problem.order.iter().copied().zip(values).collect(), fitness)
}

fn assert_point(actual: &Individual<f64, f64>, expected: &Individual<f64, f64>) {
    assert_eq!(actual.solution, expected.solution); assert_eq!(actual.fitness, expected.fitness);
}

fn particles(problem: &Problem) -> Vec<ParticleState<f64>> {
    vec![ParticleState { individual: point(problem, [0.0, 0.0], 4.0), velocity: vec![0.5, 0.25], best: point(problem, [-1.0, -1.0], 0.0) },
        ParticleState { individual: point(problem, [10.0, 10.0], 9.0), velocity: vec![1.0, 1.5], best: point(problem, [8.0, 8.0], 1.0) }]
}

#[test]
fn incoming_particles_match_original_nearest_sources_and_replace_worst_in_reverse_order() {
    let problem = problem(); let bridge = ContextStateSynchronizer::new(&problem, 2.0).unwrap();
    let state = particles(&problem);
    let incoming = vec![point(&problem, [9.0, 9.0], 5.0), point(&problem, [-9.0, 2.0], 6.0)];
    let result = bridge.particles(&state, &incoming).unwrap();
    assert_point(&result[1].individual, &incoming[0]); assert_eq!(result[1].velocity, vec![-1.0, -1.0]);
    assert_point(&result[1].best, &state[1].best);
    assert_point(&result[0].individual, &incoming[1]); assert_eq!(result[0].velocity, vec![-2.0, 2.0]);
    assert_point(&result[0].best, &state[0].best);
    let improved = point(&problem, [9.0, 9.0], -1.0);
    assert_point(&bridge.particles(&state, &[improved.clone()]).unwrap()[1].best, &improved);
}

#[test]
fn bridge_velocity_clamps_overflow_and_rejects_invalid_dimensions_or_business_exclusions() {
    let problem = problem(); let bridge = ContextStateSynchronizer::new(&problem, 3.0).unwrap();
    assert_eq!(bridge.velocity(&point(&problem, [-f64::MAX, f64::MAX], 0.0).solution,
        &point(&problem, [f64::MAX, -f64::MAX], 0.0).solution).unwrap(), vec![3.0, -3.0]);
    let mut bad = point(&problem, [0.0, 0.0], 0.0).solution; bad.remove(&problem.order[1]);
    assert!(bridge.velocity(&bad, &point(&problem, [0.0, 0.0], 0.0).solution).is_err());
    for incoming in [point(&problem, [0.0, f64::INFINITY], 1.0), point(&problem, [0.0, 0.0], f64::NAN),
        point(&problem, [999.0, 0.0], 1.0)] { assert!(bridge.particles(&particles(&problem), &[incoming]).is_err()); }
    assert!(ContextStateSynchronizer::new(&problem, 0.0).is_err());
    assert!(ContextStateSynchronizer::new(&problem, f64::NAN).is_err());
}

#[test]
fn clpso_rebuilds_all_exemplars_without_losing_synchronized_motion_or_best() {
    let problem = problem(); let bridge = ContextStateSynchronizer::new(&problem, 2.0).unwrap();
    let state: Vec<_> = particles(&problem).into_iter().map(|particle| ClpsoParticle {
        individual: particle.individual, velocity: particle.velocity, best: particle.best, exemplars: vec![1, 0], stagnation: 4,
    }).collect();
    let incoming = point(&problem, [9.0, 9.0], 5.0);
    let result = bridge.clpso(&state, &[incoming]).unwrap();
    for (index, particle) in result.iter().enumerate() { assert_eq!(particle.exemplars, vec![index; 2]); assert_eq!(particle.stagnation, 0); }
    assert_eq!(result[1].velocity, vec![-1.0, -1.0]); assert_point(&result[1].best, &state[1].best);
    assert_eq!(bridge.clpso(&state, &[]).unwrap()[0].stagnation, 4);
}

#[test]
fn shade_imports_deduplicate_archive_displaced_members_and_preserve_true_memory() {
    let problem = problem(); let bridge = ContextStateSynchronizer::new(&problem, 2.0).unwrap();
    let old = vec![point(&problem, [0.0, 0.0], 4.0), point(&problem, [1.0, 1.0], 3.0), point(&problem, [2.0, 2.0], 2.0)];
    let state = ShadeState { population: old.clone(), memory: ShadeMemory { memory_f: vec![0.2, 0.7], memory_cr: vec![0.3, 0.9], cursor: 1 },
        archive: vec![old[0].solution.clone(), old[0].solution.clone(), point(&problem, [7.0, 7.0], 0.0).solution] };
    let best = point(&problem, [-1.0, -1.0], 0.5);
    let result = bridge.shade(&state, &[best.clone(), best.clone(), old[2].clone()]).unwrap();
    assert_eq!(result.population.len(), 3); assert_point(&result.population[0], &best);
    assert!(!result.population.iter().any(|value| value.solution == old[0].solution));
    assert!(result.archive.contains(&old[0].solution)); assert_eq!(result.archive.len(), 2);
    assert_eq!(result.memory, state.memory);
    let worst = point(&problem, [8.0, 8.0], 10.0);
    let result = bridge.shade(&state, &[worst.clone()]).unwrap();
    assert!(!result.archive.contains(&worst.solution)); assert_eq!(result.memory, state.memory);
}

#[test]
fn empty_particle_imports_preserve_destination_state_and_empty_destinations_stay_empty() {
    let problem = problem(); let bridge = ContextStateSynchronizer::new(&problem, 2.0).unwrap();
    let state = particles(&problem);
    let result = bridge.particles(&state, &[]).unwrap();
    assert_point(&result[0].individual, &state[0].individual); assert_eq!(result[0].velocity, state[0].velocity);
    assert!(bridge.particles(&[], &[point(&problem, [0.0, 0.0], 0.0)]).unwrap().is_empty());
}
