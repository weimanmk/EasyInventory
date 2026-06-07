use crate::models::{
    BatchUpdateProductsRequest, BatchUpdateResultDto, ListProductsRequest, ProductDto,
    ProductPayload,
};
use crate::repositories::product_repository;

pub fn list_products(
    conn: &rusqlite::Connection,
    filter: Option<ListProductsRequest>,
) -> anyhow::Result<Vec<ProductDto>> {
    product_repository::list_products(conn, filter.unwrap_or_else(default_product_filter))
}

pub fn create_product(
    conn: &rusqlite::Connection,
    payload: ProductPayload,
) -> anyhow::Result<ProductDto> {
    if payload.name.trim().is_empty() || payload.category.trim().is_empty() {
        anyhow::bail!("商品名称和类别必填");
    }
    product_repository::create_product(conn, payload)
}

pub fn update_product(
    conn: &rusqlite::Connection,
    id: i64,
    payload: ProductPayload,
) -> anyhow::Result<ProductDto> {
    product_repository::update_product(conn, id, payload)
}

pub fn disable_product(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<bool> {
    product_repository::disable_product(conn, id)
}

pub fn batch_update_products(
    conn: &rusqlite::Connection,
    payload: BatchUpdateProductsRequest,
) -> anyhow::Result<BatchUpdateResultDto> {
    product_repository::batch_update_products(conn, payload)
}

pub fn find_product_by_barcode(
    conn: &rusqlite::Connection,
    barcode: &str,
) -> anyhow::Result<Option<ProductDto>> {
    let barcode = barcode.trim();
    if barcode.is_empty() {
        return Ok(None);
    }
    product_repository::find_by_barcode(conn, barcode)
}

fn default_product_filter() -> ListProductsRequest {
    ListProductsRequest {
        category: None,
        keyword: None,
        only_low_stock: None,
        only_in_stock: None,
        is_active: Some(true),
    }
}
