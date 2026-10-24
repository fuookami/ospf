//! 自适应奖励与独立几何黄金向量 / Adaptive reward and independent geometry golden vectors

use super::*;

fn context(reference: Vec<Vec<f64>>, candidates: Vec<Vec<f64>>) -> RewardContext {
    RewardContext { action: "a".into(), reference, candidates, ..Default::default() }
}

fn hypervolume(reference: Vec<f64>) -> HypervolumeDeltaReward {
    HypervolumeDeltaReward::new(HypervolumeConfig { reference, ..Default::default() }).unwrap()
}

fn inclusion_exclusion(points: &[Vec<f64>], reference: &[f64]) -> f64 {
    let mut volume = 0.0;
    for mask in 1_usize..1 << points.len() {
        let corner: Vec<_> = (0..reference.len()).map(|axis| points.iter().enumerate()
            .filter(|(index, _)| mask & (1 << index) != 0).map(|(_, point)| point[axis]).fold(f64::NEG_INFINITY, f64::max)).collect();
        let overlap = reference.iter().zip(corner).fold(1.0, |value, (high, low)| value * (high - low).max(0.0));
        volume += if mask.count_ones() % 2 == 1 { overlap } else { -overlap };
    }
    volume
}

#[test]
fn scalar_credit_handles_negative_minimization_and_explicit_maximization_projection() {
    let mut reward = TimeNormalizedReward::new(0.5).unwrap();
    let mut credit = context(vec![vec![-2.0]], vec![vec![-5.0], vec![-3.0]]);
    credit.elapsed_nanos = 500_000_000;
    assert_eq!(reward.reward(&credit).unwrap(), 3.0);
    credit.reference = vec![vec![-10.0]]; credit.candidates = vec![vec![-12.0]];
    assert_eq!(reward.reward(&credit).unwrap(), 2.0);
    assert!(reward.requires_elapsed_time()); assert!(reward.validate_problem(false).is_err());
    credit.candidates = vec![vec![1.0, 2.0]]; assert!(reward.reward(&credit).is_err());
    credit.reference = vec![vec![f64::MAX]]; credit.candidates = vec![vec![-f64::MAX]];
    assert!(reward.reward(&credit).is_err());
    assert!(TimeNormalizedReward::new(0.0).is_err()); assert!(TimeNormalizedReward::new(f64::NAN).is_err());
}

#[test]
fn extreme_credit_expires_globally_and_recovers_full_action_history() {
    let mut reward = ExtremeValueReward::new(2).unwrap();
    let mut credit = context(vec![vec![10.0]], vec![vec![0.0]]);
    assert_eq!(reward.reward(&credit).unwrap(), 10.0);
    credit.candidates = vec![vec![9.0]]; assert_eq!(reward.reward(&credit).unwrap(), 10.0);
    credit.action = "b".into(); assert_eq!(reward.reward(&credit).unwrap(), 1.0);
    let saved = reward.save().unwrap();
    let mut restored = ExtremeValueReward::new(2).unwrap(); restored.load(&saved).unwrap();
    credit.action = "a".into(); assert_eq!(reward.reward(&credit).unwrap(), 1.0);
    assert_eq!(restored.reward(&credit).unwrap(), 1.0);
    assert_eq!(restored.save().unwrap(), reward.save().unwrap());
    let before = restored.save().unwrap();
    let mut bad = before.clone(); bad["observations"][0]["second"] = (-1.0).into();
    assert!(restored.load(&bad).is_err()); assert_eq!(restored.save().unwrap(), before);
}

#[test]
fn dominance_uses_effective_contribution_instead_of_candidate_count() {
    let mut reward = DominanceReward;
    let mut credit = RewardContext { contribution: 0.25, candidates: vec![vec![1.0]; 20], ..Default::default() };
    assert_eq!(reward.reward(&credit).unwrap(), 0.25);
    credit.contribution = f64::NAN; assert!(reward.reward(&credit).is_err());
}

