use super::{fail, ok};
use crate::app::AppState;
use crate::db;
use crate::excel;
use crate::logger;
use crate::models::*;
use crate::services::backup_service;
use tauri::State;

#[tauri::command]
pub fn import_excel(state: State<AppState>, file_path: String) -> ApiResponse<ImportResult> {
    let result = (|| {
        let backup_path = db::create_backup_file(&state, "pre_legacy_import")?;
        logger::info(
            "import",
            format!("历史兼容 Excel 导入前已自动备份：{backup_path}"),
        );
        excel::import_excel_file(&state, &file_path)
    })();
    if let Ok(result) = &result {
        logger::info(
            "import",
            format!(
                "Excel导入完成：商品 {}，客户 {}，流水 {}，利润行 {}",
                result.product_count,
                result.customer_count,
                result.movement_count,
                result.profit_count
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_import_status(
    state: State<AppState>,
    _import_id: Option<String>,
) -> ApiResponse<Option<ImportResult>> {
    ok(state.import_result())
}

#[tauri::command]
pub fn create_backup(state: State<AppState>) -> ApiResponse<String> {
    let result = db::create_backup_file(&state, "manual");
    if let Ok(path) = &result {
        logger::info("backup", format!("手动备份成功：{path}"));
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_backups(state: State<AppState>) -> ApiResponse<Vec<BackupDto>> {
    let result = (|| {
        let conn = state.connection()?;
        backup_service::list_backups(&conn)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn open_backup_folder(state: State<AppState>) -> ApiResponse<String> {
    let result = (|| {
        let path = state.backups_dir();
        open::that(&path)?;
        Ok(path.to_string_lossy().to_string())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn restore_backup(
    state: State<AppState>,
    backup_id: i64,
) -> ApiResponse<RestoreBackupResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        let restored = backup_service::restore_backup(
            &conn,
            backup_id,
            &state.db_path(),
            &state.backups_dir(),
        )?;
        drop(conn);
        let conn = state.connection()?;
        backup_service::finalize_restore(&conn, backup_id, &restored)?;
        Ok(restored)
    })();
    result.map(ok).unwrap_or_else(fail)
}
