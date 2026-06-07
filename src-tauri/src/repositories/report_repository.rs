use crate::models::{
    InboundRecordDto, InventoryReportRowDto, SupplierPurchaseLedgerDto, SupplierPurchaseSummaryDto,
    SupplierPurchaseTrendPointDto,
};
use crate::utils::money;
use rusqlite::{params_from_iter, types::Value};

#[derive(Debug, Clone)]
pub struct SqlFilter {
    pub where_sql: String,
    pub params: Vec<Value>,
}

pub fn list_inventory_report(
    conn: &rusqlite::Connection,
    movement_filter: &SqlFilter,
    category: Option<&str>,
    keyword: Option<&str>,
) -> anyhow::Result<Vec<InventoryReportRowDto>> {
    let mut sql = format!(
        "SELECT p.id, p.name, p.category, p.barcode,
                COALESCE(inbound.inbound_quantity, 0),
                COALESCE(inbound.inbound_amount, 0),
                COALESCE(outbound.outbound_quantity, 0),
                COALESCE(outbound.outbound_amount, 0),
                COALESCE(gift.gift_quantity, 0),
                COALESCE(s.current_stock, 0),
                COALESCE(s.avg_cost, 0),
                COALESCE(s.stock_value, 0)
         FROM products p
         LEFT JOIN stock_balances s ON s.product_id = p.id
         LEFT JOIN (
           SELECT product_id, SUM(quantity) AS inbound_quantity, SUM(amount) AS inbound_amount
           FROM inventory_movements
           WHERE movement_type = 'inbound' {}
           GROUP BY product_id
         ) inbound ON inbound.product_id = p.id
         LEFT JOIN (
           SELECT product_id, SUM(quantity) AS outbound_quantity, SUM(amount) AS outbound_amount
           FROM inventory_movements
           WHERE movement_type = 'outbound' {}
           GROUP BY product_id
         ) outbound ON outbound.product_id = p.id
         LEFT JOIN (
           SELECT product_id, SUM(quantity) AS gift_quantity
           FROM inventory_movements
           WHERE movement_type = 'gift_outbound' {}
           GROUP BY product_id
         ) gift ON gift.product_id = p.id
         WHERE p.is_active = 1",
        movement_filter.where_sql, movement_filter.where_sql, movement_filter.where_sql
    );
    let mut sql_params = Vec::new();
    for _ in 0..3 {
        sql_params.extend(movement_filter.params.clone());
    }
    if let Some(category) = category {
        sql.push_str(" AND p.category = ?");
        sql_params.push(Value::Text(category.to_string()));
    }
    if let Some(keyword) = keyword {
        let keyword = format!("%{keyword}%");
        sql.push_str(" AND (p.name LIKE ? OR p.barcode LIKE ?)");
        sql_params.push(Value::Text(keyword.clone()));
        sql_params.push(Value::Text(keyword));
    }
    sql.push_str(" ORDER BY p.category, p.name LIMIT 10000");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), |row| {
        Ok(InventoryReportRowDto {
            product_id: row.get(0)?,
            product_name: row.get(1)?,
            category: row.get(2)?,
            barcode: row.get(3)?,
            inbound_quantity: row.get(4)?,
            inbound_amount: row.get(5)?,
            outbound_quantity: row.get(6)?,
            outbound_amount: row.get(7)?,
            gift_quantity: row.get(8)?,
            current_stock: row.get(9)?,
            avg_cost: row.get(10)?,
            stock_value: row.get(11)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn supplier_purchase_summaries(
    conn: &rusqlite::Connection,
    where_sql: &str,
    params: &[Value],
) -> anyhow::Result<Vec<SupplierPurchaseSummaryDto>> {
    let sql = format!(
        "SELECT i.supplier_id,
                COALESCE(NULLIF(TRIM(i.supplier_name), ''), '未指定供应商') AS supplier_name,
                COUNT(*) AS inbound_count,
                COALESCE(SUM(i.amount), 0) AS inbound_amount,
                MAX(i.inbound_date) AS recent_inbound_date
         FROM inbound_records i
         WHERE {where_sql}
         GROUP BY i.supplier_id, supplier_name
         ORDER BY inbound_amount DESC, supplier_name"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok(SupplierPurchaseSummaryDto {
            supplier_id: row.get(0)?,
            supplier_name: row.get(1)?,
            inbound_count: row.get(2)?,
            inbound_amount: money(row.get::<_, f64>(3)?),
            recent_inbound_date: row.get(4)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn supplier_purchase_details(
    conn: &rusqlite::Connection,
    where_sql: &str,
    params: &[Value],
) -> anyhow::Result<Vec<InboundRecordDto>> {
    let sql = format!(
        "SELECT i.id, i.inbound_date, i.product_id, p.name, p.category,
                i.supplier_id, i.supplier_name, i.quantity, i.unit_cost, i.amount, i.remark
         FROM inbound_records i
         JOIN products p ON p.id = i.product_id
         WHERE {where_sql}
         ORDER BY i.inbound_date DESC, i.id DESC
         LIMIT 1000"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok(InboundRecordDto {
            id: row.get(0)?,
            inbound_date: row.get(1)?,
            product_id: row.get(2)?,
            product_name: row.get(3)?,
            category: row.get(4)?,
            supplier_id: row.get(5)?,
            supplier_name: row.get(6)?,
            quantity: money(row.get::<_, f64>(7)?),
            unit_cost: money(row.get::<_, f64>(8)?),
            amount: money(row.get::<_, f64>(9)?),
            remark: row.get(10)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn supplier_purchase_monthly_trend(
    conn: &rusqlite::Connection,
    where_sql: &str,
    params: &[Value],
) -> anyhow::Result<Vec<SupplierPurchaseTrendPointDto>> {
    let sql = format!(
        "SELECT substr(i.inbound_date, 1, 7) AS period,
                COUNT(*) AS inbound_count,
                COALESCE(SUM(i.amount), 0) AS inbound_amount
         FROM inbound_records i
         WHERE {where_sql}
         GROUP BY period
         ORDER BY period"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        Ok(SupplierPurchaseTrendPointDto {
            period: row.get(0)?,
            inbound_count: row.get(1)?,
            inbound_amount: money(row.get::<_, f64>(2)?),
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn supplier_purchase_ledger(
    summaries: Vec<SupplierPurchaseSummaryDto>,
    details: Vec<InboundRecordDto>,
    monthly_trend: Vec<SupplierPurchaseTrendPointDto>,
) -> SupplierPurchaseLedgerDto {
    SupplierPurchaseLedgerDto {
        summaries,
        details,
        monthly_trend,
    }
}
