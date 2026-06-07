use crate::db;
use crate::models::{
    CreateInventoryAdjustmentRequest, CreateStocktakeRequest, InventoryAdjustmentDto,
    InventoryAdjustmentFilterRequest, StocktakeFilterRequest, StocktakeRecordDto,
};
use crate::repositories::inventory_control_repository;
use crate::services::audit_service::{record_audit, AuditEvent};
use crate::utils::{money, normalize_date, now_text};
use rusqlite::params;

pub fn create_inventory_adjustment(
    conn: &mut rusqlite::Connection,
    payload: CreateInventoryAdjustmentRequest,
) -> anyhow::Result<InventoryAdjustmentDto> {
    if payload.product_id <= 0 || payload.quantity_delta.abs() < f64::EPSILON {
        anyhow::bail!("调整商品和调整数量不合法");
    }
    let adjustment_type = validate_adjustment_type(&payload.adjustment_type)?;
    let reason = payload.reason.trim();
    if reason.is_empty() {
        anyhow::bail!("库存调整原因必填");
    }
    let tx = conn.transaction()?;
    let product = db::product_by_id(&tx, payload.product_id)?;
    if !product.is_active {
        anyhow::bail!("商品已停用，不能调整库存");
    }
    let unit_cost = money(product.avg_cost);
    let amount = money(payload.quantity_delta * unit_cost);
    let now = now_text();
    tx.execute(
        "INSERT INTO inventory_adjustments
         (adjustment_date, product_id, product_name, category, adjustment_type, quantity_delta,
          unit_cost, amount, reason, remark, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'normal', ?11, ?11)",
        params![
            normalize_date(&payload.adjustment_date),
            product.id,
            product.name,
            product.category,
            adjustment_type,
            money(payload.quantity_delta),
            unit_cost,
            amount,
            reason,
            payload.remark,
            now
        ],
    )?;
    let adjustment_id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO inventory_movements
         (movement_date, product_id, movement_type, quantity, unit_price, amount,
          source_type, source_id, source_no, remark, created_at)
         VALUES (?1, ?2, 'stocktake_adjustment', ?3, ?4, ?5,
                 'inventory_adjustment', ?6, ?7, ?8, ?9)",
        params![
            normalize_date(&payload.adjustment_date),
            product.id,
            money(payload.quantity_delta),
            unit_cost,
            amount,
            adjustment_id,
            format!("ADJ{adjustment_id:06}"),
            reason,
            now
        ],
    )?;
    db::recalc_stock_balance(&tx, product.id)?;
    record_audit(
        &tx,
        AuditEvent {
            module: "inventory",
            action: "adjust",
            target_type: Some("products"),
            target_id: Some(product.id),
            target_label: Some(&product.name),
            result: "success",
            message: Some(reason),
            details: Some(&format!("quantityDelta={}", money(payload.quantity_delta))),
        },
    )?;
    tx.commit()?;
    inventory_control_repository::inventory_adjustment_by_id(conn, adjustment_id)
}

pub fn list_inventory_adjustments(
    conn: &rusqlite::Connection,
    filter: InventoryAdjustmentFilterRequest,
) -> anyhow::Result<Vec<InventoryAdjustmentDto>> {
    inventory_control_repository::list_inventory_adjustments(conn, filter)
}

pub fn void_inventory_adjustment(
    conn: &mut rusqlite::Connection,
    id: i64,
    reason: Option<String>,
) -> anyhow::Result<InventoryAdjustmentDto> {
    let tx = conn.transaction()?;
    let (product_id, product_name, adjustment_date, quantity_delta, unit_cost, amount, status): (
        i64,
        String,
        String,
        f64,
        f64,
        f64,
        String,
    ) = tx.query_row(
        "SELECT product_id, product_name, adjustment_date, quantity_delta, unit_cost, amount, status
         FROM inventory_adjustments WHERE id = ?1",
        [id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        },
    )?;
    if status != "normal" {
        anyhow::bail!("库存调整记录已作废");
    }
    let void_reason = reason.unwrap_or_else(|| "作废库存调整".to_string());
    let now = now_text();
    tx.execute(
        "UPDATE inventory_adjustments
         SET status = 'voided', void_reason = ?1, voided_at = ?2, updated_at = ?2
         WHERE id = ?3",
        params![void_reason, now, id],
    )?;
    tx.execute(
        "INSERT INTO inventory_movements
         (movement_date, product_id, movement_type, quantity, unit_price, amount,
          source_type, source_id, source_no, remark, created_at)
         VALUES (?1, ?2, 'stocktake_adjustment', ?3, ?4, ?5,
                 'inventory_adjustment_void', ?6, ?7, ?8, ?9)",
        params![
            adjustment_date,
            product_id,
            money(-quantity_delta),
            unit_cost,
            money(-amount),
            id,
            format!("ADJ{id:06}"),
            void_reason,
            now
        ],
    )?;
    db::recalc_stock_balance(&tx, product_id)?;
    record_audit(
        &tx,
        AuditEvent {
            module: "inventory",
            action: "void_adjustment",
            target_type: Some("inventory_adjustments"),
            target_id: Some(id),
            target_label: Some(&product_name),
            result: "success",
            message: Some("库存调整已作废"),
            details: None,
        },
    )?;
    tx.commit()?;
    inventory_control_repository::inventory_adjustment_by_id(conn, id)
}

