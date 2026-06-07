use crate::models::{
    CustomerStatementDto, CustomerStatementRequest, CustomerStatementRowDto,
    CustomerStatementSummaryDto,
};
use crate::repositories::customer_statement_repository;
use crate::utils::{money, normalize_date};
use anyhow::anyhow;
use rusqlite::Connection;

pub fn customer_statement(
    conn: &Connection,
    request: CustomerStatementRequest,
) -> anyhow::Result<CustomerStatementDto> {
    if request.customer_id <= 0 {
        return Err(anyhow!("客户不合法"));
    }
    let start_date = normalize_date(&request.start_date);
    let end_date = normalize_date(&request.end_date);
    if start_date > end_date {
        return Err(anyhow!("对账开始日期不能晚于结束日期"));
    }

    let customer_name = customer_statement_repository::customer_name(conn, request.customer_id)?;
    let opening_payable =
        customer_statement_repository::opening_payable(conn, request.customer_id, &start_date)?;
    let opening_paid =
        customer_statement_repository::opening_paid(conn, request.customer_id, &start_date)?;
    let opening_balance = money(opening_payable - opening_paid);
    let period_discount_amount = customer_statement_repository::period_discount_amount(
        conn,
        request.customer_id,
        &start_date,
        &end_date,
    )?;
    let items = customer_statement_repository::ledger_rows(
        conn,
        request.customer_id,
        &start_date,
        &end_date,
    )?;

    let mut balance = opening_balance;
    let mut period_payable = 0.0;
    let mut period_paid = 0.0;
    let rows = items
        .into_iter()
        .map(|item| {
            period_payable = money(period_payable + item.debit_amount);
            period_paid = money(period_paid + item.credit_amount);
            balance = money(balance + item.debit_amount - item.credit_amount);
            CustomerStatementRowDto {
                record_date: item.record_date,
                record_type: item.record_type,
                record_no: item.record_no,
                description: item.description,
                debit_amount: money(item.debit_amount),
                credit_amount: money(item.credit_amount),
                balance_after: balance,
                remark: item.remark,
            }
        })
        .collect::<Vec<_>>();

    Ok(CustomerStatementDto {
        summary: CustomerStatementSummaryDto {
            customer_id: request.customer_id,
            customer_name,
            start_date,
            end_date,
            opening_balance,
            period_payable,
            period_paid,
            period_discount_amount: money(period_discount_amount),
            closing_balance: balance,
        },
        rows,
    })
}
