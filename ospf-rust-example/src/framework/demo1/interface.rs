//! 接口层：框架示例1 入口与数据解析 / Interface layer: framework demo1 entry point and data parsing

use crate::framework::demo1::application::Ssp;
use crate::framework::demo1::infrastructure::dto::{ClientNodeDTO, EdgeDTO, Input};
use std::error::Error;

const DATA: &str = r#"28 45 12
100
0 16 8 2
0 26 13 2
0 9 14 2
0 8 36 2
0 7 25 2
0 6 13 2
0 1 20 1
0 2 16 1
0 3 13 1
1 19 26 2
1 18 31 2
1 16 24 2
1 15 16 2
1 2 4 1
1 3 11 1
2 4 37 2
2 25 24 2
2 21 5 2
2 20 2 2
2 3 7 1
3 19 24 2
3 24 17 2
3 27 26 2
4 5 26 1
4 6 12 1
5 6 14 1
8 21 36 5
9 10 6 1
9 11 14 1
10 26 11 5
10 11 9 1
12 13 15 1
12 14 9 1
12 15 12 1
13 14 11 1
13 15 27 1
14 15 19 1
17 18 22 1
21 22 22 1
21 23 18 1
21 24 14 1
22 23 23 1
22 24 11 1
23 24 23 1
26 27 19 1
0 8 40
1 11 13
2 22 28
3 3 45
4 17 11
5 19 26
6 16 15
7 13 13
8 5 18
9 25 15
10 7 10
11 24 23"#;

/// 运行框架示例1入口 / Run framework demo1 entry point
pub fn run() -> Result<(), Box<dyn Error>> {
    let input = read(DATA)?;
    let mut app = Ssp::new();
    let output = app.solve(input)?;

    println!("=== Framework Demo1 ===");
    for link in output.links {
        println!("{:?}", link);
    }
    Ok(())
}

/// 从原始数据解析输入 / Parse input from raw data
fn read(data: &str) -> Result<Input, Box<dyn Error>> {
    let lines: Vec<&str> = data
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.len() < 2 {
        return Err(String::from("invalid demo1 data").into());
    }

    let first: Vec<&str> = lines[0].split_whitespace().collect();
    if first.len() != 3 {
        return Err(String::from("invalid first line").into());
    }
    let normal_node_amount = first[0].parse::<usize>()?;
    let edge_amount = first[1].parse::<usize>()?;
    let client_node_amount = first[2].parse::<usize>()?;
    let service_cost = lines[1].parse::<u64>()?;

    let mut edges = Vec::with_capacity(edge_amount);
    for line in lines.iter().skip(2).take(edge_amount) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 4 {
            return Err(format!("invalid edge line: {line}").into());
        }
        edges.push(EdgeDTO {
            from_node_id: fields[0].parse::<u64>()?,
            to_node_id: fields[1].parse::<u64>()?,
            max_bandwidth: fields[2].parse::<u64>()?,
            cost_per_bandwidth: fields[3].parse::<u64>()?,
        });
    }

    let mut client_nodes = Vec::with_capacity(client_node_amount);
    for line in lines.iter().skip(2 + edge_amount).take(client_node_amount) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 3 {
            return Err(format!("invalid client line: {line}").into());
        }
        client_nodes.push(ClientNodeDTO {
            id: fields[0].parse::<u64>()?,
            normal_node_id: fields[1].parse::<u64>()?,
            demand: fields[2].parse::<u64>()?,
        });
    }

    Ok(Input {
        service_cost,
        normal_node_amount,
        edges,
        client_nodes,
    })
}