pub fn create_stocktake(
    conn: &mut rusqlite::Connection,
    payload: CreateStocktakeRequest,
) -> anyhow::Result<StocktakeRecordDto> {
    if payload.product_id <= 0 || payload.actual_stock < 0.0 {
        anyhow::bail!("盘点商品和实盘库存不合法");
    }
    let reason = payload.reason.trim();
    if reason.is_empty() {
        anyhow::bail!("盘点原因必填");
    }
    let tx = conn.transaction()?;
    let product = db::product_by_id(&tx, payload.product_id)?;
    if !product.is_active {
        anyhow::bail!("商品已停用，不能盘点");
    }
    let system_stock = money(product.current_stock);
    let actual_stock = money(payload.actual_stock);
    let difference_quantity = money(actual_stock - system_stock);
    let unit_cost = money(product.avg_cost);
    let difference_amount = money(difference_quantity * unit_cost);
    let now = now_text();
    tx.execute(
        "INSERT INTO stocktake_records
         (stocktake_date, product_id, product_name, category, system_stock, actual_stock,
          difference_quantity, unit_cost, difference_amount, reason, remark, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'normal', ?12, ?12)",
        params![
            normalize_date(&payload.stocktake_date),
            product.id,
            product.name,
            product.category,
            system_stock,
            actual_stock,
            difference_quantity,
            unit_cost,
            difference_amount,
            reason,
            payload.remark,
            now
        ],
    )?;
    let stocktake_id = tx.last_insert_rowid();
    if difference_quantity.abs() > f64::EPSILON {
        tx.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount,
              source_type, source_id, source_no, remark, created_at)
             VALUES (?1, ?2, 'stocktake_adjustment', ?3, ?4, ?5,
                     'stocktake', ?6, ?7, ?8, ?9)",
            params![
                normalize_date(&payload.stocktake_date),
                product.id,
                difference_quantity,
                unit_cost,
                difference_amount,
                stocktake_id,
                format!("STK{stocktake_id:06}"),
                reason,
                now
            ],
        )?;
        db::recalc_stock_balance(&tx, product.id)?;
    }
    record_audit(
        &tx,
        AuditEvent {
            module: "inventory",
            action: "stocktake",
            target_type: Some("products"),
            target_id: Some(product.id),
            target_label: Some(&product.name),
            result: "success",
            message: Some(reason),
            details: Some(&format!("difference={difference_quantity}")),
        },
    )?;
    tx.commit()?;
    inventory_control_repository::stocktake_by_id(conn, stocktake_id)
}

pub fn list_stocktakes(
    conn: &rusqlite::Connection,
    filter: StocktakeFilterRequest,
) -> anyhow::Result<Vec<StocktakeRecordDto>> {
    inventory_control_repository::list_stocktakes(conn, filter)
}

pub fn void_stocktake(
    conn: &mut rusqlite::Connection,
    id: i64,
    reason: Option<String>,
) -> anyhow::Result<StocktakeRecordDto> {
    let tx = conn.transaction()?;
    let (
        product_id,
        product_name,
        stocktake_date,
        difference_quantity,
        unit_cost,
        difference_amount,
        status,
    ): (i64, String, String, f64, f64, f64, String) = tx.query_row(
        "SELECT product_id, product_name, stocktake_date, difference_quantity, unit_cost,
                difference_amount, status
         FROM stocktake_records WHERE id = ?1",
        [id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        },
    )?;
    if status != "normal" {
        anyhow::bail!("盘点记录已作废");
    }
    let void_reason = reason.unwrap_or_else(|| "作废盘点记录".to_string());
    let now = now_text();
    tx.execute(
        "UPDATE stocktake_records
         SET status = 'voided', void_reason = ?1, voided_at = ?2, updated_at = ?2
         WHERE id = ?3",
        params![void_reason, now, id],
    )?;
    if difference_quantity.abs() > f64::EPSILON {
        tx.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount,
              source_type, source_id, source_no, remark, created_at)
             VALUES (?1, ?2, 'stocktake_adjustment', ?3, ?4, ?5,
                     'stocktake_void', ?6, ?7, ?8, ?9)",
            params![
                stocktake_date,
                product_id,
                money(-difference_quantity),
                unit_cost,
                money(-difference_amount),
                id,
                format!("STK{id:06}"),
                void_reason,
                now
            ],
        )?;
        db::recalc_stock_balance(&tx, product_id)?;
    }
    record_audit(
        &tx,
        AuditEvent {
            module: "inventory",
            action: "void_stocktake",
            target_type: Some("stocktake_records"),
            target_id: Some(id),
            target_label: Some(&product_name),
            result: "success",
            message: Some("盘点记录已作废"),
            details: None,
        },
    )?;
    tx.commit()?;
    inventory_control_repository::stocktake_by_id(conn, id)
}

fn validate_adjustment_type(value: &str) -> anyhow::Result<String> {
    let normalized = value.trim();
    match normalized {
        "loss" | "increase" | "scrap" | "self_use" | "other" => Ok(normalized.to_string()),
        _ => anyhow::bail!("不支持的库存调整类型"),
    }
}
