use crate::db;
use crate::models::{SaveSettingsRequest, SettingDto};

pub fn list_settings(conn: &rusqlite::Connection) -> anyhow::Result<Vec<SettingDto>> {
    let mut stmt = conn.prepare("SELECT key, COALESCE(value, '') FROM settings ORDER BY key")?;
    let rows = stmt.query_map([], |row| {
        Ok(SettingDto {
            key: row.get(0)?,
            value: row.get(1)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn save_settings(
    conn: &rusqlite::Connection,
    payload: SaveSettingsRequest,
) -> anyhow::Result<bool> {
    set_bool_if_some(conn, "daily_auto_backup", payload.daily_auto_backup)?;
    set_text_if_some(
        conn,
        "default_print_template",
        payload.default_print_template,
    )?;
    set_text_if_some(conn, "default_export_format", payload.default_export_format)?;
    set_text_if_some(conn, "default_printer", payload.default_printer)?;
    set_text_if_some(conn, "template_store_name", payload.template_store_name)?;
    set_text_if_some(conn, "template_footer_text", payload.template_footer_text)?;
    set_bool_if_some(conn, "template_show_barcode", payload.template_show_barcode)?;
    set_text_if_some(
        conn,
        "template_product_label",
        payload.template_product_label,
    )?;
    set_text_if_some(
        conn,
        "template_quantity_label",
        payload.template_quantity_label,
    )?;
    set_text_if_some(conn, "template_price_label", payload.template_price_label)?;
    set_text_if_some(conn, "template_amount_label", payload.template_amount_label)?;
    set_text_if_some(conn, "template_remark_label", payload.template_remark_label)?;
    set_text_if_some(conn, "template_orientation", payload.template_orientation)?;
    if let Some(value) = payload.template_margin {
        db::set_setting(conn, "template_margin", &value.to_string())?;
    }
    Ok(true)
}

fn set_text_if_some(
    conn: &rusqlite::Connection,
    key: &str,
    value: Option<String>,
) -> anyhow::Result<()> {
    if let Some(value) = value {
        db::set_setting(conn, key, &value)?;
    }
    Ok(())
}

fn set_bool_if_some(
    conn: &rusqlite::Connection,
    key: &str,
    value: Option<bool>,
) -> anyhow::Result<()> {
    if let Some(value) = value {
        db::set_setting(conn, key, if value { "true" } else { "false" })?;
    }
    Ok(())
}
