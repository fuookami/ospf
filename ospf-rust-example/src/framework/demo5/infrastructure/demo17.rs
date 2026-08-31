//! Demo17 可复现 fixture / Reproducible Demo17 fixtures.

use std::sync::Arc;

use ospf_rust_framework_network_scheduling::domain::vrp::VrptwInstance;

use super::adapter::instance_from_solomon;
use super::dto::{SolomonData, SolomonNodeData, SolomonVehicleData};

const DEMO17_CUSTOMERS: &[(f64, f64, f64, f64, f64, f64)] = &[
    (45.0, 68.0, 10.0, 912.0, 967.0, 90.0),
    (45.0, 70.0, 30.0, 825.0, 870.0, 90.0),
    (42.0, 66.0, 10.0, 65.0, 146.0, 90.0),
    (42.0, 68.0, 10.0, 727.0, 782.0, 90.0),
    (42.0, 65.0, 10.0, 15.0, 67.0, 90.0),
    (40.0, 69.0, 20.0, 621.0, 702.0, 90.0),
    (40.0, 66.0, 20.0, 170.0, 225.0, 90.0),
    (38.0, 68.0, 20.0, 255.0, 324.0, 90.0),
    (38.0, 70.0, 10.0, 534.0, 605.0, 90.0),
    (35.0, 66.0, 10.0, 357.0, 410.0, 90.0),
    (35.0, 69.0, 10.0, 448.0, 505.0, 90.0),
    (25.0, 85.0, 20.0, 652.0, 721.0, 90.0),
    (22.0, 75.0, 30.0, 30.0, 92.0, 90.0),
    (22.0, 85.0, 10.0, 567.0, 620.0, 90.0),
    (20.0, 80.0, 40.0, 384.0, 429.0, 90.0),
    (20.0, 85.0, 40.0, 475.0, 528.0, 90.0),
    (18.0, 75.0, 20.0, 99.0, 148.0, 90.0),
    (15.0, 75.0, 20.0, 179.0, 254.0, 90.0),
    (15.0, 80.0, 10.0, 278.0, 345.0, 90.0),
    (30.0, 50.0, 10.0, 10.0, 73.0, 90.0),
    (30.0, 52.0, 20.0, 914.0, 965.0, 90.0),
    (28.0, 52.0, 20.0, 812.0, 883.0, 90.0),
    (28.0, 55.0, 10.0, 732.0, 777.0, 90.0),
    (25.0, 50.0, 10.0, 65.0, 144.0, 90.0),
    (25.0, 52.0, 40.0, 169.0, 224.0, 90.0),
    (25.0, 55.0, 10.0, 622.0, 701.0, 90.0),
    (23.0, 52.0, 10.0, 261.0, 316.0, 90.0),
    (23.0, 55.0, 20.0, 546.0, 593.0, 90.0),
    (20.0, 50.0, 10.0, 358.0, 405.0, 90.0),
    (20.0, 55.0, 10.0, 449.0, 504.0, 90.0),
    (10.0, 35.0, 20.0, 200.0, 237.0, 90.0),
    (10.0, 40.0, 30.0, 31.0, 100.0, 90.0),
    (8.0, 40.0, 40.0, 87.0, 158.0, 90.0),
    (8.0, 45.0, 20.0, 751.0, 816.0, 90.0),
    (5.0, 35.0, 10.0, 283.0, 344.0, 90.0),
    (5.0, 45.0, 10.0, 665.0, 716.0, 90.0),
    (2.0, 40.0, 20.0, 383.0, 434.0, 90.0),
    (0.0, 40.0, 30.0, 479.0, 522.0, 90.0),
    (0.0, 45.0, 20.0, 567.0, 624.0, 90.0),
    (35.0, 30.0, 10.0, 264.0, 321.0, 90.0),
    (35.0, 32.0, 10.0, 166.0, 235.0, 90.0),
    (33.0, 32.0, 20.0, 68.0, 149.0, 90.0),
    (33.0, 35.0, 10.0, 16.0, 80.0, 90.0),
    (32.0, 30.0, 10.0, 359.0, 412.0, 90.0),
    (30.0, 30.0, 10.0, 541.0, 600.0, 90.0),
    (30.0, 32.0, 30.0, 448.0, 509.0, 90.0),
    (30.0, 35.0, 10.0, 1054.0, 1127.0, 90.0),
    (28.0, 30.0, 10.0, 632.0, 693.0, 90.0),
    (28.0, 35.0, 10.0, 1001.0, 1066.0, 90.0),
    (26.0, 32.0, 10.0, 815.0, 880.0, 90.0),
    (25.0, 30.0, 10.0, 725.0, 786.0, 90.0),
    (25.0, 35.0, 10.0, 912.0, 969.0, 90.0),
    (44.0, 5.0, 20.0, 286.0, 347.0, 90.0),
    (42.0, 10.0, 40.0, 186.0, 257.0, 90.0),
    (42.0, 15.0, 10.0, 95.0, 158.0, 90.0),
    (40.0, 5.0, 30.0, 385.0, 436.0, 90.0),
    (40.0, 15.0, 40.0, 35.0, 87.0, 90.0),
    (38.0, 5.0, 30.0, 471.0, 534.0, 90.0),
    (38.0, 15.0, 10.0, 651.0, 740.0, 90.0),
    (35.0, 5.0, 20.0, 562.0, 629.0, 90.0),
    (50.0, 30.0, 10.0, 531.0, 610.0, 90.0),
    (50.0, 35.0, 20.0, 262.0, 317.0, 90.0),
    (50.0, 40.0, 50.0, 171.0, 218.0, 90.0),
    (48.0, 30.0, 10.0, 632.0, 693.0, 90.0),
    (48.0, 40.0, 10.0, 76.0, 129.0, 90.0),
    (47.0, 35.0, 10.0, 826.0, 875.0, 90.0),
    (47.0, 40.0, 10.0, 12.0, 77.0, 90.0),
    (45.0, 30.0, 10.0, 734.0, 777.0, 90.0),
    (45.0, 35.0, 10.0, 916.0, 969.0, 90.0),
    (95.0, 30.0, 30.0, 387.0, 456.0, 90.0),
    (95.0, 35.0, 20.0, 293.0, 360.0, 90.0),
    (53.0, 30.0, 10.0, 450.0, 505.0, 90.0),
    (92.0, 30.0, 10.0, 478.0, 551.0, 90.0),
    (53.0, 35.0, 50.0, 353.0, 412.0, 90.0),
    (45.0, 65.0, 20.0, 997.0, 1068.0, 90.0),
    (90.0, 35.0, 10.0, 203.0, 260.0, 90.0),
    (88.0, 30.0, 10.0, 574.0, 643.0, 90.0),
    (88.0, 35.0, 20.0, 109.0, 170.0, 90.0),
    (87.0, 30.0, 10.0, 668.0, 731.0, 90.0),
    (85.0, 25.0, 10.0, 769.0, 820.0, 90.0),
    (85.0, 35.0, 30.0, 47.0, 124.0, 90.0),
    (75.0, 55.0, 20.0, 369.0, 420.0, 90.0),
    (72.0, 55.0, 10.0, 265.0, 338.0, 90.0),
    (70.0, 58.0, 20.0, 458.0, 523.0, 90.0),
    (68.0, 60.0, 30.0, 555.0, 612.0, 90.0),
    (66.0, 55.0, 10.0, 173.0, 238.0, 90.0),
    (65.0, 55.0, 20.0, 85.0, 144.0, 90.0),
    (65.0, 60.0, 30.0, 645.0, 708.0, 90.0),
    (63.0, 58.0, 10.0, 737.0, 802.0, 90.0),
    (60.0, 55.0, 10.0, 20.0, 84.0, 90.0),
    (60.0, 60.0, 10.0, 836.0, 889.0, 90.0),
    (67.0, 85.0, 20.0, 368.0, 441.0, 90.0),
    (65.0, 85.0, 40.0, 475.0, 518.0, 90.0),
    (65.0, 82.0, 10.0, 285.0, 336.0, 90.0),
    (62.0, 80.0, 30.0, 196.0, 239.0, 90.0),
    (60.0, 80.0, 10.0, 95.0, 156.0, 90.0),
    (60.0, 85.0, 30.0, 561.0, 622.0, 90.0),
    (58.0, 75.0, 20.0, 30.0, 84.0, 90.0),
    (55.0, 80.0, 10.0, 743.0, 820.0, 90.0),
    (55.0, 85.0, 20.0, 647.0, 726.0, 90.0),
];

