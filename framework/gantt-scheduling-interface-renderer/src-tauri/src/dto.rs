use std::collections::BTreeMap;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::serializer::*;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubTaskDTO {
    pub name: String,
    pub category: String,
    #[serde(
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub start_time: NaiveDateTime,
    #[serde(
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub end_time: NaiveDateTime,
    #[serde(default)]
    pub info: BTreeMap<String, String>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskDTO {
    pub name: String,
    pub category: String,
    #[serde(
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub start_time: NaiveDateTime,
    #[serde(
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub end_time: NaiveDateTime,
    #[serde(default)]
    pub resources: BTreeMap<String, String>,
    #[serde(default)]
    pub info: BTreeMap<String, String>,
    pub executor: String,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub produce: Option<String>,
    #[serde(default)]
    pub products: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub materials: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub consumption: BTreeMap<String, String>,
    #[serde(default)]
    #[serde(
        serialize_with = "optional_naive_date_time_to_str",
        deserialize_with = "optional_naive_date_time_from_str"
    )]
    pub scheduled_start_time: Option<NaiveDateTime>,
    #[serde(default)]
    #[serde(
        serialize_with = "optional_naive_date_time_to_str",
        deserialize_with = "optional_naive_date_time_from_str"
    )]
    pub scheduled_end_time: Option<NaiveDateTime>,
    #[serde(default)]
    pub sub_tasks: Vec<SubTaskDTO>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDTO {
    pub tasks: Vec<TaskDTO>,
}
