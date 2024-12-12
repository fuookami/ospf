use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct LoadingPlanItemDTO {
    name: String,
    #[serde(rename = "packageType")]
    package_type: Option<String>,
    width: f64,
    height: f64,
    depth: f64,
    x: f64,
    y: f64,
    z: f64,
    weight: f64,
    #[serde(rename = "loadingOrder")]
    loading_order: usize,
    info: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoadingPlanDTO {
    group: Vec<String>,
    #[serde(rename = "typeCode")]
    type_code: String,
    width: f64,
    height: f64,
    depth: f64,
    #[serde(rename = "loadingRate")]
    loading_rate: f64,
    weight: f64,
    volume: f64,
    items: Vec<LoadingPlanItemDTO>,
    info: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SchemaDTO {
    kpi: BTreeMap<String, String>,
    #[serde(rename = "loadingPlans")]
    loading_plans: Vec<LoadingPlanDTO>
}