/// 构造指定客户前缀的 Demo17 Solomon 数据 / Build a Demo17 Solomon data prefix.
pub(crate) fn data(customer_count: usize) -> Result<SolomonData, String> {
    if !(1..=DEMO17_CUSTOMERS.len()).contains(&customer_count) {
        return Err(format!(
            "Demo17 客户数量必须在 1..={} 内 / Demo17 customer count must be in 1..={}",
            DEMO17_CUSTOMERS.len(),
            DEMO17_CUSTOMERS.len()
        ));
    }
    let depot = SolomonNodeData {
        id: "0".to_owned(),
        x: 40.0,
        y: 50.0,
        demand: 0.0,
        ready_time: 0.0,
        due_time: 1236.0,
        service_time: 0.0,
    };
    let customers = DEMO17_CUSTOMERS
        .iter()
        .take(customer_count)
        .enumerate()
        .map(
            |(index, &(x, y, demand, ready_time, due_time, service_time))| SolomonNodeData {
                id: (index + 1).to_string(),
                x,
                y,
                demand,
                ready_time,
                due_time,
                service_time,
            },
        )
        .collect();
    Ok(SolomonData {
        name: format!("demo17-{customer_count}"),
        vehicle: SolomonVehicleData {
            amount: 25,
            capacity: 200.0,
        },
        depot,
        customers,
    })
}

