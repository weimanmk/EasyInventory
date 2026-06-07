use crate::db;
use crate::models::{
    BatchUpdateProductsRequest, BatchUpdateResultDto, ListProductsRequest, ProductDto,
    ProductPayload,
};
use crate::utils::{money, now_text};
use rusqlite::{params, params_from_iter, types::Value, OptionalExtension};

pub fn list_products(
    conn: &rusqlite::Connection,
    filter: ListProductsRequest,
) -> anyhow::Result<Vec<ProductDto>> {
    let mut sql = String::from(
        "SELECT p.id, p.name, p.category, p.barcode, p.default_price, p.safety_stock, p.unit,
                COALESCE(s.current_stock, 0), COALESCE(s.avg_cost, 0), COALESCE(s.stock_value, 0),
                p.is_active, p.remark
         FROM products p
         LEFT JOIN stock_balances s ON s.product_id = p.id
         WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(category) = filter
        .category
        .filter(|value| !value.is_empty() && value != "全部")
    {
        sql.push_str(" AND p.category = ?");
        sql_params.push(Value::Text(category));
    }
    if let Some(keyword) = filter.keyword.filter(|value| !value.is_empty()) {
        let keyword = format!("%{keyword}%");
        sql.push_str(" AND (p.name LIKE ? OR p.barcode LIKE ?)");
        sql_params.push(Value::Text(keyword.clone()));
        sql_params.push(Value::Text(keyword));
    }
    if filter.only_low_stock.unwrap_or(false) {
        sql.push_str(" AND COALESCE(s.current_stock, 0) <= p.safety_stock");
    }
    if filter.only_in_stock.unwrap_or(false) {
        sql.push_str(" AND COALESCE(s.current_stock, 0) > 0");
    }
    if let Some(active) = filter.is_active {
        sql.push_str(if active {
            " AND p.is_active = 1"
        } else {
            " AND p.is_active = 0"
        });
    }
    sql.push_str(" ORDER BY p.category, p.name LIMIT 1000");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(sql_params.iter()), db::map_product)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn create_product(
    conn: &rusqlite::Connection,
    payload: ProductPayload,
) -> anyhow::Result<ProductDto> {
    let now = now_text();
    conn.execute(
        "INSERT INTO products
         (name, category, barcode, default_price, safety_stock, unit, is_active, remark, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?8)",
        params![
            payload.name.trim(),
            payload.category.trim(),
            payload.barcode,
            payload.default_price.unwrap_or(0.0),
            payload.safety_stock.unwrap_or(0.0),
            payload.unit,
            payload.remark,
            now
        ],
    )?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO stock_balances (product_id, current_stock, avg_cost, stock_value, updated_at)
         VALUES (?1, 0, 0, 0, ?2)",
        params![id, now_text()],
    )?;
    db::product_by_id(conn, id)
}

pub fn update_product(
    conn: &rusqlite::Connection,
    id: i64,
    payload: ProductPayload,
) -> anyhow::Result<ProductDto> {
    conn.execute(
        "UPDATE products SET name = ?1, category = ?2, barcode = ?3, default_price = ?4,
         safety_stock = ?5, unit = ?6, remark = ?7, updated_at = ?8 WHERE id = ?9",
        params![
            payload.name.trim(),
            payload.category.trim(),
            payload.barcode,
            payload.default_price.unwrap_or(0.0),
            payload.safety_stock.unwrap_or(0.0),
            payload.unit,
            payload.remark,
            now_text(),
            id
        ],
    )?;
    db::product_by_id(conn, id)
}

pub fn disable_product(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<bool> {
    conn.execute(
        "UPDATE products SET is_active = 0, updated_at = ?1 WHERE id = ?2",
        params![now_text(), id],
    )?;
    Ok(true)
}

pub fn batch_update_products(
    conn: &rusqlite::Connection,
    payload: BatchUpdateProductsRequest,
) -> anyhow::Result<BatchUpdateResultDto> {
    let ids_sql = batch_ids_placeholders(&payload.ids)?;
    let mut sets = Vec::new();
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(category) = payload.category {
        let category = category.trim();
        if category.is_empty() {
            anyhow::bail!("商品类别不能为空");
        }
        sets.push("category = ?".to_string());
        sql_params.push(Value::Text(category.to_string()));
    }
    if let Some(value) = payload.safety_stock {
        if value < 0.0 {
            anyhow::bail!("安全库存不能小于 0");
        }
        sets.push("safety_stock = ?".to_string());
        sql_params.push(Value::Real(money(value)));
    }
    if let Some(value) = payload.default_price {
        if value < 0.0 {
            anyhow::bail!("默认售价不能小于 0");
        }
        sets.push("default_price = ?".to_string());
        sql_params.push(Value::Real(money(value)));
    }
    if let Some(unit) = payload.unit {
        append_text_assignment(&mut sets, &mut sql_params, "unit", &unit);
    }
    if let Some(active) = payload.is_active {
        sets.push("is_active = ?".to_string());
        sql_params.push(Value::Integer(if active { 1 } else { 0 }));
    }
    if sets.is_empty() {
        anyhow::bail!("没有要更新的商品字段");
    }
    sets.push("updated_at = ?".to_string());
    sql_params.push(Value::Text(now_text()));
    for id in payload.ids {
        sql_params.push(Value::Integer(id));
    }
    let sql = format!(
        "UPDATE products SET {} WHERE id IN ({ids_sql})",
        sets.join(", ")
    );
    let affected = conn.execute(&sql, params_from_iter(sql_params.iter()))?;
    Ok(BatchUpdateResultDto {
        affected_count: affected as i64,
    })
}

pub fn find_by_barcode(
    conn: &rusqlite::Connection,
    barcode: &str,
) -> anyhow::Result<Option<ProductDto>> {
    conn.query_row(
        "SELECT p.id, p.name, p.category, p.barcode, p.default_price, p.safety_stock, p.unit,
                COALESCE(s.current_stock, 0), COALESCE(s.avg_cost, 0), COALESCE(s.stock_value, 0),
                p.is_active, p.remark
         FROM products p
         LEFT JOIN stock_balances s ON s.product_id = p.id
         WHERE p.barcode = ?1 AND p.is_active = 1
         ORDER BY p.id LIMIT 1",
        [barcode],
        db::map_product,
    )
    .optional()
    .map_err(Into::into)
}

fn batch_ids_placeholders(ids: &[i64]) -> anyhow::Result<String> {
    if ids.is_empty() {
        anyhow::bail!("请选择要批量编辑的记录");
    }
    if ids.iter().any(|id| *id <= 0) {
        anyhow::bail!("批量编辑记录不合法");
    }
    Ok((0..ids.len()).map(|_| "?").collect::<Vec<_>>().join(","))
}

fn append_text_assignment(
    sets: &mut Vec<String>,
    sql_params: &mut Vec<Value>,
    column: &str,
    value: &str,
) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        sets.push(format!("{column} = ?"));
        sql_params.push(Value::Null);
    } else {
        sets.push(format!("{column} = ?"));
        sql_params.push(Value::Text(trimmed.to_string()));
    }
}