#[test]
fn exact_three_and_four_dimensional_hv_matches_independent_inclusion_exclusion() {
    for dimension in [3, 4] {
        let reference = vec![8.0; dimension];
        let points = vec![(0..dimension).map(|axis| if axis == 0 { -1.0 } else { 4.0 }).collect::<Vec<_>>(),
            (0..dimension).map(|axis| if axis == 1 { 0.0 } else { 3.0 }).collect::<Vec<_>>(),
            (0..dimension).map(|axis| if axis == 2 { 1.0 } else { 2.0 }).collect::<Vec<_>>()];
        let expected = inclusion_exclusion(&points, &reference) - inclusion_exclusion(&points[..1], &reference);
        let value = hypervolume(reference).measure(&context(points[..1].to_vec(), points[1..].to_vec())).unwrap();
        assert_eq!(value.delta, expected); assert!(value.exact); assert!(value.work > 0);
    }
}

#[test]
fn exact_hv_handles_duplicates_overlap_and_points_outside_reference() {
    let mut reward = hypervolume(vec![5.0, 5.0]);
    let credit = context(vec![vec![2.0, 3.0]], vec![vec![1.0, 4.0], vec![1.0, 4.0], vec![2.0, 4.0], vec![6.0, 0.0]]);
    assert_eq!(reward.measure(&credit).unwrap().delta, 1.0);
    let repeated = context(vec![vec![2.0, 3.0]], vec![vec![2.0, 3.0]]);
    assert_eq!(reward.measure(&repeated).unwrap().delta, 0.0);
    assert_eq!(hypervolume(vec![4.0]).measure(&context(Vec::new(), vec![vec![-2.0], vec![0.0]])).unwrap().delta, 6.0);
}

#[test]
fn exact_hv_rejects_work_depth_bad_geometry_and_overflow_without_fallback() {
    let credit = context(vec![vec![2.0; 3]], vec![vec![1.0; 3]]);
    let mut reward = HypervolumeDeltaReward::new(HypervolumeConfig { reference: vec![4.0; 3], max_work: 1, ..Default::default() }).unwrap();
    let before = reward.save().unwrap();
    assert!(reward.measure(&credit).unwrap_err().to_string().contains("work exhausted"));
    assert_eq!(reward.save().unwrap(), before);
    let mut deep = hypervolume(vec![1.0; 257]);
    assert!(deep.measure(&context(Vec::new(), vec![vec![0.0; 257]])).unwrap_err().to_string().contains("depth"));
    let mut reward = hypervolume(vec![f64::MAX, f64::MAX]);
    assert!(reward.measure(&context(Vec::new(), vec![vec![0.0, 0.0]])).is_err());
    assert!(reward.measure(&context(Vec::new(), vec![vec![0.0]])).is_err());
    assert!(reward.measure(&context(Vec::new(), vec![vec![0.0, f64::NAN]])).is_err());
    assert!(reward.last_measurement().is_none());
}

#[test]
fn sampled_hv_uses_reproducible_common_random_numbers_and_reports_uncertainty() {
    let config = HypervolumeConfig { reference: vec![5.0; 3], mode: HypervolumeMode::Sampled, seed: -7, ..Default::default() };
    let credit = context(vec![vec![2.0; 3]], vec![vec![1.0, 3.0, 1.0], vec![3.0, 1.0, 2.0]]);
    let exact = hypervolume(config.reference.clone()).measure(&credit).unwrap().delta;
    let mut reward = HypervolumeDeltaReward::new(config.clone()).unwrap();
    let value = reward.measure(&credit).unwrap();
    assert!(!value.exact); assert_eq!(value.samples, 16384);
    assert!((value.delta - exact).abs() < value.error_bound);
    assert!(value.standard_error > 0.0 && value.error_bound > value.standard_error);
    assert_eq!(reward.measure(&credit).unwrap(), value);
    let mut restored = HypervolumeDeltaReward::new(config).unwrap(); restored.load(&reward.save().unwrap()).unwrap();
    assert_eq!(restored.last_measurement(), Some(&value)); assert_eq!(restored.measure(&credit).unwrap(), value);
}

