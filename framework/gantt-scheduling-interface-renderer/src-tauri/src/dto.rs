use crate::serializer::*;
use chrono::NaiveDateTime;
use ospf_renderer_contract::{optional_string_map, string_map};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubTaskDTO {
    pub name: String,
    #[serde(default = "normal_category")]
    pub category: String,
    #[serde(
        default,
        alias = "start_time",
        serialize_with = "optional_naive_date_time_to_str",
        deserialize_with = "optional_naive_date_time_from_str"
    )]
    pub start_time: Option<NaiveDateTime>,
    #[serde(
        default,
        alias = "end_time",
        serialize_with = "optional_naive_date_time_to_str",
        deserialize_with = "optional_naive_date_time_from_str"
    )]
    pub end_time: Option<NaiveDateTime>,
    #[serde(default, deserialize_with = "string_map")]
    pub info: BTreeMap<String, String>,
}

fn normal_category() -> String {
    "Normal".into()
}

fn optional_label<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    match Option::<Value>::deserialize(deserializer)? {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        Some(Value::Number(value)) => Ok(Some(value.to_string())),
        _ => Err(serde::de::Error::custom(
            "expected a string, number or null",
        )),
    }
}

#[derive(Deserialize, Debug)]
struct TimeRangeDTO {
    #[serde(deserialize_with = "optional_naive_date_time_from_str")]
    start: Option<NaiveDateTime>,
    #[serde(deserialize_with = "optional_naive_date_time_from_str")]
    end: Option<NaiveDateTime>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TaskInput {
    name: Option<String>,
    #[serde(default = "normal_category")]
    category: String,
    executor: String,
    #[serde(default, deserialize_with = "optional_label")]
    order: Option<String>,
    produce: Option<String>,
    #[serde(default, deserialize_with = "optional_string_map")]
    products: Option<BTreeMap<String, String>>,
    #[serde(default, deserialize_with = "optional_string_map")]
    materials: Option<BTreeMap<String, String>>,
    #[serde(default, deserialize_with = "string_map")]
    consumption: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "string_map")]
    resources: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "string_map")]
    info: BTreeMap<String, String>,
    #[serde(
        default,
        alias = "scheduled_start_time",
        deserialize_with = "optional_naive_date_time_from_str"
    )]
    scheduled_start_time: Option<NaiveDateTime>,
    #[serde(
        default,
        alias = "scheduled_end_time",
        deserialize_with = "optional_naive_date_time_from_str"
    )]
    scheduled_end_time: Option<NaiveDateTime>,
    #[serde(
        default,
        alias = "start_time",
        deserialize_with = "optional_naive_date_time_from_str"
    )]
    start_time: Option<NaiveDateTime>,
    #[serde(
        default,
        alias = "end_time",
        deserialize_with = "optional_naive_date_time_from_str"
    )]
    end_time: Option<NaiveDateTime>,
    #[serde(default, alias = "actual_time")]
    actual_time: Option<TimeRangeDTO>,
    #[serde(default, alias = "scheduled_time")]
    scheduled_time: Option<TimeRangeDTO>,
    #[serde(default, alias = "sub_tasks")]
    sub_tasks: Vec<SubTaskDTO>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", try_from = "TaskInput")]
pub struct TaskDTO {
    pub name: String,
    pub category: String,
    pub executor: String,
    pub order: Option<String>,
    pub produce: Option<String>,
    pub products: Option<BTreeMap<String, String>>,
    pub materials: Option<BTreeMap<String, String>>,
    pub consumption: BTreeMap<String, String>,
    pub resources: BTreeMap<String, String>,
    pub info: BTreeMap<String, String>,
    #[serde(serialize_with = "optional_naive_date_time_to_str")]
    pub scheduled_start_time: Option<NaiveDateTime>,
    #[serde(serialize_with = "optional_naive_date_time_to_str")]
    pub scheduled_end_time: Option<NaiveDateTime>,
    #[serde(serialize_with = "optional_naive_date_time_to_str")]
    pub start_time: Option<NaiveDateTime>,
    #[serde(serialize_with = "optional_naive_date_time_to_str")]
    pub end_time: Option<NaiveDateTime>,
    pub sub_tasks: Vec<SubTaskDTO>,
}

fn time_range(
    start: Option<NaiveDateTime>,
    end: Option<NaiveDateTime>,
) -> Result<Option<(NaiveDateTime, NaiveDateTime)>, String> {
    match (start, end) {
        (None, None) => Ok(None),
        (Some(start), Some(end)) if start <= end => Ok(Some((start, end))),
        _ => Err("time range must have both endpoints and start <= end".into()),
    }
}

impl TryFrom<TaskInput> for TaskDTO {
    type Error = String;