/// 构造 Demo17 VRPTW 实例 / Build a Demo17 VRPTW instance.
pub(crate) fn instance(customer_count: usize) -> Result<Arc<VrptwInstance<f64>>, String> {
    let data = data(customer_count)?;
    let mut instance = Arc::try_unwrap(instance_from_solomon(&data)?).map_err(|_| {
        "Demo17 实例所有权转换失败 / failed to obtain unique Demo17 instance ownership".to_owned()
    })?;
    if let Some(vehicle_type) = instance.vehicle_types.first_mut() {
        vehicle_type.fixed_cost.value = 500.0;
    }
    Ok(Arc::new(instance))
}

/// 构造前 25 个客户的 Demo17 实例 / Build the first-25-customer Demo17 instance.
pub fn first25_instance() -> Result<Arc<VrptwInstance<f64>>, String> {
    instance(25)
}

/// 构造用于全路线 oracle 的小型 Demo17 实例 / Build the small Demo17 instance used by the full-route oracle.
pub fn small_instance() -> Result<Arc<VrptwInstance<f64>>, String> {
    instance(5)
}

/// 构造全部 100 个客户的 Demo17 实例 / Build the all-100-customer Demo17 instance.
pub fn all100_instance() -> Result<Arc<VrptwInstance<f64>>, String> {
    instance(100)
}

/// 构造用于严格最优性证明的 100 客户 fixture / Build a 100-customer fixture for a strict optimality proof.
///
/// 每个客户需求为 1、车辆容量为 1，因此任何可行解都必须使用 100 条路线；
/// 所有节点重合使行驶成本为零，单车固定成本为 1，数学最优值固定为 100。
/// Every customer has demand 1 and every vehicle has capacity 1, so every feasible
/// solution must use 100 routes. Coincident coordinates make travel cost zero and
/// the fixed cost is 1, giving a known mathematical optimum of 100.
pub fn proof100_instance() -> Result<Arc<VrptwInstance<f64>>, String> {
    let depot = SolomonNodeData {
        id: "0".to_owned(),
        x: 0.0,
        y: 0.0,
        demand: 0.0,
        ready_time: 0.0,
        due_time: 100.0,
        service_time: 0.0,
    };
    let customers = (1..=100)
        .map(|id| SolomonNodeData {
            id: id.to_string(),
            x: 0.0,
            y: 0.0,
            demand: 1.0,
            ready_time: 0.0,
            due_time: 100.0,
            service_time: 0.0,
        })
        .collect();
    let data = SolomonData {
        name: "demo17-proof-100".to_owned(),
        vehicle: SolomonVehicleData {
            amount: 100,
            capacity: 1.0,
        },
        depot,
        customers,
    };
    let mut instance = Arc::try_unwrap(instance_from_solomon(&data)?).map_err(|_| {
        "证明 fixture 所有权转换失败 / failed to obtain unique proof fixture ownership".to_owned()
    })?;
    if let Some(vehicle_type) = instance.vehicle_types.first_mut() {
        vehicle_type.fixed_cost.value = 1.0;
    }
    Ok(Arc::new(instance))
}

#[cfg(test)]
mod tests {
    use super::{all100_instance, data, first25_instance, proof100_instance};

    #[test]
    fn demo17_fixture_preserves_25_and_100_customer_prefixes() {
        let first25 = first25_instance().expect("first 25 Demo17 customers");
        let all100 = all100_instance().expect("all 100 Demo17 customers");
        assert_eq!(first25.customers.len(), 25);
        assert_eq!(all100.customers.len(), 100);
        assert_eq!(first25.customers[0].demand.value, 10.0);
        assert_eq!(all100.customers[99].demand.value, 20.0);
        assert_eq!(first25.vehicle_types[0].amount, 25);
        assert_eq!(first25.vehicle_types[0].capacity.value, 200.0);
        assert_eq!(first25.vehicle_types[0].fixed_cost.value, 500.0);
    }

    #[test]
    fn demo17_fixture_rejects_invalid_customer_count() {
        assert!(data(0).is_err());
        assert!(data(101).is_err());
    }

    #[test]
    fn proof_fixture_has_a_known_100_route_lower_bound() {
        let instance = proof100_instance().expect("proof fixture");
        assert_eq!(instance.customers.len(), 100);
        assert_eq!(instance.vehicle_types[0].capacity.value, 1.0);
        assert_eq!(instance.vehicle_types[0].amount, 100);
        assert_eq!(instance.vehicle_types[0].fixed_cost.value, 1.0);
        assert!(instance.customers.iter().all(|customer| {
            customer.demand.value == 1.0
                && customer
                    .node
                    .payload
                    .axes
                    .values()
                    .all(|axis| axis.value == 0.0)
        }));
    }
}
