use crate::app::AppState;
use crate::db;
use crate::models::ImportResult;
use crate::utils::{money, normalize_date, now_text};
use calamine::{open_workbook_auto, Data, Reader};
use rusqlite::{params, Connection};
use std::collections::HashMap;

pub fn import_excel_file(state: &AppState, file_path: &str) -> anyhow::Result<ImportResult> {
    let mut workbook = open_workbook_auto(file_path)?;
    let mut conn = state.connection()?;
    let tx = conn.transaction()?;
    clear_business_tables(&tx)?;

    let mut result = ImportResult {
        product_count: 0,
        customer_count: 0,
        movement_count: 0,
        profit_count: 0,
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    let product_map = import_products(&mut workbook, &tx, &mut result)?;
    let customer_map = import_customers(&mut workbook, &tx, &mut result)?;
    db::ensure_guest_customer(&tx)?;
    import_movements(&mut workbook, &tx, &product_map, &mut result)?;
    import_profit_sheet(&mut workbook, &mut result)?;

    for product_id in product_map.values() {
        db::recalc_stock_balance(&tx, *product_id)?;
    }

    tx.commit()?;

    if product_map.len() != 280 {
        result
            .warnings
            .push(format!("商品数量为 {}，预期约 280", product_map.len()));
    }
    if customer_map.len() != 997 {
        result
            .warnings
            .push(format!("客户数量为 {}，预期约 997", customer_map.len()));
    }
    if result.movement_count != 3 {
        result.warnings.push(format!(
            "有效库存流水为 {}，预期约 3",
            result.movement_count
        ));
    }

    state.set_import_result(result.clone());
    Ok(result)
}

fn clear_business_tables(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        DELETE FROM documents;
        DELETE FROM monthly_credits;
        DELETE FROM order_items;
        DELETE FROM payment_records;
        DELETE FROM orders;
        DELETE FROM customer_product_rules;
        DELETE FROM inbound_records;
        DELETE FROM inventory_movements;
        DELETE FROM stock_balances;
        DELETE FROM customers;
        DELETE FROM products;
        DELETE FROM backup_logs WHERE backup_type = 'import';
        DELETE FROM sqlite_sequence WHERE name IN (
          'documents','monthly_credits','order_items','orders','customer_product_rules',
          'payment_records','inbound_records','inventory_movements','customers','products'
        );
        "#,
    )?;
    Ok(())
}

fn import_products<R: std::io::Read + std::io::Seek>(
    workbook: &mut calamine::Sheets<R>,
    conn: &Connection,
    result: &mut ImportResult,
) -> anyhow::Result<HashMap<String, i64>> {
    let range = workbook.worksheet_range("库存")?;
    let mut product_map = HashMap::new();
    let now = now_text();

    for row_index in 1..range.height() {
        let name = cell_text(range.get((row_index, 0)));
        if name.is_empty() {
            continue;
        }
        let category = cell_text(range.get((row_index, 1)));
        let safety_stock = cell_number(range.get((row_index, 5))).unwrap_or(0.0);
        let barcode = optional_text(range.get((row_index, 6)));
        conn.execute(
            "INSERT INTO products
             (name, category, barcode, default_price, safety_stock, unit, is_active, remark, created_at, updated_at)
             VALUES (?1, ?2, ?3, 0, ?4, ?5, 1, ?6, ?7, ?7)",
            params![
                name,
                if category.is_empty() { "其他" } else { category.as_str() },
                barcode,
                safety_stock,
                "件",
                format!("Excel库存有效区行 {}", row_index + 1),
                now
            ],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO stock_balances (product_id, current_stock, avg_cost, stock_value, updated_at)
             VALUES (?1, 0, 0, 0, ?2)",
            params![id, now],
        )?;
        product_map.insert(name, id);
        result.product_count += 1;
    }
    Ok(product_map)
}

fn import_customers<R: std::io::Read + std::io::Seek>(
    workbook: &mut calamine::Sheets<R>,
    conn: &Connection,
    result: &mut ImportResult,
) -> anyhow::Result<HashMap<String, i64>> {
    let range = workbook.worksheet_range("客户信息")?;
    let mut customer_map = HashMap::new();
    let now = now_text();

    for row_index in 1..range.height() {
        let region = cell_text(range.get((row_index, 0)));
        let name = cell_text(range.get((row_index, 1)));
        let address = cell_text(range.get((row_index, 2)));
        if region.is_empty() && name.is_empty() && address.is_empty() {
            continue;
        }
        let safe_name = if name.is_empty() {
            format!("未命名客户{}", row_index + 1)
        } else {
            name
        };
        conn.execute(
            "INSERT INTO customers
             (region, name, address, phone, is_active, remark, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, 1, ?4, ?5, ?5)",
            params![
                none_if_empty(region),
                safe_name,
                none_if_empty(address),
                format!("Excel客户有效区行 {}", row_index + 1),
                now
            ],
        )?;
        let id = conn.last_insert_rowid();
        customer_map.insert(safe_name, id);
        result.customer_count += 1;
    }
    Ok(customer_map)
}

