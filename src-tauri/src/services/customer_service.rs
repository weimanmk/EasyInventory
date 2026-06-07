use crate::db;
use crate::models::{
    BatchUpdateCustomersRequest, BatchUpdateResultDto, CustomerDto, CustomerPayload,
    ListCustomersRequest,
};
use crate::repositories::customer_repository;

pub fn list_customers(
    conn: &rusqlite::Connection,
    filter: Option<ListCustomersRequest>,
) -> anyhow::Result<Vec<CustomerDto>> {
    let guest_name = db::guest_customer_name(conn)?;
    customer_repository::list_customers(
        conn,
        filter.unwrap_or_else(default_customer_filter),
        &guest_name,
    )
}

pub fn create_customer(
    conn: &rusqlite::Connection,
    payload: CustomerPayload,
) -> anyhow::Result<CustomerDto> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        anyhow::bail!("客户名称必填");
    }
    let guest_name = db::guest_customer_name(conn)?;
    if name == guest_name || name == db::GUEST_CUSTOMER_NAME {
        let id = db::ensure_guest_customer(conn)?;
        return db::customer_by_id(conn, id);
    }
    customer_repository::create_customer(conn, payload, &name)
}

pub fn update_customer(
    conn: &rusqlite::Connection,
    id: i64,
    payload: CustomerPayload,
) -> anyhow::Result<CustomerDto> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        anyhow::bail!("客户名称必填");
    }

    let guest_name = db::guest_customer_name(conn)?;
    let is_guest = db::is_guest_customer(conn, id)?;
    if is_guest && name != guest_name {
        anyhow::bail!("{guest_name}是系统默认客户，名称不能修改");
    }
    if !is_guest && (name == guest_name || name == db::GUEST_CUSTOMER_NAME) {
        anyhow::bail!("{guest_name}是系统默认客户，不能重复创建");
    }

    customer_repository::update_customer(conn, id, payload, &name)
}

pub fn disable_customer(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<bool> {
    if db::is_guest_customer(conn, id)? {
        let guest_name = db::guest_customer_name(conn)?;
        anyhow::bail!("{guest_name}是系统默认客户，不能删除");
    }
    customer_repository::disable_customer(conn, id)
}

pub fn batch_update_customers(
    conn: &rusqlite::Connection,
    payload: BatchUpdateCustomersRequest,
) -> anyhow::Result<BatchUpdateResultDto> {
    if payload.is_active == Some(false) {
        let guest_id = db::ensure_guest_customer(conn)?;
        if payload.ids.contains(&guest_id) {
            let guest_name = db::guest_customer_name(conn)?;
            anyhow::bail!("{guest_name}是系统默认客户，不能批量停用");
        }
    }
    customer_repository::batch_update_customers(conn, payload)
}

fn default_customer_filter() -> ListCustomersRequest {
    ListCustomersRequest {
        region: None,
        keyword: None,
        is_active: Some(true),
    }
}
