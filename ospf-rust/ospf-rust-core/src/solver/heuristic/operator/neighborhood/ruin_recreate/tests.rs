use std::cell::RefCell;
use std::rc::Rc;
use super::*;

#[test]
fn random_ruin_matches_splitmix_golden_partition_without_replacement() {
    let mut random = SearchRandom::new(0);
    let result = RandomRuin.remove(&[0, 1, 2, 3], 2, &mut random).unwrap();
    assert_eq!(result, RuinResult { retained: vec![0, 1], removed: vec![2, 3] });
    assert_eq!(random.state(), 0x9E37_79B9_7F4A_7C15_u64.wrapping_mul(2));
    for size in 0..12 {
        let source: Vec<_> = (0..size).collect();
        for amount in 0..=size {
            let result = RandomRuin.remove(&source, amount, &mut random).unwrap();
            assert_eq!((result.retained.len(), result.removed.len()), (size - amount, amount));
            let mut restored = result.retained; restored.extend(result.removed); restored.sort();
            assert_eq!(restored, source);
        }
    }
    assert!(RandomRuin.remove(&[1, 1], 1, &mut random).is_err());
    assert!(RandomRuin.remove(&[f64::NAN], 0, &mut random).is_err());
    assert!(RandomRuin.remove(&[1], 2, &mut random).is_err());
}

#[test]
fn worst_cost_is_stable_for_ties_and_validates_all_business_costs_without_random_draws() {
    let mut random = SearchRandom::new(42);
    let ruin = WorstCostRuin::new(|items: &[i32], index| Ok([2.0, 5.0, 5.0, 1.0][items[index] as usize]));
    assert_eq!(ruin.remove(&[0, 1, 2, 3], 2, &mut random).unwrap(), RuinResult { retained: vec![0, 3], removed: vec![1, 2] });
    assert_eq!(random.state(), 42);
    assert!(WorstCostRuin::new(|_: &[i32], _| Ok(f64::INFINITY)).remove(&[0], 0, &mut random).is_err());
    assert!(WorstCostRuin::new(|_: &[i32], _| Err(invalid("domain failed"))).remove(&[0], 1, &mut random).is_err());
    assert!(WorstCostRuin::new(|_: &[i32], _| -> Result<f64> { panic!("domain panic") }).remove(&[0], 1, &mut random).is_err());
}

#[test]
fn finite_cost_sorting_preserves_jvm_signed_zero_order_and_stable_positions() {
    let mut random = SearchRandom::new(0);
    let ruin = WorstCostRuin::new(|_: &[i32], index| Ok(if index == 0 { -0.0 } else { 0.0 }));
    assert_eq!(ruin.remove(&[0, 1], 1, &mut random).unwrap().removed, vec![1]);
    let repair = GreedyInsertRepair::new(|_: &[i32], _: &i32, position| Ok(Some(if position == 0 { 0.0 } else { -0.0 })));
    assert_eq!(repair.repair(&RuinResult { retained: vec![99], removed: vec![1] }).unwrap(), vec![99, 1]);
}

#[test]
fn relatedness_expands_nearest_cluster_and_rejects_bad_distances() {
    let mut random = SearchRandom::new(0);
    let ruin = RelatednessRuin::new(|a: &i32, b: &i32| Ok((a - b).abs() as f64));
    assert_eq!(ruin.remove(&[0, 1, 2, 3, 4], 2, &mut random).unwrap(), RuinResult { retained: vec![0, 1, 2], removed: vec![3, 4] });
    assert_eq!(random.state(), 0x9E37_79B9_7F4A_7C15_u64.wrapping_mul(2));
    let before = random.state();
    assert_eq!(ruin.remove(&[0, 1], 0, &mut random).unwrap().removed, Vec::<i32>::new());
    assert_eq!(random.state(), before);
    for bad in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(RelatednessRuin::new(move |_: &i32, _: &i32| Ok(bad)).remove(&[0, 1], 2, &mut random).is_err());
    }
    assert!(RelatednessRuin::new(|_: &i32, _: &i32| -> Result<f64> { panic!("relatedness panic") }).remove(&[0, 1], 2, &mut random).is_err());
}

