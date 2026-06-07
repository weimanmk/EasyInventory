use crate::models::{
    CreatePaymentRequest, CustomerBalanceDto, CustomerBalanceFilterRequest, PaymentFilterRequest,
    PaymentRecordDto,
};
use crate::repositories::customer_account_repository;

pub fn list_customer_balances(
    conn: &rusqlite::Connection,
    filter: Option<CustomerBalanceFilterRequest>,
) -> anyhow::Result<Vec<CustomerBalanceDto>> {
    customer_account_repository::list_customer_balances(
        conn,
        filter.unwrap_or_else(default_customer_balance_filter),
    )
}

pub fn list_payment_records(
    conn: &rusqlite::Connection,
    filter: Option<PaymentFilterRequest>,
) -> anyhow::Result<Vec<PaymentRecordDto>> {
    customer_account_repository::list_payment_records(
        conn,
        filter.unwrap_or_else(default_payment_filter),
    )
}

pub fn create_payment(
    conn: &rusqlite::Connection,
    payload: CreatePaymentRequest,
) -> anyhow::Result<PaymentRecordDto> {
    if payload.customer_id <= 0 || payload.amount <= 0.0 {
        anyhow::bail!("收款客户和金额不合法");
    }
    if !customer_account_repository::active_customer_exists(conn, payload.customer_id)? {
        anyhow::bail!("客户不存在或已停用");
    }
    if let Some(order_id) = payload.related_order_id {
        let valid_order = customer_account_repository::normal_order_belongs_to_customer(
            conn,
            order_id,
            payload.customer_id,
        )?;
        if !valid_order {
            anyhow::bail!("关联订单不存在或不属于该客户");
        }
    }
    customer_account_repository::create_payment(conn, payload)
}

pub fn void_payment(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<PaymentRecordDto> {
    customer_account_repository::void_payment(conn, id)
}

fn default_customer_balance_filter() -> CustomerBalanceFilterRequest {
    CustomerBalanceFilterRequest {
        region: None,
        keyword: None,
        only_unpaid: None,
    }
}

fn default_payment_filter() -> PaymentFilterRequest {
    PaymentFilterRequest {
        customer_id: None,
        start_date: None,
        end_date: None,
        status: Some("normal".to_string()),
    }
}
