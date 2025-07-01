use std::collections::BTreeMap;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::serializer::*;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GanttSubItemDTO {
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
pub struct GanttItemDTO {
    pub name: String,
    pub category: String,
    #[serde(
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub scheduled_start_time: NaiveDateTime,
    #[serde(
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub scheduled_end_time: NaiveDateTime,
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
    pub produces: BTreeMap<String, String>,
    pub consumption: BTreeMap<String, String>,
    pub info: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GanttLineDTO {
    pub name: String,
    pub category: String,
    pub items: Vec<GanttItemDTO>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GanttChartDTO {
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
    pub link_info: Vec<String>,
    pub lines: Vec<GanttLineDTO>,
}
