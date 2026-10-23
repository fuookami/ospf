//! 根据选中业务事件重放库存，不读取已准备或注册的累计行。 / Replay selected business events without reading prepared or registered cumulative rows.

use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_quantities::{Quantity, unit::UnitConversionValue};
use ospf_rust_core::solver::value::SolveValue;
use crate::GanttResult;
use super::compilation::ActivityPreparation;
use super::model::*;
use super::numeric::{input, sum_activity_values, zero};

/// 原域库存重放及其全部边界违反。 / Native inventory replay and all bound violations.
pub fn replay_activity_inventory<V>(
    prepared: &ActivityPreparation<V>,
    selected: &BTreeSet<ActivityChoiceKey>,
) -> GanttResult<(Vec<ActivityMaterialInventory<V>>, Vec<ActivityViolation>)>
where
    V: SolveValue + UnitConversionValue,
{
    if selected.iter().any(|key| !prepared.choices().contains_key(key)) { return Err(input("inventory replay references an unknown candidate")); }
    let problem = prepared.problem();
    let mut trails = Vec::new();
    let mut violations = Vec::new();
    for material in &problem.materials {
        let minimum = material.minimum.as_ref().map(|quantity| quantity.value.clone()).unwrap_or(zero()?);
        let outside = |value: &V| *value < minimum || material.maximum.as_ref().is_some_and(|maximum| *value > maximum.value);
        if outside(&material.initial.value) {
            violations.push(ActivityViolation { kind: ActivityViolationKind::Inventory, subject: material.id.clone(),
                time: None, message: "opening stock violates bounds before horizon-start events".into() });
        }
        let mut events = BTreeMap::<_, Vec<V>>::new();
        events.entry(problem.horizon.start).or_default();
        events.entry(problem.horizon.end).or_default();
        for event in problem.material_events.iter().chain(selected.iter().flat_map(|key| &prepared.choices()[key].material_events)) {
            if event.material == material.id { events.entry(event.time).or_default().push(event.quantity.value.clone()); }
        }
        let mut prefix = vec![material.initial.value.clone()];
        let mut levels = Vec::new();
        for (time, simultaneous) in events {
            // 同点只检查一次，并保留原始前缀，避免同点预舍入抹去跨时点抵消。
            // Check simultaneous events atomically, retaining the raw prefix to avoid premature rounding across times.
            prefix.extend(simultaneous);
            let stock = sum_activity_values(prefix.clone())?;
            if outside(&stock) {
                violations.push(ActivityViolation { kind: ActivityViolationKind::Inventory, subject: material.id.clone(),
                    time: Some(time), message: "post-event stock violates native-domain bounds".into() });
            }
            levels.push(ActivityInventoryLevel { time, quantity: Quantity::new(stock, material.initial.unit.clone()) });
        }
        trails.push(ActivityMaterialInventory { material: material.id.clone(), initial: material.initial.clone(), levels });
    }
    Ok((trails, violations))
}
