//! Solomon 文本解析器 / Solomon text parser.

use std::collections::HashSet;

use super::dto::{SolomonData, SolomonNodeData, SolomonVehicleData};

/// 解析标准 Solomon 文本 / Parse standard Solomon text.
pub fn parse_solomon(text: &str) -> Result<SolomonData, String> {
    let lines = text
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err(failure("输入为空", "input is empty"));
    }

    let vehicle_header = lines
        .iter()
        .position(|(_, line)| line.eq_ignore_ascii_case("VEHICLE"));
    let customer_header = lines
        .iter()
        .position(|(_, line)| line.eq_ignore_ascii_case("CUSTOMER"));
    let (Some(vehicle_header), Some(customer_header)) = (vehicle_header, customer_header) else {
        return Err(failure(
            "缺少 VEHICLE 或 CUSTOMER 段",
            "VEHICLE or CUSTOMER section is missing",
        ));
    };
    if customer_header <= vehicle_header {
        return Err(failure(
            "CUSTOMER 段必须位于 VEHICLE 段之后",
            "CUSTOMER section must follow VEHICLE section",
        ));
    }

    let vehicle_line = lines[vehicle_header + 1..customer_header]
        .iter()
        .find_map(|(number, line)| {
            let fields = fields(line);
            if fields.len() != 2 {
                return None;
            }
            let amount = fields[0].parse::<usize>().ok()?;
            let capacity = fields[1].parse::<f64>().ok()?;
            Some((*number, amount, capacity))
        })
        .ok_or_else(|| {
            failure(
                "VEHICLE 段缺少车辆数量和容量",
                "vehicle amount and capacity are missing",
            )
        })?;
    if vehicle_line.1 == 0 || !vehicle_line.2.is_finite() || vehicle_line.2 <= 0.0 {
        return Err(line_failure(
            vehicle_line.0,
            "车辆数量和容量必须为正数",
            "vehicle amount and capacity must be positive",
        ));
    }

    let mut nodes = Vec::new();
    for (line_number, line) in lines.iter().skip(customer_header + 1) {
        let columns = fields(line);
        if columns
            .first()
            .and_then(|value| value.parse::<f64>().ok())
            .is_none()
        {
            continue;
        }
        if columns.len() != 7 {
            return Err(line_failure(
                *line_number,
                &format!("客户数据必须恰好包含 7 列，实际为 {} 列", columns.len()),
                &format!(
                    "customer data must contain exactly 7 columns, got {}",
                    columns.len()
                ),
            ));
        }
        let values = columns[1..]
            .iter()
            .map(|value| value.parse::<f64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                line_failure(
                    *line_number,
                    "客户数据包含非法数值",
                    "customer data contains an invalid number",
                )
            })?;
        if values.iter().any(|value| !value.is_finite())
            || values[2] < 0.0
            || values[3] < 0.0
            || values[4] < values[3]
            || values[5] < 0.0
        {
            return Err(line_failure(
                *line_number,
                "需求、时间窗或服务时间非法",
                "demand, time window, or service time is invalid",
            ));
        }
        nodes.push(SolomonNodeData {
            id: columns[0].to_owned(),
            x: values[0],
            y: values[1],
            demand: values[2],
            ready_time: values[3],
            due_time: values[4],
            service_time: values[5],
        });
    }
    if nodes.is_empty() {
        return Err(failure(
            "CUSTOMER 段没有节点数据",
            "CUSTOMER section contains no node data",
        ));
    }
    let mut ids = HashSet::new();
    for node in &nodes {
        if !ids.insert(node.id.clone()) {
            return Err(failure(
                &format!("节点 ID {} 重复", node.id),
                &format!("duplicate node ID {}", node.id),
            ));
        }
    }
    let depot = nodes
        .iter()
        .find(|node| node.id == "0")
        .cloned()
        .ok_or_else(|| {
            failure(
                "必须存在且只能使用 ID 为 0 的 depot",
                "exactly one depot with ID 0 is required",
            )
        })?;
    let customers = nodes
        .into_iter()
        .filter(|node| node.id != depot.id)
        .collect();
    Ok(SolomonData {
        name: lines[0].1.to_owned(),
        vehicle: SolomonVehicleData {
            amount: vehicle_line.1,
            capacity: vehicle_line.2,
        },
        depot,
        customers,
    })
}

fn fields(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

fn failure(chinese: &str, english: &str) -> String {
    format!(
        "读取 Solomon 数据失败：{} / Failed to read Solomon data: {}",
        chinese, english
    )
}

fn line_failure(line: usize, chinese: &str, english: &str) -> String {
    failure(
        &format!("第 {} 行{}", line, chinese),
        &format!("line {} {}", line, english),
    )
}

#[cfg(test)]
mod tests {
    use super::parse_solomon;

    const INPUT: &str = "C1\nVEHICLE\nNUMBER CAPACITY\n2 10\nCUSTOMER\nCUST NO. XCOORD YCOORD DEMAND READY DUE SERVICE\n0 0 0 0 0 100 0\n1 1 0 2 0 100 5\n";

    #[test]
    fn parser_keeps_the_last_customer_row() {
        let data = parse_solomon(INPUT).expect("valid Solomon fixture");
        assert_eq!(data.customers.len(), 1);
        assert_eq!(data.customers[0].id, "1");
    }

    #[test]
    fn parser_reports_column_errors_with_a_line_number() {
        let error = parse_solomon("C1\nVEHICLE\n2 10\nCUSTOMER\n1 0 0").expect_err("invalid row");
        assert!(error.contains("第 5 行"));
        assert!(error.contains("line 5"));
    }
}