    fn try_from(input: TaskInput) -> Result<Self, Self::Error> {
        let from_range = |range: Option<TimeRangeDTO>| match range {
            Some(range) => time_range(range.start, range.end),
            None => Ok(None),
        };
        let scheduled = time_range(input.scheduled_start_time, input.scheduled_end_time)?
            .or(from_range(input.scheduled_time)?);
        let actual =
            time_range(input.start_time, input.end_time)?.or(from_range(input.actual_time)?);
        // 未排期任务保留空时间，前端不伪造排程。 / Keep unscheduled tasks untimed; never fabricate a schedule.
        let visible = actual.or(scheduled);
        for task in &input.sub_tasks {
            time_range(task.start_time, task.end_time)?;
        }
        Ok(Self {
            name: input
                .name
                .or_else(|| input.produce.clone())
                .unwrap_or_else(|| {
                    format!(
                        "{} #{}",
                        input.executor,
                        input.order.as_deref().unwrap_or("?")
                    )
                }),
            category: input.category,
            executor: input.executor,
            order: input.order,
            produce: input.produce,
            products: input.products,
            materials: input.materials,
            consumption: input.consumption,
            resources: input.resources,
            info: input.info,
            scheduled_start_time: scheduled.map(|range| range.0),
            scheduled_end_time: scheduled.map(|range| range.1),
            start_time: visible.map(|range| range.0),
            end_time: visible.map(|range| range.1),
            sub_tasks: input.sub_tasks,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDTO {
    pub tasks: Vec<TaskDTO>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kotlin_fields_and_consumption_roundtrip() {
        let schema: SchemaDTO = serde_json::from_value(json!({"tasks": [{
            "name": "A", "category": "Normal", "executor": "machine", "order": "order-a",
            "startTime": "2026-10-24 08:00:00", "endTime": "2026-10-24 09:00:00",
            "consumption": {"steel": "2"}, "resources": {"operator": "Alice"}, "products": {"a": "1"},
            "subTasks": [{"name": "setup", "startTime": "2026-10-24 08:00:00", "endTime": "2026-10-24 08:10:00"}]
        }]})).unwrap();
        let output = serde_json::to_value(&schema).unwrap();
        assert_eq!(output["tasks"][0]["consumption"]["steel"], "2");
        assert_eq!(output["tasks"][0]["subTasks"][0]["info"], json!({}));
        assert_eq!(output["tasks"][0]["order"], "order-a");
        serde_json::from_value::<SchemaDTO>(output).unwrap();
    }

    #[test]
    fn rust_ranges_lists_tuple_times_and_nulls() {
        let start = json!([2026, 297, 8, 0, 0, 123456789, 8, 0, 0]);
        let end = json!([2026, 297, 9, 0, 0, 123456789, 8, 0, 0]);
        let schema: SchemaDTO = serde_json::from_value(json!({"tasks": [{
            "executor": "machine", "order": 0, "produce": "A", "products": ["A", "A"],
            "consumption": ["steel"], "resources": ["Alice"], "scheduled_time": null,
            "actual_time": {"start": start, "end": end}, "sub_tasks": [{
                "name": "setup", "category": "Testing", "start_time": start, "end_time": end, "info": "note"
            }]
        }]})).unwrap();
        let output = serde_json::to_value(schema).unwrap();
        let task = &output["tasks"][0];
        assert_eq!(task["name"], "A");
        assert_eq!(task["startTime"], "2026-10-24 00:00:00.123456789");
        assert_eq!(task["products"]["1"], "A");
        assert_eq!(task["order"], "0");
        assert_eq!(task["subTasks"][0]["info"]["info"], "note");
    }

    #[test]
    fn scheduled_only_unscheduled_and_iso_offsets() {
        let schema: SchemaDTO = serde_json::from_value(json!({"tasks": [
            {"executor": "M", "order": 2, "scheduled_time": {"start": "2026-10-24T08:00:00+08:00", "end": "2026-10-24T09:00:00+08:00"}},
            {"executor": "M", "order": 3, "actual_time": null, "scheduled_time": null, "sub_tasks": []}
        ]})).unwrap();
        assert_eq!(
            schema.tasks[0].start_time,
            schema.tasks[0].scheduled_start_time
        );
        assert_eq!(schema.tasks[0].start_time.unwrap().hour(), 0);
        assert!(schema.tasks[1].start_time.is_none());
        assert_eq!(schema.tasks[1].name, "M #3");
    }

    #[test]
    fn rejects_invalid_ranges_and_dates() {
        for range in [
            json!({"start": "wrong", "end": "wrong"}),
            json!({"start": "2026-10-24T09:00:00Z", "end": "2026-10-24T08:00:00Z"}),
            json!({"start": [2026, 297, -1, 0, 0, 0, 0, 0, 0], "end": null}),
        ] {
            assert!(serde_json::from_value::<SchemaDTO>(
                json!({"tasks": [{"executor": "M", "actual_time": range}]})
            )
            .is_err());
        }
    }

    use chrono::Timelike;
}
