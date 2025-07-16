use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoadingPlanItemDTO {
    name: String,
    package_type: Option<String>,
    width: f64,
    height: f64,
    depth: f64,
    x: f64,
    y: f64,
    z: f64,
    weight: f64,
    loading_order: usize,
    info: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoadingPlanDTO {
    group: Vec<String>,
    name: String,
    type_code: String,
    width: f64,
    height: f64,
    depth: f64,
    loading_rate: f64,
    weight: f64,
    volume: f64,
    items: Vec<LoadingPlanItemDTO>,
    info: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDTO {
    kpi: BTreeMap<String, String>,
    loading_plans: Vec<LoadingPlanDTO>
}