#[test]
fn sampled_hv_is_explicitly_available_beyond_exact_depth_and_remains_work_bounded() {
    let config = HypervolumeConfig { reference: vec![1.0; 257], mode: HypervolumeMode::Sampled, samples: 4, max_work: 4, ..Default::default() };
    let credit = context(Vec::new(), vec![vec![0.0; 257]]);
    let mut reward = HypervolumeDeltaReward::new(config.clone()).unwrap();
    assert_eq!(reward.measure(&credit).unwrap().delta, 1.0);
    let mut too_small = HypervolumeDeltaReward::new(HypervolumeConfig { max_work: 3, ..config }).unwrap();
    assert!(too_small.measure(&credit).is_err()); assert!(too_small.last_measurement().is_none());
}

#[test]
fn automatic_reference_freezes_after_valid_candidates_and_restores_calibration_and_measurement() {
    let config = HypervolumeConfig { calibration_observations: 2, ..Default::default() };
    let mut reward = HypervolumeDeltaReward::new(config.clone()).unwrap();
    let first = reward.measure(&context(vec![vec![0.0, 4.0]], vec![vec![1.0, 2.0]])).unwrap();
    assert!(!first.calibrated); assert_eq!(first.delta, 0.0);
    let saved = reward.save().unwrap();
    let mut restored = HypervolumeDeltaReward::new(config).unwrap(); restored.load(&saved).unwrap();
    let next = context(vec![vec![0.0, 4.0]], vec![vec![2.0, 1.0]]);
    assert!(reward.measure(&next).unwrap().calibrated);
    assert_eq!(restored.measure(&next).unwrap().delta, 0.0);
    assert_eq!(restored.save().unwrap(), reward.save().unwrap());
    let reference = reward.save().unwrap()["reference"].clone();
    let last = context(vec![vec![0.0, 4.0]], vec![vec![0.0, 1.0]]);
    assert!(reward.measure(&last).unwrap().delta > 0.0);
    assert_eq!(restored.measure(&last).unwrap(), *reward.last_measurement().unwrap());
    assert_eq!(reward.save().unwrap()["reference"], reference);
}

#[test]
fn invalid_hv_states_are_rejected_atomically() {
    let mut reward = hypervolume(vec![4.0, 4.0]);
    reward.measure(&context(Vec::new(), vec![vec![1.0, 1.0]])).unwrap();
    let original = reward.save().unwrap();
    let mut bad = original.clone(); bad["measurement"]["calibrated"] = false.into();
    assert!(reward.load(&bad).is_err()); assert_eq!(reward.save().unwrap(), original);
    let mut bad = original.clone(); bad["reference"] = serde_json::json!([3.0, 4.0]);
    assert!(reward.load(&bad).is_err()); assert_eq!(reward.save().unwrap(), original);
    let mut bad = original.clone(); bad["measurement"]["work"] = 1_000_001.into();
    assert!(reward.load(&bad).is_err()); assert_eq!(reward.save().unwrap(), original);
}

#[test]
fn automatic_reference_overflow_rolls_back_calibration_instead_of_leaving_unrestorable_state() {
    let mut reward = HypervolumeDeltaReward::new(HypervolumeConfig { calibration_observations: 1, ..Default::default() }).unwrap();
    let before = reward.save().unwrap();
    assert!(reward.measure(&context(Vec::new(), vec![vec![f64::MAX]])).is_err());
    assert_eq!(reward.save().unwrap(), before);
    reward.load(&before).unwrap();
}

#[test]
fn reward_configuration_validates_aliases_versions_and_numeric_resource_bounds() {
    for spec in [ComponentSpec::new("hypervolume-delta").parameter("referenceX", 5.0),
        ComponentSpec::new("hypervolume-delta").parameter("mode", "automatic"),
        ComponentSpec::new("hypervolume-delta").parameter("samples", 0),
        ComponentSpec::new("hypervolume-delta").parameter("maxWork", 0),
        ComponentSpec::new("extreme-value").parameter("window", 0),
        ComponentSpec::new("dominance").parameter("unused", true)] { assert!(builtin_reward(&spec).is_err(), "{spec:?}"); }
    let mut alias = builtin_reward(&ComponentSpec::new("hypervolume-delta").parameter("referenceX", 5.0).parameter("referenceY", 5.0)).unwrap();
    assert_eq!(alias.reward(&context(Vec::new(), vec![vec![1.0, 1.0]])).unwrap(), 16.0);
}
