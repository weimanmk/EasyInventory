use super::{fail, ok};
use crate::app::AppState;
use crate::logger;
use crate::models::*;
use crate::services::{customer_service, product_service, supplier_service};
use tauri::State;

#[tauri::command]
pub fn list_products(
    state: State<AppState>,
    filter: Option<ListProductsRequest>,
) -> ApiResponse<Vec<ProductDto>> {
    let result = (|| {
        let conn = state.connection()?;
        logger::info("product", format!("list_products filter={filter:?}"));
        product_service::list_products(&conn, filter)
    })();
    if let Ok(items) = &result {
        logger::info(
            "product",
            format!("list_products result count={}", items.len()),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_product(state: State<AppState>, payload: ProductPayload) -> ApiResponse<ProductDto> {
    logger::info("product", format!("create_product payload={payload:?}"));
    let result = (|| {
        let conn = state.connection()?;
        product_service::create_product(&conn, payload)
    })();
    if let Ok(product) = &result {
        logger::info(
            "product",
            format!(
                "create_product success id={} name={} category={} barcode={:?}",
                product.id, product.name, product.category, product.barcode
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn update_product(
    state: State<AppState>,
    id: i64,
    payload: ProductPayload,
) -> ApiResponse<ProductDto> {
    logger::info(
        "product",
        format!("update_product id={id} payload={payload:?}"),
    );
    let result = (|| {
        let conn = state.connection()?;
        product_service::update_product(&conn, id, payload)
    })();
    if let Ok(product) = &result {
        logger::info(
            "product",
            format!(
                "update_product success id={} name={} category={} barcode={:?}",
                product.id, product.name, product.category, product.barcode
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn disable_product(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        product_service::disable_product(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn batch_update_products(
    state: State<AppState>,
    payload: BatchUpdateProductsRequest,
) -> ApiResponse<BatchUpdateResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        product_service::batch_update_products(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn find_product_by_barcode(
    state: State<AppState>,
    barcode: String,
) -> ApiResponse<Option<ProductDto>> {
    let result = (|| {
        let conn = state.connection()?;
        product_service::find_product_by_barcode(&conn, &barcode)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_customers(
    state: State<AppState>,
    filter: Option<ListCustomersRequest>,
) -> ApiResponse<Vec<CustomerDto>> {
    let result = (|| {
        let conn = state.connection()?;
        logger::info("customer", format!("list_customers filter={filter:?}"));
        customer_service::list_customers(&conn, filter)
    })();
    if let Ok(items) = &result {
        logger::info(
            "customer",
            format!("list_customers result count={}", items.len()),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_customer(
    state: State<AppState>,
    payload: CustomerPayload,
) -> ApiResponse<CustomerDto> {
    logger::info("customer", format!("create_customer payload={payload:?}"));
    let result = (|| {
        let conn = state.connection()?;
        customer_service::create_customer(&conn, payload)
    })();
    if let Ok(customer) = &result {
        logger::info(
            "customer",
            format!(
                "create_customer success id={} name={} region={:?}",
                customer.id, customer.name, customer.region
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn update_customer(
    state: State<AppState>,
    id: i64,
    payload: CustomerPayload,
) -> ApiResponse<CustomerDto> {
    logger::info(
        "customer",
        format!("update_customer id={id} payload={payload:?}"),
    );
    let result = (|| {
        let conn = state.connection()?;
        customer_service::update_customer(&conn, id, payload)
    })();
    if let Ok(customer) = &result {
        logger::info(
            "customer",
            format!(
                "update_customer success id={} name={} region={:?}",
                customer.id, customer.name, customer.region
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn disable_customer(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        customer_service::disable_customer(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn batch_update_customers(
    state: State<AppState>,
    payload: BatchUpdateCustomersRequest,
) -> ApiResponse<BatchUpdateResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        customer_service::batch_update_customers(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_suppliers(
    state: State<AppState>,
    filter: Option<ListSuppliersRequest>,
) -> ApiResponse<Vec<SupplierDto>> {
    let result = (|| {
        let conn = state.connection()?;
        supplier_service::list_suppliers(&conn, filter)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_supplier(
    state: State<AppState>,
    payload: SupplierPayload,
) -> ApiResponse<SupplierDto> {
    let result = (|| {
        let conn = state.connection()?;
        supplier_service::create_supplier(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn update_supplier(
    state: State<AppState>,
    id: i64,
    payload: SupplierPayload,
) -> ApiResponse<SupplierDto> {
    let result = (|| {
        let conn = state.connection()?;
        supplier_service::update_supplier(&conn, id, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn disable_supplier(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        supplier_service::disable_supplier(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn batch_update_suppliers(
    state: State<AppState>,
    payload: BatchUpdateSuppliersRequest,
) -> ApiResponse<BatchUpdateResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        supplier_service::batch_update_suppliers(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}
