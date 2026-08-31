//! Solomon 到 VRPTW 实例适配 / Solomon-to-VRPTW instance adapter.

use std::sync::Arc;

use ospf_rust_framework_gantt_scheduling::infrastructure::{TimeRange, TimeWindow};
use ospf_rust_framework_network_scheduling::domain::vrp::{
    Customer, CustomerId, Depot, ServiceTimeWindow, VehicleType, VehicleTypeId, VrptwInstance,
    VrptwUnits, coordinate_node,
};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{CTUnit, Kilogram, Meter};
use time::{Duration, OffsetDateTime};

use super::dto::SolomonData;

/// 将 Solomon 原始数据转换为 f64 VRPTW 实例 / Convert raw Solomon data to an f64 VRPTW instance.
pub fn instance_from_solomon(data: &SolomonData) -> Result<Arc<VrptwInstance<f64>>, String> {
    let start = OffsetDateTime::UNIX_EPOCH;
    let max_due = data
        .customers
        .iter()
        .map(|node| node.due_time + node.service_time)
        .chain(std::iter::once(data.depot.due_time))
        .fold(0.0, f64::max);
    let end = offset(start, max_due + 10_000.0)?;
    let scheduling_window = TimeWindow::seconds(TimeRange::new(start, end), 0.0, false, 1.0);
    let units = VrptwUnits::default();
    let depot_window = ServiceTimeWindow::new(start, end).map_err(|error| error.to_string())?;
    let start_node = coordinate_node(
        "solomon-start",
        data.depot.x,
        data.depot.y,
        Meter::INSTANT.clone(),
    )
    .map_err(|error| error.to_string())?;
    let end_node = coordinate_node(
        "solomon-end",
        data.depot.x,
        data.depot.y,
        Meter::INSTANT.clone(),
    )
    .map_err(|error| error.to_string())?;

    let mut customers = Vec::with_capacity(data.customers.len());
    for node in &data.customers {
        let node_id = format!("solomon-customer-{}", node.id);
        let customer_node = coordinate_node(node_id, node.x, node.y, Meter::INSTANT.clone())
            .map_err(|error| error.to_string())?;
        let service_window = ServiceTimeWindow::new(
            offset(start, node.ready_time)?,
            offset(start, node.due_time)?,
        )
        .map_err(|error| error.to_string())?;
        customers.push(
            Customer::new(
                CustomerId::from(node.id.clone()),
                customer_node,
                Quantity::new(node.demand, Kilogram::INSTANT.clone()),
                service_window,
                duration(node.service_time)?,
            )
            .map_err(|error| error.to_string())?,
        );
    }
    let vehicle = VehicleType::new(
        VehicleTypeId::from("solomon-v1"),
        Quantity::new(data.vehicle.capacity, Kilogram::INSTANT.clone()),
        Quantity::new(1.0, units.cost_unit.clone()),
        data.vehicle.amount,
    )
    .map_err(|error| error.to_string())?;
    let instance = VrptwInstance::new(
        data.name.clone(),
        Depot {
            node: start_node,
            time_window: depot_window,
        },
        Depot {
            node: end_node,
            time_window: depot_window,
        },
        customers,
        vec![vehicle],
        scheduling_window,
        units,
        Default::default(),
    )
    .map_err(|error| error.to_string())?;
    Ok(Arc::new(instance))
}

fn offset(start: OffsetDateTime, seconds: f64) -> Result<OffsetDateTime, String> {
    if !seconds.is_finite() || seconds < 0.0 || seconds > i64::MAX as f64 / 1e9 {
        return Err(
            "时间值必须是有限非负数 / time value must be finite and non-negative".to_owned(),
        );
    }
    Ok(start + Duration::nanoseconds((seconds * 1e9).round() as i64))
}

fn duration(seconds: f64) -> Result<Duration, String> {
    if !seconds.is_finite() || seconds < 0.0 || seconds > i64::MAX as f64 / 1e9 {
        return Err(
            "服务时间必须是有限非负数 / service time must be finite and non-negative".to_owned(),
        );
    }
    Ok(Duration::nanoseconds((seconds * 1e9).round() as i64))
}

#[cfg(test)]
mod tests {
    use super::instance_from_solomon;
    use crate::framework::demo5::infrastructure::{
        SolomonData, SolomonNodeData, SolomonVehicleData,
    };

    #[test]
    fn adapter_preserves_the_final_customer_and_builds_a_valid_instance() {
        let data = SolomonData {
            name: "offline-adapter".to_owned(),
            vehicle: SolomonVehicleData {
                amount: 2,
                capacity: 10.0,
            },
            depot: SolomonNodeData {
                id: "0".to_owned(),
                x: 0.0,
                y: 0.0,
                demand: 0.0,
                ready_time: 0.0,
                due_time: 100.0,
                service_time: 0.0,
            },
            customers: vec![SolomonNodeData {
                id: "1".to_owned(),
                x: 1.0,
                y: 0.0,
                demand: 1.0,
                ready_time: 0.0,
                due_time: 100.0,
                service_time: 5.0,
            }],
        };
        let instance = instance_from_solomon(&data).expect("offline adapter instance");
        assert_eq!(instance.customers.len(), 1);
        assert_eq!(instance.customers[0].id.as_str(), "1");
        assert_eq!(instance.vehicle_types[0].amount, 2);
    }
}