fn urgent_cost(partial: &[i32], item: &i32, position: usize) -> Result<Option<f64>> {
    let _ = partial;
    Ok(if *item == 2 { (position == 1).then_some(100.0) } else { Some(10.0 * position as f64) })
}

#[test]
fn regret_prioritizes_fewer_than_k_positions_while_k_one_is_global_greedy() {
    let ruined = RuinResult { retained: vec![0, 9], removed: vec![1, 2] };
    let trace = Rc::new(RefCell::new(Vec::new()));
    let observed = trace.clone();
    let repair = RegretKInsertRepair::new(move |partial: &[i32], item: &i32, position| {
        observed.borrow_mut().push(partial.to_vec()); urgent_cost(partial, item, position)
    }, 2);
    assert_eq!(repair.repair(&ruined).unwrap(), vec![1, 0, 2, 9]);
    assert!(trace.borrow().iter().any(|partial| partial == &[0, 2, 9]));
    assert_eq!(GreedyInsertRepair::new(urgent_cost).repair(&ruined).unwrap(), vec![1, 2, 0, 9]);
    assert_eq!(RegretKInsertRepair::new(urgent_cost, 1).repair(&ruined).unwrap(), vec![1, 2, 0, 9]);
}

#[test]
fn repair_rejects_duplicate_uninsertable_nonfinite_or_failed_estimates_without_dropping_elements() {
    let ruined = RuinResult { retained: vec![0], removed: vec![1] };
    assert!(RegretKInsertRepair::new(urgent_cost, 0).repair(&ruined).is_err());
    assert!(RegretKInsertRepair::new(urgent_cost, 2).repair(&RuinResult { retained: vec![1], removed: vec![1] }).is_err());
    assert!(GreedyInsertRepair::new(|_: &[i32], _: &i32, _| Ok(None)).repair(&ruined).is_err());
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(GreedyInsertRepair::new(move |_: &[i32], _: &i32, _| Ok(Some(bad))).repair(&ruined).is_err());
    }
    assert!(RegretKInsertRepair::new(|_: &[i32], _: &i32, position| Ok(Some(if position == 0 { -f64::MAX } else { f64::MAX })), 2).repair(&ruined).is_err());
    assert!(GreedyInsertRepair::new(|_: &[i32], _: &i32, _| Err(invalid("estimate failed"))).repair(&ruined).is_err());
    assert!(GreedyInsertRepair::new(|_: &[i32], _: &i32, _| -> Result<Option<f64>> { panic!("insertion panic") }).repair(&ruined).is_err());
    assert_eq!(ruined, RuinResult { retained: vec![0], removed: vec![1] });
}

struct LostRuin;

impl Ruin<i32> for LostRuin {
    fn remove(&self, _solution: &[i32], _amount: usize, _random: &mut SearchRandom) -> Result<RuinResult<i32>> {
        Ok(RuinResult { retained: vec![], removed: vec![] })
    }
}

struct LostRepair;

impl Repair<i32> for LostRepair { fn repair(&self, _ruined: &RuinResult<i32>) -> Result<Vec<i32>> { Ok(vec![99]) } }

#[test]
fn recreate_conserves_every_identity_for_empty_full_and_partial_ruin_and_checks_custom_extensions() {
    let operator = RuinRecreateOperator::new(RandomRuin, GreedyInsertRepair::new(|_: &[i32], item: &i32, position| Ok(Some((*item - position as i32).abs() as f64))));
    let mut random = SearchRandom::new(1);
    assert_eq!(operator.recreate(&[], 0, &mut random).unwrap(), Vec::<i32>::new());
    for size in 1..10 {
        let source: Vec<_> = (0..size as i32).collect();
        for amount in 0..=size {
            let mut result = operator.recreate(&source, amount, &mut random).unwrap(); result.sort(); assert_eq!(result, source);
        }
    }
    let missing = RuinRecreateOperator::new(LostRuin, GreedyInsertRepair::new(urgent_cost));
    assert!(missing.recreate(&[0, 1], 1, &mut random).is_err());
    let replaced = RuinRecreateOperator::new(RandomRuin, LostRepair);
    assert!(replaced.recreate(&[0, 1], 1, &mut random).is_err());
}
