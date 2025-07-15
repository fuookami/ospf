#![feature(sync_unsafe_cell)]

use std::cell::{Cell, SyncUnsafeCell};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

mod loading_plan;
use crate::loading_plan::SchemaDTO;

static mut CURRENT_DIR: SyncUnsafeCell<Cell<Option<String>>> = SyncUnsafeCell::new(Cell::new(None));

#[tauri::command]
fn get_current_directory() -> Option<&'static String> {
    unsafe {
        if (*(*CURRENT_DIR.get()).as_ptr()).is_none() {
            CURRENT_DIR.get_mut().set(Some(String::from(
                std::env::current_dir()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .to_str()
                    .unwrap(),
            )));
        }
        (*(*CURRENT_DIR.get()).as_ptr()).as_ref()
    }
}

fn load_json<T: serde::de::DeserializeOwned>(path: String) -> Option<T> {
    unsafe {
        let parent_path = Path::new(&path).parent().unwrap();
        CURRENT_DIR.get_mut().set(Some(String::from(parent_path.to_str().unwrap())));
    }

    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    match serde_json::from_reader(reader) {
        Ok(data) => Some(data),
        Err(err) => {
            println!("Message from Rust: {}", err);
            None
        }
    }
}

#[tauri::command]
fn load_data(path: String) -> Result<SchemaDTO, String> {
    if let Some(data) = load_json::<SchemaDTO>(path.clone()) {
        return Ok(data);
    }
    Err(String::from("无法识别的数据文件"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![get_current_directory, load_data])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
