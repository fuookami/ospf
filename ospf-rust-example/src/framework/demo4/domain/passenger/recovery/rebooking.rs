//! 在实际恢复版本间路由旅客，避免只检查原航班时间或逐对总人数的错误。 / Route passengers between actual recovery variants instead of checking original-flight times or only pairwise totals.

use std::collections::{BTreeMap, BTreeSet};
use super::*;

/// 绑定相邻航段实际恢复版本的整数流边。 / Integer flow edge bound to actual recovery variants of adjacent legs.
#[derive(Debug, Clone)]
pub(super) struct Connection {
    next: usize,
    variable: usize,
    left: FlightTaskRecovery,
    right: FlightTaskRecovery,
}

impl Leg {
    fn flight_ids(&self) -> BTreeSet<&str> {
        std::iter::once(self.flight.as_str()).chain(self.flights.keys().map(|(flight, _)| flight.as_str())).collect()
    }

    fn variants<'a>(&self, variants: &'a [FlightTaskRecovery]) -> Vec<&'a FlightTaskRecovery> {
        let flights = self.flight_ids();
        variants.iter().filter(|activity| {
            let flight = activity.original().plan.id.as_str();
            flights.contains(flight) && activity.route() == &self.route
        }).collect()
    }

    fn mass(&self, flight: &str) -> (f64, Vec<(usize, f64)>) {
        if flight == self.flight {
            let terms = std::iter::once((self.cancel, -1.0)).chain(self.flights.values().map(|variable| (*variable, -1.0))).collect();
            (self.amount as f64, terms)
        } else {
            (0.0, self.flights.iter().filter(|((target, _), _)| target == flight).map(|(_, variable)| (*variable, 1.0)).collect())
        }
    }
}

impl RecoveryPassengerRecovery {
    pub(super) fn prepare_connections(&mut self, variants: &[FlightTaskRecovery]) -> GanttResult<()> {
        for index in 0..self.legs.len() {
            let leg = self.legs[index].clone();
            let activities = leg.variants(variants).into_iter().filter(|activity| activity.original().plan.id == leg.flight).cloned().collect();
            let terms = std::iter::once((leg.cancel, 1.0)).chain(leg.flights.values().map(|variable| (*variable, 1.0))).collect();
            self.rows.push(Row { name: format!("recovery_passenger_rebooking_od_{index}_original"),
                usage: Usage::Allowed { activities, amount: leg.amount as f64 }, terms, bound: leg.amount as f64 });
            for (target_index, target) in leg.flight_ids().into_iter().filter(|flight| *flight != leg.flight).enumerate() {
                let activities = leg.variants(variants).into_iter().filter(|activity| activity.original().plan.id == target).cloned().collect();
                let terms = leg.flights.iter().filter(|((flight, _), _)| flight == target).map(|(_, variable)| (*variable, -1.0)).collect();
                self.rows.push(Row { name: format!("recovery_passenger_rebooking_od_{index}_{target_index}"),
                    usage: Usage::Allowed { activities, amount: leg.amount as f64 }, terms, bound: 0.0 });
            }
            let Some(previous_index) = leg.previous else { continue; };
            let previous = self.legs[previous_index].clone();
            if previous.flights.is_empty() && leg.flights.is_empty() { continue; }
            let start = self.connections.len();
            for left in previous.variants(variants) {
                for right in leg.variants(variants) {
                    if left.original().plan.id == right.original().plan.id || !compatible(left, right) { continue; }
                    let variable = self.decisions.len();
                    self.decisions.push(Decision { bound: leg.amount as f64, cost: 0.0 });
                    let edge = self.connections.len() - start;
                    for (suffix, activity) in [("left", left), ("right", right)] {
                        self.rows.push(Row { name: format!("recovery_passenger_rebooking_active_{index}_{edge}_{suffix}"),
                            usage: Usage::Enabled { activity: Box::new(activity.clone()), amount: leg.amount as f64 },
                            terms: vec![(variable, -1.0)], bound: 0.0 });
                    }
                    self.connections.push(Connection { next: index, variable, left: left.clone(), right: right.clone() });
                }
            }
            // 先前航段允许在到达后取消，本航段每位继续旅行的旅客必须有真实可行的前序连接。 / A previous leg permits cancellation after arrival; every continuing passenger needs a real feasible predecessor connection.
            for (option, flight) in previous.flight_ids().into_iter().enumerate() {
                let (constant, mut terms) = previous.mass(flight);
                terms.extend(self.connections[start..].iter().filter(|edge| edge.left.original().plan.id == flight).map(|edge| (edge.variable, -1.0)));
                self.rows.push(Row { name: format!("recovery_passenger_rebooking_outgoing_{index}_{option}"),
                    usage: Usage::None, terms, bound: -constant });
            }
            for (option, flight) in leg.flight_ids().into_iter().enumerate() {
                let (constant, mut terms) = leg.mass(flight);
                terms.extend(self.connections[start..].iter().filter(|edge| edge.right.original().plan.id == flight).map(|edge| (edge.variable, -1.0)));
                self.rows.push(Row { name: format!("recovery_passenger_rebooking_incoming_upper_{index}_{option}"),
                    usage: Usage::None, terms: terms.clone(), bound: -constant });
                self.rows.push(Row { name: format!("recovery_passenger_rebooking_incoming_lower_{index}_{option}"),
                    usage: Usage::None, terms: terms.into_iter().map(|(variable, coefficient)| (variable, -coefficient)).collect(), bound: constant });
            }
        }
        Ok(())
    }

