use crate::models::{
    BatchUpdateResultDto, BatchUpdateSuppliersRequest, ListSuppliersRequest, SupplierDto,
    SupplierPayload,
};
use crate::repositories::supplier_repository;

pub fn list_suppliers(
    conn: &rusqlite::Connection,
    filter: Option<ListSuppliersRequest>,
) -> anyhow::Result<Vec<SupplierDto>> {
    supplier_repository::list_suppliers(conn, filter.unwrap_or_else(default_supplier_filter))
}

pub fn create_supplier(
    conn: &rusqlite::Connection,
    payload: SupplierPayload,
) -> anyhow::Result<SupplierDto> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        anyhow::bail!("供应商名称必填");
    }
    supplier_repository::create_supplier(conn, payload, &name)
}

pub fn update_supplier(
    conn: &rusqlite::Connection,
    id: i64,
    payload: SupplierPayload,
) -> anyhow::Result<SupplierDto> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        anyhow::bail!("供应商名称必填");
    }
    supplier_repository::update_supplier(conn, id, payload, &name)
}

pub fn disable_supplier(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<bool> {
    supplier_repository::disable_supplier(conn, id)
}

pub fn batch_update_suppliers(
    conn: &rusqlite::Connection,
    payload: BatchUpdateSuppliersRequest,
) -> anyhow::Result<BatchUpdateResultDto> {
    supplier_repository::batch_update_suppliers(conn, payload)
}

fn default_supplier_filter() -> ListSuppliersRequest {
    ListSuppliersRequest {
        keyword: None,
        is_active: Some(true),
    }
}
