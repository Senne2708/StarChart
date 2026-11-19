mod auth;
mod commands;
mod error;
mod server;
mod state;

use tracing_subscriber::EnvFilter;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::from_env())
        .invoke_handler(tauri::generate_handler![
            commands::create_login,
            commands::login,
            commands::logout,
            commands::get_students,
            commands::get_teachers,
            commands::get_activities,
            commands::add_student,
            commands::add_activity,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