    pub(super) fn check_connections(
        &self,
        activities: &BTreeMap<&str, &FlightTaskRecovery>,
        values: &[u64],
        results: &mut [RecoveryPassengerLegResult],
    ) -> GanttResult<()> {
        for (index, leg) in self.legs.iter().enumerate() {
            if served_on(&results[index], &leg.flight)? > 0
                && !activities.get(leg.flight.as_str()).is_some_and(|activity| activity.route() == &leg.route)
            { return Err(invalid("original passenger flight is absent or its actual OD differs from the declared leg")); }
            for ((flight, _), variable) in &leg.flights {
                if values[*variable] > 0 && !activities.get(flight.as_str()).is_some_and(|activity| activity.route() == &leg.route) {
                    return Err(invalid("rebooked flight is absent or its actual OD differs from the passenger leg"));
                }
            }
            let Some(previous_index) = leg.previous else { continue; };
            let previous = &self.legs[previous_index];
            if previous.flights.is_empty() && leg.flights.is_empty() { continue; }
            let mut outgoing = BTreeMap::<&str, u64>::new();
            let mut incoming = BTreeMap::<&str, u64>::new();
            for edge in self.connections.iter().filter(|edge| edge.next == index) {
                let amount = values[edge.variable];
                let left = edge.left.original().plan.id.as_str();
                let right = edge.right.original().plan.id.as_str();
                if amount > 0 && (!activities.get(left).is_some_and(|selected| same_recovery(selected, &edge.left))
                    || !activities.get(right).is_some_and(|selected| same_recovery(selected, &edge.right)) || !compatible(&edge.left, &edge.right))
                { return Err(invalid("connection flow uses an unselected or incompatible recovery variant")); }
                checked_add(outgoing.entry(left).or_default(), amount)?;
                checked_add(incoming.entry(right).or_default(), amount)?;
                checked_add(results[index].connections.entry((left.into(), right.into())).or_default(), amount)?;
            }
            for flight in previous.flight_ids() {
                if outgoing.get(flight).copied().unwrap_or(0) > served_on(&results[previous_index], flight)? {
                    return Err(invalid("connection flow exceeds actual previous-flight passenger count"));
                }
            }
            for flight in leg.flight_ids() {
                if incoming.get(flight).copied().unwrap_or(0) != served_on(&results[index], flight)? {
                    return Err(invalid("incoming connection flow does not cover actual continuing passengers"));
                }
            }
        }
        Ok(())
    }

    pub(super) fn replay_connections(
        &self,
        selected: &[RecoveryBunchCandidate],
        results: &[RecoveryPassengerLegResult],
        values: &mut [f64],
    ) -> GanttResult<()> {
        let selected = selected.iter().flat_map(|candidate| candidate.bunch.activities()).collect::<Vec<_>>();
        for (index, result) in results.iter().enumerate() {
            for ((left, right), amount) in &result.connections {
                if *amount == 0 { continue; }
                let edge = self.connections.iter().find(|edge| edge.next == index
                    && edge.left.original().plan.id == *left && edge.right.original().plan.id == *right
                    && selected.iter().any(|activity| same_recovery(activity, &edge.left))
                    && selected.iter().any(|activity| same_recovery(activity, &edge.right)))
                    .ok_or_else(|| invalid("reported connection has no actual compatible selected variant"))?;
                values[edge.variable] = *amount as f64;
            }
        }
        Ok(())
    }
}

fn served_on(result: &RecoveryPassengerLegResult, flight: &str) -> GanttResult<u64> {
    let mut amounts = result.transported.values().filter(|_| result.flight == flight)
        .chain(result.flight_changes.iter().filter(|((target, _), _)| target == flight).map(|(_, amount)| amount));
    amounts.try_fold(0_u64, |total, amount| total.checked_add(*amount).ok_or_else(|| invalid("passenger connection count overflow")))
}

fn checked_add(total: &mut u64, amount: u64) -> GanttResult<()> {
    *total = total.checked_add(amount).ok_or_else(|| invalid("passenger connection flow overflow"))?;
    Ok(())
}
