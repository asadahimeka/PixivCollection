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

#[tokio::main]
async fn main() {
    // Define the path to the static files directory
    let static_dir = warp::fs::dir("E:/Pictures/Pixiv");

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
        warp::serve(routes)
            .run(([127, 0, 0, 1], 32154))
            .await;
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!("{}, {argv:?}, {cwd}", app.package_info().name);

            app.emit_all("single-instance", Payload { args: argv, cwd })
                .unwrap();
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
