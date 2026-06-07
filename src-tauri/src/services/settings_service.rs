use crate::models::{SaveSettingsRequest, SettingDto};
use crate::repositories::settings_repository;

pub fn list_settings(conn: &rusqlite::Connection) -> anyhow::Result<Vec<SettingDto>> {
    settings_repository::list_settings(conn)
}

pub fn save_settings(
    conn: &rusqlite::Connection,
    payload: SaveSettingsRequest,
) -> anyhow::Result<bool> {
    settings_repository::save_settings(conn, payload)
}
