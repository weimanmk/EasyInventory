use super::{fail, ok};
use crate::app::AppState;
use crate::logger;
use crate::models::*;
use crate::reports;
use crate::services::audit_service::{record_audit, AuditEvent};
use crate::services::order_service;
use tauri::State;

#[tauri::command]
pub fn preview_quote(
    state: State<AppState>,
    payload: PreviewQuoteRequest,
) -> ApiResponse<QuotePreviewDto> {
    let result = (|| {
        let conn = state.connection()?;
        order_service::preview_quote(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_order(
    state: State<AppState>,
    payload: SaveOrderRequest,
) -> ApiResponse<SaveOrderResponse> {
    let result = (|| {
        let mut conn = state.connection()?;
        let mut response = order_service::save_order(&mut conn, payload)?;
        response.document_path = reports::export_order_document(&state, response.order_id)?;
        let audit_conn = state.connection()?;
        record_audit(
            &audit_conn,
            AuditEvent {
                module: "order",
                action: "save",
                target_type: Some("orders"),
                target_id: Some(response.order_id),
                target_label: Some(&response.order_no),
                result: "success",
                message: Some("订单保存成功"),
                details: Some(&format!("documentPath={}", response.document_path)),
            },
        )?;
        logger::info(
            "order",
            format!(
                "保存订单成功：{}，单据：{}",
                response.order_no, response.document_path
            ),
        );
        Ok(response)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_order(state: State<AppState>, id: i64) -> ApiResponse<OrderDetailDto> {
    let result = (|| {
        let conn = state.connection()?;
        order_service::get_order_detail(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_orders(
    state: State<AppState>,
    filter: Option<ListOrdersRequest>,
) -> ApiResponse<Vec<OrderDto>> {
    let result = (|| {
        let conn = state.connection()?;
        order_service::list_orders_with_default_filter(&conn, filter)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_order_document(state: State<AppState>, order_id: i64) -> ApiResponse<String> {
    reports::export_order_document(&state, order_id)
        .map(ok)
        .unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_order_pdf_document(state: State<AppState>, order_id: i64) -> ApiResponse<String> {
    reports::export_order_pdf_document(&state, order_id)
        .map(ok)
        .unwrap_or_else(fail)
}

#[tauri::command]
pub fn print_order_document(state: State<AppState>, order_id: i64) -> ApiResponse<String> {
    let result = (|| {
        let path = reports::export_order_document(&state, order_id)?;
        open::that(&path)?;
        Ok(path)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn print_order_document_with_options(
    state: State<AppState>,
    order_id: i64,
    payload: Option<PrintRequest>,
) -> ApiResponse<PrintStatusDto> {
    let result = (|| {
        let path = reports::export_order_document(&state, order_id)?;
        let printer_name = payload.and_then(|value| value.printer_name);
        let message = if let Some(printer) = printer_name
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            open::that(&path)?;
            format!("已打开单据文件，请在关联程序中选择打印机：{printer}")
        } else {
            open::that(&path)?;
            "已打开单据文件，请在关联程序中确认打印".to_string()
        };
        Ok(PrintStatusDto {
            file_path: path,
            printer_name,
            message,
        })
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn void_order(
    state: State<AppState>,
    id: i64,
    payload: Option<VoidOrderRequest>,
) -> ApiResponse<OrderDto> {
    let result = (|| {
        let mut conn = state.connection()?;
        let order =
            order_service::void_order(&mut conn, id, payload.and_then(|value| value.reason))?;
        record_audit(
            &conn,
            AuditEvent {
                module: "order",
                action: "void",
                target_type: Some("orders"),
                target_id: Some(order.id),
                target_label: Some(&order.order_no),
                result: "success",
                message: Some("订单已作废"),
                details: Some(&format!("customer={}", order.customer_name)),
            },
        )?;
        logger::warn("order", format!("订单已作废：{}", order.order_no));
        Ok(order)
    })();
    result.map(ok).unwrap_or_else(fail)
}
