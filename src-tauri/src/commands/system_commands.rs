use super::ok;
use crate::app::AppState;
use crate::logger;
use crate::models::{ApiResponse, AppStatusDto, ClientLogRequest};
use tauri::State;

#[tauri::command]
pub fn get_app_status(state: State<AppState>) -> ApiResponse<AppStatusDto> {
    ok(state.app_status())
}

#[tauri::command]
pub fn write_client_log(payload: ClientLogRequest) -> ApiResponse<bool> {
    let module = format!(
        "client:{}",
        payload
            .module
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("app")
    );
    let mut message = payload.message;
    if let Some(details) = payload.details.filter(|value| !value.trim().is_empty()) {
        message.push_str(" | details: ");
        message.push_str(&details);
    }
    match payload.level.trim().to_ascii_uppercase().as_str() {
        "ERROR" => logger::error(&module, message),
        "WARN" | "WARNING" => logger::warn(&module, message),
        _ => logger::info(&module, message),
    }
    ok(true)
}
