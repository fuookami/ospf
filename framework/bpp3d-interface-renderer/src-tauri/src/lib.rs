use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Mutex;

mod dto;
use dto::SchemaDTO;

static CURRENT_DIR: Mutex<Option<String>> = Mutex::new(None);

#[tauri::command]
fn get_current_directory() -> Result<String, String> {
    let current = CURRENT_DIR.lock().map_err(|error| error.to_string())?;
    if let Some(path) = current.as_ref() {
        return Ok(path.clone());
    }
    std::env::current_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn load_data(path: String) -> Result<SchemaDTO, String> {
    let file = File::open(&path).map_err(|error| format!("Cannot open {path}: {error}"))?;
    let schema = serde_json::from_reader(BufReader::new(file))
        .map_err(|error| format!("Invalid renderer data: {error}"))?;
    let parent = Path::new(&path)
        .parent()
        .ok_or("Missing parent directory")?;
    *CURRENT_DIR.lock().map_err(|error| error.to_string())? =
        Some(parent.to_string_lossy().into_owned());
    Ok(schema)
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