fn import_movements<R: std::io::Read + std::io::Seek>(
    workbook: &mut calamine::Sheets<R>,
    conn: &Connection,
    product_map: &HashMap<String, i64>,
    result: &mut ImportResult,
) -> anyhow::Result<()> {
    let range = workbook.worksheet_range("明细")?;
    let now = now_text();

    for row_index in 1..range.height() {
        let date = cell_text(range.get((row_index, 1)));
        let category = cell_text(range.get((row_index, 2)));
        let product_name = cell_text(range.get((row_index, 3)));
        let movement_text = cell_text(range.get((row_index, 4)));
        let quantity = cell_number(range.get((row_index, 5))).unwrap_or(0.0);
        let unit_price = cell_number(range.get((row_index, 6))).unwrap_or(0.0);
        let amount =
            cell_number(range.get((row_index, 7))).unwrap_or_else(|| money(quantity * unit_price));

        if product_name.is_empty() && movement_text.is_empty() && quantity.abs() < f64::EPSILON {
            continue;
        }

        let Some(product_id) = product_map.get(&product_name).copied() else {
            result.warnings.push(format!(
                "明细第 {} 行商品未在库存表找到：{}",
                row_index + 1,
                product_name
            ));
            continue;
        };
        let movement_type = match movement_text.as_str() {
            "入库" => "inbound",
            "出库" => "outbound",
            other => {
                result.warnings.push(format!(
                    "明细第 {} 行未知出入库类型：{}",
                    row_index + 1,
                    other
                ));
                continue;
            }
        };
        let movement_date = if date.is_empty() {
            "2026-01-16".to_string()
        } else {
            normalize_date(&date)
        };

        conn.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, source_id, source_no, remark, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'excel_import', NULL, NULL, ?7, ?8)",
            params![
                movement_date,
                product_id,
                movement_type,
                quantity,
                unit_price,
                amount,
                format!("Excel明细行 {}，类别 {}", row_index + 1, category),
                now
            ],
        )?;

        if movement_type == "inbound" {
            conn.execute(
                "INSERT INTO inbound_records
                 (inbound_date, product_id, quantity, unit_cost, amount, remark, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    movement_date,
                    product_id,
                    quantity,
                    unit_price,
                    amount,
                    format!("Excel迁移，明细行 {}", row_index + 1),
                    now
                ],
            )?;
        }
        result.movement_count += 1;
    }
    Ok(())
}

fn import_profit_sheet<R: std::io::Read + std::io::Seek>(
    workbook: &mut calamine::Sheets<R>,
    result: &mut ImportResult,
) -> anyhow::Result<()> {
    let range = workbook.worksheet_range("利润")?;
    for row_index in 0..24 {
        let has_value = (0..4).any(|col| !cell_text(range.get((row_index, col))).is_empty());
        if has_value {
            result.profit_count += 1;
        }
    }
    Ok(())
}

fn cell_text(cell: Option<&Data>) -> String {
    match cell {
        Some(Data::String(value)) => value.trim().to_string(),
        Some(Data::Float(value)) => {
            if value.fract().abs() < f64::EPSILON {
                format!("{}", *value as i64)
            } else {
                value.to_string()
            }
        }
        Some(Data::Int(value)) => value.to_string(),
        Some(Data::Bool(value)) => value.to_string(),
        Some(Data::DateTime(value)) => value.to_string(),
        Some(Data::DateTimeIso(value)) => value.clone(),
        Some(Data::DurationIso(value)) => value.clone(),
        Some(Data::Error(value)) => format!("{value:?}"),
        Some(Data::Empty) | None => String::new(),
    }
}

fn optional_text(cell: Option<&Data>) -> Option<String> {
    none_if_empty(cell_text(cell))
}

fn none_if_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn cell_number(cell: Option<&Data>) -> Option<f64> {
    match cell {
        Some(Data::Float(value)) => Some(*value),
        Some(Data::Int(value)) => Some(*value as f64),
        Some(Data::String(value)) => value.trim().parse::<f64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use rusqlite::Connection;

    #[test]
    fn imports_source_workbook_counts() {
        let source =
            std::path::Path::new("C:/Users/ww/Desktop/work/订单库存表3.02 - 副本 (2).xlsm");
        if !source.exists() {
            return;
        }
        let mut workbook = open_workbook_auto(source).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::seed_settings(&conn).unwrap();
        let mut result = ImportResult {
            product_count: 0,
            customer_count: 0,
            movement_count: 0,
            profit_count: 0,
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        let product_map = import_products(&mut workbook, &conn, &mut result).unwrap();
        let customer_map = import_customers(&mut workbook, &conn, &mut result).unwrap();
        import_movements(&mut workbook, &conn, &product_map, &mut result).unwrap();
        import_profit_sheet(&mut workbook, &mut result).unwrap();
        assert_eq!(product_map.len(), 280);
        assert_eq!(customer_map.len(), 997);
        assert_eq!(result.movement_count, 3);
    }
}
