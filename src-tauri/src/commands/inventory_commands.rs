use super::{fail, ok};
use crate::app::AppState;
use crate::models::*;
use crate::services::{inventory_control_service, inventory_service};
use tauri::State;

#[tauri::command]
pub fn create_inbound(
    state: State<AppState>,
    payload: CreateInboundRequest,
) -> ApiResponse<CreateInboundResponse> {
    let result = (|| {
        let mut conn = state.connection()?;
        inventory_service::create_inbound(&mut conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_inbound_records(
    state: State<AppState>,
    filter: Option<ListInboundRecordsRequest>,
) -> ApiResponse<Vec<InboundRecordDto>> {
    let result = (|| {
        let conn = state.connection()?;
        inventory_service::list_inbound_records(&conn, filter)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_inventory_adjustment(
    state: State<AppState>,
    payload: CreateInventoryAdjustmentRequest,
) -> ApiResponse<InventoryAdjustmentDto> {
    let result = (|| {
        let mut conn = state.connection()?;
        inventory_control_service::create_inventory_adjustment(&mut conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_inventory_adjustments(
    state: State<AppState>,
    filter: Option<InventoryAdjustmentFilterRequest>,
) -> ApiResponse<Vec<InventoryAdjustmentDto>> {
    let result = (|| {
        let conn = state.connection()?;
        inventory_control_service::list_inventory_adjustments(&conn, filter.unwrap_or_default())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn void_inventory_adjustment(
    state: State<AppState>,
    id: i64,
    payload: Option<VoidRecordRequest>,
) -> ApiResponse<InventoryAdjustmentDto> {
    let result = (|| {
        let mut conn = state.connection()?;
        inventory_control_service::void_inventory_adjustment(
            &mut conn,
            id,
            payload.and_then(|value| value.reason),
        )
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_stocktake(
    state: State<AppState>,
    payload: CreateStocktakeRequest,
) -> ApiResponse<StocktakeRecordDto> {
    let result = (|| {
        let mut conn = state.connection()?;
        inventory_control_service::create_stocktake(&mut conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_stocktakes(
    state: State<AppState>,
    filter: Option<StocktakeFilterRequest>,
) -> ApiResponse<Vec<StocktakeRecordDto>> {
    let result = (|| {
        let conn = state.connection()?;
        inventory_control_service::list_stocktakes(&conn, filter.unwrap_or_default())
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn void_stocktake(
    state: State<AppState>,
    id: i64,
    payload: Option<VoidRecordRequest>,
) -> ApiResponse<StocktakeRecordDto> {
    let result = (|| {
        let mut conn = state.connection()?;
        inventory_control_service::void_stocktake(
            &mut conn,
            id,
            payload.and_then(|value| value.reason),
        )
    })();
    result.map(ok).unwrap_or_else(fail)
}
