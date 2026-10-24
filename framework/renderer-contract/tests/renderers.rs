// 同时测试 Tauri 实际使用的 DTO，不依赖 GUI。 / Test the actual Tauri DTOs without GUI dependencies.
#[path = "../../bpp3d-interface-renderer/src-tauri/src/dto.rs"]
mod bpp3d;
#[path = "../../csp1d-interface-renderer/src-tauri/src/dto.rs"]
mod csp1d;
#[path = "../../gantt-scheduling-interface-renderer/src-tauri/src/dto.rs"]
mod gantt;
#[path = "../../gantt-scheduling-interface-renderer/src-tauri/src/serializer.rs"]
mod serializer;

#[test]
fn actual_kotlin_bpp3d_renderer_fixtures() {
    for fixture in [
        include_str!("../../../ospf-kotlin/ospf-kotlin-framework-bpp3d/bpp3d-infrastructure/src/test/resources/renderer/mixed-shape-renderer-schema.json"),
        include_str!("../../../ospf-kotlin/ospf-kotlin-framework-bpp3d/bpp3d-infrastructure/src/test/resources/renderer/cylinder-axis-renderer-schema.json"),
    ] {
        let schema: bpp3d::SchemaDTO = serde_json::from_str(fixture).unwrap();
        let output = serde_json::to_value(schema).unwrap();
        assert!(!output["loadingPlans"].as_array().unwrap().is_empty());
        assert!(output["loadingPlans"][0]["items"][0]["width"].is_number());
    }
}

#[test]
fn actual_time_crate_serde_output_is_accepted() {
    let start = time::OffsetDateTime::from_unix_timestamp(1792800000).unwrap();
    let end = start + time::Duration::hours(1);
    let schema: gantt::SchemaDTO = serde_json::from_value(serde_json::json!({"tasks": [{
        "executor": "M", "order": 0, "actual_time": {"start": start, "end": end},
        "products": [], "consumption": [], "resources": [], "sub_tasks": []
    }]}))
    .unwrap();
    assert_eq!(
        schema.tasks[0].start_time.unwrap().and_utc().timestamp(),
        start.unix_timestamp()
    );
}

#[test]
fn human_readable_time_crate_dates_are_accepted() {
    let schema: gantt::SchemaDTO = serde_json::from_value(serde_json::json!({"tasks": [{
        "executor": "M", "actual_time": {
            "start": "2026-10-24 08:00:00.123456789 +08:00:00",
            "end": "2026-10-24 09:00:00.123456789 +08:00:00"
        }
    }]}))
    .unwrap();
    assert_eq!(
        schema.tasks[0].start_time.unwrap().to_string(),
        "2026-10-24 00:00:00.123456789"
    );
}
