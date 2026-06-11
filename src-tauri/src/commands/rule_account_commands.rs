use super::{fail, ok};
use crate::app::AppState;
use crate::logger;
use crate::models::*;
use crate::reports;
use crate::services::{
    customer_account_service, customer_rule_service, customer_statement_service, order_service,
};
use tauri::State;

#[tauri::command]
pub fn list_customer_product_rules(
    state: State<AppState>,
    filter: Option<RuleFilterRequest>,
) -> ApiResponse<Vec<CustomerProductRuleDto>> {
    let result = (|| {
        let conn = state.connection()?;
        logger::info(
            "rule",
            format!(
                "list_customer_product_rules filter_keys={}",
                rule_filter_keys(filter.as_ref()).join(",")
            ),
        );
        customer_rule_service::list_customer_product_rules(&conn, filter)
    })();
    if let Ok(items) = &result {
        logger::info(
            "rule",
            format!("list_customer_product_rules result count={}", items.len()),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_customer_product_rule(
    state: State<AppState>,
    payload: SaveCustomerProductRuleRequest,
) -> ApiResponse<i64> {
    logger::info(
        "rule",
        format!(
            "save_customer_product_rule start has_id={} has_gift={} has_credit={}",
            payload.id.is_some(),
            payload.gift_product_id.is_some(),
            payload.monthly_credit_amount.is_some()
        ),
    );
    let result = (|| {
        let conn = state.connection()?;
        customer_rule_service::save_customer_product_rule(&conn, payload)
    })();
    if let Ok(id) = &result {
        logger::info(
            "rule",
            format!("save_customer_product_rule success id={id}"),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn disable_customer_product_rule(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        customer_rule_service::disable_customer_product_rule(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn delete_customer_product_rule(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        customer_rule_service::delete_customer_product_rule(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn preview_customer_product_rule_import(
    state: State<AppState>,
    file_path: String,
) -> ApiResponse<CustomerProductRuleImportPreviewDto> {
    let result = (|| {
        let conn = state.connection()?;
        customer_rule_service::preview_customer_product_rule_import(&conn, &file_path)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn import_customer_product_rules(
    state: State<AppState>,
    file_path: String,
) -> ApiResponse<CustomerProductRuleImportResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        customer_rule_service::import_customer_product_rules(&conn, &file_path)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_monthly_credits(
    state: State<AppState>,
    filter: Option<MonthlyCreditFilterRequest>,
) -> ApiResponse<Vec<MonthlyCreditDto>> {
    let result = (|| {
        let conn = state.connection()?;
        order_service::list_monthly_credits_with_default_filter(&conn, filter)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_available_monthly_credits(
    state: State<AppState>,
    customer_id: i64,
    category: String,
    order_date: String,
) -> ApiResponse<Vec<MonthlyCreditDto>> {
    let result = (|| {
        let conn = state.connection()?;
        order_service::available_monthly_credits(&conn, customer_id, category, order_date)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn close_monthly_credit(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        order_service::close_or_void_credit(&conn, id, "closed")?;
        Ok(true)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn void_monthly_credit(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        order_service::close_or_void_credit(&conn, id, "voided")?;
        Ok(true)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_customer_balances(
    state: State<AppState>,
    filter: Option<CustomerBalanceFilterRequest>,
) -> ApiResponse<Vec<CustomerBalanceDto>> {
    let result = (|| {
        let conn = state.connection()?;
        customer_account_service::list_customer_balances(&conn, filter)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_payment_records(
    state: State<AppState>,
    filter: Option<PaymentFilterRequest>,
) -> ApiResponse<Vec<PaymentRecordDto>> {
    let result = (|| {
        let conn = state.connection()?;
        customer_account_service::list_payment_records(&conn, filter)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_payment(
    state: State<AppState>,
    payload: CreatePaymentRequest,
) -> ApiResponse<PaymentRecordDto> {
    let result = (|| {
        let conn = state.connection()?;
        customer_account_service::create_payment(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn void_payment(state: State<AppState>, id: i64) -> ApiResponse<PaymentRecordDto> {
    let result = (|| {
        let conn = state.connection()?;
        customer_account_service::void_payment(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_customer_statement(
    state: State<AppState>,
    request: CustomerStatementRequest,
) -> ApiResponse<CustomerStatementDto> {
    let result = (|| {
        let conn = state.connection()?;
        customer_statement_service::customer_statement(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_customer_statement_pdf(
    state: State<AppState>,
    request: CustomerStatementRequest,
) -> ApiResponse<String> {
    reports::export_customer_statement_pdf_document(&state, request)
        .map(ok)
        .unwrap_or_else(fail)
}

fn rule_filter_keys(filter: Option<&RuleFilterRequest>) -> Vec<&'static str> {
    let Some(filter) = filter else {
        return Vec::new();
    };
    let mut keys = Vec::new();
    if filter.customer_id.is_some() {
        keys.push("customerId");
    }
    if filter.product_id.is_some() {
        keys.push("productId");
    }
    if filter.category.is_some() {
        keys.push("category");
    }
    if filter.keyword.is_some() {
        keys.push("keyword");
    }
    if filter.is_active.is_some() {
        keys.push("isActive");
    }
    if filter.rule_type.is_some() {
        keys.push("ruleType");
    }
    keys
}
