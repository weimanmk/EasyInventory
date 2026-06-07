use crate::models::{DocumentDto, DocumentFilterRequest, PrintStatusDto};
use crate::repositories::document_repository;
use anyhow::Context;
use std::process::Command;

pub fn list_documents(
    conn: &rusqlite::Connection,
    filter: DocumentFilterRequest,
) -> anyhow::Result<Vec<DocumentDto>> {
    document_repository::list_documents(conn, filter)
}

pub fn list_documents_with_default_filter(
    conn: &rusqlite::Connection,
    filter: Option<DocumentFilterRequest>,
) -> anyhow::Result<Vec<DocumentDto>> {
    list_documents(conn, filter.unwrap_or_else(default_document_filter))
}

pub fn open_document(conn: &rusqlite::Connection, document_id: i64) -> anyhow::Result<String> {
    let document = document_repository::document_by_id(conn, document_id)?;
    open::that(&document.file_path).with_context(|| "无法打开单据文件，请检查文件是否存在")?;
    Ok(document.file_path)
}

pub fn print_document(
    conn: &rusqlite::Connection,
    document_id: i64,
    printer_name: Option<String>,
) -> anyhow::Result<PrintStatusDto> {
    let document = document_repository::document_by_id(conn, document_id)?;
    let message = open_or_print_file(&document.file_path, printer_name.as_deref())?;
    document_repository::increment_print_count(conn, document_id, document.order_id)?;
    Ok(PrintStatusDto {
        file_path: document.file_path,
        printer_name,
        message,
    })
}

fn default_document_filter() -> DocumentFilterRequest {
    DocumentFilterRequest {
        customer_id: None,
        start_date: None,
        end_date: None,
        order_no: None,
        printed: None,
        status: None,
    }
}

fn open_or_print_file(file_path: &str, printer_name: Option<&str>) -> anyhow::Result<String> {
    if let Some(printer) = printer_name.filter(|value| !value.trim().is_empty()) {
        if cfg!(target_os = "windows") {
            let mut command = Command::new("powershell");
            command.args([
                "-NoProfile",
                "-Command",
                "Start-Process -FilePath $args[0] -Verb PrintTo -ArgumentList $args[1]",
                file_path,
                printer,
            ]);
            #[cfg(target_os = "windows")]
            use std::os::windows::process::CommandExt;
            #[cfg(target_os = "windows")]
            command.creation_flags(0x08000000);
            let result = command.output();
            if let Ok(output) = result {
                if output.status.success() {
                    return Ok(format!("已提交到打印机：{printer}"));
                }
            }
        }
        open::that(file_path).with_context(|| "无法打开单据文件，请检查文件是否存在")?;
        return Ok(format!(
            "无法直接提交到打印机：{printer}，已打开文件供手动打印"
        ));
    }

    open::that(file_path).with_context(|| "无法打开单据文件，请检查文件是否存在")?;
    Ok("已打开单据文件，请在关联程序中确认打印".to_string())
}
