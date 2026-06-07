use super::{fail, ok};
use crate::app::AppState;
use crate::logger;
use crate::models::*;
use crate::reports;
use crate::services::{
    analytics_service, diagnostics_service, document_service, profit_service, report_service,
};
use std::path::Path;
use tauri::State;

#[tauri::command]
pub fn get_daily_profit_summary(
    state: State<AppState>,
    date: String,
) -> ApiResponse<DailyProfitSummary> {
    let result = (|| {
        let conn = state.connection()?;
        profit_service::daily_profit_summary(&conn, &date)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_profit_analytics(
    state: State<AppState>,
    request: ProfitAnalyticsRequest,
) -> ApiResponse<ProfitAnalyticsResponse> {
    let result = (|| {
        let conn = state.connection()?;
        profit_service::get_profit_analytics(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_profit_records(
    state: State<AppState>,
    filter: Option<ProfitFilterRequest>,
) -> ApiResponse<Vec<OrderDto>> {
    let result = (|| {
        let conn = state.connection()?;
        profit_service::list_profit_records_with_default_filter(&conn, filter)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_inventory_report(
    state: State<AppState>,
    filter: Option<InventoryReportRequest>,
) -> ApiResponse<Vec<InventoryReportRowDto>> {
    let result = (|| {
        let conn = state.connection()?;
        report_service::list_inventory_report_with_default_filter(&conn, filter)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_product_ranking(
    state: State<AppState>,
    request: ProductRankingRequest,
) -> ApiResponse<Vec<ProductRankingRowDto>> {
    let result = (|| {
        let conn = state.connection()?;
        analytics_service::product_ranking(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_customer_analysis(
    state: State<AppState>,
    request: CustomerAnalysisRequest,
) -> ApiResponse<CustomerAnalysisDto> {
    let result = (|| {
        let conn = state.connection()?;
        analytics_service::customer_analysis(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_supplier_purchase_ledger(
    state: State<AppState>,
    filter: Option<SupplierPurchaseLedgerRequest>,
) -> ApiResponse<SupplierPurchaseLedgerDto> {
    let result = (|| {
        let conn = state.connection()?;
        report_service::supplier_purchase_ledger_with_default_filter(&conn, filter)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_documents(
    state: State<AppState>,
    filter: Option<DocumentFilterRequest>,
) -> ApiResponse<Vec<DocumentDto>> {
    let result = (|| {
        let conn = state.connection()?;
        document_service::list_documents_with_default_filter(&conn, filter)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn open_document(state: State<AppState>, document_id: i64) -> ApiResponse<String> {
    let result = (|| {
        let conn = state.connection()?;
        document_service::open_document(&conn, document_id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_document(state: State<AppState>, order_id: i64) -> ApiResponse<String> {
    reports::export_order_document(&state, order_id)
        .map(ok)
        .unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_document_pdf(state: State<AppState>, order_id: i64) -> ApiResponse<String> {
    reports::export_order_pdf_document(&state, order_id)
        .map(ok)
        .unwrap_or_else(fail)
}

#[tauri::command]
pub fn print_document(
    state: State<AppState>,
    document_id: i64,
    payload: Option<PrintRequest>,
) -> ApiResponse<PrintStatusDto> {
    let result = (|| {
        let conn = state.connection()?;
        document_service::print_document(
            &conn,
            document_id,
            payload.and_then(|value| value.printer_name),
        )
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_data(state: State<AppState>, payload: ExportDataRequest) -> ApiResponse<String> {
    let result = reports::export_data(&state, payload);
    if let Ok(path) = &result {
        logger::info("export", format!("导出成功：{path}"));
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn open_exports_folder(state: State<AppState>) -> ApiResponse<String> {
    let result = (|| {
        let path = state.exports_dir();
        open::that(&path)?;
        Ok(path.to_string_lossy().to_string())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn open_logs_folder(state: State<AppState>) -> ApiResponse<String> {
    let result = (|| {
        let path = state.logs_dir();
        open::that(&path)?;
        Ok(path.to_string_lossy().to_string())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn run_data_self_check(state: State<AppState>) -> ApiResponse<DataSelfCheckDto> {
    let result = (|| {
        let conn = state.connection()?;
        diagnostics_service::run_data_self_check(&conn, |path| Path::new(path).exists())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_data_self_check(state: State<AppState>) -> ApiResponse<String> {
    let result = (|| {
        let conn = state.connection()?;
        let check =
            diagnostics_service::run_data_self_check(&conn, |path| Path::new(path).exists())?;
        let path = state.exports_dir().join(format!(
            "data_self_check_{}.txt",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ));
        diagnostics_service::write_self_check_export(&path, &check)?;
        Ok(path.to_string_lossy().to_string())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_diagnostic_summary(state: State<AppState>) -> ApiResponse<DiagnosticSummaryDto> {
    let result = (|| {
        let conn = state.connection()?;
        diagnostics_service::diagnostic_summary(
            &conn,
            &state.db_path(),
            &state.logs_dir(),
            &state.backups_dir(),
            &state.exports_dir(),
            env!("CARGO_PKG_VERSION"),
        )
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_diagnostic_package(state: State<AppState>) -> ApiResponse<DiagnosticPackageDto> {
    let result = (|| {
        let conn = state.connection()?;
        diagnostics_service::export_diagnostic_package(
            &conn,
            &state.db_path(),
            &state.logs_dir(),
            &state.backups_dir(),
            &state.exports_dir(),
            env!("CARGO_PKG_VERSION"),
        )
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_audit_logs(
    state: State<AppState>,
    filter: Option<AuditLogFilterRequest>,
) -> ApiResponse<Vec<AuditLogDto>> {
    let result = (|| {
        let conn = state.connection()?;
        crate::services::audit_service::list_audit_logs(&conn, filter)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_printers() -> ApiResponse<Vec<String>> {
    reports::list_system_printers().map(ok).unwrap_or_else(fail)
}
