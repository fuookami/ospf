use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CuttingPlanProductionDTO {
    pub name: String,
    pub x: f64,
    pub width: f64,
    pub production_type: String,
    #[serde(default)]
    pub info: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CuttingPlanDTO {
    pub group: Vec<String>,
    pub productions: Vec<CuttingPlanProductionDTO>,
    pub width: f64,
    pub standard_width: f64,
    pub amount: u64,
    #[serde(default)]
    pub info: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDTO {
    #[serde(default)]
    pub kpi: BTreeMap<String, String>,
    pub cutting_plans: Vec<CuttingPlanDTO>,
}
