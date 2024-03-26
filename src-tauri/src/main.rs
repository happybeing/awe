// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // tauri::Builder::default()
    // .run(tauri::generate_context!())
    // .expect("error while running tauri application");

    println!("adding xor-scheme...");
    tauri::Builder::default()
        // .invoke_handler(tauri::generate_handler![greet])
        .register_uri_scheme_protocol("xor", |_app, req| {
            let url = req.uri();
            let content = format!("<HTML><HEAD></HEAD><BODY><h1>{url:?}</h1> content</BODY></HTML>");
            tauri::http::ResponseBuilder::new()
                .body(content.into_bytes())
        })
        .register_uri_scheme_protocol("http", |_app, req| {
            println!("http-scheme: {req:?}");
            tauri::http::ResponseBuilder::new().body(Vec::new())
        })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

// #[tauri::command]
// fn greet(name: &str) -> String {
//    format!("Hello, {}!", name)
// }
