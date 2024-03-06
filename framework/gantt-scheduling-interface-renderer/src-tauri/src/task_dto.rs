use chrono::NaiveDateTime;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

use super::serializer::*;

fn list_from_key_value<'de, D>(deserializer: D) -> Result<Vec<(String, String)>, D::Error>
    where
        D: Deserializer<'de>,
{
    let m: HashMap<String, String> = Deserialize::deserialize(deserializer)?;
    let v = m
        .iter()
        .map(|entry| (entry.0.to_owned(), entry.1.to_owned()))
        .collect();
    Ok(v)
}

fn optional_list_from_key_value<'de, D>(deserializer: D) -> Result<Option<Vec<(String, String)>>, D::Error>
    where
        D: Deserializer<'de>,
{
    let m: Option<HashMap<String, String>> = Deserialize::deserialize(deserializer)?;
    Ok(m.map(|m| m.iter().map(|entry| (entry.0.to_owned(), entry.1.to_owned())).collect()))
}

pub trait TaskBaseDTO: Sized {
    fn name(&self) -> &str;

    fn category(&self) -> &str;
    fn start_time(&self) -> NaiveDateTime;
    fn end_time(&self) -> NaiveDateTime;
    fn info(&self) -> &[(String, String)];
}

#[derive(Deserialize, Debug)]
pub struct SubTaskDTO {
    pub name: String,
    pub category: String,
    #[serde(
        rename = "startTime",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub start_time: NaiveDateTime,
    #[serde(
        rename = "endTime",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub end_time: NaiveDateTime,
    #[serde(default)]
    #[serde(deserialize_with = "list_from_key_value")]
    pub info: Vec<(String, String)>,
}

pub trait TaskDTO: TaskBaseDTO {
    type SubTask: TaskBaseDTO;

    fn executor(&self) -> &str;
    fn order(&self) -> Option<&str>;
    fn produce(&self) -> Option<&str>;
    fn products(&self) -> Option<&[(String, String)]>;
    fn materials(&self) -> Option<&[(String, String)]>;
    fn resources(&self) -> &[(String, String)];
    fn scheduled_start_time(&self) -> Option<NaiveDateTime>;
    fn scheduled_end_time(&self) -> Option<NaiveDateTime>;
    fn sub_tasks(&self) -> &[Self::SubTask];

    fn group(tasks: &[Self]) -> HashMap<String, Vec<&Self>> {
        let mut task_groups = HashMap::new();
        for i in 0..tasks.len() {
            task_groups
                .entry(String::from(tasks[i].executor()))
                .and_modify(|entry: &mut Vec<_>| entry.push(&tasks[i]))
                .or_insert(vec![&tasks[i]]);
        }
        task_groups
    }
}

#[derive(Deserialize, Debug)]
pub struct NormalTaskDTO {
    pub name: String,
    pub category: String,
    pub executor: String,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub produce: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "optional_list_from_key_value")]
    pub products: Option<Vec<(String, String)>>,
    #[serde(default)]
    #[serde(deserialize_with = "optional_list_from_key_value")]
    pub materials: Option<Vec<(String, String)>>,
    #[serde(default)]
    #[serde(deserialize_with = "list_from_key_value")]
    pub resources: Vec<(String, String)>,
    #[serde(default)]
    #[serde(
        rename = "scheduledStartTime",
        deserialize_with = "optional_naive_date_time_from_str"
    )]
    pub scheduled_start_time: Option<NaiveDateTime>,
    #[serde(default)]
    #[serde(
        rename = "scheduledEndTime",
        deserialize_with = "optional_naive_date_time_from_str"
    )]
    pub scheduled_end_time: Option<NaiveDateTime>,
    #[serde(
        rename = "startTime",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub start_time: NaiveDateTime,
    #[serde(
        rename = "endTime",
        deserialize_with = "naive_date_time_from_str"
    )]
    pub end_time: NaiveDateTime,
    #[serde(default)]
    #[serde(deserialize_with = "list_from_key_value")]
    pub info: Vec<(String, String)>,
    #[serde(default)]
    pub sub_tasks: Vec<SubTaskDTO>,
}

impl TaskBaseDTO for SubTaskDTO {
    fn name(&self) -> &str {
        return &self.name;
    }

    fn category(&self) -> &str {
        return &self.category;
    }

    fn start_time(&self) -> NaiveDateTime {
        return self.start_time;
    }

    fn end_time(&self) -> NaiveDateTime {
        return self.end_time;
    }

    fn info(&self) -> &[(String, String)] {
        return &self.info;
    }
}

impl TaskBaseDTO for NormalTaskDTO {
    fn name(&self) -> &str {
        return &self.name;
    }

    fn category(&self) -> &str {
        return &self.category;
    }

    fn start_time(&self) -> NaiveDateTime {
        return self.start_time;
    }

    fn end_time(&self) -> NaiveDateTime {
        return self.end_time;
    }

    fn info(&self) -> &[(String, String)] {
        return &self.info;
    }
}

impl TaskDTO for NormalTaskDTO {
    type SubTask = SubTaskDTO;

    fn executor(&self) -> &str {
        return &self.executor;
    }

    fn order(&self) -> Option<&str> {
        return self.order.as_ref().map(|str| str.as_str());
    }

    fn produce(&self) -> Option<&str> {
        return self.produce.as_ref().map(|str| str.as_str());
    }

    fn products(&self) -> Option<&[(String, String)]> {
        return self.products.as_ref().map(|slice| slice.as_slice());
    }

    fn materials(&self) -> Option<&[(String, String)]> {
        return self.materials.as_ref().map(|slice| slice.as_slice());
    }

    fn resources(&self) -> &[(String, String)] {
        return &self.resources;
    }

    fn scheduled_start_time(&self) -> Option<NaiveDateTime> {
        return self.scheduled_start_time.clone();
    }

    fn scheduled_end_time(&self) -> Option<NaiveDateTime> {
        return self.scheduled_end_time.clone();
    }

    fn sub_tasks(&self) -> &[SubTaskDTO] {
        return &self.sub_tasks;
    }
}

#[derive(Deserialize, Debug)]
pub struct ResponseDTO {
    pub tasks: Vec<NormalTaskDTO>,
}
