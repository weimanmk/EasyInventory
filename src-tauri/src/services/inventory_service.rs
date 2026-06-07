use crate::db;
use crate::models::{
    CreateInboundRequest, CreateInboundResponse, InboundRecordDto, ListInboundRecordsRequest,
};
use crate::repositories::inventory_repository;
use crate::utils::{money, normalize_date, now_text};
use rusqlite::params;

pub fn create_inbound(
    conn: &mut rusqlite::Connection,
    payload: CreateInboundRequest,
) -> anyhow::Result<CreateInboundResponse> {
    if payload.product_id <= 0 || payload.quantity <= 0.0 || payload.unit_cost < 0.0 {
        anyhow::bail!("入库商品、数量和进货价不合法");
    }
    let tx = conn.transaction()?;
    let amount = money(payload.quantity * payload.unit_cost);
    let now = now_text();
    let supplier_name = match payload.supplier_id {
        Some(id) => inventory_repository::active_supplier_name(&tx, id)?,
        None => None,
    };
    if payload.supplier_id.is_some() && supplier_name.is_none() {
        anyhow::bail!("供应商不存在或已停用");
    }
    tx.execute(
        "INSERT INTO inbound_records
         (inbound_date, product_id, supplier_id, supplier_name, quantity, unit_cost, amount, remark, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            normalize_date(&payload.inbound_date),
            payload.product_id,
            payload.supplier_id,
            supplier_name,
            payload.quantity,
            payload.unit_cost,
            amount,
            payload.remark,
            now
        ],
    )?;
    let inbound_id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO inventory_movements
         (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, source_id, remark, created_at)
         VALUES (?1, ?2, 'inbound', ?3, ?4, ?5, 'inbound', ?6, ?7, ?8)",
        params![
            normalize_date(&payload.inbound_date),
            payload.product_id,
            payload.quantity,
            payload.unit_cost,
            amount,
            inbound_id,
            "入库",
            now
        ],
    )?;
    let (current_stock, avg_cost) = db::recalc_stock_balance(&tx, payload.product_id)?;
    tx.commit()?;
    Ok(CreateInboundResponse {
        inbound_id,
        product_id: payload.product_id,
        current_stock,
        avg_cost,
    })
}

pub fn list_inbound_records(
    conn: &rusqlite::Connection,
    filter: Option<ListInboundRecordsRequest>,
) -> anyhow::Result<Vec<InboundRecordDto>> {
    inventory_repository::list_inbound_records(conn, filter.unwrap_or_else(default_inbound_filter))
}

fn default_inbound_filter() -> ListInboundRecordsRequest {
    ListInboundRecordsRequest {
        start_date: None,
        end_date: None,
        product_id: None,
        category: None,
    }
}
