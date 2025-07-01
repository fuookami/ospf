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
    pub info: BTreeMap<String, String>,
    pub executor: String,
    pub order: Option<String>,
    pub produce: Option<String>,
    pub products: Option<BTreeMap<String, String>>,
    pub materials: Option<BTreeMap<String, String>>,
    pub consumption: BTreeMap<String, String>,
    #[serde(
        serialize_with = "optional_naive_date_time_to_str",
        deserialize_with = "optional_naive_date_time_from_str"
    )]
    pub scheduled_start_time: Option<NaiveDateTime>,
    #[serde(
        serialize_with = "optional_naive_date_time_to_str",
        deserialize_with = "optional_naive_date_time_from_str"
    )]
    pub scheduled_end_time: Option<NaiveDateTime>,
    pub sub_tasks: Vec<SubTaskDTO>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDTO {
    pub tasks: Vec<TaskDTO>,
}
