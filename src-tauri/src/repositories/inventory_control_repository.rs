use crate::models::{
    InventoryAdjustmentDto, InventoryAdjustmentFilterRequest, StocktakeFilterRequest,
    StocktakeRecordDto,
};
use crate::utils::normalize_date;
use rusqlite::{params_from_iter, types::Value};

pub fn list_inventory_adjustments(
    conn: &rusqlite::Connection,
    filter: InventoryAdjustmentFilterRequest,
) -> anyhow::Result<Vec<InventoryAdjustmentDto>> {
    let mut sql = String::from(
        "SELECT id, adjustment_date, product_id, product_name, category, adjustment_type,
                quantity_delta, unit_cost, amount, reason, remark, status, void_reason,
                voided_at, created_at
         FROM inventory_adjustments WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    append_common_inventory_filters(
        &mut sql,
        &mut sql_params,
        CommonInventoryFilter {
            date_column: "adjustment_date",
            start_date: filter.start_date,
            end_date: filter.end_date,
            product_id: filter.product_id,
            category: filter.category,
            status: filter.status,
        },
    );
    sql.push_str(" ORDER BY adjustment_date DESC, id DESC LIMIT 1000");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        params_from_iter(sql_params.iter()),
        map_inventory_adjustment,
    )?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn list_stocktakes(
    conn: &rusqlite::Connection,
    filter: StocktakeFilterRequest,
) -> anyhow::Result<Vec<StocktakeRecordDto>> {
    let mut sql = String::from(
        "SELECT id, stocktake_date, product_id, product_name, category, system_stock,
                actual_stock, difference_quantity, unit_cost, difference_amount, reason,
                remark, status, void_reason, voided_at, created_at
         FROM stocktake_records WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    append_common_inventory_filters(
        &mut sql,
        &mut sql_params,
        CommonInventoryFilter {
            date_column: "stocktake_date",
            start_date: filter.start_date,
            end_date: filter.end_date,
            product_id: filter.product_id,
            category: filter.category,
            status: filter.status,
        },
    );
    sql.push_str(" ORDER BY stocktake_date DESC, id DESC LIMIT 1000");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), map_stocktake)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn inventory_adjustment_by_id(
    conn: &rusqlite::Connection,
    id: i64,
) -> anyhow::Result<InventoryAdjustmentDto> {
    conn.query_row(
        "SELECT id, adjustment_date, product_id, product_name, category, adjustment_type,
                quantity_delta, unit_cost, amount, reason, remark, status, void_reason,
                voided_at, created_at
         FROM inventory_adjustments WHERE id = ?1",
        [id],
        map_inventory_adjustment,
    )
    .map_err(Into::into)
}

pub fn stocktake_by_id(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<StocktakeRecordDto> {
    conn.query_row(
        "SELECT id, stocktake_date, product_id, product_name, category, system_stock,
                actual_stock, difference_quantity, unit_cost, difference_amount, reason,
                remark, status, void_reason, voided_at, created_at
         FROM stocktake_records WHERE id = ?1",
        [id],
        map_stocktake,
    )
    .map_err(Into::into)
}

struct CommonInventoryFilter {
    date_column: &'static str,
    start_date: Option<String>,
    end_date: Option<String>,
    product_id: Option<i64>,
    category: Option<String>,
    status: Option<String>,
}

fn append_common_inventory_filters(
    sql: &mut String,
    sql_params: &mut Vec<Value>,
    filter: CommonInventoryFilter,
) {
    if let Some(start) = filter.start_date.filter(|value| !value.is_empty()) {
        sql.push_str(&format!(" AND {} >= ?", filter.date_column));
        sql_params.push(Value::Text(normalize_date(&start)));
    }
    if let Some(end) = filter.end_date.filter(|value| !value.is_empty()) {
        sql.push_str(&format!(" AND {} <= ?", filter.date_column));
        sql_params.push(Value::Text(normalize_date(&end)));
    }
    if let Some(product_id) = filter.product_id {
        sql.push_str(" AND product_id = ?");
        sql_params.push(Value::Integer(product_id));
    }
    if let Some(category) = filter
        .category
        .filter(|value| !value.is_empty() && value != "全部")
    {
        sql.push_str(" AND category = ?");
        sql_params.push(Value::Text(category));
    }
    if let Some(status) = filter
        .status
        .filter(|value| !value.is_empty() && value != "全部")
    {
        sql.push_str(" AND status = ?");
        sql_params.push(Value::Text(status));
    }
}

fn map_inventory_adjustment(row: &rusqlite::Row<'_>) -> rusqlite::Result<InventoryAdjustmentDto> {
    Ok(InventoryAdjustmentDto {
        id: row.get(0)?,
        adjustment_date: row.get(1)?,
        product_id: row.get(2)?,
        product_name: row.get(3)?,
        category: row.get(4)?,
        adjustment_type: row.get(5)?,
        quantity_delta: row.get(6)?,
        unit_cost: row.get(7)?,
        amount: row.get(8)?,
        reason: row.get(9)?,
        remark: row.get(10)?,
        status: row.get(11)?,
        void_reason: row.get(12)?,
        voided_at: row.get(13)?,
        created_at: row.get(14)?,
    })
}

fn map_stocktake(row: &rusqlite::Row<'_>) -> rusqlite::Result<StocktakeRecordDto> {
    Ok(StocktakeRecordDto {
        id: row.get(0)?,
        stocktake_date: row.get(1)?,
        product_id: row.get(2)?,
        product_name: row.get(3)?,
        category: row.get(4)?,
        system_stock: row.get(5)?,
        actual_stock: row.get(6)?,
        difference_quantity: row.get(7)?,
        unit_cost: row.get(8)?,
        difference_amount: row.get(9)?,
        reason: row.get(10)?,
        remark: row.get(11)?,
        status: row.get(12)?,
        void_reason: row.get(13)?,
        voided_at: row.get(14)?,
        created_at: row.get(15)?,
    })
}
