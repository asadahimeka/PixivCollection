#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;
use warp::Filter;

#[derive(Clone, serde::Serialize)]
struct Payload {
    args: Vec<String>,
    cwd: String,
}

#[tauri::command]
fn start_local_server(base: String) -> Result<(), String> {
    // Define the path to the static files directory
    let static_dir = warp::fs::dir(base);

    // Create a CORS filter
    let cors = warp::cors()
        .allow_any_origin()
        .allow_header("content-type")
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);

    // Create a warp filter for the static files with CORS support
    let routes = static_dir.with(cors);
    // .with(warp::filters::compression::gzip());

    // Spawn a new thread to run the warp server
    tokio::spawn(async move {
        warp::serve(routes).run(([127, 0, 0, 1], 32154)).await;
    });

    Ok(())
}

#[tauri::command]
fn get_executable_dir() -> Result<std::path::PathBuf, String> {
    match std::env::current_exe() {
        Ok(path) => match path.parent() {
            Some(parent) => Ok(parent.to_path_buf()),
            None => Err("Failed to get parent directory".to_string()),
        },
        Err(error) => Err(format!("{error}")),
    }
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!("{}, {argv:?}, {cwd}", app.package_info().name);

            app.emit_all("single-instance", Payload { args: argv, cwd })
                .unwrap();
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![start_local_server, get_executable_dir])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
