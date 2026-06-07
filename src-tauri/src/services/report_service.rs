use crate::models::{
    InventoryReportRequest, InventoryReportRowDto, SupplierPurchaseLedgerDto,
    SupplierPurchaseLedgerRequest,
};
use crate::repositories::report_repository::{self, SqlFilter};
use crate::utils::normalize_date;
use anyhow::anyhow;
use rusqlite::types::Value;

pub fn list_inventory_report(
    conn: &rusqlite::Connection,
    request: InventoryReportRequest,
) -> anyhow::Result<Vec<InventoryReportRowDto>> {
    let movement_filter = movement_date_filter(&request);
    report_repository::list_inventory_report(
        conn,
        &movement_filter,
        active_text(request.category.as_deref()),
        active_text(request.keyword.as_deref()),
    )
}

pub fn list_inventory_report_with_default_filter(
    conn: &rusqlite::Connection,
    filter: Option<InventoryReportRequest>,
) -> anyhow::Result<Vec<InventoryReportRowDto>> {
    list_inventory_report(conn, filter.unwrap_or_else(default_inventory_report_filter))
}

pub fn supplier_purchase_ledger(
    conn: &rusqlite::Connection,
    request: SupplierPurchaseLedgerRequest,
) -> anyhow::Result<SupplierPurchaseLedgerDto> {
    let start_date = request
        .start_date
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| normalize_date(value));
    let end_date = request
        .end_date
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| normalize_date(value));
    if let (Some(start), Some(end)) = (&start_date, &end_date) {
        if start > end {
            return Err(anyhow!("采购台账开始日期不能晚于结束日期"));
        }
    }
    let mut filters = Vec::new();
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(start) = &start_date {
        filters.push("i.inbound_date >= ?".to_string());
        sql_params.push(Value::Text(start.to_string()));
    }
    if let Some(end) = &end_date {
        filters.push("i.inbound_date <= ?".to_string());
        sql_params.push(Value::Text(end.to_string()));
    }
    if let Some(supplier_id) = request.supplier_id {
        filters.push("i.supplier_id = ?".to_string());
        sql_params.push(Value::Integer(supplier_id));
    }
    let where_sql = if filters.is_empty() {
        "1 = 1".to_string()
    } else {
        filters.join(" AND ")
    };

    let summaries = report_repository::supplier_purchase_summaries(conn, &where_sql, &sql_params)?;
    let details = report_repository::supplier_purchase_details(conn, &where_sql, &sql_params)?;
    let monthly_trend =
        report_repository::supplier_purchase_monthly_trend(conn, &where_sql, &sql_params)?;
    Ok(report_repository::supplier_purchase_ledger(
        summaries,
        details,
        monthly_trend,
    ))
}

pub fn supplier_purchase_ledger_with_default_filter(
    conn: &rusqlite::Connection,
    filter: Option<SupplierPurchaseLedgerRequest>,
) -> anyhow::Result<SupplierPurchaseLedgerDto> {
    supplier_purchase_ledger(
        conn,
        filter.unwrap_or(SupplierPurchaseLedgerRequest {
            start_date: None,
            end_date: None,
            supplier_id: None,
        }),
    )
}

fn movement_date_filter(request: &InventoryReportRequest) -> SqlFilter {
    let mut conditions = String::new();
    let mut sql_params = Vec::new();
    if let Some(start_date) = request
        .start_date
        .as_ref()
        .filter(|value| !value.is_empty())
    {
        conditions.push_str(" AND movement_date >= ?");
        sql_params.push(Value::Text(start_date.to_string()));
    }
    if let Some(end_date) = request.end_date.as_ref().filter(|value| !value.is_empty()) {
        conditions.push_str(" AND movement_date <= ?");
        sql_params.push(Value::Text(end_date.to_string()));
    }
    SqlFilter {
        where_sql: conditions,
        params: sql_params,
    }
}

fn active_text(value: Option<&str>) -> Option<&str> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "全部")
}

fn default_inventory_report_filter() -> InventoryReportRequest {
    InventoryReportRequest {
        start_date: None,
        end_date: None,
        category: None,
        keyword: None,
    }
}
