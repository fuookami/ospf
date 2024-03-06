use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::gantt::*;
use crate::serializer::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct GanttSubItemDTO {
    pub name: String,
    pub category: String,
    #[serde(
        rename = "startTime",
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub start_time: NaiveDateTime,
    #[serde(
        rename = "endTime",
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub end_time: NaiveDateTime,
}

impl From<&GanttSubItem> for GanttSubItemDTO {
    fn from(value: &GanttSubItem) -> Self {
        Self {
            name: value.name.clone(),
            category: value.category.clone(),
            start_time: value.time.start,
            end_time: value.time.end,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GanttItemInfoDTO {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GanttItemDTO {
    pub name: String,
    pub category: String,
    #[serde(rename="subItems")]
    pub sub_items: Vec<GanttSubItemDTO>,
    #[serde(
        rename = "scheduledStartTime",
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub scheduled_start_time: NaiveDateTime,
    #[serde(
        rename = "scheduledEndTime",
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub scheduled_end_time: NaiveDateTime,
    #[serde(
        rename = "startTime",
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub start_time: NaiveDateTime,
    #[serde(
        rename = "endTime",
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub end_time: NaiveDateTime,
    pub produces: Vec<GanttItemInfoDTO>,
    pub resources: Vec<GanttItemInfoDTO>,
    pub info: Vec<GanttItemInfoDTO>,
}

impl From<&GanttItem> for GanttItemDTO {
    fn from(value: &GanttItem) -> Self {
        Self {
            name: value.name.clone(),
            category: value.category.clone(),
            sub_items: value
                .sub_items
                .iter()
                .map(|sub_item| GanttSubItemDTO::from(sub_item))
                .collect(),
            scheduled_start_time: value.scheduled_time.start,
            scheduled_end_time: value.scheduled_time.end,
            start_time: value.time.start,
            end_time: value.time.end,
            produces: value
                .produces
                .iter()
                .map(|(key, value)| GanttItemInfoDTO {
                    key: key.clone(),
                    value: value.clone(),
                })
                .collect(),
            resources: value
                .resources
                .iter()
                .map(|(key, value)| GanttItemInfoDTO {
                    key: key.clone(),
                    value: value.clone(),
                })
                .collect(),
            info: value
                .info
                .iter()
                .map(|(key, value)| GanttItemInfoDTO {
                    key: key.clone(),
                    value: value.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GanttLineDTO {
    pub name: String,
    pub category: String,
    pub items: Vec<GanttItemDTO>,
}

impl From<&GanttLine> for GanttLineDTO {
    fn from(value: &GanttLine) -> Self {
        Self {
            name: value.name.clone(),
            category: value.category.clone(),
            items: value
                .items
                .iter()
                .map(|item| GanttItemDTO::from(item))
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GanttDTO {
    #[serde(
        rename = "startTime",
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub start_time: NaiveDateTime,
    #[serde(
        rename = "endTime",
        serialize_with = "native_date_time_to_str",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub end_time: NaiveDateTime,
    #[serde(rename = "linkInfo")]
    pub link_info: Vec<String>,
    pub lines: Vec<GanttLineDTO>,
}

impl From<&Gantt> for GanttDTO {
    fn from(value: &Gantt) -> Self {
        Self {
            start_time: value.time.start,
            end_time: value.time.end,
            link_info: value.link_info.clone(),
            lines: value
                .lines
                .iter()
                .map(|line| GanttLineDTO::from(line))
                .collect(),
        }
    }
}
